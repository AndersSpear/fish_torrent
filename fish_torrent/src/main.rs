#![feature(lazy_cell)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
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
use rand::distributions::{Alphanumeric, DistString};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::net::{self, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};
use url::Url;

use crate::file::OutputFile;
use crate::p2p::MessageType;
use crate::peers::Peers;
use crate::strategy::{Strategy, Update};
use crate::torrent::*;
use crate::tracker::*;

const STRATEGY_TIMEOUT: Duration = Duration::new(0, 500000000); // 100 milliseconds TODO: change back to .1 sec
const KEEPALIVE_TIMEOUT: Duration = Duration::new(10, 0); // 2 minutes because T H E S P E C
const CLEAR_REQUESTS_TIMEOUT: Duration = Duration::new(30, 0); // 2 minutes because T H E S P E C

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

#[derive(PartialOrd, PartialEq)]
struct Timer {
    timeout: Duration,
    instant: Instant,
}

impl Timer {
    pub fn timeout_huh(&self) -> bool {
        self.instant.elapsed() > self.timeout
    }

    pub fn update_instant(&mut self) -> () {
        self.instant = Instant::now();
    }
}

struct Timers {
    strategy: Timer,
    tracker: Timer,
    keepalive: Timer,
    clear_requests: Timer,
    // if another timer gets added, remember to change min_remaining too
}

impl Timers {
    pub fn min_remaining(&self) -> Duration {
        let mut all_timers = Vec::new();
        all_timers.push(&self.strategy);
        all_timers.push(&self.tracker);
        all_timers.push(&self.keepalive);
        all_timers.push(&self.clear_requests);

        let mut min_remaining: Duration = Duration::new(std::u64::MAX, 0);
        for timer in all_timers {
            let remaining = timer.timeout - timer.instant.elapsed();
            if remaining < min_remaining {
                min_remaining = remaining;
            }
        }

        min_remaining
    }
}

