#![allow(dead_code)] 
//implements game theory
//tells bittorrent what do so with a particular peer
use crate::peers::Peers;
use crate::p2p::*;


/// gives a list of messages that need to be sent out
pub fn what_do(peers: &mut Peers) -> Vec<Messages>{
    !unimplemented();
}

/// tell strategy what didnt get sent out, 
pub fn these_didnt_happen(messages: Vec<Messages>){
    !unimplemented();
}
