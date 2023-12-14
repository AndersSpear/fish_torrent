#![warn(missing_docs)]
//! this is responsible for low level p2p communication
//! send messages to peers, recieve messages from peers, does not handle the logic of what to do with the messages.
//! call send_all(peers) for sending to all peers
//! call handle_messages(peer) for recieving from one peer
//! call send_handshake(sock, my_id) to send a handshake to a peer
//! call recv_handshake(peer)
use crate::file::OutputFile;
use crate::peers::{Peer, Peers};

use crate::torrent;
use bitvec::prelude::*;

use byteorder::ByteOrder;
use mio::net::TcpStream;
use std::io::Read;
use std::io::Write;

use byteorder::BigEndian;

use anyhow::{Error, Result};

#[derive(Debug, Default, Clone)]
pub struct Messages {
    pub messages: Vec<MessageType>,
}

/// A little added enum with associated data structs from Tien :)
/// length is ususlaly 16Kib, 2^14
#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {
        index: u32,
    },
    Bitfield {
        field: BitVec<u8, Msb0>,
    }, // BitVec is a bitvector from the bitvec crate
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
    KeepAlive, // KeepAlive is last because it does not have an associated
               // id in the protocol. This way choke starts at id 0.
               //Port // DHT Tracker is not supported, so this msg is not handled.
}

/// sends all messages in the peers struct
pub fn send_all(peers: &mut Peers) -> Result<(), Error> {

    for (_, peer) in peers.get_peers_list() {
        let msgs = peer.messages.clone();
        peer.messages = Messages::new();
        msgs.send_messages(peer.get_mut_socket())?;
    }
    Ok(())
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

        let mut sendbuf:Vec<u8> = vec![];

        for msg in self.messages {
            msg.send(&mut sendbuf)?;
        }
        sock.write_all(&sendbuf)?;
        Ok(())
    }
}

//TODO look at types of send failures
/// called when socket triggers, pass in a peer that got triggered
pub fn handle_messages(peer: &mut Peer) -> Result<()> {
    let mut return_msgs = Messages { messages: vec![] };

    let mut local_buf = vec![];
    let sock = peer.get_mut_socket();

    //let readcount = match sock.read_to_end(&mut local_buf) {
    //    Ok(n) => n,
    //    Err(e) => {
    //        println!("Error reading from socket: {}", e);
    //        0
    //    }
    //};
    if let Err(e) = sock.read_to_end(&mut local_buf) {
        // If the expect WouldBlock for partial read does not occur,
        // then some other IO error occurred!
        if e.kind() != std::io::ErrorKind::WouldBlock {
            return Err(e.into());
        }
    }
    println!("read {} bytes from socket", local_buf.len());

    let mut buf = &mut peer.recv_buffer;
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

    //peer.set_messages(return_msgs);
    peer.messages = return_msgs;
    Ok(())
}

// TODO make sure this handles handshakes smile
/// tries to parse one message from the buffer
fn parse_message(buf: &mut Vec<u8>) -> Option<MessageType> {
    dbg!(buf.len());

    if buf.len() < 4 {
        dbg!("less than 4 bytes in buffer");
        return None;
    }
    let len = BigEndian::read_u32(&buf[0..4]);
    dbg!(len);
    //this could break if buf[0..4] gets corrupted and is big
    if buf.len() < len as usize + 4 {
        dbg!("not enough bytes in buffer");
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
            dbg!(buf[4]);
            match buf[4] {
                4 => {
                    if n == 5 {
                        let index = BigEndian::read_u32(&buf[5..9]);
                        buf.drain(0..9);
                        MessageType::Have { index }
                    } else {
                        return None;
                    }
                }
                5 => {
                    let field = BitVec::from_vec(buf[5..].to_vec());
                    buf.drain(0..5);
                    buf.drain(0..field.len());
                    MessageType::Bitfield { field }
                }
                6 => {
                    if n != 13 {
                        return None;
                    };
                    // length 13 and messageID 6 or 8, must be a Request or Cancel message
                    let index = BigEndian::read_u32(&buf[5..9]);
                    let begin = BigEndian::read_u32(&buf[9..13]);
                    let length = BigEndian::read_u32(&buf[13..17]);
                    buf.drain(0..17);

                    MessageType::Request {
                        index,
                        begin,
                        length,
                    }
                }
                7 => {
                    if n <= 9 {
                        return None;
                    };
                    let index = BigEndian::read_u32(&buf[5..9]);
                    let begin = BigEndian::read_u32(&buf[9..13]);
                    let block = buf[13..].to_vec();
                    buf.drain(0..13);
                    buf.drain(0..block.len());
                    MessageType::Piece {
                        index,
                        begin,
                        block,
                    }
                }
                8 => {
                    if n != 13 {
                        return None;
                    };
                    // length 13 and messageID 6 or 8, must be a Request or Cancel message
                    let index = BigEndian::read_u32(&buf[5..9]);
                    let begin = BigEndian::read_u32(&buf[9..13]);
                    let length = BigEndian::read_u32(&buf[13..17]);
                    buf.drain(0..17);

                    MessageType::Cancel {
                        index,
                        begin,
                        length,
                    }
                }
                _ => {
                    println!("malformed message id, clearing buffer");
                    buf.clear();
                    return None;
                }
            }
        }
    })
}