/// main handles the initialization of stuff and keeping the event loop logic going
fn main() {
    // you'll never guess what this line does
    let args = Args::parse();

    // Initialize structs and data.
    let mut self_info = SelfInfo {
        tracker_id: [0; 20],
        peer_id: create_peer_id(),
        port: args.port,
        uploaded: 0,
        downloaded: 0,
        left: 0,
        tracker_event: Event::STARTED,
    };
    let mut peer_list = Peers::new();
    let mut sockets: HashMap<Token, SocketAddr> = HashMap::new();

    // Create a server listening socket that is bound to INADDRANY.
    let mut serv_sock = TcpListener::bind(net::SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::UNSPECIFIED,
        args.port,
    )))
    .expect("bind failed");
    const SERVER: Token = Token(0); // Poll token associated with listening socket.

    // Creates the events and poll instance used by the event loop.
    let mut events = Events::with_capacity(727);
    let mut poll = Poll::new().expect("poll failed");

    // Register listening socket in poll instance.
    poll.registry()
        .register(&mut serv_sock, SERVER, Interest::READABLE)
        .expect("serv register failed");

    // Read in torrent file.
    parse_torrent_file(&args.file);

    // Initialize output file using info from torrent file.
    let mut output_file = OutputFile::new(
        get_file_name(),
        get_file_length().try_into().unwrap(),
        get_number_of_pieces().try_into().unwrap(),
        get_piece_length().try_into().unwrap(),
        file::BLOCK_SIZE,
    )
    .unwrap();

    // Initialize strategy state.
    let mut strategy_state = Strategy::new(get_number_of_pieces().try_into().unwrap(), 5); // TODO make not 5

    // Holds the partially read data from a tracker response.
    let mut partial_tracker_data = Vec::new();

    // Parse URL from tracker and open the initial socket.
    // NOTE: Like other sockets, this is non-blocking.
    dbg!(get_tracker_url());
    let mut tracker_sock = TcpStream::connect(
        *Url::parse(get_tracker_url())
            .unwrap()
            .socket_addrs(|| None)
            .unwrap()
            .first()
            .unwrap(),
    )
    .expect("connect failed");
    const TRACKER: Token = Token(1); // Poll token associated with tracker socket.

    // Register tracker socket in poll instance.
    poll.registry()
        .register(&mut tracker_sock, TRACKER, Interest::WRITABLE)
        .expect("tracker register failed");

    // Set up the initial timers.
    // this struct just holds all the timers which we are keeping track of
    let mut timers = Timers {
        strategy: Timer {
            timeout: STRATEGY_TIMEOUT,
            instant: Instant::now(),
        },
        tracker: Timer {
            timeout: Duration::new(std::u64::MAX, 0),
            instant: Instant::now(),
        },
        keepalive: Timer {
            timeout: KEEPALIVE_TIMEOUT,
            instant: Instant::now(),
        },
        clear_requests: Timer {
            timeout: CLEAR_REQUESTS_TIMEOUT,
            instant: Instant::now(),
        },
    };

    loop {
        // check if you have downloaded the file
        if output_file.is_file_finished() {
            println!("🦀🦀🦀🦀 You have downloaded {} successfully!! Congrats!!! 🦀🦀🦀🦀", get_file_name());
        }
        // should we send a keepalive?
        if timers.keepalive.timeout_huh() {
            println!(" === KeepAlive Timeout === ");
            strategy_state.push_update(None, MessageType::KeepAlive);
            timers.keepalive.update_instant();
        }

        // should we ask our peers that we've already requested again?
        if timers.clear_requests.timeout_huh() {
            println!(" === Clear Requests Timeout === ");
            strategy_state.rm_all_requests();
            timers.clear_requests.update_instant();
        }

        // Strategy timeout.
        if timers.strategy.timeout_huh() {
            println!(" === Strategy Timeout === ");

            strategy_state.what_do(&mut peer_list, &mut output_file);
            p2p::send_all(&mut peer_list).expect("failed to send all");

            timers.strategy.update_instant();
        }

        // Tracker request timeout.
        if timers.tracker.timeout_huh() {
            println!(" === Tracker Timeout === ");

            // Create a new HTTP connection to the tracker.
            tracker_sock = TcpStream::connect(
                *Url::parse(get_tracker_url())
                    .unwrap()
                    .socket_addrs(|| None)
                    .unwrap()
                    .first()
                    .unwrap(),
            )
            .expect("connect failed");

            // Registers writing to tracker socket in poll instance.
            poll.registry()
                .register(&mut tracker_sock, TRACKER, Interest::WRITABLE)
                .expect("tracker register failed");

            // Reset tracker timer.
            timers.tracker.update_instant();
        }

        println!(" === Polling... === ");
        // Calculate time remaining for each of the timers.
        poll.poll(&mut events, Some(timers.min_remaining()))
            .expect("poll_wait failed");

        // For every event...
        for event in &events {
            println!("Event llopp with event {:?}", event);
            // See if the event is associated with listening socket, tracker, or peers.
            match event.token() {
                SERVER => {
                    println!("- Server socket activity -");
                    if let Ok((socket, peer_addr)) = serv_sock.accept() {
                        println!("- New client accepted {:?} -", &peer_addr);

                        // Add the peer to the Peers struct.
                        if let Ok(peer) = peer_list.add_peer(peer_addr, socket, None) {
                            // Register this peer for reading and writing in poll instance.
                            // This is to allow reading and writing the handshake messages.
                            let token = get_new_token();
                            poll.registry()
                                .register(
                                    peer.get_mut_socket(),
                                    token,
                                    Interest::WRITABLE | Interest::READABLE,
                                )
                                .expect(&format!("failed to register peer {:?}", peer_addr));
                            // Save the mapping from token to peer's addr.
                            sockets.insert(token, peer_addr);
                        } else {
                            println!("already accepted peer {:?}", peer_addr);
                        }
                    } else {
                        println!("couldn't accept the client");
                        println!("- Failed to accept the peer -");
                    }
                }
                TRACKER => {
                    println!("- Tracker socket activity -");
                    // If readable, receive response from tracker.
                    if event.is_readable() {
                        println!("- Tracker read event -");
                        let (data, response) = tracker::handle_tracker_response(
                            partial_tracker_data,
                            &mut tracker_sock,
                        );
                        partial_tracker_data = data; // Keep the buffer for the next (partial) read.

                        if let Some(tracker_response) = response {
                            println!("- Tracker response parsed {:?} -", &tracker_response);
                            // Reset the tracker timer based on the interval received from tracker.
                            timers.tracker.timeout = Duration::new(tracker_response.interval.try_into().unwrap(), 0);

                            // Add all peers received from tracker to peer_list and register them
                            // with the poll instance.
                            add_all_peers(
                                &mut poll,
                                &mut peer_list,
                                &mut sockets,
                                tracker_response,
                            );

                            // Deregister tracker socket, as response means connection is no longer needed.
                            poll.registry()
                                .deregister(&mut tracker_sock)
                                .expect("tracker deregister fail");

                            // Shut down the tracker for reasons mentioned above.
                            tracker_sock
                                .shutdown(net::Shutdown::Both)
                                .expect("tracker was not shutdown :(");
                            println!("- Tracker connection closed -");
                        } else {
                            println!("- Tracker partial read -");
                        }
                    }
                    // If writable, send request to tracker.
                    else if event.is_writable() {
                        println!("- Tracker write event -");
                        // Create and send a tracker request.
                        let tracker_request = TrackerRequest::new(
                            get_info_hash(),
                            &self_info.peer_id.to_vec(),
                            self_info.port,
                            self_info.uploaded,
                            self_info.downloaded,
                            self_info.left,
                            self_info.tracker_event,
                            Url::parse(get_tracker_url())
                                .unwrap()
                                .host_str()
                                .unwrap()
                                .to_string(),
                        );
                        send_tracker_request(&tracker_request, &mut tracker_sock).unwrap();

                        // If this is the first request, change tracker event from
                        // started to periodic.
                        if self_info.tracker_event == Event::STARTED {
                            self_info.tracker_event = Event::PERIODIC;
                        }

                        // Register the socket for reading response.
                        poll.registry()
                            .reregister(&mut tracker_sock, TRACKER, Interest::READABLE)
                            .expect("tracker rereg fail");
                        println!("- Tracker request sent {:?}-", tracker_request);
                    }
                }
                token => {
                    println!("- Peer socket activity with {:?} -", &token);
                    // something went wrong so uh idc about you anymore
                    if event.is_error() || event.is_read_closed() {
                        println!("- Peer socket error or connection closed. Dropping peer... -");
                        // let mut buf = String::new();
                        // std::io::stdin().read_line(&mut buf);
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
                            println!("- Handling peer {:?} -", &peer_addr);
                            handle_peer(peer_addr, peer, &mut output_file, &mut strategy_state);
                        }
                        // no we havent
                        else {
                            if event.is_readable() {
                                println!("- Received handshake from peer {:?} -", &peer_addr);
                                if let Ok(peer_id) = p2p::recv_handshake(peer.get_mut_socket()) {
                                    peer_list
                                        .complete_peer(peer_addr, &peer_id.try_into().unwrap())
                                        .unwrap();
                                }
                                // i am offended you sent a bad handshake
                                else {
                                    println!("Bad handshake received, bye bye loser!");
                                    let peer = peer_list.remove_peer(peer_addr).unwrap();
                                    peer.disconnect().expect("failed to disconnect peer");
                                }
                            } else if event.is_writable() {
                                println!("- Sent handshake to peer {:?} -", &peer_addr);
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
                println!("- Syn'd and added peer {:?} with {:?} -", peer_addr, token);
            } else {
                println!("already connected to peer {:?}", peer_addr);
            }
        } else {
            println!("failed to connect to peer {:?}", peer_addr);
        }
    }
}

/// Obtains new token with every function call by incrementing static variable.
fn get_new_token() -> Token {
    static TOKEN_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(2);
    Token(TOKEN_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

/// Handles the message received from a peer.
fn handle_peer(
    peer_addr: SocketAddr,
    peer: &mut Peer,
    output_file: &mut OutputFile,
    strategy_state: &mut Strategy,
) {
    p2p::handle_messages(peer).expect("failed to read message"); // TOOD: shout at anders this funtion doesnt work properly (read_to_end)
                                                                 // TODO: should remove peer if error reading? (question mark?)
                                                                 //
    let messages = peer.messages.messages.clone();

    for msg in messages {
        println!("Message is {:?}", msg);
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
            MessageType::Bitfield { mut field } => {
                let _ = field.drain(field.len() - (8 - (output_file.get_num_pieces() % 8))..);
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
                // did we finish the piece?
                if let Ok(true) = output_file.write_block(
                    index.try_into().unwrap(),
                    begin.try_into().unwrap(),
                    block,
                ) {
                    dbg!(format!(
                        "Piece {} is complete, but not hashed.",
                        index
                    ));
                    // does the piece match with our hash
                    if let Ok(true) = output_file.compare_piece_hash(
                        index.try_into().unwrap(),
                        &get_piece_hash(index.try_into().unwrap())
                            .expect("failed to get piece hash"),
                    ) {
                        dbg!(format!("Hash for piece {} matches!!!", index));
                        output_file
                            .set_piece_finished(index.try_into().unwrap())
                            .expect("failed to set piece to finished");
                        strategy_state.rm_requests_for_piece(index.try_into().unwrap());
                        // if so, we can push the update that we have completed a piece !!
                        strategy_state
                            .push_update(Some(peer_addr), MessageType::Have { index: index });
                    } else {
                        dbg!(format!("Hash for piece {} does not match >:(", index));
                        // otherwise, something went wrong when downloading the piece so lets try again :)
                        output_file
                            .clear_piece(index.try_into().unwrap())
                            .expect("failed to clear piece");
                        strategy_state.rm_requests_for_piece(index.try_into().unwrap());
                    }
                }
                dbg!(output_file.get_file_bitfield());
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

    peer.messages.messages.clear();
}

fn create_peer_id() -> [u8; 20] {
    let my_peer_id = Alphanumeric.sample_string(&mut rand::thread_rng(), 20);
    dbg!(&my_peer_id);
    my_peer_id.into_bytes().try_into().unwrap()
}

#[cfg(test)]
mod test {
    use super::*;
    use bitvec::{bits, order::Msb0, vec::BitVec};

    #[test]
    fn test_bitfield_drain() {
        let num_pieces = 3;
        let mut correct_field = BitVec::<u8, Msb0>::new();
        correct_field.push(true);
        correct_field.push(false);
        correct_field.push(true);

        let mut field = BitVec::<u8, Msb0>::new();
        // bitvec is [1, 0, 1, 0, 0, 0, 0, 0]
        field.push(true);
        field.push(false);
        field.push(true);

        field.push(false);
        field.push(false);
        field.push(false);
        field.push(false);
        field.push(false);

        // should make bitvec [1, 0, 1]
        assert_eq!(field.len(), 8);
        let _ = field.drain(field.len() - (8 - (num_pieces % 8))..);
        dbg!(&field);
        dbg!(&correct_field);
        assert_eq!(field.len(), num_pieces);
        assert_eq!(field, correct_field);
    }

    #[test]
    #[ignore]
    fn test_bitfield_drain2() {
        let num_pieces = 10;
        let correct_field = BitVec::from_bitslice(bits![u8, Msb0; 1, 0, 1, 1, 0, 1, 1, 0, 1, 1]);

        let mut field =
            BitVec::from_bitslice(bits![u8, Msb0; 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0]);

        assert_eq!(field.len(), 16);
        let _ = field.drain(field.len() - (8 - (num_pieces % 8))..);
        dbg!(&field);
        dbg!(&correct_field);
        assert_eq!(field.len(), num_pieces);
        assert_eq!(field, correct_field);
    }
}
