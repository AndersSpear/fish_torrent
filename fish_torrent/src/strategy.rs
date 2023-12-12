#![allow(dead_code)] 
//! implements game theory
//! tells bittorrent EVERY MESSSAGE THAT NEEDS TO BE SENT at the moment that its called
//! MY SOLE GOAL: make every bit in the bitvec 1

use crate::peers::Peers;
use crate::p2p::*;
use crate::file::OutputFile;



/// gives a list of messages that need to be sent out
pub fn what_do(peers: &mut Peers) -> Vec<Messages>{
    let we_have = file.get_piece_bitfields();
    !unimplemented();
}

/// tell strategy what didnt get sent out, 
pub fn these_didnt_happen(messages: Vec<Messages>){
    !unimplemented();
}
