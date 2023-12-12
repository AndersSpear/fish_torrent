#![allow(dead_code)]
#![warn(missing_docs)]
//! this is responsible for low level p2p communication
//! send messages to peers, recieve messages from peers, does not handle the logic of what to do with the messages.
use crate::peers::Peers;

use super::peers::Peer;
use super::torrent;
use bitvec::prelude::*;

use byteorder::ByteOrder;
use mio::net::TcpStream;
use std::io::Error;
use std::io::Read;
use std::io::Write;

use byteorder::BigEndian;

use anyhow::Result;

#[derive(Debug, Default)]
pub struct Messages {
    messages: Vec<MessageType>,
}

/// A little added enum with associated data structs from Tien :)
/// length is ususlaly 16Kib, 2^14
#[derive(Debug)]
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

/// sends all messages in the peers struct
pub fn send_all(peers: &mut Peers) -> Result<(), Error> {
    unimplemented!();

    // for peer in peers {
    //     peer.messages.send_messages()?;
    // }
    // Ok(())
}

impl Messages {
    pub fn new() -> Self {
        Messages {
            messages: Vec::new(),
        }
    }

    /// can handle sending any type of message
    /// queues in some sort of send list
    fn send_messages(self, sock: &mut TcpStream) -> Result<(), Error> {
        // <length prefix><message ID><payload>
        // 4 bytes        1 byte      ? bytes

        // TODO catch if it would block
        for msg in self.messages {
            msg.send(sock)?;
        }
        Ok(())
    }
}

//TODO look at types of send failures
/// called when socket triggers, pass in a peer that got triggered
pub fn handle_messages(peer: &mut Peer) -> Result<()> {
    let mut return_msgs = Messages { messages: vec![] };

    let mut local_buf = vec![];
    let sock = peer.get_socket();

    let readcount = match sock.read_to_end(&mut local_buf) {
        Ok(n) => n,
        Err(e) => {
            println!("Error reading from socket: {}", e);
            0
        }
    };
    println!("read {} bytes from socket", readcount);

    let mut buf = peer.get_mut_recv_buffer();
    buf.append(&mut local_buf);

    loop {
        match parse_message(&mut buf) {
            Some(msg) => {
                return_msgs.messages.push(msg);
            }
            None => {
                break;
            }
        }
    }

    peer.set_messages(return_msgs);
    Ok(())
}

// TODO make sure this handles handshakes smile
/// tries to parse one message from the buffer
fn parse_message(buf: &mut Vec<u8>) -> Option<MessageType> {
    if buf.len() < 4 {
        return None;
    }
    let len = BigEndian::read_u32(&buf[0..4]);

    //this could break if buf[0..4] gets corrupted and is big
    if buf.len() < len as usize + 4 {
        return None;
    }

    //matching on the length
    //panics if first 4 bytes arent a big endian number!
    Some(match len {
        0 => {
            buf.drain(0..4);
            MessageType::KeepAlive
        }
        1 => match buf[4] {
            0 => {
                buf.drain(0..5);
                MessageType::Choke
            }
            1 => {
                buf.drain(0..5);
                MessageType::Unchoke
            }
            2 => {
                buf.drain(0..5);
                MessageType::Interested
            }
            3 => {
                buf.drain(0..5);
                MessageType::NotInterested
            }
            _ => {
                println!("malformed message, clearing buffer");
                buf.clear();
                return None;
            }
        },
        n => {
            if buf[4] == 4 && n == 5 {
                let index = BigEndian::read_u32(&buf[5..9]);
                buf.drain(0..9);
                MessageType::Have {
                    index: index as usize,
                }
            } else if (buf[4] == 8 || buf[4] == 6) && n == 13 {
                let index = BigEndian::read_u32(&buf[5..9]);
                let begin = BigEndian::read_u32(&buf[9..13]);
                let length = BigEndian::read_u32(&buf[13..17]);
                buf.drain(0..17);

                match buf[4] {
                    6 => MessageType::Request {
                        index: index as usize,
                        begin: begin as usize,
                        length: length as usize,
                    },
                    8 => MessageType::Cancel {
                        index: index as usize,
                        begin: begin as usize,
                        length: length as usize,
                    },
                    _ => {
                        println!("malformed message, clearing buffer");
                        buf.clear();
                        return None;
                    }
                }
            } else if (buf[4] == 7) && n > 9 {
                let index = BigEndian::read_u32(&buf[5..9]);
                let begin = BigEndian::read_u32(&buf[9..13]);
                let block = buf[13..].to_vec();
                buf.drain(0..13);
                buf.drain(0..block.len());
                MessageType::Piece {
                    index: index as usize,
                    begin: begin as usize,
                    block,
                }
            } else if (buf[4] == 5) && n > 5 {
                let field = BitVec::from_vec(buf[5..].to_vec());
                buf.drain(0..5);
                buf.drain(0..field.len());
                MessageType::Bitfield { field }
            } else {
                println!("malformed message, clearing buffer");
                buf.clear();
                return None;
            }
        }
    })
}

impl MessageType {
    fn send(self, sock: &mut TcpStream) -> Result<(), Error> {
        match self {
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
