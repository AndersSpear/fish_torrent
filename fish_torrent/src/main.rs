#![feature(lazy_cell)]
// handles initial call to tracker and peer
// handles epoll event loop
// triggers peer tracker, p2p, strategy, on a timer

// mod p2p;
mod peers;
mod torrent;

use mio::{Events, Poll, Interest, Token};
use std::net::{self, SocketAddr};
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

    // set up the server addr
    // TODO: read in torrent file, ask tracker.rs to talk with tracker
    //
    // set up the initial poll

// loop{
//     epollwait();
//     for(events){
//         if(tracker_interval){
//             update_tracker();
//         }
//         if(tracker_response){
//             handle_tracker_response();
//         }
//         if(peer_response){
//             handle_peer_response();
//         }
//     }
// }
}
