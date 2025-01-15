// use std::{sync::{mpsc::{Receiver, Sender}, Arc, Mutex}, collections::HashMap};
pub mod hypercube {
    use std::{thread::{self, JoinHandle}, sync::{mpsc::{channel, Receiver, Sender}, Arc, Mutex, MutexGuard}, collections::HashMap};
    
    pub fn run(n: u32, f: fn(usize, u32, HashMap<usize, Sender<String>>, Arc<Mutex<HashMap<usize, Receiver<String>>>>)) -> Vec<JoinHandle<()>> {
        let mut handles = vec![];
    
        // Set up data structure for all tx/rx channels.
        let mut all_senders = vec![];
        let mut all_receivers = vec![];
        (0..(2 << n-1)).for_each(|_| {
            let cur_sender_map: HashMap<usize, Sender<String>> = HashMap::new();
            let cur_receiver_map: HashMap<usize, Receiver<String>> = HashMap::new();
            let cur_rx_map_mutex = Arc::new(Mutex::new(cur_receiver_map));
            all_senders.push(cur_sender_map);
            all_receivers.push(cur_rx_map_mutex);
        });
    
        // Create all channels.
        for id in 0usize..(2 << n-1) {
            // Calculate this id's receivers.
            let mut receiver_ids: Vec<usize> = vec![];
            for i in 0..n {
                receiver_ids.push(id ^ (1 << i));
            }
            receiver_ids.sort(); // completely unnecessary
    
            // Create the channels with this id as a tx.
            for partner_id in receiver_ids {
                let (tx, rx) = channel();
                all_senders[id].insert(partner_id, tx);
    
                let all_receivers_clone = all_receivers[partner_id].clone();
                let mut all_rx_local = all_receivers_clone.lock().unwrap();
                all_rx_local.insert(id, rx); // tx'r id identifies the Receiver
            }
        }
        
        for id in 0usize..(2 << n-1) {
            // Spawn the thread.
            let thread_name = format!("thread{}", id);
            let builder = thread::Builder::new().name(thread_name.into());
            let all_my_tx = all_senders[id].clone();
            let my_rx_clone = Arc::clone(&all_receivers[id]);
    
            let handle = builder.spawn(move || {
                f(id, n, all_my_tx, my_rx_clone);
            }).unwrap();
            
            handles.push(handle);
        }
        
        return handles;
    }

    pub fn broadcast_send(mut msg: String, dims: u32, my_id: usize, all_my_tx: &HashMap<usize, Sender<String>>) {
        // Preparation.
        let my_id_u: u32 = my_id.try_into().unwrap(); // TODO: Could also just iterate through the keys of all_my_tx
        let mut mask = 2u32.pow(dims)-1;
        
        // Add some tracking info to the message.
        let footer = format!(" Seq: {}", my_id);
        msg.push_str(&footer);
    
        // Send to all connected nodes, highest order first.
        for i in (0u32..dims).rev() {
            // Determine ID of destination.
            mask = mask ^ (2_u32.pow(i));
            let i_u32: u32 = i.try_into().unwrap();
            let dest = my_id_u ^ (2_u32.pow(i_u32));
    
            let tx_chan = all_my_tx.get(&dest.try_into().unwrap()).unwrap();
            tx_chan.send(msg.to_string()).unwrap();
        }
    }

