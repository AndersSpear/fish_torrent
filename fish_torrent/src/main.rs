// handles initial call to tracker and peer
// handles epoll event loop
// triggers peer tracker, p2p, strategy, on a timer

mod peers;
mod p2p;

use mio::{Events, Poll, Interest, Token};
use mio::net::TcpStream
use stl::net::{self, SocketAddr};

fn main() {

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
