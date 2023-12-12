#![allow(dead_code)]
#![allow(unused_variables)]
// holds globabl peer list
// recieves peer list from tracker
// updates which peers we are communicating with

//! # peers.rs
//! `peers.rs` contains structs and methods for the storage and handling of `Peer`s
//! within the `Peers` struct.
//! The `Peers` struct will be the ultimate owner of all `Peer`s, from which
//! the individual `Peer` structs can be borrowed and modified as necessary.
use bitvec::prelude::*;
use std::collections::HashMap;

use mio::net::TcpStream;
use std::net::Shutdown;

use anyhow::{Error, Result};

use crate::p2p::Messages;

#[derive(Debug)]
pub struct Peer {
    peer_id: [u8; 20],
    socket: TcpStream,
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
    piece_bitfield: BitVec<u8, Msb0>,
    interested_bitfield: BitVec<u8, Msb0>,
    recv_buffer: Vec<u8>,
    messages: Messages,
}

impl Peer {
    pub fn new(peer_id: &[u8; 20], socket: TcpStream) -> Self {
        Self {
            peer_id: peer_id.clone(),
            socket,
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
            piece_bitfield: BitVec::new(),
            interested_bitfield: BitVec::new(),
            recv_buffer: Vec::new(),
            messages: Messages::default(),
        }
    }

    pub fn init_piece_bitfield(&mut self, bitfield: BitVec<u8, Msb0>) {
        self.piece_bitfield = bitfield;
    }

    pub fn init_interested_bitfield(&mut self, bitfield: BitVec<u8, Msb0>) {
        self.interested_bitfield = bitfield;
    }

    pub fn disconnect(&self) {
        self.socket
            .shutdown(Shutdown::Both)
            .expect(format!("Connection to {:?} failed", self.socket).as_str());
    }

    pub fn get_socket(&mut self) -> &mut TcpStream {
        &mut self.socket
    }

    pub fn get_mut_recv_buffer(&mut self) -> &mut Vec<u8> {
        &mut self.recv_buffer
    }

    pub fn get_mut_messages(&mut self) -> &mut Messages {
        &mut self.messages
    }

    pub fn set_messages(&mut self, messages: Messages) {
        self.messages = messages;
    }

    pub fn set_piece_bit(&mut self, index: usize) {
        self.piece_bitfield.set(index, true);
    }

    pub fn unset_piece_bit(&mut self, index: usize) {
        self.piece_bitfield.set(index, false);
    }

    pub fn set_interested_bit(&mut self, index: usize) {
        self.interested_bitfield.set(index, true);
    }

    pub fn unset_interested_bit(&mut self, index: usize) {
        self.interested_bitfield.set(index, false);
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
            && self.piece_bitfield == other.piece_bitfield
            && self.interested_bitfield == other.interested_bitfield
        // anders - i did not add recv_buffer to this because fuck that
    }
}

pub struct Peers {
    list: HashMap<[u8; 20], Peer>,
    incomplete: HashMap<[u8; 20], Peer>,
}

impl Peers {
    pub fn new() -> Self {
        Peers {
            list: HashMap::new(),
        }
    }

    /// Adds a Peer to the Peers struct.
    /// The Peers struct will take ownership of the peer given.
    /// Additionally, it will return an Error if the peer id is already in the Peers struct.
    /// However, even on Error, ownership of the Peer will be taken.
    pub fn add_peer(&mut self, peer: Peer) -> Result<()> {
        if self.list.get(&peer.peer_id) == None {
            self.list.insert(peer.peer_id, peer);
            Ok(())
        } else {
            Err(Error::msg(
                "peer's peer_id was already found in the Peers struct!",
            ))
        }
    }

    pub fn remove_peer(&mut self, peer: Peer) {
        peer.disconnect();
        self.list.remove(&peer.peer_id);
    }

    pub fn find_peer(&mut self, peer_id: [u8; 20]) -> Option<&mut Peer> {
        self.list.get_mut(&peer_id)
    }

    pub fn get_peers_list(&mut self) -> HashMap<> {
        self.list
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use mio::net::{TcpListener, TcpStream};
    use std::io::{Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    fn networking_setup(port: u16) -> (TcpStream, TcpStream) {
        let serv_sock = TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            port,
        )))
        .unwrap();
        let self_sock = TcpStream::connect(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            port,
        )))
        .unwrap();
        let (other_sock, _) = serv_sock.accept().unwrap();
        (self_sock, other_sock)
    }

    #[test]
    fn test_peer_get_socket() {
        // Set up networking.
        let (self_sock, mut other_sock) = networking_setup(8000);

        // Create a peer, give it the TcpStream, and then see if the stream
        // can be written to and read from.
        let mut peer = Peer::new(&[b'a'; 20], self_sock);
        let get_sock = peer.get_socket();

        // Write
        let string = b"test";
        dbg!(get_sock.write(string).unwrap());
        let mut buf: [u8; 4] = [0; 4];
        dbg!(other_sock.read(&mut buf).unwrap());
        dbg!(std::str::from_utf8(&buf).unwrap());
        assert_eq!(string, &buf);

        // Read
        let string = b"helo";
        dbg!(other_sock.write(string).unwrap());
        dbg!(get_sock.read(&mut buf).unwrap());
        dbg!(std::str::from_utf8(&buf).unwrap());
        assert_eq!(string, &buf);
    }

    #[test]
    fn test_peer_piece_bit() {}
    #[test]
    fn test_peer_interested_bit() {}
    #[test]
    fn test_peer_eq() {}

    #[test]
    fn test_peers_new() {}
    #[test]
    fn test_peers_add_peer() {}
    #[test]
    fn test_peers_remove_peer() {}
    #[test]
    fn test_peers_find_peer() {}
}
