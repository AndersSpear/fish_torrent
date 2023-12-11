#![allow(dead_code)]
use bendy::serde::from_bytes;
//self contained code to interface with tracker
//updates peers every interval
use mio::net::TcpStream;
use std::ascii::escape_default;
use std::io::prelude::*;
use urlencoding::encode;
use serde::{Deserialize, Serialize};
// enum Peers {
//     Compact(Vec<u8>),
//     NonCompact(Vec<Peer>),
// }

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct TrackerResponse {

    interval: i64,
    peers: String  // Assuming compact format
}

pub struct TrackerRequest {
    info_hash: String,
    peer_id: String,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
}

struct Peer {
    ip: std::net::Ipv4Addr,
    port: u16,
}

impl TrackerRequest {
    // Constructor to create a new TrackerRequest
    pub fn new(
        info_hash: &str,
        peer_id: &str,
        port: u16,
        uploaded: u64,
        downloaded: u64,
        left: u64,
    ) -> TrackerRequest {
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
pub fn construct_tracker_request(
    info_hash: &str,
    peer_id: &str,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
) -> String {
    let encoded_info_hash = encode(info_hash);
    let encoded_peer_id = encode(peer_id);

    format!(
        "GET /announce?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1 HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n",
        encoded_info_hash, encoded_peer_id, port, uploaded, downloaded, left
    )
}

pub fn parse_body_from_response(response: &Vec<u8>) -> std::io::Result<Vec<u8>> {
    let separator_pos = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|pos| pos + 4);

    match separator_pos {
        Some(pos) => {
            let body_bytes = &response[pos..];
            println!("BODY_BYTES BRUH{}", String::from_utf8_lossy(body_bytes).to_string());
            Ok(body_bytes.to_vec())

        }
        None => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to find separator in HTTP response",
        )),
    }
}


/// Sends a tracker request and returns the response as a Vec<u8>.
///
/// # Arguments
/// * `request` - The tracker request string to be sent.
pub fn send_tracker_request(
    tracker_request: &TrackerRequest,
    stream: &mut TcpStream,
) -> std::io::Result<Vec<u8>> {
    let request = tracker_request.construct_request_url();
    stream.write_all(request.as_bytes())?;
    stream.flush()?;

    // Read the HTTP response
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    // Find the position of the double CRLF separator
    //iterates through byte vector
    Ok(response)
}
use bendy::decoding::{FromBencode, Error, Object};

fn show(bs: &[u8]) -> String {
    let mut visible = String::new();
    for &b in bs {
        let part: Vec<u8> = escape_default(b).collect();
        visible.push_str(&String::from_utf8(part).unwrap());
    }
    visible
}

use bendy::decoding::{Decoder};
use std::io::Cursor;
use std::net::Ipv4Addr;

/// Processes the HTTP response received from the tracker.
///
/// This function should parse the bencoded response and extract a list of peers.
pub fn handle_tracker_response(response_data: &Vec<u8>) -> Result<TrackerResponse, bendy::decoding::Error> {
    let body = parse_body_from_response(response_data)?;
    //println!("Response Body: {}", show(body.as_slice()));

    let mut decoder = Decoder::new(body.as_slice());
    let infodata = 'outer: loop {
        match decoder.next_object() {
            Ok(Some(Object::Dict(mut d))) => loop {
                match d.next_pair() {
                    Ok(Some((b"interval", Object::Integer(d)))) => {
                        break 'outer d;
                    }
                    Ok(Some((_, _))) => (),
                    Ok(None) => break,
                    Err(e) => panic!("meow trying to gety/decode infohash failed: {}", e),
                }
            },
            _ => (),
        }
    };

    println!("yay an int {}", infodata);


    let response =
    from_bytes::<TrackerResponse>(body.as_slice()).expect("decode response failed");
    Ok(response)



}

// fn parse_peers(peers_data: &[u8]) -> Result<Vec<Peer>, bendy::Error> {
//     let mut peers = Vec::new();
//     let mut cursor = Cursor::new(peers_data);

//     while (cursor.position() as usize) < peers_data.len() {
//         let ip = Ipv4Addr::new(cursor.get_u8(), cursor.get_u8(), cursor.get_u8(), cursor.get_u8());
//         let port = cursor.get_u16_be();
//         peers.push(Peer { ip, port });
//     }

//     Ok(peers)
// }

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
        //let response = send_tracker_request(&tracker_request, "poole.cs.umd.edu:6969").unwrap();

        // for s in &response {
        //     println!("{}", s);
        // }

        //test that the string representation of response contains 'interval'
        // let string_representation = String::from_utf8_lossy(&response).to_string();
        // println!("{}", &string_representation);
        // assert!(&string_representation.contains("interval"));


    }


    fn test_handle_tracker_response() {
        let tracker_request = TrackerRequest::new(
            "aaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbb",
            6881,
            0,
            0,
            0,
        );

        // this should only fail if the UMD server is down.
        //let response = send_tracker_request(&tracker_request, "poole.cs.umd.edu:6969").unwrap();

        //for s in &response {
            //println!("{}", s);
        //}

        //test that the string representation of response contains 'interval'
        //let string_representation = String::from_utf8_lossy(&response).to_string();
        // println!("{}", &string_representation);
        // assert!(&string_representation.contains("interval"));

        // let v = string_representation.clone();
        
        //let parsed_message = handle_tracker_response(&response);
        // for s in parsed_message.unwrap() {
        //     println!("{}", s);
        // }



    }
}
