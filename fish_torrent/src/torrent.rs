#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]
#![warn(missing_docs)]
//! parses the .torrent file
//!

use bendy::decoding::{Error, FromBencode, Object};
use std::fs::read;

static mut TORRENT: Torrent = Torrent {
    announce: String::new(),
    info_hash: Vec::new(),
    piece_length: 0,
    number_of_pieces: 0,
    pieces: Vec::new(),
    length: 0,
    name: String::new(),
};

struct Torrent {
    announce: String,      // url of the tracker
    info_hash: Vec<u8>,    // 20 byte SHA1 hash value of info dictionary
    piece_length: u32,     // number of bytes per piece
    number_of_pieces: u32, // number of pieces
    pieces: Vec<Vec<u8>>,  // 20 byte SHA1 hash value of each piece
    length: u32,           // number of bytes of the file
    name: String, // name of the file, name of the suggested directory if multiple file mode
}

/// Parses the .torrent file
/// unsafe because it modifies a static variable
pub fn parse_torrent_file(filename: &str) -> Result<(), Error> {
    let contents: Vec<u8> = read(filename)?;
    unsafe {
        TORRENT = Torrent::from_bencode(&contents)?;
    }
    Ok(())
}


impl FromBencode for Torrent {
    const EXPECTED_RECURSION_DEPTH: usize = 1;

    fn decode_bencode_object(object: Object) -> Result<Self, Error> {
        unimplemented!("Torrent::decode_bencode_object");

        let mut announce = None;
        let mut info_hash = None;
        let mut piece_length = None;
        let mut pieces = None;
        let mut length = None;
        let mut name = None;

        // let mut dict: DictDecoder<'_, '_> = object.try_into_dictionary();

        // bruh this is fucking hard

        // while let Some(pair) = dict.next_pair()? {
        //     match pair {
        //         (b"announce", value) => {
        //             announce = String::decode_bencode_object(value)
        //                 .context("announce")
        //                 .map(Some)?;
        //         },
        //         (b"info", value) => {
        //             info_hash = value.
        //             infodict = String::decode_bencode_object(value)
        //                 .context("label")
        //                 .map(Some)?;
        //         },
        //         (unknown_field, _) => {
        //             return Err(Error::unexpected_field(String::from_utf8_lossy(
        //                 unknown_field,
        //             )));
        //         },
        //     }
        // }

        // let announce = announce.ok_or_else(|| Error::missing_field("counter"))?;
        // let label= label.ok_or_else(|| Error::missing_field("label"))?;

        let announce = announce.ok_or_else(|| Error::missing_field("announce"))?;
        let info_hash = info_hash.ok_or_else(|| Error::missing_field("info"))?;
        let piece_length = piece_length.ok_or_else(|| Error::missing_field("piece length"))?;
        let pieces: Vec<Vec<u8>> = pieces.ok_or_else(|| Error::missing_field("pieces"))?;
        let length = length.ok_or_else(|| Error::missing_field("length"))?;
        let name = name.ok_or_else(|| Error::missing_field("name"))?;

        let number_of_pieces = pieces.len() as u32;

        Ok(Torrent {
            announce,
            info_hash,
            piece_length,
            pieces,
            length,
            name,
            number_of_pieces,
        })
    }
}

/// 20 byte SHA1 hashvalue of the swarm
pub fn get_info_hash() -> &'static Vec<u8> {
    unsafe { &TORRENT.info_hash }
}

/// url of the tracker
pub fn get_tracker_url() -> &'static String {
    unsafe { &TORRENT.announce }
}

/// length of each piece in bytes
pub fn get_piece_length() -> u32 {
    unsafe { TORRENT.piece_length }
}

/// number of pieces in the file
pub fn get_number_of_pieces() -> u32 {
    unsafe { TORRENT.number_of_pieces }
}

/// vector of 20 byte SHA1 hashes of each piece
/// each hash is a vector of 20 bytes
pub fn get_pieces() -> &'static Vec<Vec<u8>> {
    unsafe { &TORRENT.pieces }
}

/// file length in bytes
pub fn get_file_length() -> u32 {
    unsafe { TORRENT.length }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_function() {}
}
