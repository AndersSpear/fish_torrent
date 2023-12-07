// handle file reading and writing
use std::fs::File;
use std::io::prelude::*;

use memmap2::MmapMut;

use std::sync::RwLock;

//static FILE: RwLock<Vec<Piece>> = RwLock::new(Vec::new());
//static FILE
static mut FILE: Option<MmapMut> = None;
static mut SIZE: Option<usize> = None;

type Piece = Vec<u8>;

fn file_init(name: &str, piece_size: usize) {
    let file = File::create(name).expect("awudh");
    unsafe {
        FILE = Some(MmapMut::map_mut(&file).expect("wacky"));
        SIZE = Some(piece_size);
    }
}
// we just recieved a block, figure out what to do with it
fn write_block(index: usize, begin: usize, block: Vec<u8>){
    unsafe {
        if let Some(file) = &mut FILE {
            if let Some(size) = SIZE {
                file[((index * size) + begin)..block.len()].copy_from_slice(&block);
            }
        }
    }
}

// we just recieved a request for a block, read it and send it out too?
fn read_block(){}