impl MessageType {
    fn send(self, buf: &mut Vec<u8>) -> Result<(), Error> {
        match self {
            MessageType::Choke => {
                send_len_id(buf, 1, 0)?;
            }
            MessageType::Unchoke => {
                send_len_id(buf, 1, 1)?;
            }
            MessageType::Interested => {
                send_len_id(buf, 1, 2)?;
            }
            MessageType::NotInterested => {
                send_len_id(buf, 1, 3)?;
            }
            MessageType::Have { index } => {
                send_have(buf, index)?;
            }
            MessageType::Bitfield { field } => {
                send_bitfield(buf, field)?;
            }
            MessageType::Request {
                index,
                begin,
                length,
            } => {
                send_request_or_cancel(buf, true, index, begin, length)?;
            }
            MessageType::Piece {
                index,
                begin,
                block,
            } => {
                send_piece(buf, index, begin, block)?;
            }
            MessageType::Cancel {
                index,
                begin,
                length,
            } => {
                send_request_or_cancel(buf, false, index, begin, length)?;
            }
            MessageType::KeepAlive => {
                let b:[u8;4] = [0; 4];
                buf.write_all(&b)?;
            }
        }
        Ok(())
    }
}
fn send_len_id(sendbuf: &mut Vec<u8>, len: u32, id: u8) -> Result<(), Error> {
    let mut buf = vec![0; 5];
    buf[0..4].copy_from_slice(&len.to_be_bytes());
    buf[4] = id;
    sendbuf.append(&mut buf);
    Ok(())
}

fn send_have(sock: &mut Vec<u8>, index: u32) -> Result<(), Error> {
    let mut buf = vec![0; 9];
    buf[0..4].copy_from_slice(&5_u32.to_be_bytes());
    buf[4] = 4; // message id 4 is have
    buf[5..9].copy_from_slice(&index.to_be_bytes());
    sock.write_all(&buf)?;
    Ok(())
}

fn send_bitfield(sock: &mut Vec<u8>, mut field: BitVec<u8, Msb0>) -> Result<(), Error> {
    //bitvec manipulation
    field.force_align();
    field.set_uninitialized(false);

    //dbg!(&field);
    let vecfield = field.into_vec();

    // TODO make sure length is in bytes not bits
    let length = vecfield.len() as u32;

    let mut buf = vec![0; 5];
    buf[0..4].copy_from_slice(&(length + 1).to_be_bytes());
    buf[4] = 5; // message id 5 is bitfield

    buf.extend(vecfield);
    sock.write_all(&buf)?;
    assert_eq!(buf.len(), length as usize + 5);
    Ok(())
}

/// if second argument is true, send a request, else send a cancel
fn send_request_or_cancel(
    sock: &mut Vec<u8>,
    is_request_message: bool,
    index: u32,
    begin: u32,
    length: u32,
) -> Result<(), Error> {
    let mut buf = vec![0; 18];
    buf[0..4].copy_from_slice(&13_u32.to_be_bytes());
    buf[4] = if is_request_message { 6 } else { 8 }; // message id 6 is request, 8 is cancel
    buf[5..9].copy_from_slice(&index.to_be_bytes());
    buf[9..13].copy_from_slice(&begin.to_be_bytes());
    buf[13..17].copy_from_slice(&length.to_be_bytes());
    sock.write_all(&buf)?;
    Ok(())
}

