#![allow(dead_code)] 
//self contained code to interface with tracker
//updates peers every interval

// you just got a http response back from the tracker, process it!
// call peers::update_peer_list() with the list of peers
fn handle_tracker_response(){}

// send the initial request to the tracker
fn init_tracker(){}

// timeout has passed, ask the tracker for new data, and update it on our status as well
fn update_tracker(){}