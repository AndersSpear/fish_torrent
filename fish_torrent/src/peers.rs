#![allow(dead_code)] 
#![allow(unused_variables)]
// holds globabl peer list
// recieves peer list from tracker
// updates which peers we are communicating with
use std::net::TcpStream;
use std::sync::RwLock;
use std::collections::HashMap;
use std::sync::LazyLock;
use bitvec::prelude::*;

static PEER_LIST: LazyLock<RwLock<HashMap<Vec<u8>, Peer>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug)]
pub struct Peer {
    peer_id: Vec<u8>,
    socket: TcpStream, //TODO ???
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
    piece_bitmap: BitVec,
    interested_bitmap: BitVec
}

impl Peer {
    pub fn new() {}
    pub fn add_peer(&self) {}
    pub fn remove_peer(&self) {
        let test = Vec::from([1,2,2,4]);
        PEER_LIST.write().expect("RwLock on PEER_LIST was poisoned.").remove(&test);
    }
    pub fn disconnect_peer(&self) {}

    pub fn get_socket(&mut self) -> &mut TcpStream {
        &mut self.socket
    }
}

impl PartialEq for Peer {
    /// This function will return true if all fields EXCEPT socket are equal.
    fn eq(&self, other: &Self) -> bool {
        self.peer_id == other.peer_id &&
        self.am_choking == other.am_choking &&
        self.am_interested == other.am_interested &&
        self.peer_choking == other.peer_choking &&
        self.peer_interested == other.peer_interested &&
        self.piece_bitmap == other.piece_bitmap &&
        self.interested_bitmap == other.interested_bitmap
    }
}

//pub fn find_peer(peer_id: &[u8; 20]) -> &'static Peer {}

//pub fn find_peer_by_sockfd(sockfd: u32) -> &'static Peer {}

fn update_peer_list(peerid: u32, ip: u32, port: u32){

}

fn get_peer_list(){

}
