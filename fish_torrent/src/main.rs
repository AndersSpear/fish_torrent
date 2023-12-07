#![feature(lazy_cell)]
// handles initial call to tracker and peer
// handles epoll event loop
// triggers peer tracker, p2p, strategy, on a timer

// mod p2p;
mod peers;

use mio::{Events, Poll, Interest, Token};
use mio::net::{TcpStream, TcpListener};
use std::net::{self, SocketAddrV4, Ipv4Addr};
use clap::Parser;

/// Takes in the port and torrent file
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
    
    // binds to INADDR_ANY
    let mut serv_sock = TcpListener::bind(net::SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, args.port))).expect("bind failed");
    const SERVER:Token = Token(0);

    let mut events = Events::with_capacity(1024);
    let mut poll = mio::Poll::new().expect("poll failed");
    poll.registry().register(&mut serv_sock, Token(0), Interest::READABLE).expect("serv register failed");

    // TODO: read in torrent file, ask tracker.rs to talk with tracker
    // TODO: set up the initial poll

    loop {
        poll.poll(&mut events, None).expect("poll_wait failed");

        for event in &events {
            if event.token() == Token(0) {
                println!("an accept occurred!");
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