//self contained code to interface with tracker
//updates peers every interval
//import something to have access to the TCP stream


use std::io::prelude::*;
use std::net::TcpStream;
use urlencoding::encode;

// you just got a http response back from the tracker, process it!
// call peers::update_peer_list() with the list of peers
fn handle_tracker_response(/*no arguments*/){
    // called in alexandra's code (main.rs) when a response comes in.
    //1. HTTP get response
    //2. look at TcpStream for the response
    //3. parse response
    // 
    //the response will be bencoded
    // unbencode
    // extract a list of peers from the bencoded data

}

//take in url.
// form req
    //specific format
// send req
pub fn send_tracker_request() -> std::io::Result<()>{
    // BitTorrent-specific parameters
    let info_hash = "aaaaaaaaaaaaaaaaaaaa"; // needs to be 20 bytes
    let peer_id = "aaaaaaaaaaaaaaaaaaaa";     // needs to be 20 bytes
    let port = 6881; //port that we're running on
    let uploaded = 0; //
    let downloaded = 0;
    let left = 0;

    // URL-encode the info_hash and peer_id
    let encoded_info_hash = encode(info_hash);
    let encoded_peer_id = encode(peer_id);

    // Construct the tracker GET request URL
    let tracker_url = format!(
        "/announce?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1",
        encoded_info_hash, encoded_peer_id, port, uploaded, downloaded, left
    );

    // Construct the GET request string
    let request_str = format!("GET {} HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n", tracker_url);
    //let request_str = "GET /announce HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n";

    // Convert the request string to bytes
    let request_bytes = request_str.as_bytes();

    // Connect to the tracker
    let mut stream = TcpStream::connect("poole.cs.umd.edu:6969")?;
    stream.write_all(request_bytes)?;
    stream.flush()?;

    // Read the response from the tracker
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    // Convert the response to a string and print it
    let response_str = String::from_utf8_lossy(&response);
    println!("{}", response_str);

    Ok(())
}

// send the initial request to the tracker. (not necessary?)
fn init_tracker(){
    //1. send_tracker_request
    //2. handle_tracker_response
}

// timeout has passed, ask the tracker for new data, and update it on our status as well
fn update_tracker(){
    //
}