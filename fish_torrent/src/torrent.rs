#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]
#![warn(missing_docs)]
//! parses the .torrent file
//!

use bendy::decoding::{Decoder, Object};
use bendy::serde::from_bytes;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs::read;
use std::sync::OnceLock;

static TORRENT: OnceLock<Torrent> = OnceLock::new();

/// part of the torrent struct so you know how to parse the data
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum TorrentMode {
    SingleFile, // if single file use Info field `info.length` for file length and `info.name` for file name
    MultipleFile, // if multiple file use Info field `info.files[index].length` and `info.files[index].path` to get name and path of each file and `name` for the directory name
}

impl Default for TorrentMode {
    fn default() -> Self {
        TorrentMode::SingleFile
    }
}

/// main torrent struct, is initilalized during parse_torrent_file
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct Torrent {
    announce: String, // url of the tracker (http or udp)
    info: Info,

    //non-encoded info, computed by me
    #[serde(default)]
    info_hash: Vec<u8>, // 20 byte SHA1 hashvalue of the swarm
    #[serde(default)]
    torrent_mode: TorrentMode, // single file or multiple file mode. tells you how to deal with the fields of Info (info.files or info.length and info.name)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct Info {
    #[serde(default)]
    length: u32, // number of bytes of the file
    name: String, // name of the file, name of the suggested directory if multiple file mode
    #[serde(rename = "piece length")]
    piece_length: u32, // number of bytes per piece
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>, // 20 byte SHA1 hash value of each piece, the files are concatenated in the order they appear in the files list, will need to split based on file length
    #[serde(default)]
    files: Vec<File>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct File {
    length: u32,       // length of the file in bytes
    path: Vec<String>, // list of UTF-8 encoded strings corresponding to subdirectory names, the last element is the file name
}

/// Parses the .torrent file
/// unsafe because it modifies a static variable
pub fn parse_torrent_file(filename: &str) {
    let contents = read(filename).expect("invalid .torrent filename");
    let torrent =
        from_bytes::<Torrent>(contents.as_slice()).expect("bruv debnencoding the .torrent failed");

    //in the morning ill figure out if this is actually pulling the right object, this mayu be getting the external struct, so ill need to recurse on it till i find *another* struct, and return that
    let mut decoder = Decoder::new(contents.as_slice());
    let infodata = 'outer: loop {
        match decoder.next_object() {
            Ok(Some(Object::Dict(mut d))) => loop {
                match d.next_pair() {
                    Ok(Some((b"info", Object::Dict(d)))) => {
                        break 'outer d.into_raw();
                    }
                    Ok(Some((_, _))) => (),
                    Ok(None) => break,
                    Err(e) => panic!("meow trying to gety/decode infohash failed: {}", e),
                }
            },
            _ => (),
        }
    }
    .expect("meow trying to gety/decode infohash failed");

    let mut hash = Sha1::new();
    hash.update(infodata);

    let torrent = Torrent {
        info_hash: hash.finalize().to_vec(),
        torrent_mode: {
            if torrent.info.files.len() > 0 {
                TorrentMode::MultipleFile
            } else {
                TorrentMode::SingleFile
            }
        },
        ..torrent
    };

    TORRENT.set(torrent).expect("Failed to set torrent");

    println!("announce: {}", TORRENT.get().unwrap().announce);
    println!("length: {}", TORRENT.get().unwrap().info.length);
    println!("name: {}", TORRENT.get().unwrap().info.name);
    println!("piece length: {}", TORRENT.get().unwrap().info.piece_length);
    println!("pieces vec: {:?}", TORRENT.get().unwrap().info.pieces);
    println!("infohash: {:?}", TORRENT.get().unwrap().info_hash);
}

/// 20 byte SHA1 hashvalue of the swarm
pub fn get_info_hash() -> &'static Vec<u8> {
    &TORRENT.get().unwrap().info_hash
}

/// url of the tracker
pub fn get_tracker_url() -> &'static String {
    &TORRENT.get().unwrap().announce
}

/// length of each piece in bytes
pub fn get_piece_length() -> u32 {
    TORRENT.get().unwrap().info.piece_length
}

/// number of pieces in the file
pub fn get_number_of_pieces() -> u32 {
    TORRENT.get().unwrap().info.pieces.len() as u32 / 20
}

/// vector of 20 byte SHA1 hashes of each piece
/// each hash is a vector of 20 bytes
pub fn get_pieces() -> &'static Vec<u8> {
    &TORRENT.get().unwrap().info.pieces
}

/// file length in bytes
pub fn get_file_length() -> u32 {
    TORRENT.get().unwrap().info.length
}

#[cfg(test)]
mod test {
    use rusty_fork::rusty_fork_test;
    rusty_fork_test! {
    #[test]
    fn test_parse_torrent_file() {
        super::parse_torrent_file("../artofwar.torrent");
        assert_eq!(
            super::TORRENT.get().unwrap().announce,
            "http://128.8.126.63:6969/announce"
        );
        assert_eq!(super::TORRENT.get().unwrap().info.length, 63371);
        assert_eq!(super::TORRENT.get().unwrap().info.name, "artofwar.txt");
        assert_eq!(super::TORRENT.get().unwrap().info.piece_length, 32768);
        assert_eq!(
            super::TORRENT.get().unwrap().info.pieces,
            hex::decode(
                "148C74D24BC89E9C7BC1EA97B354AA0DFAD7041BA7C239739231CC40A30879640C7C390BBEE8BFF8"
            )
            .unwrap()
        );
        assert_eq!(super::TORRENT.get().unwrap().info.files.len(), 0);
        assert_eq!(super::TORRENT.get().unwrap().info_hash.len(), 20);
        assert_eq!(
            super::TORRENT.get().unwrap().torrent_mode,
            super::TorrentMode::SingleFile
        );
        assert_eq!(
            super::TORRENT.get().unwrap().info_hash,
            hex::decode("a994e40f6c625f26834dfaafcb40d5c5f59fa648").unwrap()
        );
    }}

    rusty_fork_test! {
    #[test]
    fn test_all_the_accessors() {
        super::parse_torrent_file("../artofwar.torrent");
        assert_eq!(
            super::get_info_hash(),
            &hex::decode("a994e40f6c625f26834dfaafcb40d5c5f59fa648").unwrap()
        );
        assert_eq!(
            super::get_tracker_url(),
            &"http://128.8.126.63:6969/announce"
        );
        assert_eq!(super::get_piece_length(), 32768);
        assert_eq!(super::get_number_of_pieces(), 2);
        assert_eq!(
            super::get_pieces(),
            &hex::decode(
                "148C74D24BC89E9C7BC1EA97B354AA0DFAD7041BA7C239739231CC40A30879640C7C390BBEE8BFF8"
            )
            .unwrap()
        );
        assert_eq!(super::get_file_length(), 63371);
    }}
}
