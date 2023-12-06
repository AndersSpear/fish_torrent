#![allow(dead_code)] 
// sending and recieving from peers
use super::peers::{self, Peer};
use bitvec::prelude::*;

struct Message<'a> {
    // TODO: Some information about peer
    peer: &'a Peer,
    m_type: MessageType
}

// A little added enum with associated data structs from Tien :)
// Types and names are not final, just figured I'd create this since I'm here
// TODO: Confirm that this is desired.
//length is ususlaly 16Kib, 2^14
enum MessageType {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {index: usize},
    Bitfield {field: BitVec}, // BitVec is a bitvector from the bitvec crate
    Request {index: usize, begin: usize, length: usize},
    Piece {index: usize, begin: usize, block: Vec<u8>},
    Cancel {index: usize, begin: usize, length: usize},
    KeepAlive, // KeepAlive is last because it does not have an associated
              // id in the protocol. This way choke starts at id 0.
    //Port // DHT Tracker is not supported, so this msg is not handled.
    Undefined,
    HandshakeResponse,
}



// called when socket triggers, pass in a peer that got triggered
fn handle_message<'a>(peer: &'a Peer) -> Message<'a>{
    //let msg: Message<'a> = get_message(peer);

    //read the message into a buffer
    //see if its a new handshake, a handshakr response 

    Message {
        peer: peer,
        m_type: MessageType::Undefined
    }
}

// fn recv_message(sockfd: u32) -> Message<'a> {
//     //read the message into a buffer
//     //see if its a new handshake, a handshakr response 
//     //if its a handshake response, return a handshake response message
//     //if its a message, return a message

// }

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
fn handle_choke(msg: &Message) {}

//can handle sending any type of message
//queues in some sort of send list
fn send_message(msg: Message) {}

// called right after we created a new peer
// sends the initial handshake
fn send_handshake(peer: &Peer) {
    //get tcpstream
    //create handshake message
    //send the handshake message
}
