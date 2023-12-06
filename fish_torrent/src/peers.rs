// holds globabl peer list
// recieves peer list from tracker
// updates which peers we are communicating with
use std::net::TcpStream;
use std::collections::HashMap;
use bitvec::prelude::*;

thread_local! {
    static peer_list;
}

pub struct Peer {
    peer_id: [u8; 20],
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
    pub fn remove_peer(&self) {}
    pub fn disconnect_peer(&self) {}

    pub fn get_socket(&self) -> &TcpStream {
        &self.socket
    }
}

pub fn find_peer(peer: &Peer) {}

pub fn find_peer_by_sockfd(sockfd: u32) {}

fn update_peer_list(peerid: u32, ip: u32, port: u32){

}

fn get_peer_list(){

}
