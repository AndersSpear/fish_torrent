#![allow(dead_code)]
#![warn(missing_docs)]
//! this is responsible for low level p2p communication
//! send messages to peers, recieve messages from peers, does not handle the logic of what to do with the messages.
use super::peers::Peer;
use super::torrent;
use bitvec::prelude::*;
use std::io::Write;
use std::net::TcpStream;

pub struct Message<'a> {
    // TODO: Some information about peer
    peer: &'a Peer,
    m_type: MessageType,
}

// A little added enum with associated data structs from Tien :)
// Types and names are not final, just figured I'd create this since I'm here
// TODO: Confirm that this is desired.
//length is ususlaly 16Kib, 2^14
pub enum MessageType {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {
        index: usize,
    },
    Bitfield {
        field: BitVec,
    }, // BitVec is a bitvector from the bitvec crate
    Request {
        index: usize,
        begin: usize,
        length: usize,
    },
    Piece {
        index: usize,
        begin: usize,
        block: Vec<u8>,
    },
    Cancel {
        index: usize,
        begin: usize,
        length: usize,
    },
    KeepAlive, // KeepAlive is last because it does not have an associated
    // id in the protocol. This way choke starts at id 0.
    //Port // DHT Tracker is not supported, so this msg is not handled.
    Undefined,
    HandshakeResponse,
}

// called when socket triggers, pass in a peer that got triggered
pub fn handle_message<'a>(peer: &'a Peer) -> Message<'a> {  
    unimplemented!();
    //let msg: Message<'a> = get_message(peer);

    //read the message into a buffer
    //see if its a new handshake, a handshakr response

    Message {
        peer: peer,
        m_type: MessageType::Undefined,
    }
}

fn recv_message<'a>(sockfd: u32) -> Message<'a> {
    unimplemented!();
    //read the message into a buffer
    //see if its a new handshake, a handshakr response
    //if its a handshake response, return a handshake response message
    //if its a message, return a message

}

// TODO: Another way to implement the above is an associated function/method
// impl Message<'_> {
//     // Instead of passing the msg in, now we can call the function via
//     // msg.handle_message() <--- Isn't that cool?
//     // Your preference!
//     fn handle_message(&self) {
//         match self.m_type {
//             MessageType::Choke => handle_choke(&self),
//             _ => todo!() //TODO
//         }
//     }
// }

// if we get chcked, make sure to remove the send buffer for that person
fn handle_choke(msg: &Message) {
    unimplemented!();
}

//can handle sending any type of message
//queues in some sort of send list
fn send_message(msg: Message) {
    unimplemented!();
}

// called right after we created a new peer
// sends the initial handshake
pub fn send_handshake(peer: &Peer) {
    unimplemented!();
    //let sock: &mut TcpStream = peer.get_socket();
    let mut buf: Vec<u8> = vec![0; 68];
    buf[0] = 19;
    buf[1..20].copy_from_slice(b"BitTorrent protocol");
    buf[28..48].copy_from_slice(&torrent::get_info_hash());
    // TODO get peer id from somewhere
    // buf[48..68].copy_from_slice(&tracker::get_peer_id());
    //sock.write(&buf);
}
