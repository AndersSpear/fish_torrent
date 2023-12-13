#![allow(dead_code)]
#![warn(missing_docs)]
//! this is responsible for low level p2p communication
//! send messages to peers, recieve messages from peers, does not handle the logic of what to do with the messages.
//! call send_all(peers) for sending to all peers
//! call handle_messages(peer) for recieving from one peer
//! call send_handshake(sock, my_id) to send a handshake to a peer
//! call 
use crate::peers::Peers;

use super::peers::Peer;
use super::torrent;
use bitvec::prelude::*;

use byteorder::ByteOrder;
use mio::net::TcpStream;
use std::io::Read;
use std::io::Write;

use byteorder::BigEndian;

use anyhow::{Result, Error};

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
    let sock = peer.get_mut_socket();

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
        // 0 length is a keep alive
        0 => {
            buf.drain(0..4);
            MessageType::KeepAlive
        }
        // 1 length has 4 options (i guess could be a bitfield with no data but that would be weird and throwing it out is ok)
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
            // length 5 and messageID 4, must be a Have message
            if buf[4] == 4 && n == 5 {
                let index = BigEndian::read_u32(&buf[5..9]);
                buf.drain(0..9);
                MessageType::Have {
                    index: index as usize,
                }
            } else if (buf[4] == 8 || buf[4] == 6) && n == 13 {
                // length 13 and messageID 6 or 8, must be a Request or Cancel message
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
                // this is a piece message smile
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
                //found a bitfield message
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

//TODO make sure alex is okay calling handshake separatley than the rest of the messages
/// called right after we created a new peer
/// sends the initial handshake
pub fn send_handshake(sock: &mut TcpStream, my_id: &[u8; 20]) -> Result<()> {
    let mut buf: Vec<u8> = vec![0; 68];
    buf[0] = 19;
    buf[1..20].copy_from_slice(b"BitTorrent protocol");
    buf[28..48].copy_from_slice(&torrent::get_info_hash());
    buf[48..68].copy_from_slice(my_id);
    sock.write_all(&buf)?;
    Ok(())
}

/// called when we recieve a handshake (the first message ever sent from a user)
/// TODO make sure alex knows to clal this separately
/// TODO make this handle partial recieves but its annoying and insanely unlikely for a handshake
/// returns the peer id of the peer that sent the handshake
pub fn recv_handshake(peer:&mut Peer) -> Result<Vec<u8>> {
    let sock = peer.get_mut_socket();
    
    let mut buf: Vec<u8> = vec![0; 68];
    sock.read_exact(&mut buf)?;

    if buf[0] != 19 {
        println!("broken handshake");
        return Err(Error::msg("handshake invalid length"));
    }
    if &buf[1..20] != b"BitTorrent protocol" {
        println!("broken handshake");
        return Err(Error::msg("handshake invalid protocol"));
    }
    if &buf[28..48] != torrent::get_info_hash().as_slice() {
        println!("broken handshake");
        return Err(Error::msg("handshake invalid info hash"));
    }
    return Ok(buf[48..68].to_vec());
}

#[cfg(test)]
mod test {
    use super::*;
    use mio::net::{TcpListener, TcpStream};
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
    fn test_p2p_handshakes()->Result<()> {
        // Set up networking.
        let (self_sock, mut other_sock) = networking_setup(8001);

        // Create a peer, give it the TcpStream, and then see if the stream
        // can be written to and read from.
        let mut peer = Peer::new(&[b'a'; 20], self_sock);
        let get_sock = peer.get_mut_socket();

        //send handshake
        send_handshake(&mut other_sock, &[b'a'; 20])?;
        //recv handshake
        dbg!(recv_handshake(&mut peer).unwrap());


        Ok(())
    }

    #[test]
    fn test_p2p_send_msgs()->Result<()> {
        // Set up networking.
        let (self_sock, mut other_sock) = networking_setup(8002);

        // Create a peer, give it the TcpStream, and then see if the stream
        // can be written to and read from.
        let mut peer = Peer::new(&[b'a'; 20], self_sock);
        let get_sock = peer.get_mut_socket();

        //send handshake
        send_handshake(&mut other_sock, &[b'a'; 20])?;
        //recv handshake
        dbg!(recv_handshake(&mut peer).unwrap());

        // // Write
        // let string = b"test";
        // dbg!(get_sock.write(string).unwrap());
        // let mut buf: [u8; 4] = [0; 4];
        // dbg!(other_sock.read(&mut buf).unwrap());
        // dbg!(std::str::from_utf8(&buf).unwrap());
        // assert_eq!(string, &buf);

        // // Read
        // let string = b"helo";
        // dbg!(other_sock.write(string).unwrap());
        // dbg!(get_sock.read(&mut buf).unwrap());
        // dbg!(std::str::from_utf8(&buf).unwrap());
        // assert_eq!(string, &buf);

        Ok(())
    }
}
