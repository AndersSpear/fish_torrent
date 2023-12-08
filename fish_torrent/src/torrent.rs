#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]
#![warn(missing_docs)]
//! parses the .torrent file
//!

use bendy::decoding::Error;
use bendy::serde::from_bytes;
use serde::{Deserialize, Serialize, Deserializer};
use std::fs::read;
use sha1::{Sha1, Digest};

static mut TORRENT: Torrent = Torrent {
    announce: String::new(),
    info: Info {
        length: 0,
        name: String::new(),
        piece_length: 0,
        pieces: Vec::new(),
        files: Vec::new(),
    },
    info_hash: Vec::new(),
    torrent_mode: TorrentMode::SingleFile,
};

/// part of the torrent struct so you know how to parse the data
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum TorrentMode {
    SingleFile, // if single file use Info field `info.length` for file length and `info.name` for file name
    MultipleFile, // if multiple file use Info field `info.files[index].length` and `info.files[index].path` to get name and path of each file and `name` for the directory name
}

impl Default for TorrentMode {
    fn default() -> Self { TorrentMode::SingleFile }
}

/// main torrent struct, is initilalized during parse_torrent_file
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Torrent {
    announce: String,      // url of the tracker (http or udp)
    info: Info,

    //non-encoded info, computed by me
    #[serde(default)]
    info_hash: Vec<u8>,    // 20 byte SHA1 hashvalue of the swarm
    #[serde(default)]
    torrent_mode: TorrentMode, // single file or multiple file mode
}


#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Info {
    #[serde(default)]
    length: u32,           // number of bytes of the file
    name: String, // name of the file, name of the suggested directory if multiple file mode
    #[serde(rename = "piece length")]
    piece_length: u32,     // number of bytes per piece
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>,  // 20 byte SHA1 hash value of each piece, the files are concatenated in the order they appear in the files list, will need to split based on file length
    #[serde(default)]
    files: Vec<File>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct File {
    length: u32,       // length of the file in bytes
    path: Vec<String>, // list of UTF-8 encoded strings corresponding to subdirectory names, the last element is the file name
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct InfoHashStruct{
    #[serde(alias = "info")]
    #[serde(deserialize_with = "set_info_hash")]
    info_hash: Vec<u8>,
}

pub fn set_info_hash<'de, D>(d: Deserializer<'de>) -> Result<Vec<u8>, Error> {
    let mut hasher = Sha1::new();
    hasher.update(d.into_bytes()?);
    let result = hasher.finalize();
    Ok(vec!(result))
}


/// Parses the .torrent file
/// unsafe because it modifies a static variable
pub fn parse_torrent_file(filename: &str) -> Result<(), Error> {
    let contents = read(filename)?;
    unsafe { TORRENT = from_bytes::<Torrent>(contents.as_slice())?; }
    unsafe { TORRENT.info_hash = from_bytes(&from_bytes::<InfoHashStruct>(contents.as_slice())?.info_hash)?; }

    unsafe{
    println!("announce: {}", TORRENT.announce);
    println!("length: {}", TORRENT.info.length);
    println!("name: {}", TORRENT.info.name);
    println!("piece length: {}", TORRENT.info.piece_length);
    println!("pieces vec: {:?}", TORRENT.info.pieces);
    }
    Ok(())
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
    unsafe { TORRENT.info.piece_length }
}

/// number of pieces in the file
pub fn get_number_of_pieces() -> u32 {
    unsafe { TORRENT.info.pieces.len() as u32 }
}

/// vector of 20 byte SHA1 hashes of each piece
/// each hash is a vector of 20 bytes
pub fn get_pieces() -> &'static Vec<u8> {
    unsafe { &TORRENT.info.pieces }
}

/// file length in bytes
pub fn get_file_length() -> u32 {
    unsafe { TORRENT.info.length }
}



#[cfg(test)]
mod test {
    #[test]
    fn test_function() {}
}