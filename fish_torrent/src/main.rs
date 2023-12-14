#![feature(lazy_cell)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]
#![warn(missing_docs)]
//! handles initial call to tracker and peer
//! handles epoll event loop
//! triggers peer tracker, p2p, strategy, on a timer

mod file;
mod p2p;
mod peers;
mod strategy;
mod torrent;
mod tracker;

use clap::Parser;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use peers::Peer;
use std::collections::HashMap;
use std::net::{self, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};
use url::Url;

use crate::file::OutputFile;
use crate::p2p::MessageType;
use crate::peers::Peers;
use crate::torrent::*;
use crate::tracker::*;

const STRATEGY_TIMEOUT: Duration = Duration::new(0, 100000000); // 100 milliseconds

// Takes in the port and torrent file
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Port to bind this client to
    #[arg(short, long, default_value_t = 4170)]
    port: u16,

    /// Torrent file you wish to parse
    #[arg(short, long)]
    file: String,
}

struct SelfInfo {
    tracker_id: [u8; 20],
    peer_id: [u8; 20],
    port: u16,
    uploaded: usize,
    downloaded: usize,
    left: usize,
    tracker_event: Event,
}

/// main handles the initialization of stuff and keeping the event loop logic going
fn main() {
    // you'll never guess what this line does
    let args = Args::parse();

    // things i own :)
    let mut self_info = SelfInfo {
        tracker_id: [0; 20],
        peer_id: [0; 20], // TODO make peerid not zero :D
        port: args.port,
        uploaded: 0,
        downloaded: 0,
        left: 0,
        tracker_event: Event::STARTED,
    };
    let mut peer_list = Peers::new();
    let mut sockets: HashMap<Token, SocketAddr> = HashMap::new();

    // binds to INADDR_ANY
    let mut serv_sock = TcpListener::bind(net::SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        args.port,
    )))
    .expect("bind failed");
    const SERVER: Token = Token(0);

    // creates the events and poll instance used by the event loop
    let mut events = Events::with_capacity(727);
    let mut poll = Poll::new().expect("poll failed");

    // registers our listening socket in the epoll
    poll.registry()
        .register(&mut serv_sock, SERVER, Interest::READABLE)
        .expect("serv register failed");

    // read in torrent file:W
    parse_torrent_file(&args.file);

    let mut output_file = OutputFile::new(
        get_file_name(),
        get_number_of_pieces().try_into().unwrap(),
        get_piece_length().try_into().unwrap(),
    )
    .unwrap();

    // parse that url and open the initial socket
    // this blocks because wth are you gonna do while you wait for a response
    println!("{}", get_tracker_url());
    // let mut tracker_sock = TcpStream::from_std(
    //     std::net::TcpStream::connect(
    //         *Url::parse(get_tracker_url())
    //             .unwrap()
    //             .socket_addrs(|| None)
    //             .unwrap()
    //             .first()
    //             .unwrap(),
    //     )
    //     .expect("connect failed"),
    // );
    let mut tracker_sock = TcpStream::connect(
        *Url::parse(get_tracker_url())
            .unwrap()
            .socket_addrs(|| None)
            .unwrap()
            .first()
            .unwrap(),
    )
    .expect("connect failed");
    const TRACKER: Token = Token(1);
    dbg!(&tracker_sock);

    // let mut tracker_sock2 = TcpStream::from_std(
    //     std::net::TcpStream::connect("128.8.126.63:6969").expect("connect failed"),
    // );

    // println!(
    //     "addr {:?}\nuh: {:?}",
    //     tracker_sock2.peer_addr(),
    //     Url::parse("http://128.8.126.63:6969/announce")
    //         .unwrap()
    //         .socket_addrs(|| None)
    //         .unwrap()
    //         .first()
    //         .unwrap()
    // );
    poll.registry()
        .register(&mut tracker_sock, TRACKER, Interest::WRITABLE)
        .expect("tracker register failed");

    // set up the initial timers
    let mut strategy_timer = Instant::now();
    let mut tracker_timer = Instant::now();
    let mut tracker_timeout: Duration = Duration::new(std::u64::MAX, 0);

    // holds the partially read data from a tracker response
    let mut partial_tracker_data = Vec::new();

    loop {
        // timer for blasting send
        if strategy_timer.elapsed() > STRATEGY_TIMEOUT {
            // dbg!("strategy timeout occurred !!");
            // strategy::what_do(&mut peer_list);
            // p2p::send_all(&mut peer_list);
            strategy_timer = Instant::now();
        }

        // timer for blasting tracker
        if tracker_timer.elapsed() > tracker_timeout {
            dbg!("tracker timeout occurred !!");

            // we simply reconnect to the tracker
            tracker_sock = TcpStream::connect(
                *Url::parse(get_tracker_url())
                    .unwrap()
                    .socket_addrs(|| None)
                    .unwrap()
                    .first()
                    .unwrap(),
            )
            .expect("connect failed");
            dbg!(&tracker_sock);

            // registers our tracker socket in the epoll
            poll.registry()
                .register(&mut tracker_sock, TRACKER, Interest::WRITABLE)
                .expect("tracker register failed");
            tracker_timer = Instant::now();
        }

        // calculate how much time is remaining for each of the timers
        let strategy_remaining = STRATEGY_TIMEOUT - strategy_timer.elapsed();
        let tracker_remaining = tracker_timeout - tracker_timer.elapsed();
        poll.poll(&mut events, Some(strategy_remaining.min(tracker_remaining)))
            .expect("poll_wait failed");

        // who did something
        for event in &events {
            dbg!(format!("event llopp with event {:?}", event));
            match event.token() {
                SERVER => {
                    dbg!("server socket activity");
                    if let Ok((socket, peer_addr)) = serv_sock.accept() {
                        println!("new client: {peer_addr:?}");

                        // add peer !!! !!
                        if let Ok(peer) = peer_list.add_peer(peer_addr, socket, None) {
                            let token = get_new_token();
                            poll.registry()
                                .register(
                                    peer.get_mut_socket(),
                                    token,
                                    Interest::WRITABLE | Interest::READABLE,
                                )
                                .expect(&format!("failed to register peer {:?}", peer_addr));
                            sockets.insert(token, peer_addr);
                        } else {
                            println!("already accepted peer {:?}", peer_addr);
                        }
                    } else {
                        println!("couldn't accept the client");
                    }
                }
                TRACKER => {
                    dbg!("tracker socket activity");
                    // is it a readable ?? (receive blasted message)

                    if event.is_readable() {
                        dbg!("tracker readable");
                        let (data, response) = tracker::handle_tracker_response(
                            partial_tracker_data,
                            &mut tracker_sock,
                        );
                        dbg!(&data);
                        dbg!(&response);
                        partial_tracker_data = data;

                        match response {
                            Some(tracker_response) => {
                                // yay we got a full response, time to do things :)
                                tracker_timeout =
                                    Duration::new(tracker_response.interval.try_into().unwrap(), 0);
                                // Duration::new(10, 0);

                                // if let Some(tracker_id) = tracker_response.tracker_id {
                                //     self_info.tracker_id = tracker_response.tracker_id;
                                // }

                                add_all_peers(
                                    &mut poll,
                                    &mut peer_list,
                                    &mut sockets,
                                    tracker_response,
                                );

                                poll.registry()
                                    .deregister(&mut tracker_sock)
                                    .expect("tracker deregister fail");

                                tracker_sock
                                    .shutdown(net::Shutdown::Both)
                                    .expect("tracker was not shutdown :(");
                                dbg!("tracker shutdown!!");
                            }
                            None => {
                                // cringe !!! 727 WYSI !!! (it was 7:27 at the time of writing this code)
                            }
                        }
                    }
                    // is it a writable ?? (blast message out)
                    else if event.is_writable() {
                        dbg!("tracker writable");
                        dbg!(get_info_hash());
                        dbg!(tracker::bytes_to_urlencoding(&get_info_hash()));
                        let tracker_request = TrackerRequest::new(
                            get_info_hash(),
                            &self_info.peer_id.to_vec(),
                            self_info.port,
                            self_info.uploaded,
                            self_info.downloaded,
                            self_info.left,
                            self_info.tracker_event,
                            Url::parse(get_tracker_url()).unwrap().host_str().unwrap().to_string(),
                        );
                        send_tracker_request(&tracker_request, &mut tracker_sock).unwrap();

                        if self_info.tracker_event == Event::STARTED {
                            self_info.tracker_event = Event::PERIODIC;
                        }

                        poll.registry()
                            .reregister(&mut tracker_sock, TRACKER, Interest::READABLE)
                            .expect("tracker rereg fail");
                    }
                }
                token => {
                    dbg!("peer socket activity with token {:?}", token);
                    // something went wrong so uh idc about you anymore
                    if event.is_error() || event.is_read_closed() {
                        let peer_addr = sockets.remove(&token).unwrap();
                        let peer = peer_list.remove_peer(peer_addr).unwrap();
                        // you cant shutdown a non connected socket (as we have figured out very quickly)
                        if !event.is_error() {
                            peer.disconnect().expect("failed to disconnect peer");
                        }
                        continue;
                    }

                    // lets see what they said to us
                    if let Some(&peer_addr) = sockets.get(&token) {
                        // have we completed the peer yet?
                        let peer = peer_list.find_peer(peer_addr).unwrap();
                        if peer.is_complete() {
                            handle_peer(peer, &mut output_file);
                        }
                        // no we havent
                        else {
                            if event.is_readable() {
                                dbg!("Got a handshake from peer");
                                let peer_id = p2p::recv_handshake(peer.get_mut_socket()).unwrap();
                                peer_list
                                    .complete_peer(peer_addr, &peer_id.try_into().unwrap())
                                    .unwrap();
                            } else if event.is_writable() {
                                dbg!("Sent a handshake to peer");
                                p2p::send_handshake(peer, &self_info.peer_id, &output_file)
                                    .expect("failed to send handshake");
                                // only care about readable events from now on
                                poll.registry()
                                    .reregister(peer.get_mut_socket(), token, Interest::READABLE)
                                    .expect("peer rereg fail");
                            }
                        }
                    } else {
                        println!("there is no socket associated with token {:?}", token);
                    }
                }
            }
        }
    }
}

