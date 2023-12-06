#![allow(dead_code)] 
// sending and recieving from peers
use super::peers::Peer;
use bitvec::prelude::*;

struct Message {
    // TODO: Some information about peer
    peer: &'static Peer,
    m_type: MessageType
}

// A little added enum with associated data structs from Tien :)
// Types and names are not final, just figured I'd create this since I'm here
// TODO: Confirm that this is desired.
enum MessageType {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {index: usize},
    Bitfield {bitfield: BitVec}, // TODO: Temp type for bitfield
    Request {index: usize, begin: usize, length: usize},
    Piece {index: usize, begin: usize, block: usize},
    Cancel {index: usize, begin: usize, length: usize},
    KeepAlive, // KeepAlive is last because it does not have an associated
              // id in the protocol. This way choke starts at id 0.
    //Port // DHT Tracker is not supported, so this msg is not handled.
    Undefined,
}

fn handle_message(sockfd: &u32){
    let peer:&'static Peer = get_peer_from_sockfd(sockfd);
    let msg = get_message(peer);
}


fun get_message(peer: &Peer) -> Message {
    let msg = Message{peer, m_type: MessageType::Undefined};
    msg
}

// TODO: Another way to implement the above is an associated function/method
impl Message {
    // Instead of passing the msg in, now we can call the function via
    // msg.handle_message() <--- Isn't that cool?
    // Your preference!
    fn handle_message(&self) {
        match self.m_type {
            MessageType::Choke => handle_choke(&self),
            _ => todo!() //TODO
        }
    }
}

// TODO add the remaining functions!
fn handle_choke(msg: &Message) {}
