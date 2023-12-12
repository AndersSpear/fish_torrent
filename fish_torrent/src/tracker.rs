#![allow(dead_code)]
use bendy::serde::from_bytes;
//self contained code to interface with tracker
//updates peers every interval
use mio::net::TcpStream;
use serde::{Deserialize, Serialize};
use std::ascii::escape_default;
use std::io::prelude::*;
use urlencoding::encode;
// enum Peers {
//     Compact(Vec<u8>),
//     NonCompact(Vec<Peer>),
// }

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct TrackerResponseBeta {
    interval: u64,
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>, // Assuming compact format
}

#[derive(Debug)]
pub struct TrackerResponse {
    pub interval: u64,
    pub socket_addr_list: Vec<SocketAddr>,
}

struct Peer {
    ip: std::net::Ipv4Addr,
    port: u16,
}

pub struct TrackerRequest {
    info_hash: String,
    peer_id: String,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Event,
}

pub enum Event {
    STARTED,
    STOPPED,
    COMPLETED,
    PERIODIC,
}

impl Event {
    fn as_str(&self) -> &str {
        match self {
            Event::STARTED => "started",
            Event::STOPPED => "stopped",
            Event::COMPLETED => "completed",
            Event::PERIODIC => "periodic", // or an empty string if you prefer
        }
    }
}

impl TrackerRequest {
    pub fn new(
        info_hash: &str,
        peer_id: &str,
        port: u16,
        uploaded: u64,
        downloaded: u64,
        left: u64,
        event: Event,
    ) -> TrackerRequest {
        TrackerRequest {
            info_hash: info_hash.to_string(),
            peer_id: peer_id.to_string(),
            port,
            uploaded,
            downloaded,
            left,
            event,
        }
    }

    pub fn construct_tracker_request(&self) -> String {
        let encoded_info_hash = encode(&self.info_hash);
        let encoded_peer_id = encode(&self.peer_id);
        let event_str = self.event.as_str();

        format!(
            "GET /announce?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact=1 HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n",
            encoded_info_hash, encoded_peer_id, self.port, self.uploaded, self.downloaded, self.left, event_str
        )
    }
}


pub fn parse_body_from_response(response: &Vec<u8>) -> std::io::Result<Vec<u8>> {
    let separator_pos = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|pos| pos + 4);

    match separator_pos {
        Some(pos) => {
            let body_bytes = &response[pos..];
            println!(
                "BODY_BYTES BRUH{}",
                String::from_utf8_lossy(body_bytes).to_string()
            );
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
) -> std::io::Result<()> {
    let request = TrackerRequest::construct_tracker_request(tracker_request);
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    Ok(())
}
use bendy::decoding::{Error, FromBencode, Object};

fn show(bs: &[u8]) -> String {
    let mut visible = String::new();
    for &b in bs {
        let part: Vec<u8> = escape_default(b).collect();
        visible.push_str(&String::from_utf8(part).unwrap());
    }
    visible
}

use bendy::decoding::Decoder;
use std::io::Cursor;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Processes the HTTP response received from the tracker.
///
/// This function should parse the bencoded response and extract a list of peers.

pub fn handle_tracker_response(
    stream: &mut TcpStream,
) -> Result<TrackerResponse, bendy::decoding::Error> {
    // Read the HTTP response
    let mut response_data = Vec::new();
    stream.read_to_end(&mut response_data)?;

    // Find the position of the double CRLF separator
    //iterates through byte vector
    let body = parse_body_from_response(&response_data)?;

    let trb =
        from_bytes::<TrackerResponseBeta>(body.as_slice()).expect("Decoding the .torrent failed");
    let mut sock_addr_list = Vec::new();
    for chunk in trb.peers.chunks_exact(6) {
        let ip_bytes = &chunk[0..4];
        let port_bytes = &chunk[4..6];
        let ip_addr = std::net::Ipv4Addr::from(<[u8; 4]>::try_from(ip_bytes).unwrap());
        let port = u16::from_be_bytes(<[u8; 2]>::try_from(port_bytes).unwrap());

        let socket = SocketAddr::new(IpAddr::V4(ip_addr), port);
        sock_addr_list.push(socket);
    }
    Ok(TrackerResponse {
        interval: trb.interval,
        socket_addr_list: sock_addr_list,
    })
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
        let tr = TrackerRequest::new(
            "aaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbb",
            6881,
            0,
            0,
            0,
            Event::STARTED
        );
        let request = TrackerRequest::construct_tracker_request(&tr);

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
            Event::STARTED
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
            Event::STARTED
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
