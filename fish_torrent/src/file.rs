// handle file reading and writing
//! # file.rs
//! `file.rs` handles the creation of, writing to, reading from, and hashing of
//! the target file of a torrent.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::prelude::FileExt;

use anyhow::{Error, Result};

struct OutputFile {
    file: File,
    num_pieces: usize,
    piece_size: usize,
}

impl OutputFile {
    /// Creates a new output file with the specified name, number of pieces, and
    /// piece size.
    /// The latter two arguments will be checked on any given read
    /// or write call.
    /// Returns None if the file was not able to be created for any reason.
    fn new(name: &str, num_pieces: usize, piece_size: usize) -> Option<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(name)
            .ok();

        if let Some(mut file) = file {
            // Allocates the file full of null bytes ahead of time.
            // This prevents possible panics from reading memory that has not
            // been written to yet.
            use std::io::{Seek, SeekFrom};
            file.seek(SeekFrom::Start(((num_pieces * piece_size) - 1).try_into().ok()?)).ok()?;
            file.write_all(&[0]).ok()?;
            file.seek(SeekFrom::Start(0)).ok()?;

            Some(OutputFile {
                file,
                num_pieces,
                piece_size,
            })
        }
        else {
            None
        }
    }

    /// Writes a block (Vector) of bytes to the specified piece index and
    /// beginning offset.
    /// Propagates any errors due to arguments or file i/o issues.
    fn write_block(&mut self, index: usize, begin: usize, block: Vec<u8>) -> Result<()> {
        if index >= self.num_pieces {
            Err(Error::msg("index was larger than or equal to num_pieces!"))
        } else if (begin + block.len()) > self.piece_size {
            Err(Error::msg("begin + block len was larger than piece size!"))
        } else if block.len() == 0 {
            Err(Error::msg("block is empty!"))
        }
        else {
            self.file
                .write_at(&block, ((index * self.piece_size) + begin).try_into()?)?;
            self.file.flush()?;
            Ok(())
        }
    }

    /// Reads a block (Vector) of bytes from the specified piece index and
    /// beginning offset.
    /// Propagates any errors due to arguments or file i/o issues.
    /// Panics if the file does not return the expected number of bytes
    /// as there should be no reason that it returns less bytes than expected.
    fn read_block(&self, index: usize, begin: usize, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];

        if index >= self.num_pieces {
            Err(Error::msg("index was larger than or equal to num_pieces!"))
        }
        // This math is a little confusing--begin is an index but length is not,
        // so an >= would cause a false error.
        // Ex. num_pieces 5 piece_size 10
        // index 4 begin 5 length 5. This should be valid, despite 5 + 5 = 10.
        else if begin + length > self.piece_size {
            Err(Error::msg("begin + length was larger than piece size!"))
        } else {
            let res = self.file
                .read_at(&mut buf, ((index * self.piece_size) + begin).try_into()?)?;
            if length != res { panic!("This should not have occurred. Inform Tien.") }
            Ok(buf)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    #[test]
    fn test_general() {
        let filename = "file.rs.test_general.output";
        let _ = fs::remove_file(filename);
        let mut test_file = OutputFile::new(filename, 5, 10).unwrap();
        // Write some data.
        test_file.write_block(0, 0, Vec::from([b'a', b'b', b'c', b'd']));
        test_file.write_block(1, 0, Vec::from([b'x', b'y', b'z']));
        // See if that data reads back.
        let test = test_file.read_block(0, 0, 4).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'a', b'b', b'c', b'd']));
        let test = test_file.read_block(1, 0, 3).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'x', b'y', b'z']));
        // Write some more data, this time at an offset. And read it back.
        test_file.write_block(0, 5, Vec::from([b't', b'v']));
        let test = test_file.read_block(0, 5, 2).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b't', b'v']));
        // Write data up to the very end of the params
        test_file.write_block(4, 8, Vec::from([b'a', b'b']));
        let test = test_file.read_block(4, 8, 2).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'a', b'b']));

        // Make sure that the file is the expected size.
        let metadata = fs::metadata(filename).unwrap();
        assert_eq!(50, metadata.len());
    }

    #[test]
    fn test_write_fail() {
        let filename = "file.rs.test_write_fail.output";
        let _ = fs::remove_file(filename);
        let mut test_file = OutputFile::new(filename, 5, 10).unwrap();
        // Write to bounds of file and file pieces. Expect errors.
        assert!(test_file.write_block(5, 0, Vec::from([b'a'])).is_err());
        assert!(test_file.write_block(0, 10, Vec::new()).is_err());
        assert!(test_file.write_block(0, 8, Vec::from([b'a', b'b', b'c'])).is_err());

        // Check to see if failed writes overwrite anything by accident.
        assert!(test_file.write_block(4, 5, Vec::from([b'a'])).is_ok());
        assert!(test_file.write_block(4, 5, Vec::from([b'x', b'b', b'c', b'd', b'e', b'f'])).is_err());
        let tmp = test_file.read_block(4, 5, 1).unwrap();
        assert_eq!(tmp, Vec::from([b'a']));
        let tmp = test_file.read_block(4, 9, 1).unwrap();
        assert_eq!(tmp, Vec::from([0]));

        // Make sure that the file is the expected size.
        let metadata = fs::metadata(filename).unwrap();
        assert_eq!(50, metadata.len());
    }

    #[test]
    fn test_read_fail() {
        let filename = "file.rs.test_read_fail.output";
        let _ = fs::remove_file(filename);
        let mut test_file = OutputFile::new(filename, 5, 10).unwrap();
        // Read to bounds of file and file pieces.
        // Reading past end of piece.
        assert!(test_file.read_block(0, 9, 1).is_ok());
        assert!(test_file.read_block(0, 9, 2).is_err());
        // Reading from too high of a piece index.
        assert!(test_file.read_block(4, 0, 0).is_ok());
        assert!(test_file.read_block(5, 0, 0).is_err());
        // Reading past end of piece due to length of read.
        assert!(test_file.read_block(0, 5, 5).is_ok());
        assert!(test_file.read_block(0, 5, 6).is_err());

        test_file.write_block(0, 5, Vec::from([b'a', b'b', b'c', b'd', b'e']));
        test_file.write_block(4, 5, Vec::from([b'v', b'w', b'x', b'y', b'z']));
        let tmp = test_file.read_block(0, 5, 5).unwrap();
        assert_eq!(tmp, Vec::from([b'a', b'b', b'c', b'd', b'e']));
        let tmp = test_file.read_block(4, 5, 5).unwrap();
        assert_eq!(tmp, Vec::from([b'v', b'w', b'x', b'y', b'z']));

        // Make sure that the file is the expected size.
        let metadata = fs::metadata(filename).unwrap();
        assert_eq!(50, metadata.len());
    }
}
