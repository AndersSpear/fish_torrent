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
mod torrent;

use clap::Parser;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::net::{self, Ipv4Addr, SocketAddrV4};

use crate::peers::Peers;
use crate::torrent::{parse_torrent_file};

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
/// main handles the initialization of stuff and keeping the event loop logic going
fn main() {
    // you'll never guess what this line does
    let args = Args::parse();
    let mut peer_list = Peers::new();
    let mut sockets: HashMap<Token, TcpStream> = HashMap::new();

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

    // read in torrent file 
    parse_torrent_file(&args.file).expect("Failed to parse torrent file");

    // println!("{}", get_tracker_url());
    // TODO: ask tracker.rs to talk with tracker

    loop {
        poll.poll(&mut events, None).expect("poll_wait failed");

        for event in &events {
            match event.token() {
                SERVER => {
                    println!("an accept occurred!");
                }
                TRACKER => {
                    handle_tracker_response();
                }
                token => {
                    if let Some(socket) = sockets.get(&token) {
                        handle_peer(&socket);
                    } else {
                        println!("there is no socket associated with token {:#?}", token);
                    }
                }
            }
        }
        // match listener.accept() {
        //     Ok((_socket, addr)) => println!("new client: {addr:?}"),
        //     Err(e) => println!("couldn't get client: {e:?}"),
        // }

        // epollwait();
        // for(events) {
        //     if(tracker_interval){
        //         update_tracker();
        //     }
        //     if(tracker_response){
        //         handle_tracker_response();
        //     }
        //     if(peer_response){
        //         handle_peer_response();
        //     }
        // }
    }
}

/// handles the response the tracker got, creates new peers for all the peers it received if needed
fn handle_tracker_response() {
    //     get "list" of peer metadata

    //     foreach peer data {

    //         let mut socket = TcpStream::connect(peer data)?;
    //         let peer = Peer::new(peerid, stream);

    //         if peer_list.add_peer(peer) == false {

    //             error or seomthign idk
    //         }

    //         let token = get_new_token();
    //         poll.registry().register(&mut socket, token, Interest::READABLE)?;
    //         sockets.insert(get_new_token, peer_list.find_peer(peerid).unwrap().get_socket());
    //     }
}

/// handles the message it got from a peer
fn handle_peer(socket: &TcpStream) {
    // recv_message(socket);
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