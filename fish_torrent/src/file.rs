// handle file reading and writing
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::prelude::FileExt;
use std::sync::OnceLock;

use anyhow::{Error, Result};

use memmap2::MmapMut;

//static FILE: RwLock<Vec<Piece>> = RwLock::new(Vec::new());
//static FILE
//static mut FILE: Option<MmapMut> = None;
//static mut FILE: Option<File> = None;
//static mut NUM_PIECES: Option<usize> = None;
//static mut SIZE: Option<usize> = None;

struct OutputFile {
    file: File,
    num_pieces: usize,
    piece_size: usize,
}

impl OutputFile {
    fn new(name: &str, num_pieces: usize, piece_size: usize) -> Self {
        //let file = File::create(name).expect("awudh");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(name)
            .expect("File could not be opened for writing :(");

        //Space allocation
        //file.seek(SeekFrom::Start(piece_size.try_into().unwrap()))
        //    .unwrap();
        //file.write_all(&[0]).unwrap();
        //file.seek(SeekFrom::Start(0)).unwrap();

        OutputFile { file, num_pieces, piece_size }
        
        //unsafe {
        //    if let Some(file) = &FILE {
        //        panic!("What the fuck are you doing, file was set already");
        //    }
        //    else {
        //        //FILE = Some(MmapMut::map_mut(&file).expect("wacky"));
        //        FILE = Some(file);
        //        SIZE = Some(piece_size);
        //    }
        //}
    }
    // we just recieved a block, figure out what to do with it
    fn write_block(&mut self, index: usize, begin: usize, block: Vec<u8>) -> Result<()> {
        //unsafe {
        //    if let Some(file) = &mut FILE {
        //        if let Some(size) = &SIZE { //            if (begin + block.len()) >= *size { panic!("write_block called which goes over piece boundaries!") }
        //            //file[((index * size) + begin)..block.len()].copy_from_slice(&block);
        //            file.write_at(&block, ((index * size) + begin).try_into().unwrap());
        //        }

        //        file.flush().unwrap();
        //    }
        //}

        if (begin + block.len()) >= self.piece_size { return Err(Error::msg("begin + block len was larger than piece size!")); }
        self.file.write_at(&block, ((index * self.piece_size) + begin).try_into()?)?;
        self.file.flush()?;
        Ok(())
    }

    // we just recieved a request for a block, read it and send it out too?
    fn read_block(&self, index: usize, begin: usize, length: usize) -> Result<Vec<u8>> {
        //let mut buf: [u8; length];
        let mut buf = vec![0u8; length];
        //unsafe {
        //    if let Some(file) = &mut FILE {
        //        if let Some(size) = &SIZE {
        //            if begin > *size { panic!("read_block called with begin larger than size!") }
        //            file.read_at(&mut buf, ((index * size) + begin).try_into().unwrap());
        //        }
        //    }
        //}

        if begin > self.piece_size { return Err(Error::msg("begin was larger than piece size!")); }
        self.file.read_at(&mut buf, ((index * self.piece_size) + begin).try_into()?)?;
        Ok(Vec::from(buf))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_1() {
        let mut test_file = OutputFile::new("Hi", 5, 10);
        test_file.write_block(0, 0, Vec::from([b'a', b'b', b'c', b'd']));
        test_file.write_block(1, 0, Vec::from([b'x', b'y', b'z']));
        let test = test_file.read_block(0, 0, 4).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'a', b'b', b'c', b'd']));
        let test = test_file.read_block(1, 0, 3).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'x', b'y', b'z']));
        test_file.write_block(0, 5, Vec::from([b't', b'v']));
        let test = test_file.read_block(0, 5, 2).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b't', b'v']));
    }
}