/// handles the response the tracker got, aka creates new peers for all the peers it received if needed
fn add_all_peers(
    poll: &mut Poll,
    peer_list: &mut Peers,
    sockets: &mut HashMap<Token, SocketAddr>,
    tracker_response: TrackerResponse,
) {
    // each peer is a SockAddr initially
    for peer_addr in tracker_response.socket_addr_list {
        if let Ok(socket) = TcpStream::connect(peer_addr) {
            // add peer ?? ?? ?? (is it there already !! ??)
            if let Ok(peer) = peer_list.add_peer(peer_addr, socket, None) {
                let token = get_new_token();
                poll.registry()
                    .register(
                        peer.get_mut_socket(),
                        token,
                        Interest::WRITABLE | Interest::READABLE,
                    )
                    .expect(&format!("failed to register peer {:?}", peer_addr));
                sockets.insert(token, peer_addr);
                dbg!(format!(
                    "syn'd and added peer {:?} with token {:?}",
                    peer_addr, token
                ));
            } else {
                println!("already connected to peer {:?}", peer_addr);
            }
        } else {
            println!("failed to connect to peer {:?}", peer_addr);
        }
    }
}

fn get_new_token() -> Token {
    static TOKEN_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(2);
    Token(TOKEN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

/// handles the message it got from a peer
fn handle_peer(peer: &mut Peer, output_file: &mut OutputFile) {
    dbg!(format!("handling peer {:?}", peer));
    p2p::handle_messages(peer).expect("failed to read message"); // TOOD: shout at anders this funtion doesnt work properly (read_to_end)
                                                                 // TODO: should remove peer if error reading? (question mark?)
    let messages = peer.messages.messages.clone();

    for msg in messages {
        dbg!(format!("message is {:?}", msg));
        match msg {
            MessageType::Choke => {
                peer.peer_choking = true;
            }
            MessageType::Unchoke => {
                peer.peer_choking = false;
            }
            MessageType::Interested => {
                peer.peer_interested = true;
            }
            MessageType::NotInterested => {
                peer.peer_interested = false;
            }
            MessageType::Have { index } => {
                peer.set_piece_bit(index.try_into().unwrap(), true);
            }
            MessageType::Bitfield { field } => {
                peer.init_piece_bitfield(field);
            }
            MessageType::Request {
                index,
                begin,
                length,
            } => {
                // check if we are choking them? or do we just send?
                peer.push_request(
                    index.try_into().unwrap(),
                    begin.try_into().unwrap(),
                    length.try_into().unwrap(),
                );
            }
            MessageType::Piece {
                index,
                begin,
                block,
            } => {
                output_file
                    .write_block(index.try_into().unwrap(), begin.try_into().unwrap(), block)
                    .expect("failed to write block");
            }
            MessageType::Cancel {
                index,
                begin,
                length,
            } => {
                // remove them as being interested ??
                // peer.remove_request(index, begin, length);
            }
            MessageType::KeepAlive => {
                // uh i dont htink we do anything hrere yet
                // reset a peer timeout if we ever implement that
            } // MessageType::Undefined => {
              //     // does this still exist questio mark
              // }
              // MessageType::HandshakeResponse => {
              //     // do i care?
              // }
        }
    }
}
