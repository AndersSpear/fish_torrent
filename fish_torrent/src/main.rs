// handles initial call to tracker and peer
// handles epoll event loop
// triggers peer tracker, p2p, strategy, on a timer

mod peers;
mod p2p;

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


mod tracker;

fn main() -> std::io::Result<()> {
    Ok(())
}
