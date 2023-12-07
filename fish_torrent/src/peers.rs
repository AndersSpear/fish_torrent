#![allow(dead_code)]
#![allow(unused_variables)]
// holds globabl peer list
// recieves peer list from tracker
// updates which peers we are communicating with
use bitvec::prelude::*;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;

use std::net::Shutdown;
use std::net::SocketAddrV4;
use std::net::TcpStream;

static PEER_LIST: LazyLock<RwLock<HashMap<[u8; 20], Peer>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug)]
pub struct Peer {
    peer_id: [u8; 20],
    socket: TcpStream, //TODO ???
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
    piece_bitmap: BitVec,
    interested_bitmap: BitVec,
}

impl Peer {
    pub fn new(peer_id: [u8; 20], addr: SocketAddrV4) -> Option<Self> {
        if let Ok(socket) = TcpStream::connect(&addr) {
            Some(Self {
                peer_id,
                socket,
                am_choking: true,
                am_interested: false,
                peer_choking: true,
                peer_interested: false,
                piece_bitmap: BitVec::new(),
                interested_bitmap: BitVec::new(),
            })
        } else {
            None
        }
    }

    pub fn add_peer(self) {
        if PEER_LIST
            .read()
            .expect("RwLock on PEER_LIST was poisoned.")
            .get(&self.peer_id)
            == None
        {
            PEER_LIST
                .write()
                .expect("RwLock on PEER_LIST was poisoned.")
                .insert(self.peer_id, self);
        }
    }

    pub fn remove_peer(&self) {
        self.disconnect_peer();
        PEER_LIST
            .write()
            .expect("RwLock on PEER_LIST was poisoned.")
            .remove(&self.peer_id);
    }

    pub fn disconnect_peer(&self) {
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
    }
}

pub fn find_peer(peer_id: &[u8; 20]) -> Option<&Peer> {
    PEER_LIST.read().expect("RwLock on PEER_LIST was poisoned.").get(peer_id)
}

pub fn get_peer_list() -> &'static HashMap<[u8; 20], Peer> {
    &PEER_LIST.read().expect("RwLock on PEER_LIST was poisoned.")
}
