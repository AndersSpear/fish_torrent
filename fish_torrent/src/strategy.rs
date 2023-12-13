#![allow(dead_code)]
//! implements game theory
//! tells bittorrent EVERY MESSSAGE THAT NEEDS TO BE SENT at the moment that its called
//! MY SOLE GOAL: make every bit in the bitvec 1

use std::net::SocketAddr;

use bitvec::order::Msb0;
use bitvec::vec::BitVec;

use super::file::OutputFile;
use super::p2p::{MessageType, Messages};
use super::peers::Peers;

pub struct Update {
    peer_addr: SocketAddr,
    message: MessageType,
}

pub struct Request {
    peer_addr: SocketAddr,
    index: usize,
    begin: usize,
    length: usize,
}

pub struct Strategy {
    updates: Vec<Update>,
    rqs: Vec<Request>,
    focused_pieces: Vec<usize>,
    num_tasks: usize,
}

impl Strategy {
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
            rqs: Vec::new(),
            focused_pieces: Vec::new(),
            num_tasks: 0,
        }
    }

    pub fn what_do(&mut self, peers: &mut Peers, file: &OutputFile) {
        for update in &self.updates {
            if let MessageType::Have { index } = update.message {
                for (addr, peer) in peers.get_peers_list() {
                    peer.get_mut_messages()
                        .messages
                        .push(MessageType::Have { index });
                }
            }
        }
    }
}