fn send_piece(sock: &mut Vec<u8>, index: u32, begin: u32, block: Vec<u8>) -> Result<(), Error> {
    let length = block.len() as u32;

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
pub fn send_handshake(peer: &mut Peer, my_id: &[u8; 20], file: &OutputFile) -> Result<()> {
    let sock = peer.get_mut_socket();
    let mut buf: Vec<u8> = vec![0; 68];
    buf[0] = 19;
    buf[1..20].copy_from_slice(b"BitTorrent protocol");
    buf[28..48].copy_from_slice(&torrent::get_info_hash());
    buf[48..68].copy_from_slice(my_id);
    sock.write_all(&buf)?;

    //TODO GET THE BITFIELD
    peer.messages.messages.push(MessageType::Bitfield {
        field: file.get_file_bitfield(),
    });

    Ok(())
}

/// called when we recieve a handshake (the first message ever sent from a user)
/// TODO make sure alex knows to clal this separately
/// TODO make this handle partial recieves but its annoying and insanely unlikely for a handshake
/// returns the peer id of the peer that sent the handshake
pub fn recv_handshake(sock: &mut TcpStream) -> Result<Vec<u8>> {
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
    use bitvec::slice::BitSlice;
    use mio::net::{TcpListener, TcpStream};
    use rusty_fork::rusty_fork_test;
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

    rusty_fork_test! {
        #[test]
        fn test_p2p_handshakes() {
            // Set up networking.
            let (mut self_sock, mut other_sock) = networking_setup(8001);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            //let mut peer = Peer::new(self_sock);
            let mut other_peer = Peer::new(other_sock);
            //let get_sock = peer.get_mut_socket();

            //init torrent struct with infohash
            torrent::parse_torrent_file("../artofwar.torrent");

            let file = OutputFile::new("p2p.rs.test", 5, 5).unwrap();
            //send handshake
            send_handshake(&mut other_peer, &[b'a'; 20], &file).unwrap();
            //recv handshake
            recv_handshake(&mut self_sock).unwrap();
        }
    }

    rusty_fork_test! {
        #[test]
        // makes sure theres no error sending, does not validate contents yet
        fn test_p2p_send_msgs() {
           // Set up networking.
           let (self_sock, other_sock) = networking_setup(8002);

           // Create a peer, give it the TcpStream, and then see if the stream
           // can be written to and read from.
           let mut sender = Peer::new(self_sock);

           torrent::parse_torrent_file("../artofwar.torrent");

           let my_id = &[b'a'; 20];

           let choke = MessageType::Choke;
           let unchoke = MessageType::Unchoke;
           let interested = MessageType::Interested;
           let not_interested = MessageType::NotInterested;
           let have = MessageType::Have { index: 23 };

           let bv:BitVec<u8, Msb0> = BitVec::from_bitslice(bits![u8, Msb0; 0, 1, 0, 1, 0, 0, 1]);
           let bitfield = MessageType::Bitfield {
               field: bv,
           };
           let request = MessageType::Request {
               index: 1,
               begin: 2,
               length: 3,
           };
           let piece = MessageType::Piece {
               index: 4,
               begin: 5,
               block: vec![6; 7],
           };
           let cancel = MessageType::Cancel {
               index: 8,
               begin: 9,
               length: 10,
           };
           let keep_alive = MessageType::KeepAlive;


           sender.messages.messages.push(choke);
           sender.messages.messages.push(unchoke);
           sender.messages.messages.push(interested);
           sender.messages.messages.push(not_interested);
           sender.messages.messages.push(have);
           sender.messages.messages.push(bitfield);
           sender.messages.messages.push(request);
           sender.messages.messages.push(piece);
           sender.messages.messages.push(cancel);
           sender.messages.messages.push(keep_alive);


           let messages = sender.messages.clone();
           sender.reset_messages();

           let get_sock = sender.get_mut_socket();
           messages.send_messages(get_sock).unwrap();

        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_have() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8005);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let have = MessageType::Have { index: 23 };
            sender.get_mut_messages().messages.push(have.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], have);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_bitfield() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8006);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let bv:BitVec<u8, Msb0> = BitVec::from_bitslice(bits![u8, Msb0; 0, 1, 0, 1, 0, 0, 1,1,1]);
            let bitfield = MessageType::Bitfield {
                field: bv,
            };
            sender.get_mut_messages().messages.push(bitfield.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], bitfield);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_request() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8007);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let request = MessageType::Request {
                index: 1,
                begin: 2,
                length: 3,
            };
            sender.get_mut_messages().messages.push(request.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], request);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_piece() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8008);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let piece = MessageType::Piece {
                index: 4,
                begin: 5,
                block: vec![6; 7],
            };
            sender.get_mut_messages().messages.push(piece.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], piece);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_cancel() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8009);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let cancel = MessageType::Cancel {
                index: 8,
                begin: 9,
                length: 10,
            };
            sender.get_mut_messages().messages.push(cancel.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], cancel);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_keepalive() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8010);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let keep_alive = MessageType::KeepAlive;
            sender.get_mut_messages().messages.push(keep_alive.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], keep_alive);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_choke() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8011);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let choke = MessageType::Choke;
            sender.get_mut_messages().messages.push(choke.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], choke);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_unchoke() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8012);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let unchoke = MessageType::Unchoke;
            sender.get_mut_messages().messages.push(unchoke.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], unchoke);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_interested() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8013);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let interested = MessageType::Interested;
            sender.get_mut_messages().messages.push(interested.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], interested);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_send_recv_not_interested() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8014);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);

            torrent::parse_torrent_file("../artofwar.torrent");

            let not_interested = MessageType::NotInterested;
            sender.get_mut_messages().messages.push(not_interested.clone());

            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            assert_eq!(recieved_messages.messages[0], not_interested);
        }
    }

    rusty_fork_test! {
        #[test]
        fn test_p2p_recv_all() {
            // Set up networking.
            let (self_sock, other_sock) = networking_setup(8003);

            // Create a peer, give it the TcpStream, and then see if the stream
            // can be written to and read from.
            let mut sender = Peer::new(self_sock);
            let mut reciever = Peer::new(other_sock);


            torrent::parse_torrent_file("../artofwar.torrent");

            let my_id = &[b'a'; 20];

            let choke = MessageType::Choke;
            let unchoke = MessageType::Unchoke;
            let interested = MessageType::Interested;
            let not_interested = MessageType::NotInterested;
            let have = MessageType::Have { index: 23 };

            let bv:BitVec<u8, Msb0> = BitVec::from_bitslice(bits![u8, Msb0; 0, 1, 0, 1, 0, 0, 1,1,1]);
            let bitfield = MessageType::Bitfield {
                field: bv,
            };
            let request = MessageType::Request {
                index: 1,
                begin: 2,
                length: 3,
            };
            let piece = MessageType::Piece {
                index: 4,
                begin: 5,
                block: vec![6; 7],
            };
            let cancel = MessageType::Cancel {
                index: 8,
                begin: 9,
                length: 10,
            };
            let keep_alive = MessageType::KeepAlive;


            sender.get_mut_messages().messages.push(choke.clone());
            sender.get_mut_messages().messages.push(unchoke.clone());
            sender.get_mut_messages().messages.push(interested.clone());
            sender.get_mut_messages().messages.push(not_interested.clone());
            sender.get_mut_messages().messages.push(have.clone());
            sender.get_mut_messages().messages.push(bitfield.clone());
            sender.get_mut_messages().messages.push(request.clone());
            sender.get_mut_messages().messages.push(piece.clone());
            sender.get_mut_messages().messages.push(cancel.clone());
            sender.get_mut_messages().messages.push(keep_alive.clone());


            let messages = sender.get_messages_clone();
            sender.reset_messages();

            let get_sock = sender.get_mut_socket();
            messages.send_messages(get_sock).unwrap();

            dbg!(reciever.get_mut_messages());
            handle_messages(&mut reciever).unwrap();
            let recieved_messages = reciever.get_messages_clone();
            dbg!(reciever.get_mut_messages());

            assert_eq!(recieved_messages.messages[0], choke);
            assert_eq!(recieved_messages.messages[1], unchoke);
            assert_eq!(recieved_messages.messages[2], interested);
            assert_eq!(recieved_messages.messages[3], not_interested);
            assert_eq!(recieved_messages.messages[4], have);
            assert_eq!(recieved_messages.messages[5], bitfield);
            assert_eq!(recieved_messages.messages[6], request);
            assert_eq!(recieved_messages.messages[7], piece);
            assert_eq!(recieved_messages.messages[8], cancel);
            assert_eq!(recieved_messages.messages[9], keep_alive);
            assert_eq!(recieved_messages.messages.len(), 10);

        }
    }
}
