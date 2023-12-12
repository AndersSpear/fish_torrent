#![allow(dead_code)] 
//! implements game theory
//! tells bittorrent EVERY MESSSAGE THAT NEEDS TO BE SENT at the moment that its called
//! MY SOLE GOAL: make every bit in the bitvec 1

use bitvec::order::Msb0;
use bitvec::vec::BitVec;

use crate::peers::Peers;
use crate::p2p::Messages;



/// gives a list of messages that need to be sent out
pub fn what_do(peers: &mut Peers, we_have: &Vec<BitVec<u8, Msb0>>) -> Vec<Messages>{
    unimplemented!();
}

/// tell strategy what didnt get sent out, 
pub fn these_didnt_happen(messages: Vec<Messages>){
    unimplemented!();
}