    pub fn broadcast_recv(src_node: usize, dims: u32, my_id: usize, all_my_tx: &HashMap<usize, Sender<String>>, all_my_rx: &MutexGuard<HashMap<usize, Receiver<String>>>) -> String{
        // Preparation.
        let my_id_u: u32 = my_id.try_into().unwrap();
        let src_node_u: u32 = src_node.try_into().unwrap();
        let my_virtual_id = my_id_u ^ src_node_u;
        let mut mask = 2u32.pow(dims)-1;
    
        
        // Find out who to receive from.
        let mut msg: String = String::new();
        for i in 0u32..dims {
            // Determine a potential source.
            mask = mask ^ (2_u32.pow(i));
            let src = my_virtual_id ^ (2_u32.pow(i));
            let virtual_src = src ^ src_node_u;
    
            // If this is the correct source to receive from...
            if src < my_virtual_id {
                let rx_chan = all_my_rx.get(&virtual_src.try_into().unwrap()).unwrap();
                msg = rx_chan.recv().unwrap();
    
                // Add some tracking information to the message.
                let footer = format!("->{}", my_id);
                msg.push_str(&footer);
                break;
            }
        }
        
        // Find out who to send to, if any.
        for i in 0u32..dims {
            // Determine a potential destination.
            mask = mask ^ (2_u32.pow(i));
            let dest = my_id_u ^ (2_u32.pow(i));
            let virtual_dest = dest ^ src_node_u;
            
            // Determine if this is a node we need to send to...
            if virtual_dest > my_virtual_id {
                let tx_chan = all_my_tx.get(&dest.try_into().unwrap()).unwrap();
                tx_chan.send(msg.to_string()).unwrap();
            } else {
                // If this node is not someone to send to, then we're done.
                break;
            }
        }
        
        return msg;
    }

    // This is the version from the book which assumes every thread has access to the message
    // which doesn't really make sense given the concept of a "broadcast"...
    pub fn broadcast_sim(src_node: usize, msg: &String, dims: u32, my_id: usize, all_my_tx: &HashMap<usize, Sender<String>>, all_my_rx: &MutexGuard<HashMap<usize, Receiver<String>>>) {
        // Preparation.
        let mut mask = 2_u32.pow(dims) - 1;
        let my_id_u: u32 = my_id.try_into().unwrap();
        let src_node_u: u32 = src_node.try_into().unwrap();
        let my_virtual_id: u32 = my_id_u ^ src_node_u;
        
        for i in (0u32..dims).rev() {
            mask = mask ^ (2_u32.pow(i));
            if (my_virtual_id & mask) == 0 {
                if (my_virtual_id & (2_u32.pow(i))) == 0 {
                    let virtual_dest = my_virtual_id ^ (2_u32.pow(i));
                    let dest = virtual_dest ^ src_node_u;
                    let tx_chan = all_my_tx.get(&dest.try_into().unwrap()).unwrap();
                    tx_chan.send(msg.to_string()).unwrap();
                } else {
                    let virtual_src = my_virtual_id ^ (2_u32.pow(i));
                    let src = virtual_src ^ src_node_u;
                    let rx_chan = all_my_rx.get(&src.try_into().unwrap()).unwrap();
                    let ans = rx_chan.recv().unwrap();
                    println!("Thread {} received {}", my_id, ans);
                }
            }
        } 
    }

    // TODO: No reason to turn this into send/recv? Only benefit would be getting rid of useless return...
    // TODO: Make this generic for any destination.
    pub fn reduction_sim(dims: u32, my_id: usize, msg: String, all_my_tx: &HashMap<usize, Sender<String>>, all_my_rx: &MutexGuard<HashMap<usize, Receiver<String>>>) -> Vec<String>{
        // Preparation
        let my_id_u: u32 = my_id.try_into().unwrap();
        let mut mask: u32 = 0;
    
        // Create the vector to contain all messages to send
        let mut msg_vec: Vec<String> = Vec::new();
        msg_vec.push(msg);
    
        for i in 0..dims {
            if my_id_u & mask == 0 {
                if my_id_u & 2u32.pow(i) != 0 {
                    let dest = my_id_u ^ 2u32.pow(i);
                    let num_to_send = msg_vec.len();
                    for _ in 0..num_to_send {
                        let chan = all_my_tx.get(&dest.try_into().unwrap()).unwrap();
                        let to_send = msg_vec.pop().unwrap();
                        chan.send(to_send).unwrap();
                    }
    
                } else {
                    let src = my_id_u ^ 2u32.pow(i);
                    let rx_chan = all_my_rx.get(&src.try_into().unwrap()).unwrap();
                    for _ in 0..2u32.pow(i) {
                        let ans = rx_chan.recv().unwrap();
                        msg_vec.push(ans);
                    }
                }
            }
            mask = mask ^ 2u32.pow(i);
        }
    
        if my_id == 0 {
            return msg_vec;
        }
        return vec![];
    }
}
