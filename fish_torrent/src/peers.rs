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
use std::net::{Shutdown, SocketAddr};

use anyhow::{Error, Result};

use super::p2p::Messages;

#[derive(PartialEq, Debug)]
pub struct Request {
    pub index: usize,
    pub begin: usize,
    pub length: usize,
}

#[derive(Debug)]
pub struct Peer {
    peer_id: Option<[u8; 20]>,
    socket: TcpStream,
    pub am_choking: bool,
    pub am_interested: bool,
    pub peer_choking: bool,
    pub peer_interested: bool,
    piece_bitfield: BitVec<u8, Msb0>, // what pieces they said they have
    piece_requests: Vec<Request>,     // what pieces they said they want
    pub recv_buffer: Vec<u8>,
    pub messages: Messages,
}

impl Peer {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            peer_id: None,
            socket,
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
            piece_bitfield: BitVec::new(),
            piece_requests: Vec::new(),
            recv_buffer: Vec::new(),
            messages: Messages::new(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.peer_id.is_some()
    }

    //fn new_incomplete(socket: TcpStream) -> Self {
    //    Self::new(&[0; 20], socket)
    //}

    pub fn init_piece_bitfield(&mut self, bitfield: BitVec<u8, Msb0>) {
        self.piece_bitfield = bitfield;
    }

    pub fn disconnect(&self) -> Result<()> {
        self.socket.shutdown(Shutdown::Both)?;
        Ok(())
    }

    pub fn get_mut_socket(&mut self) -> &mut TcpStream {
        &mut self.socket
    }

    pub fn set_piece_bit(&mut self, index: usize, status: bool) {
        self.piece_bitfield.set(index, status);
    }

    pub fn push_request(&mut self, index: usize, begin: usize, length: usize) {
        self.piece_requests.push(Request {
            index,
            begin,
            length,
        })
    }

    pub fn pop_request(&mut self) -> Option<Request> {
        self.piece_requests.pop()
    }

    pub fn remove_request(&mut self) -> Option<Request> {
        unimplemented!();
        // loop thru vec
        // remove
    }

    pub fn get_mut_recv_buffer(&mut self) -> &mut Vec<u8> {
        &mut self.recv_buffer
    }

    pub fn get_mut_messages(&mut self) -> &mut Messages {
        &mut self.messages
    }

    pub fn get_messages_clone(&self) -> Messages {
        self.messages.clone()
    }

    pub fn reset_messages(&mut self) {
        self.messages = Messages::new();
    }

    pub fn set_messages(&mut self, messages: Messages) {
        self.messages = messages;
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
            && self.piece_requests == other.piece_requests
        // anders - i did not add recv_buffer to this because fuck that
    }
}

pub struct Peers {
    list: HashMap<SocketAddr, Peer>,
    incomplete: HashMap<SocketAddr, Peer>,
}

impl Peers {
    pub fn new() -> Self {
        Peers {
            list: HashMap::new(),
            incomplete: HashMap::new(),
        }
    }

    /// Adds a Peer to the Peers struct.
    /// The Peers struct will take ownership of the peer given.
    /// Additionally, it will return an Error if the peer id is already in the Peers struct.
    /// However, even on Error, ownership of the Peer will be taken.
    //pub fn add_peer(&mut self, addr: SocketAddr, peer: Peer) -> Result<&mut TcpStream> {
    //    if self.list.contains_key(&addr) == false && self.incomplete.contains_key(&addr) == false {
    //        self.list.insert(addr, peer);
    //        Ok(self
    //            .list
    //            .get_mut(&addr)
    //            .expect("Uhhh contact tien")
    //            .get_mut_socket())
    //    } else {
    //        Err(Error::msg(
    //            "peer's peer_id was already found in the Peers struct!",
    //        ))
    //    }
    //}
    pub fn add_peer(
        &mut self,
        addr: SocketAddr,
        sock: TcpStream,
        peer_id: Option<&[u8; 20]>,
    ) -> Result<&mut Peer> {
        if self.list.contains_key(&addr) == false && self.incomplete.contains_key(&addr) == false {
            let new_peer = Peer::new(sock);
            if peer_id == None {
                self.incomplete.insert(addr, new_peer);
                Ok(self
                    .incomplete
                    .get_mut(&addr)
                    .expect("This should not have occurred."))
            } else {
                self.list.insert(addr, new_peer);
                Ok(self
                    .list
                    .get_mut(&addr)
                    .expect("This should not have occurred."))
            }
        } else {
            Err(Error::msg(
                "peer's peer_id was already found in the Peers struct!",
            ))
        }
    }

    /// Removes a Peer from the Peers struct.
    /// Returns the Peer that was removed for disconnecting.
    pub fn remove_peer(&mut self, addr: SocketAddr) -> Option<Peer> {
        if self.list.contains_key(&addr) == true {
            self.list.remove(&addr)
        } else {
            self.incomplete.remove(&addr)
        }
    }

    //pub fn add_incomplete_peer(
    //    &mut self,
    //    addr: SocketAddr,
    //    sock: TcpStream,
    //) -> Result<&mut TcpStream> {
    //    if self.list.contains_key(&addr) == false && self.incomplete.contains_key(&addr) == false {
    //        self.incomplete.insert(addr, Peer::new_incomplete(sock));
    //        Ok(self
    //            .incomplete
    //            .get_mut(&addr)
    //            .expect("Uhhh contact tien")
    //            .get_mut_socket())
    //    } else {
    //        Err(Error::msg(
    //            "peer's peer_id was already found in the Peers struct!",
    //        ))
    //    }
    //}

    /// Takes in a peer_id to complete the peer stored in the Peers struct.
    /// Will throw an error if the peer was already complete or if there
    /// was no peer to complete using the given addr.
    //pub fn complete_peer(&mut self, addr: SocketAddr, peer_id: &[u8; 20]) -> Result<()> {
    //    if self.list.contains_key(&addr) == false && self.incomplete.contains_key(&addr) == true {
    //        // Get peer off incomplete list.
    //        let mut peer = self.incomplete.remove(&addr).unwrap();
    //        // Complete it.
    //        peer.peer_id = *peer_id;
    //        // Add it to the complete list.
    //        self.add_peer(addr, peer)?;
    //        Ok(())
    //    } else {
    //        Err(Error::msg("I don't even know how you got to this state."))
    //    }
    //}
    pub fn complete_peer(&mut self, addr: SocketAddr, peer_id: &[u8; 20]) -> Result<()> {
        if self.incomplete.contains_key(&addr) == true && self.list.contains_key(&addr) == false {
            let mut peer = self.incomplete.remove(&addr).unwrap();
            peer.peer_id = Some(*peer_id);
            self.list.insert(addr, peer);
            Ok(())
        } else {
            Err(Error::msg("Either you tried to re-complete a peer or complete a peer that was never added. And I don't know what's worse."))
        }
    }

    pub fn find_peer(&mut self, addr: SocketAddr) -> Option<&mut Peer> {
        let mut res = self.list.get_mut(&addr);
        if res == None {
            res = self.incomplete.get_mut(&addr)
        }
        res
    }

    pub fn get_peers_list(&mut self) -> &mut HashMap<SocketAddr, Peer> {
        &mut self.list
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
        let mut peer = Peer::new(self_sock);
        let get_sock = peer.get_mut_socket();

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
