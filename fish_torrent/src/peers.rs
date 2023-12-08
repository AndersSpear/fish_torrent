#![allow(dead_code)]
#![allow(unused_variables)]
// holds globabl peer list
// recieves peer list from tracker
// updates which peers we are communicating with
use bitvec::prelude::*;
use std::collections::HashMap;

use mio::net::TcpStream;
use std::net::Shutdown;

#[derive(Debug)]
pub struct Peer {
    peer_id: [u8; 20],
    socket: TcpStream,
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
    piece_bitmap: BitVec,
    interested_bitmap: BitVec,
    recv_buffer: Vec<u8>,
}

impl Peer {
    pub fn new(peer_id: [u8; 20], socket: TcpStream) -> Self {
        Self {
            peer_id,
            socket,
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
            piece_bitmap: BitVec::new(),
            interested_bitmap: BitVec::new(),
            recv_buffer: Vec::new(),
        }
    }

    pub fn disconnect(&self) {
        self.socket
            .shutdown(Shutdown::Both)
            .expect(format!("Connection to {:?} failed", self.socket).as_str());
    }

    pub fn get_socket(&mut self) -> &mut TcpStream {
        &mut self.socket
    }
}

impl PartialEq for Peer {
    /// This function will return true if all fields EXCEPT socket are equal.
    fn eq(&self, other: &Self) -> bool {
        self.peer_id == other.peer_id
            && self.am_choking == other.am_choking
            && self.am_interested == other.am_interested
            && self.peer_choking == other.peer_choking
            && self.peer_interested == other.peer_interested
            && self.piece_bitmap == other.piece_bitmap
            && self.interested_bitmap == other.interested_bitmap
            // anders - i did not add recv_buffer to this because fuck that
    }
}

pub struct Peers {
    list: HashMap<[u8; 20], Peer>,
}

impl Peers {
    pub fn new() -> Self {
        Peers {
            list: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, peer: Peer) -> bool {
        if self.list.get(&peer.peer_id) == None {
            self.list.insert(peer.peer_id, peer);
            true
        } else {
            false
        }
    }

    pub fn remove_peer(&mut self, peer: Peer) {
        peer.disconnect();
        self.list.remove(&peer.peer_id);
    }

    pub fn find_peer(&self, peer_id: [u8; 20]) -> Option<&Peer> {
        self.list.get(&peer_id)
    }
}
