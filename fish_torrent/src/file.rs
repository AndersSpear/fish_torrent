//! # file.rs
//! `file.rs` handles the creation of, writing to, reading from, and hashing of
//! the target file of a torrent.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::prelude::FileExt;

use bitvec::prelude::*;

use sha1::{Digest, Sha1};

use anyhow::{Error, Result};

pub const BLOCK_SIZE: usize = 16000; //bytes

pub struct OutputFile {
    file: File,
    length: usize,
    block_size: usize,
    // The two fields below are technically redundant but eh.
    num_pieces: usize,
    piece_size: usize,
    last_piece_size: usize,
    bytes: Vec<BitVec<u8, Msb0>>,
    blocks: Vec<BitVec<u8, Msb0>>,
    pieces: BitVec<u8, Msb0>,
}

impl OutputFile {
    /// Creates a new output file with the specified name, number of pieces, and
    /// piece size.
    /// The latter two arguments will be checked on any given read
    /// or write call.
    /// Returns None if the file was not able to be created for any reason.
    pub fn new(
        name: &str,
        length: usize,
        num_pieces: usize,
        piece_size: usize,
        block_size: usize,
    ) -> Option<Self> {
        let file = OpenOptions::new()
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
            file.seek(SeekFrom::Start((length - 1).try_into().ok()?))
                .ok()?;
            file.write_all(&[0]).ok()?;
            file.seek(SeekFrom::Start(0)).ok()?;

            Some(OutputFile {
                file,
                length,
                block_size,
                num_pieces,
                piece_size,
                last_piece_size: length - ((num_pieces - 1) * piece_size),
                bytes: vec![bitvec![u8, Msb0; 0; piece_size]; num_pieces],
                blocks: vec![bitvec![u8, Msb0; 0; piece_size.div_ceil(block_size)]; num_pieces],
                pieces: bitvec![u8, Msb0; 0; num_pieces],
            })
        } else {
            None
        }
    }

    pub fn get_file_length(&self) -> usize {
        self.length
    }

    pub fn get_num_pieces(&self) -> usize {
        self.num_pieces
    }

    pub fn get_piece_size(&self) -> usize {
        self.piece_size
    }

    pub fn get_blocks(&self) -> Vec<BitVec<u8, Msb0>> {
        self.blocks.clone()
    }

    pub fn get_file_bitfield(&self) -> BitVec<u8, Msb0> {
        self.pieces.clone()
    }

    /// Writes a block (Vector) of bytes to the specified piece index and
    /// beginning offset.
    /// Returns true if this call to write_block finishes the piece specified by index.
    /// Propagates any errors due to arguments or file i/o issues.
    pub fn write_block(&mut self, index: usize, begin: usize, mut block: Vec<u8>) -> Result<bool> {
        if index >= self.num_pieces {
            Err(Error::msg("index was larger than or equal to num_pieces!"))
        } else if (begin + block.len()) > self.piece_size {
            Err(Error::msg("begin + block len was larger than piece size!"))
        } else if index == self.num_pieces - 1 && begin + block.len() > self.last_piece_size {
            Err(Error::msg(
                "begin + block len was larger than last piece size!",
            ))
        } else if block.len() == 0 {
            Err(Error::msg("block is empty!"))
        } else if begin % self.block_size != 0 {
            Err(Error::msg("begin was not aligned to block_size"))
        } else {
            if block.len() > self.block_size {
                println!(
                    "Received block size too large. Writing the first {} bytes...",
                    self.block_size
                );
                block.drain(self.block_size..);
            }

            // Write to file at specified location.
            self.file
                .write_at(&block, ((index * self.piece_size) + begin).try_into()?)?;
            self.file.flush()?;

            // Record the bytes written in pieces.
            for i in begin..(begin + block.len()) {
                self.bytes[index].set(i, true);
            }

            self.blocks[index].set(begin.div_ceil(self.block_size), true); //NOTE: Sus
            let finished = self.check_piece_finished(index)?;
            // Returns whether the piece was "finished" for hashing and using "set_piece_finished".
            //if finished == true {
            //    self.pieces.set(index, true);
            //}
            Ok(finished)
        }
    }

    /// Reads a block (Vector) of bytes from the specified piece index and
    /// beginning offset.
    /// Propagates any errors due to arguments or file i/o issues.
    /// Panics if the file does not return the expected number of bytes
    /// as there should be no reason that it returns less bytes than expected.
    pub fn read_block(&self, index: usize, begin: usize, length: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; length];

        if index >= self.num_pieces {
            Err(Error::msg("index was larger than or equal to num_pieces!"))
        } else if self.pieces[index] == false {
            Err(Error::msg("index to be read was not yet finished!"))
        } else if begin + length > self.piece_size {
            // This math is a little confusing--begin is an index but length is not,
            // so a >= would cause a false error.
            // Ex. num_pieces 5 piece_size 10
            // index 4 begin 5 length 5. This should be valid because we start writing at the 5th index.
            Err(Error::msg("begin + length was larger than piece size!"))
        } else if index == self.num_pieces - 1 && begin + length > self.last_piece_size {
            Err(Error::msg(
                "begin + length was larger than last piece size!",
            ))
        } else {
            let res = self
                .file
                .read_at(&mut buf, ((index * self.piece_size) + begin).try_into()?)?;
            if length != res {
                panic!("This should not have occurred. Inform Tien.")
            }
            Ok(buf)
        }
    }

    pub fn clear_piece(&mut self, index: usize) -> Result<()> {
        if index >= self.num_pieces {
            return Err(Error::msg(
                "Index greater than or equal to the number of pieces!",
            ));
        }

        for i in 0..self.blocks[index].len() {
            self.blocks[index].set(i, false);
        }

        self.pieces.set(index, false);
        Ok(())
    }

    pub fn set_piece_finished(&mut self, index: usize) -> Result<()> {
        if index >= self.num_pieces {
            return Err(Error::msg(
                "Index greater than or equal to the number of pieces!",
            ));
        }

        self.pieces.set(index, true);
        Ok(())
    }

    /// Compares the hash of a piece specified by the index argument
    /// and the 20-byte hash argument.
    pub fn compare_piece_hash(&self, index: usize, hash: &[u8; 20]) -> Result<bool> {
        if index >= self.num_pieces {
            return Err(Error::msg(
                "Index greater than or equal to the number of pieces!",
            ));
        }

        Ok(self.hash_piece(index)? == *hash)
    }

    /// Given a specific index, this function will read the corresponding piece
    /// from the file and return that pieces SHA1 hash.
    fn hash_piece(&self, index: usize) -> Result<[u8; 20]> {
        if index >= self.num_pieces {
            return Err(Error::msg(
                "Index greater than or equal to the number of pieces!",
            ));
        }

        let mut hash: [u8; 20] = [0; 20];
        let mut hasher = Sha1::new();
        if self.check_piece_finished(index)? == true {
            hasher.update(self.read_block(
                index,
                0,
                if index == self.num_pieces - 1 {
                    self.last_piece_size
                } else {
                    self.piece_size
                },
            )?);
            hasher.finalize_into((&mut hash).into());
            Ok(hash)
        } else {
            Err(Error::msg(
                "hash_piece() was called before the piece was finished!",
            ))
        }
    }

    pub fn is_block_finished(&self, index: usize, begin: usize) -> Option<bool> {
        if index >= self.num_pieces {
            return None;
        }
        // This handles if index or begin is too large.
        self.blocks[index]
            .get(begin.div_ceil(self.block_size))
            .as_deref()
            .copied()
    }

    /// Check to see if the piece was finished.
    fn check_piece_finished(&self, index: usize) -> Result<bool> {
        if index >= self.num_pieces {
            return Err(Error::msg(
                "Index greater than or equal to the number of pieces!",
            ));
        }

        let bound = if index == self.num_pieces - 1 {
            self.last_piece_size
        } else {
            self.piece_size
        };
        for i in 0..bound {
            let &bit = self.bytes[index].get(i).as_deref().expect(
                "Unknown edge case where OutputFile.pieces was not
                properly initialized or bounds were not properly checked.",
            );
            if bit == false {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    #[test]
    #[ignore]
    fn test_general() {
        let filename = "file.rs.test_general.output";
        let filesize = 23;
        let blocksize = 1;
        let _ = fs::remove_file(filename);
        // This results in an output file with 4 5-byte pieces and one 3-byte piece.
        let mut test_file = OutputFile::new(filename, filesize, 5, 5, 1).unwrap();
        // Write some data. MUST FILL PIECE OR READ WILL FAIL.
        _ = test_file.write_block(0, 0, Vec::from([b'a', b'b', b'c', b'd', b'e']));
        for i in 0..5 {
            _ = test_file.write_block(0, i, Vec::from([b'a']));
            _ = test_file.write_block(1, i, Vec::from([b'z']));
        }
        assert_eq!(test_file.check_piece_finished(0).unwrap(), true);
        test_file.set_piece_finished(0);
        assert_eq!(test_file.check_piece_finished(1).unwrap(), true);
        test_file.set_piece_finished(1);
        // See if that data reads back.
        let test = test_file.read_block(0, 0, 4).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'a', b'a', b'a', b'a']));
        let test = test_file.read_block(1, 0, 3).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'z', b'z', b'z']));
        // Write some more data, this time at an offset. And read it back.
        _ = test_file.write_block(0, 3, Vec::from([b't']));
        _ = test_file.write_block(0, 4, Vec::from([b'v']));
        let test = test_file.read_block(0, 3, 2).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b't', b'v']));
        // Write data up to the very end of the last piece
        _ = test_file.write_block(4, 0, Vec::from([b'a']));
        _ = test_file.write_block(4, 1, Vec::from([b'b']));
        _ = test_file.write_block(4, 2, Vec::from([b'c']));
        test_file.set_piece_finished(4);
        assert_eq!(test_file.check_piece_finished(4).unwrap(), true);
        let test = test_file.read_block(4, 0, 3).unwrap();
        dbg!(&std::str::from_utf8(&test).unwrap());
        assert_eq!(test, Vec::from([b'a', b'b', b'c']));

        // Make sure that the file is the expected size.
        let metadata = fs::metadata(filename).unwrap();
        assert_eq!(23, metadata.len());
    }

    #[test]
    #[ignore]
    fn test_write_fail() {
        let filename = "file.rs.test_write_fail.output";
        let _ = fs::remove_file(filename);
        let mut test_file = OutputFile::new(filename, 50, 5, 10, 1).unwrap();
        // Write to bounds of file and file pieces. Expect errors.
        assert!(test_file.write_block(5, 0, Vec::from([b'a'])).is_err());
        assert!(test_file.write_block(0, 10, Vec::new()).is_err());
        assert!(test_file
            .write_block(0, 8, Vec::from([b'a', b'b', b'c']))
            .is_err());

        // Check to see if failed writes overwrite anything by accident.
        assert!(test_file.write_block(4, 5, Vec::from([b'a'])).is_ok());
        assert!(test_file
            .write_block(4, 5, Vec::from([b'x', b'b', b'c', b'd', b'e', b'f']))
            .is_err());
        test_file.set_piece_finished(4); // Erroneously set piece to finished for testing lol
        let tmp = test_file.read_block(4, 5, 1).unwrap();
        assert_eq!(tmp, Vec::from([b'a']));
        let tmp = test_file.read_block(4, 9, 1).unwrap();
        assert_eq!(tmp, Vec::from([0]));
        assert!(test_file.write_block(4, 9, Vec::from([b'a'])).is_ok());
        let tmp = test_file.read_block(4, 9, 1).unwrap();
        assert_eq!(tmp, Vec::from([b'a']));

        // Make sure that the file is the expected size.
        let metadata = fs::metadata(filename).unwrap();
        assert_eq!(50, metadata.len());
    }

    #[test]
    #[ignore]
    fn test_read_fail() {
        let filename = "file.rs.test_read_fail.output";
        let _ = fs::remove_file(filename);
        let mut test_file = OutputFile::new(filename, 50, 5, 10, 1).unwrap();
        //Setting pieces to finished for testing
        test_file.set_piece_finished(0);
        test_file.set_piece_finished(4);
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

        for i in 5..10 {
            _ = test_file.write_block(0, i, Vec::from([97 + (i - 5) as u8]));
        }
        for i in 5..10 {
            _ = test_file.write_block(4, i, Vec::from([118 + (i - 5) as u8]));
        }
        let tmp = test_file.read_block(0, 5, 5).unwrap();
        assert_eq!(tmp, Vec::from([b'a', b'b', b'c', b'd', b'e']));
        let tmp = test_file.read_block(4, 5, 5).unwrap();
        assert_eq!(tmp, Vec::from([b'v', b'w', b'x', b'y', b'z']));

        // Make sure that the file is the expected size.
        let metadata = fs::metadata(filename).unwrap();
        assert_eq!(50, metadata.len());
    }

    #[test]
    #[ignore]
    fn test_bitvecs() {
        let filename = "file.rs.test_bitvec.output";
        let _ = fs::remove_file(filename);
        let num_pieces = 5;
        let piece_size = 10;
        let block_size = 1;
        let mut test_file = OutputFile::new(
            filename,
            num_pieces * piece_size,
            num_pieces,
            piece_size,
            block_size,
        )
        .unwrap();

        // Check to make sure that the BitVec intialized as expected.
        assert_eq!(test_file.bytes.len(), num_pieces);
        for i in &test_file.bytes {
            assert_eq!(i.len(), piece_size);
            for j in i {
                assert_eq!(j, false);
            }
        }

        // Write to file and check to make sure that BitVec matches the bytes written.
        //dbg!(&test_file.bytes[0]);
        //dbg!(&test_file.bytes[1]);
        // No writes should return true, as none of them fill the piece.
        assert_eq!(
            test_file
                .write_block(0, 0, Vec::from([b'a', b'b']))
                .unwrap(),
            false
        );
        test_file.write_block(0, 1, Vec::from([b'b']));
        assert_eq!(
            test_file
                .write_block(1, 0, Vec::from([b'x', b'y']))
                .unwrap(),
            false
        );
        test_file.write_block(1, 1, Vec::from([b'b']));
        assert_eq!(
            test_file
                .write_block(0, 5, Vec::from([b't', b'u', b'v']))
                .unwrap(),
            false
        );
        test_file.write_block(0, 6, Vec::from([b'u']));
        test_file.write_block(0, 7, Vec::from([b'v']));

        assert_eq!(test_file.bytes[0].get(0).as_deref().unwrap(), &true);
        assert_eq!(test_file.bytes[0].get(1).as_deref().unwrap(), &true);
        assert_eq!(test_file.bytes[0].get(5).as_deref().unwrap(), &true);
        assert_eq!(test_file.bytes[0].get(6).as_deref().unwrap(), &true);
        assert_eq!(test_file.bytes[0].get(7).as_deref().unwrap(), &true);
        assert_eq!(test_file.bytes[1].get(0).as_deref().unwrap(), &true);
        assert_eq!(test_file.bytes[1].get(1).as_deref().unwrap(), &true);
        //dbg!(&test_file.bytes[0]);
        //dbg!(&test_file.bytes[1]);

        // Random spot checks to make sure bits weren't randomly flipped.
        assert_eq!(test_file.bytes[0].get(2).as_deref().unwrap(), &false);
        assert_eq!(test_file.bytes[0].get(9).as_deref().unwrap(), &false);
        assert_eq!(test_file.bytes[1].get(2).as_deref().unwrap(), &false);
        assert_eq!(test_file.bytes[1].get(7).as_deref().unwrap(), &false);

        // This should fully fill the piece and return true.
        assert_eq!(test_file.get_file_bitfield()[1], false);
        for i in 0..piece_size - 1 {
            test_file.write_block(1, i, Vec::from([99 + i as u8]));
        }
        assert_eq!(
            test_file.write_block(1, 9, Vec::from([b'l'])).unwrap(),
            true
        );
        assert_eq!(test_file.check_piece_finished(1).unwrap(), true);
        test_file.set_piece_finished(1).unwrap();
        assert_eq!(test_file.get_file_bitfield()[1], true);
        //dbg!(&test_file.bytes[1]);
    }

    #[test]
    fn test_hash() {
        let filename = "file.rs.test_hash.output";
        let _ = fs::remove_file(filename);
        let num_pieces = 2;
        let piece_size = 5;
        let block_size = 1;
        let mut test_file = OutputFile::new(
            filename,
            num_pieces * piece_size,
            num_pieces,
            piece_size,
            block_size,
        )
        .unwrap();

        // Write a piece "abcde".
        assert_eq!(
            test_file
                .write_block(0, 0, Vec::from([b'a', b'b', b'c', b'd', b'e']))
                .unwrap(),
            true
        );
        // See if the hash produced is expected.
        assert_eq!(
            format!("{:02x?}", test_file.hash_piece(0).unwrap()),
            "[03, de, 6c, 57, 0b, fe, 24, bf, c3, 28, cc, d7, ca, 46, b7, 6e, ad, af, 43, 34]"
        );
        // See if the comparison function returns true as expected.
        assert_eq!(
            test_file
                .compare_piece_hash(
                    0,
                    &[
                        0x03, 0xde, 0x6c, 0x57, 0x0b, 0xfe, 0x24, 0xbf, 0xc3, 0x28, 0xcc, 0xd7,
                        0xca, 0x46, 0xb7, 0x6e, 0xad, 0xaf, 0x43, 0x34
                    ]
                )
                .unwrap(),
            true
        );
        // See if comparison returns false as expected.
        assert_eq!(
            test_file
                .compare_piece_hash(
                    0,
                    &[
                        0x04, 0xde, 0x6c, 0x57, 0x0b, 0xfe, 0x24, 0xbf, 0xc3, 0x28, 0xcc, 0xd7,
                        0xca, 0x46, 0xb7, 0x6e, 0xad, 0xaf, 0x43, 0x34
                    ]
                )
                .unwrap(),
            false
        );
    }

    #[test]
    #[ignore]
    fn test_bitvec_part_2() {
        let filename = "file.rs.bitvec_part_2.output";
        let _ = fs::remove_file(filename);
        let num_pieces = 2;
        let piece_size = 5;
        let block_size = 1;
        let mut test_file = OutputFile::new(
            filename,
            num_pieces * piece_size,
            num_pieces,
            piece_size,
            block_size,
        )
        .unwrap();

        // Assert that everything is initialized correctly.
        assert_eq!(
            test_file.get_file_bitfield(),
            BitVec::from_bitslice(bits![u8, Msb0; 0, 0])
        );
        assert_eq!(
            test_file.get_blocks()[0],
            BitVec::from_bitslice(bits![u8, Msb0; 0, 0, 0, 0, 0])
        );
        // This should drop a lot of bytes.
        _ = test_file.write_block(0, 0, Vec::from([b'a', b'b', b'c', b'd', b'e']));
        assert_eq!(
            test_file.get_blocks()[0],
            BitVec::from_bitslice(bits![u8, Msb0; 1, 0, 0, 0, 0])
        );
        for i in 0..piece_size {
            _ = test_file.write_block(0, i, Vec::from([b'a']));
        }
        assert_eq!(
            test_file.get_blocks()[0],
            BitVec::from_bitslice(bits![u8, Msb0; 1, 1, 1, 1, 1])
        );
        test_file.set_piece_finished(0);
        assert_eq!(
            test_file.get_file_bitfield(),
            BitVec::from_bitslice(bits![u8, Msb0; 1, 0])
        );

        test_file.clear_piece(0);
        assert_eq!(
            test_file.get_blocks()[0],
            BitVec::from_bitslice(bits![u8, Msb0; 0, 0, 0, 0, 0])
        );
        assert_eq!(
            test_file.get_file_bitfield(),
            BitVec::from_bitslice(bits![u8, Msb0; 0, 0])
        );
    }

    #[test]
    fn test_helper_methods() {
        let filename = "file.rs.helper.output";
        let _ = fs::remove_file(filename);
        let num_pieces = 2;
        let piece_size = 5;
        let block_size = 1;
        let mut test_file = OutputFile::new(
            filename,
            num_pieces * piece_size,
            num_pieces,
            piece_size,
            block_size,
        )
        .unwrap();

        let bv: BitVec<u8, Msb0> =
            BitVec::from_bitslice(bits![u8, Msb0; 0, 1, 0, 1, 0, 0, 1, 1, 1]);
    }

    #[test]
    fn test_last_piece() {}
}
