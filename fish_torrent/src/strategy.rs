#![allow(dead_code)]
//! implements game theory
//! tells bittorrent EVERY MESSSAGE THAT NEEDS TO BE SENT at the moment that its called
//! MY SOLE GOAL: make every bit in the bitvec 1

use bitvec::order::Msb0;
use bitvec::vec::BitVec;

use crate::file::OutputFile;
use crate::p2p::Messages;
use crate::peers::Peers;

pub struct StratStruct{
    
}


impl StratStruct{
    pub fn new() -> StratStruct{
        unimplemented!();
    }
    pub fn what_do(self, peers: &mut Peers, file:&OutputFile) -> StratStruct{
        unimplemented!();
    }
}
