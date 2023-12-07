// handle file reading and writing
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

use memmap2::MmapMut;

use std::sync::RwLock;

//static FILE: RwLock<Vec<Piece>> = RwLock::new(Vec::new());
//static FILE
static mut FILE: Option<MmapMut> = None;
static mut SIZE: Option<usize> = None;

type Piece = Vec<u8>;

fn init_file(name: &str, piece_size: usize) {
    //let file = File::create(name).expect("awudh");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(name)
        .expect("wow");

    //Space allocation
    file.seek(SeekFrom::Start(piece_size.try_into().unwrap())).unwrap();
    file.write_all(&[0]).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();

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

            file.flush().unwrap();
        }
    }
}

// we just recieved a request for a block, read it and send it out too?
fn read_block(){}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn write_test_1() {
        init_file("Hi", 10);
        write_block(0, 0, Vec::from([b'a', b'b', b'c', b'd']));
    }
}
