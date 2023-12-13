#![allow(dead_code)]
use bendy::serde::from_bytes;
//self contained code to interface with tracker
//updates peers every interval
use mio::net::TcpStream;
use serde::{Deserialize, Serialize};
use std::ascii::escape_default;
use std::io::prelude::*;
use url::Url;
use urlencoding::encode;
// enum Peers {
//     Compact(Vec<u8>),
//     NonCompact(Vec<Peer>),
// }
use crate::torrent::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct TrackerResponseBeta {
    interval: usize,
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>, // Assuming compact format
}

#[derive(Debug)]
pub struct TrackerResponse {
    pub interval: usize,
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
    uploaded: usize,
    downloaded: usize,
    left: usize,
    event: Event,
}

pub enum Event {
    STARTED,
    STOPPED,
    COMPLETED,
    PERIODIC,
}

impl Event {
    fn as_str(&self) -> Option<&str> {
        match self {
            Event::STARTED => Some("started"),
            Event::STOPPED => Some("stopped"),
            Event::COMPLETED => Some("completed"),
            Event::PERIODIC => None, // or an empty string if you prefer
        }
    }
}

impl TrackerRequest {
    pub fn new(
        info_hash: Vec<u8>,
        peer_id: Vec<u8>,
        port: u16,
        uploaded: usize,
        downloaded: usize,
        left: usize,
        event: Event,
    ) -> TrackerRequest {
        TrackerRequest {
            info_hash: bytes_to_urlencoding(&info_hash),
            peer_id: bytes_to_urlencoding(&peer_id),
            port,
            uploaded,
            downloaded,
            left,
            event,
        }
    }

    pub fn construct_tracker_request(&self) -> String {
        // Tien removed these because info_hash and peer_id are encoding in new() now.
        //let encoded_info_hash = encode(&self.info_hash);
        //let encoded_peer_id = encode(&self.peer_id);
        if let Some(event_str) = self.event.as_str() {
            format!(
                "GET /announce?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact=1 HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n",
                //encoded_info_hash, encoded_peer_id, self.port, self.uploaded, self.downloaded, self.left, event_str
                self.info_hash, self.peer_id, self.port, self.uploaded, self.downloaded, self.left, event_str
            )
        } else {
            format!(
                "GET /announce?info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact=1 HTTP/1.1\r\nHost: poole.cs.umd.edu\r\n\r\n",
                self.info_hash, self.peer_id, self.port, self.uploaded, self.downloaded, self.left)
        }
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

pub fn get_tracker_response_from_vec_u8(buf: &Vec<u8>) -> TrackerResponse {
    //if Ok, we have received n bytes and placed into response_data.
    // then, we process.
    // if err,

    //put it into response_data

    // Find the position of the double CRLF separator
    //iterates through byte vector
    let body = parse_body_from_response(buf)
        .expect("parse_body_from_response in get_tracker_from_response fail");

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
    let tr = TrackerResponse {
        interval: trb.interval,
        socket_addr_list: sock_addr_list,
    };
    dbg!(&tr);
    tr
}

/// Processes the HTTP response received from the tracker.
///
/// This function should parse the bencoded response and extract a list of peers.

pub fn handle_tracker_response(
    mut buf: Vec<u8>,
    stream: &mut TcpStream,
) -> (Vec<u8>, Option<TrackerResponse>) {
    match stream.read_to_end(&mut buf) {
        Ok(n) => {
            //we're good. process
            //buf is modified to be full.
            let tr = get_tracker_response_from_vec_u8(&buf);
            buf.clear();
            return (buf, Some(tr));
        }
        Err(e) => {
            //instantly leave
            dbg!("tracker partial read occurred");
            return (buf, None);
        }
    };
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

fn bytes_to_urlencoding(bytes: &[u8]) -> String {
    let mut res = String::new();
    for b in bytes {
        res.push_str(&format!("%{:02X}", b));
    }
    res
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
            Event::STARTED,
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
            Event::STARTED,
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

    #[test]
    fn test_handle_tracker_response() {
        let tracker_request = TrackerRequest::new(
            "aaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbb",
            6881,
            0,
            0,
            0,
            Event::STARTED,
        );

        let mut tracker_sock = TcpStream::from_std(
            std::net::TcpStream::connect(
                *Url::parse("http://128.8.126.63:6969/announce")
                    .unwrap()
                    .socket_addrs(|| None)
                    .expect("could not connect to announce")
                    .first()
                    .unwrap(),
            )
            .expect("connect failed"),
        );

        send_tracker_request(&tracker_request, &mut tracker_sock)
            .expect("could not send_tracker_request");
        let tr = handle_tracker_response(vec![], &mut tracker_sock);
        match tr {
            (vec, Some(tr)) => {
                dbg!(&tr.socket_addr_list);
                assert!(tr.socket_addr_list.len() >= 1);
            }
            (vec, None) => {
                // ignore
            }
        }

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

    #[test]
    fn test_bytes_to_urlencoding() {
        let res = bytes_to_urlencoding(&[0x05, 0x61, 0x61, 0x61, 0xc3, 0xb5]);
        assert_eq!("%05%61%61%61%C3%B5", res);
        dbg!(res);
    }
}
