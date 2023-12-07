#![feature(lazy_cell)]
// handles initial call to tracker and peer
// handles epoll event loop
// triggers peer tracker, p2p, strategy, on a timer

mod p2p;
mod peers;
mod torrent;

use mio::{Events, Poll, Interest, Token};
use mio::net::{TcpStream, TcpListener};
use std::collections::HashMap;
use std::net::{self, SocketAddrV4, Ipv4Addr};
use clap::Parser;

use crate::peers::Peers;

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

fn main() {

    // you'll never guess what this line does
    let args = Args::parse();
    let mut peer_list = Peers::new();
    let mut sockets: HashMap<Token, TcpStream> = HashMap::new();

    // binds to INADDR_ANY
    let mut serv_sock = TcpListener::bind(net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, args.port))).expect("bind failed");
    const SERVER:Token = Token(0);

    // creates the events and poll instance used by the event loop
    let mut events = Events::with_capacity(727);
    let mut poll = Poll::new().expect("poll failed");
    
    // registers our listening socket in the epoll
    poll.registry().register(&mut serv_sock, SERVER, Interest::READABLE).expect("serv register failed");

    // TODO: read in torrent file, ask tracker.rs to talk with tracker

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
                        handle_peer_response(&socket);
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

fn handle_tracker_response() -> () {
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

fn handle_peer_response(socket: &TcpStream) {

}