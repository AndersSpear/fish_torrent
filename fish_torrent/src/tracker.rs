//self contained code to interface with tracker
//updates peers every interval
//import something to have access to the TCP stream


use std::net::TcpStream;

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
fn send_tracker_request(tracker_url: &str){

    let request_str = "GET / HTTP/1.1\r\nHost: google.com\r\n\r\n";

    //replace with the TcpStream from alexandra
    let mut stream = TcpStream::connect("www.google.com:80")?;
    stream.write_all(request_bytes)?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    let response_str = String::from_utf8_lossy(&response);
    println!("{}", response_str);


    //create request
        // parameters
            // info_hash
            // peer_id
            // port
            // uploaded
            // downloaded
            // compact
            // left
        // URL
    
    


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