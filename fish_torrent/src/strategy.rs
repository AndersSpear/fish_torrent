#![allow(dead_code)]
//! implements game theory
//! tells bittorrent EVERY MESSSAGE THAT NEEDS TO BE SENT at the moment that its called
//! MY SOLE GOAL: make every bit in the bitvec 1

use std::collections::hash_set::HashSet;
use std::net::SocketAddr;

use bitvec::order::Msb0;
use bitvec::vec::BitVec;
use rand::random;
use sha1::digest::block_buffer::Block;

use crate::file::BLOCK_SIZE;

use super::file::OutputFile;
use super::p2p::{MessageType, Messages};
use super::peers::Peers;

pub struct Update {
    peer_addr: Option<SocketAddr>,
    message: MessageType,
}

#[derive(PartialEq)]
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

    pub fn push_update(&mut self, peer_addr: Option<SocketAddr>, message: MessageType) {
        self.updates.push(Update {
            peer_addr: peer_addr,
            message: message,
        });
    }

    // gets rid of all the requests in the list associated with some block
    pub fn rm_requests_for_piece(&mut self, index: usize) {
        let mut i = 0;
        let mut to_remove = Vec::new();

        // pepega loop to get indicies to remove
        for req in &self.rqs {
            if req.index == index as usize {
                to_remove.push(i);
            }

            i += 1;
        }

        for idx in to_remove {
            self.rqs.remove(idx);
        }
    }

    pub fn rm_all_requests(&mut self) {
        self.rqs.clear();
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

                // // also get rid of all the requests associated with that piece
                // self.rm_requests_for_piece(index.try_into().unwrap());
                // moved this logic to where the Have update was pushed because... I LOVE THE BORROW CHECKER!!!!!1
            }
            // send all peers a keep alive message
            else if let MessageType::KeepAlive = update.message {
                for (addr, peer) in peers.get_peers_list() {
                    peer.get_mut_messages()
                        .messages
                        .push(MessageType::KeepAlive);
                }
            }
        }
        self.updates.clear();

        // Choose pieces to focus on.
        // THIS CODE IS LIKE O(N2) IM SORRY OK 😭
        // 🐒
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
            // see if we can do anything with the pieces that the peer has
            for &piece in &self.focused_pieces {
                // the peer says they have the piece we want 🧩
                if let Some(true) = peer.check_piece_bitfield(piece) {
                    // but they are choking us :(((( 🐒
                    if peer.peer_choking && !peer.am_interested {
                        peer.get_mut_messages()
                            .messages
                            .push(MessageType::Interested);
                        peer.am_interested = true;
                    }
                    // they are not choking us and we can ask them for the piece :)))
                    else if !peer.peer_choking {
                        // TODO: send more than one block at once?
                        let mut i = 0;
                        while i < file.get_piece_size()
                            && (file.is_block_finished(piece, i).unwrap()
                                || self.rqs.contains(&Request {
                                    peer_addr: *addr,
                                    index: piece,
                                    begin: i,
                                    length: BLOCK_SIZE,
                                }))
                        {
                            i += BLOCK_SIZE;
                        }

                        // If i is greater than piece size, then there is nothing else
                        // to request from this peer. Otherwise, add a request.
                        if i < file.get_piece_size() {
                            // should send the next block we want to request for that piece
                            let mut block_len: u32 = BLOCK_SIZE.try_into().unwrap();
                            if file.get_piece_size() - i < BLOCK_SIZE {
                                block_len = (file.get_piece_size() % BLOCK_SIZE) as u32;
                            }

                            peer.get_mut_messages().messages.push(MessageType::Request {
                                index: piece.try_into().unwrap(),
                                begin: i.try_into().unwrap(),
                                length: block_len,
                            });
                            self.rqs.push(Request {
                                peer_addr: *addr,
                                index: piece,
                                begin: i,
                                length: BLOCK_SIZE,
                            })
                        }
                    }
                }
            }

            // we are only popping one request at a time because i hate you peers 🦀
            if let Some(req) = peer.pop_request() {
                peer.get_mut_messages().messages.push(MessageType::Piece {
                    index: req.index.try_into().unwrap(),
                    begin: req.begin.try_into().unwrap(),
                    block: file
                        .read_block(req.index, req.begin, BLOCK_SIZE)
                        .expect("could not get block"),
                });
            }

            // unchoke all the peers 🐒
            // what is the point of choking 🐒
            // we want pieces 🐒
            if peer.am_choking {
                peer.get_mut_messages().messages.push(MessageType::Unchoke);

                peer.am_choking = false;
            }

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
