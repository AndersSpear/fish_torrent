#![allow(dead_code)] 
//self contained code to interface with tracker
//updates peers every interval
use std::io::prelude::*;
use std::net::TcpStream;
use urlencoding::encode;

pub struct TrackerRequest {
    info_hash: String,
    peer_id: String,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
}

impl TrackerRequest {
    // Constructor to create a new TrackerRequest
    pub fn new(info_hash: &str, peer_id: &str, port: u16, uploaded: u64, downloaded: u64, left: u64) -> TrackerRequest {
        TrackerRequest {
            info_hash: info_hash.to_string(),
            peer_id: peer_id.to_string(),
            port,
            uploaded,
            downloaded,
            left,
        }
    }

    // Method to construct the tracker request URL
    pub fn construct_request_url(&self) -> String {
        let encoded_info_hash = encode(&self.info_hash);
        let encoded_peer_id = encode(&self.peer_id);

        format!(
            "GET /announce?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1 HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n",
            encoded_info_hash, encoded_peer_id, self.port, self.uploaded, self.downloaded, self.left
        )
    }
}

/// Constructs a tracker request URL with the given parameters.
///
/// # Arguments
/// * `info_hash` - A 20-byte string representing the info hash.
/// * `peer_id` - A 20-byte string representing the peer id.
/// * `port` - The port number the client is listening on.
/// * `uploaded` - The total amount uploaded so far.
/// * `downloaded` - The total amount downloaded so far.
/// * `left` - The total amount left to download.
pub fn construct_tracker_request(info_hash: &str, peer_id: &str, port: u16, uploaded: u64, downloaded: u64, left: u64) -> String {
    let encoded_info_hash = encode(info_hash);
    let encoded_peer_id = encode(peer_id);

    format!(
        "GET /announce?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1 HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n",
        encoded_info_hash, encoded_peer_id, port, uploaded, downloaded, left
    )
}

/// Sends a tracker request and returns the response as a String.
///
/// # Arguments
/// * `request` - The tracker request string to be sent.
pub fn send_tracker_request(tracker_request: &TrackerRequest, connect_to: &str) -> std::io::Result<String> {
    let request = tracker_request.construct_request_url();
    let mut stream = TcpStream::connect(connect_to)?;
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    Ok(String::from_utf8_lossy(&response).to_string())
}

/// Processes the HTTP response received from the tracker.
/// 
/// This function should parse the bencoded response and extract a list of peers.
fn handle_tracker_response() {
    // TODO: Implement this function.
    // The response will be bencoded. Unbencode it and extract a list of peers.
}

/// Initializes the tracker by sending the initial request.
/// This function may not be necessary depending on the protocol requirements.
fn init_tracker() {
    // TODO: Implement this function if needed.
    // This might involve calling `send_tracker_request` and `handle_tracker_response`.
}

/// Updates the tracker with the current status and requests new data.
fn update_tracker() {
    // TODO: Implement periodic updates to the tracker here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_tracker_request() {
        let request = construct_tracker_request(
            "aaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbb",
            6881,
            0,
            0,
            0,
        );

        assert!(request.contains("info_hash=aaaaaaaaaaaaaaaaaaaa"));
        assert!(request.contains("peer_id=bbbbbbbbbbbbbbbbbbbb"));
    }

    #[test]
    fn test_send_tracker_request() {
        let tracker_request = TrackerRequest::new(
            "aaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbb",
            6881,
            0,
            0,
            0,
        );

        // this should only fail if the UMD server is down.
        let response = send_tracker_request(&tracker_request, "poole.cs.umd.edu:6969").unwrap();

        // 'interval' is inside every bencode.
        assert!(response.contains("interval"));
    }
}
