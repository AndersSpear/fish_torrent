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
    // TODO: trackerid ??
    peer_id: [u8; 20],
    port: u16,
    uploaded: usize,
    downloaded: usize,
    left: usize,
}

/// main handles the initialization of stuff and keeping the event loop logic going
fn main() {
    // you'll never guess what this line does
    let args = Args::parse();

    // things i own :)
    let mut self_info = SelfInfo {
        peer_id: [0; 20],
        port: args.port,
        uploaded: 0,
        downloaded: 0,
        left: 0,
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

    // let mut output_file = OutputFile::new(get_file_name(), get_number_of_pieces(), get_piece_length());

    // parse that url and open the initial socket
    // this blocks because wth are you gonna do while you wait for a response
    println!("{}", get_tracker_url());
    let mut tracker_sock = TcpStream::from_std(
        std::net::TcpStream::connect(
            *Url::parse(get_tracker_url())
                .unwrap()
                .socket_addrs(|| None)
                .unwrap()
                .first()
                .unwrap(),
        )
        .expect("connect failed"),
    );
    const TRACKER: Token = Token(1);

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
            // strategy::what_do(&mut peer_list);
            // p2p::send_all(&mut peer_list);
            strategy_timer = Instant::now();
        }

        // timer for blasting tracker
        if tracker_timer.elapsed() > tracker_timeout {
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
            dbg!("event llopp");
            match event.token() {
                SERVER => {
                    if let Ok((socket, peer_addr)) = serv_sock.accept() {
                        println!("new client: {peer_addr:?}");

                        // add peer !!! !!
                        if let Ok(socket) = peer_list.add_incomplete_peer(peer_addr, socket) {
                            let token = get_new_token();
                            poll.registry()
                                .register(socket, token, Interest::READABLE)
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
                    // is it a readable ?? (receive blasted message)
                    if event.is_readable() {
                        dbg!("readable");
                        let (data, response) = tracker::handle_tracker_response(partial_tracker_data, &mut tracker_sock);
                        dbg!(&data);
                        dbg!(&response);
                        partial_tracker_data = data;

                        match response {
                            Some(tracker_response) => {
                                // yay we got a full response, time to do things :)
                                tracker_timeout = Duration::new(tracker_response.interval, 0);
                                add_all_peers(&mut poll, &mut peer_list, &mut sockets, tracker_response);

                                poll.registry()
                                    .deregister(&mut tracker_sock)
                                    .expect("tracker deregister fail");
                            }
                            None => {
                                // cringe !!! 727 WYSI !!! (it was 7:27 at the time of writing this code) 
                            }
                        }

                    }
                    // is it a writable ?? (blast message out)
                    else if event.is_writable() {
                        let tracker_request = TrackerRequest::new(
                            "aaaaaaaaaaaaaaaaaaaa",
                            self_info.peer_id,
                            self_info.port,
                            self_info.uploaded,
                            self_info.downloaded,
                            self_info.left,
                        );
                        send_tracker_request(&tracker_request, &mut tracker_sock).unwrap();
                        poll.registry()
                            .reregister(&mut tracker_sock, TRACKER, Interest::READABLE)
                            .expect("tracker rereg fail");
                    }
                }
                token => {
                    if let Some(socket) = sockets.get(&token) {
                        handle_peer(sockets.get(&token).unwrap());
                    } else {
                        println!("there is no socket associated with token {:#?}", token);
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
        if let Ok(mut socket) = TcpStream::connect(peer_addr) {
            // add peer ?? ?? ?? (is it there already !! ??)
            if let Ok(socket) = peer_list.add_incomplete_peer(peer_addr, socket) {
                let token = get_new_token();
                poll.registry()
                    .register(socket, token, Interest::READABLE)
                    .expect(&format!("failed to register peer {:?}", peer_addr));
                sockets.insert(token, peer_addr);
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
fn handle_peer(peer_addr: &SocketAddr) {
    // handle_message(socket);
    // match () {
    //     Choke => {

    //     }
    //     Unchoke => {

    //     }
    //     Interested => {

    //     }
    //     NotInterested => {

    //     }
    //     Have(index) => {
    //     }
    //     Bitfield(field) => {
    //     }
    //     Request(index, begin, length) => {
    //     }
    //     Piece(index, begin, block) => {
    //     }
    //     Cancel(index, begin, length) => {
    //     }
    //     KeepAlive => {

    //     }
    //     Undefined => {

    //     }
    //     HandshakeResponse => {

    //     }
    // }
}
