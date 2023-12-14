#![allow(dead_code)]
//! implements game theory
//! tells bittorrent EVERY MESSSAGE THAT NEEDS TO BE SENT at the moment that its called
//! MY SOLE GOAL: make every bit in the bitvec 1

use std::collections::hash_set::HashSet;
use std::net::SocketAddr;

use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use rand::random;

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
    pub pieces_left: HashSet<usize>,
    pub max_simul_pieces: usize,
    updates: Vec<Update>,
    rqs: Vec<Request>,
    focused_pieces: HashSet<usize>,
    num_tasks: usize,
}

impl Strategy {
    pub fn new(num_pieces: usize, max_simul_pieces: usize) -> Self {
        Self {
            pieces_left: HashSet::new(),
            max_simul_pieces,
            updates: Vec::new(),
            rqs: Vec::new(),
            focused_pieces: HashSet::new(),
            num_tasks: 0,
        }
    }

    pub fn what_do(&mut self, peers: &mut Peers, file: &OutputFile) {
        // For every update that was given to strategy...
        // Handle them individually
        for update in &self.updates {
            // If the update was have (i.e. we completed a piece),
            // blast this out to all peers.
            if let MessageType::Have { index } = update.message {
                self.focused_pieces.remove(&index.try_into().unwrap());
                for (addr, peer) in peers.get_peers_list() {
                    peer.get_mut_messages()
                        .messages
                        .push(MessageType::Have { index });
                }
            }
        }

        // Choose pieces to focus on.
        // THIS CODE IS LIKE O(N2) IM SORRY OK 😭
        let num_avail_spots = self.max_simul_pieces - self.focused_pieces.len();
        for _ in 0..num_avail_spots {
            //let r = random() % pieces_left.len; // NEEDS A BOUND LOL // i gave bound :)
            //self.focused_pieces.insert(pieces_left.get(r).unwrap());
            //if let Some(index) = file.get_file_bitfield().first_zero() {
            //    self.focused_pieces.insert(index);
            //}
            for index in file.get_file_bitfield().iter_zeros() {
                if self.focused_pieces.contains(&index) == false {
                    self.focused_pieces.insert(index);
                    break;
                }
            }
        }

        // Requesting and fulfilling requests.
        for (addr, peer) in peers.get_peers_list() {
            //for self.focused_pi
            // For every focused piece
                // Does peer have what I want?
                // If so, are we unchoked?
                    // Then request
                    // Check which blocks we haven't requested and request those (hashset)
                // Else, interested
            //determine which blocks I want to request
        }
    }
}
