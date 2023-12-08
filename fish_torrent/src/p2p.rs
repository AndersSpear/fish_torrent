#![allow(dead_code)]
#![warn(missing_docs)]
//! this is responsible for low level p2p communication
//! send messages to peers, recieve messages from peers, does not handle the logic of what to do with the messages.
use super::peers::Peer;
use super::torrent;
use bitvec::prelude::*;

use mio::net::TcpStream;
use std::io::Error;
use std::io::Write;

pub struct Messages<'a> {
    // TODO: Some information about peer
    peer: &'a mut Peer,
    messages: Vec<MessageType>,
}

/// A little added enum with associated data structs from Tien :)
/// length is ususlaly 16Kib, 2^14
pub enum MessageType {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {
        index: usize,
    },
    Bitfield {
        field: BitVec<u8>,
    }, // BitVec is a bitvector from the bitvec crate
    Request {
        index: usize,
        begin: usize,
        length: usize,
    },
    Piece {
        index: usize,
        begin: usize,
        block: Vec<u8>,
    },
    Cancel {
        index: usize,
        begin: usize,
        length: usize,
    },
    KeepAlive, // KeepAlive is last because it does not have an associated
    // id in the protocol. This way choke starts at id 0.
    //Port // DHT Tracker is not supported, so this msg is not handled.
    Handshake,
}

// called when socket triggers, pass in a peer that got triggered
pub fn handle_messages<'a>(peer: &'a mut Peer) -> Messages<'a> {
    unimplemented!();
    //let msg: Message<'a> = get_message(peer);

    //read the message into a buffer
    //see if its a new handshake, a handshakr response
    Messages {
        peer: peer,
        messages: Vec::new(),
    }
}

fn recv_message<'a>(sockfd: u32) -> Messages<'a> {
    unimplemented!();
    //read the message into a buffer
    //see if its a new handshake, a handshakr response
    //if its a handshake response, return a handshake response message
    //if its a message, return a message
}

// TODO: Another way to implement the above is an associated function/method
// impl Message<'_> {
//     // Instead of passing the msg in, now we can call the function via
//     // msg.handle_message() <--- Isn't that cool?
//     // Your preference!
//     fn handle_message(&self) {
//         match self.m_type {
//             MessageType::Choke => handle_choke(&self),
//             _ => todo!() //TODO
//         }
//     }
// }

/// if we get chcked, make sure to remove the send buffer for that person
fn handle_choke(msg: &Messages) {
    unimplemented!();
}

/// can handle sending any type of message
/// queues in some sort of send list
pub fn send_messages(msgs: Messages) -> Result<(), Error> {
    // <length prefix><message ID><payload>
    // 4 bytes        1 byte      ? bytes

    let sock: &mut TcpStream = msgs.peer.get_socket();
    for msg in msgs.messages {
        send_message_type(sock, msg)?;
    }
    Ok(())
}

fn send_message_type(sock: &mut TcpStream, msg: MessageType) -> Result<(), Error> {
    match msg {
        MessageType::Choke => {
            send_len_id(sock, 1, 0)?;
        }
        MessageType::Unchoke => {
            send_len_id(sock, 1, 1)?;
        }
        MessageType::Interested => {
            send_len_id(sock, 1, 2)?;
        }
        MessageType::NotInterested => {
            send_len_id(sock, 1, 3)?;
        }
        MessageType::Have { index } => {
            send_have(sock, index)?;
        }
        MessageType::Bitfield { field } => {
            send_bitfield(sock, field)?;
        }
        MessageType::Request {
            index,
            begin,
            length,
        } => {
            send_request_or_cancel(sock, true, index, begin, length)?;
        }
        MessageType::Piece {
            index,
            begin,
            block,
        } => {
            send_piece(sock, index, begin, block)?;
        }
        MessageType::Cancel {
            index,
            begin,
            length,
        } => {
            send_request_or_cancel(sock, false, index, begin, length)?;
        }
        MessageType::KeepAlive => {
            sock.write_all(&[0; 4])?;
        }
        MessageType::Handshake => send_handshake(sock)?,
    }
    Ok(())
}

fn send_len_id(sock: &mut TcpStream, len: u32, id: u8) -> Result<(), Error> {
    let mut buf = vec![0; 5];
    buf[0..4].copy_from_slice(&len.to_be_bytes());
    buf[4] = id;
    sock.write_all(&buf)?;
    Ok(())
}

fn send_have(sock: &mut TcpStream, index: usize) -> Result<(), Error> {
    let mut buf = vec![0; 9];
    buf[0..4].copy_from_slice(&9_u32.to_be_bytes());
    buf[4] = 4; // message id 4 is have
    buf[5..9].copy_from_slice(&index.to_be_bytes());
    sock.write_all(&buf)?;
    Ok(())
}

fn send_bitfield(sock: &mut TcpStream, field: BitVec<u8>) -> Result<(), Error> {
    // TODO make sure length is in bytes not bits
    let length = field.len() as usize;

    // TODO make sure the into_vec is byte aligned and end is padded with 0s
    //field.force_align();

    let mut buf = vec![0; 5];
    buf[0..4].copy_from_slice(&(length + 1).to_be_bytes());
    buf[4] = 5; // message id 5 is bitfield
    buf.extend(field.into_vec());
    sock.write_all(&buf)?;
    Ok(())
}

/// if second argument is true, send a request, else send a cancel
fn send_request_or_cancel(
    sock: &mut TcpStream,
    is_request_message: bool,
    index: usize,
    begin: usize,
    length: usize,
) -> Result<(), Error> {
    let mut buf = vec![0; 17];
    buf[0..4].copy_from_slice(&13_u32.to_be_bytes());
    buf[4] = if is_request_message { 6 } else { 8 }; // message id 6 is request, 8 is cancel
    buf[5..9].copy_from_slice(&index.to_be_bytes());
    buf[9..13].copy_from_slice(&begin.to_be_bytes());
    buf[13..17].copy_from_slice(&length.to_be_bytes());
    sock.write_all(&buf)?;
    Ok(())
}

fn send_piece(
    sock: &mut TcpStream,
    index: usize,
    begin: usize,
    block: Vec<u8>,
) -> Result<(), Error> {
    let length = block.len() as usize;

    let mut buf = vec![0; 13];
    buf[0..4].copy_from_slice(&(length + 9).to_be_bytes());
    buf[4] = 7; // message id 7 is piece
    buf[5..9].copy_from_slice(&index.to_be_bytes());
    buf[9..13].copy_from_slice(&begin.to_be_bytes());
    buf.extend(block);
    sock.write_all(&buf)?;
    Ok(())
}

/// called right after we created a new peer
/// sends the initial handshake
pub fn send_handshake(sock: &mut TcpStream) -> Result<(), Error> {
    let mut buf: Vec<u8> = vec![0; 68];
    buf[0] = 19;
    buf[1..20].copy_from_slice(b"BitTorrent protocol");
    buf[28..48].copy_from_slice(&torrent::get_info_hash());
    // TODO get peer id from somewhere
    // buf[48..68].copy_from_slice(&tracker::get_peer_id());
    sock.write_all(&buf)
}
