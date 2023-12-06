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


use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> std::io::Result<()> {

    let request_str = "GET / HTTP/1.1\r\nHost: 127.0.0.1:7878\r\n\r\n";
    let request_bytes = request_str.as_bytes();

    //replace with the TcpStream from alexandra
    let mut stream = TcpStream::connect("127.0.0.1:7878")?;
    //let mut stream = TcpStream::connect("www.google.com:80")?;
    stream.write_all(request_bytes)?;
    stream.flush()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    let response_str = String::from_utf8_lossy(&response);
    println!("{}", response_str);
    Ok(())
}

