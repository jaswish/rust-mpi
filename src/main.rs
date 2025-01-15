#![allow(dead_code)]
#![allow(unused_variables)]
use std::{sync::{mpsc::{Receiver, Sender}, Arc, Mutex}, collections::HashMap};
pub use crate::hypercube;
// use crate::hypercube::hypercube as my_hypercube;
// TODO: Use more files
// TODO: Add ability to use any number of threads in hypercube, not just powers of two.
// TODO: pass in src node as parameter? for testing?
// TODO: Have any number of parameters in argument function (is that possible in rust?).
// TODO: Wrap sender/receiver hash map in struct/#define them?

// Previous development code that was phased out.
fn id_to_bits(id: u8) {
    let id_bits_raw = format!("{:3b}", id);
    let id_bits_chars = id_bits_raw.chars();
    let mut id_bits: String = "".to_string();
    for bit in id_bits_chars {
        if bit == '1' {
            id_bits.push('1')
        }
        else {
            id_bits.push('0');
        }
    }
}

mod ring {
    use std::{thread::{self, JoinHandle}, sync::{mpsc::{channel, Receiver, Sender}, Arc, Mutex, MutexGuard}, collections::HashMap};

    pub fn run(p: u32, f: fn(usize, u32, HashMap<usize, Sender<String>>, Arc<Mutex<HashMap<usize, Receiver<String>>>>)) -> Vec<JoinHandle<()>> {
        let mut handles = vec![];
        let p_i32: i32 = p.try_into().unwrap();
        let p_usize: usize = p.try_into().unwrap();
    
        // Set up data structure for all tx/rx channels.
        let mut all_senders = vec![];
        let mut all_receivers = vec![];
        (0..p).for_each(|_| {
            let cur_sender_map: HashMap<usize, Sender<String>> = HashMap::new();
            let cur_receiver_map: HashMap<usize, Receiver<String>> = HashMap::new();
            let cur_rx_map_mutex = Arc::new(Mutex::new(cur_receiver_map));
            all_senders.push(cur_sender_map);
            all_receivers.push(cur_rx_map_mutex);
        });
    
        // Create all channels.
        for id in 0i32..p.try_into().unwrap() {
            let id_usize: usize = id.try_into().unwrap();
    
            // Calculate this id's receivers.
            let mut receiver_ids: Vec<usize> = vec![];
            receiver_ids.push(((id + 1) % p_i32).try_into().unwrap());
            receiver_ids.push(((id+ p_i32 - 1) % p_i32).abs().try_into().unwrap());
    
            for partner_id in receiver_ids {
                let (tx, rx) = channel();
                all_senders[id_usize].insert(partner_id, tx);
    
                let all_receivers_clone = all_receivers[partner_id].clone();
                let mut all_rx_local = all_receivers_clone.lock().unwrap();
                all_rx_local.insert(id_usize, rx);
            }
        }
    
        for id in 0usize..p_usize {
            // Spawn the thread.
            let thread_name = format!("thread{}", id);
            let builder = thread::Builder::new().name(thread_name.into());
            let all_my_tx = all_senders[id].clone();
            let my_rx_clone = Arc::clone(&all_receivers[id]);
    
            let handle = builder.spawn(move || {
                f(id, p, all_my_tx, my_rx_clone);
            }).unwrap();
    
            handles.push(handle);
        }
    
        return handles;
    }

    pub fn all_broadcast_sim(my_id: usize, p: u32, my_msg: String, all_my_tx: &HashMap<usize, Sender<String>>, all_my_rx: &MutexGuard<HashMap<usize, Receiver<String>>>) {
        let p_usize: usize = p.try_into().unwrap();
        let left = (my_id + p_usize - 1) % p_usize;
        let right = (my_id + 1) % p_usize;
        let mut result: Vec<String> = vec![my_msg];
        for i in 1..p {
            let send_chan = all_my_tx.get(&right.try_into().unwrap()).unwrap();
            let to_send = result.last().cloned().unwrap();
            send_chan.send(to_send).unwrap();

            let rx_chan = all_my_rx.get(&left.try_into().unwrap()).unwrap();
            let ans = rx_chan.recv().unwrap();
            result.push(ans);
        }

        println!("ID {}: {:?}", my_id, result);
    }

}

fn my_thread(id: usize, n: u32, all_my_tx: HashMap<usize, Sender<String>>, my_rx_clone: Arc<Mutex<HashMap<usize, Receiver<String>>>>) {
    // Create local copy of HashMap of Receivers
    let all_rx_local = my_rx_clone;
    let all_my_rx = all_rx_local.lock().unwrap(); // TODO: Perform this in the builder::spwan call

    // println!("{} {:?}", id, all_my_tx);
    // println!("{} {:?}", id, all_my_rx);

    //////////////////////////////////////////////
    // Code for simulated all-to-all broadcast. //
    //////////////////////////////////////////////
    let mut msg = String::from("Hello World");
    msg.push_str(&format!("{}", id));
    ring::all_broadcast_sim(id, n, msg, &all_my_tx, &all_my_rx)

    //////////////////////////////////////////////
    // Code for simulated one-to-all reduction. //
    //////////////////////////////////////////////
    // let id_str: String = id.to_string();
    // let mut msg: String = String::from("Hello from ");
    // msg.push_str(&id_str);
    // let ans_vec = hypercube::reduction_sim(n, id, msg, &all_my_tx, &all_my_rx);
    // if id == 0 {
    //     println!("My Answer: {:?}", ans_vec);
    // }

    /////////////////////////////////////
    // Perform a one-to-all broadcast. //
    /////////////////////////////////////
    // if id == 0 {
    //     let msg: String = String::from("Hello");
    //     hypercube::broadcast_send(msg, n, id, &all_my_tx);
    // } else {
    //     let msg = hypercube::broadcast_recv(0, n, id, &all_my_tx, &all_my_rx);
    //     println!("{}", msg);
    // }

    //////////////////////////////////////////////
    // Code for simulated one-to-all broadcast. //
    //////////////////////////////////////////////
    // let msg = format!("Hello there!");
    // hypercube::broadcast_sim(1, &msg, n, id, &all_my_tx, &all_my_rx)
}

fn main() {
    // let handles = hypercube::run(3, my_thread);
    let handles = ring::run(9, my_thread);

    for handle in handles {
        handle.join().unwrap();
    }
}

/////////////////////////////////////////////////////////
/// Tests
/////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;

    // Define the tests.
    macro_rules! hypercube_reduction_tests {
        ($($name: ident: $dims:expr; $source:expr,)*) => {
        $(
            #[test]
            fn $name() {
                // Setup the test thread.
                fn test_thread(id: usize, n: u32, all_my_tx: HashMap<usize, Sender<String>>, my_rx_clone: Arc<Mutex<HashMap<usize, Receiver<String>>>>) {
                    // Create local copy of HashMap of Receivers
                    let all_rx_local = my_rx_clone;
                    let all_my_rx = all_rx_local.lock().unwrap();
                
                    //////////////////////////////////////////////
                    // Code for simulated one-to-all broadcast. //
                    //////////////////////////////////////////////
                    let id_str: String = id.to_string();
                    let mut msg: String = String::from("Hello from ");
                    msg.push_str(&id_str);
                    let mut ans_vec = hypercube::reduction_sim(n, id, msg, &all_my_tx, &all_my_rx);
                    if id == 0 {
                        println!("My Answer: {:?}", ans_vec);
                        let mut golden_data: Vec<String> = vec![];
                        for i in 0..2u32.pow($dims) {
                            let i_str: String = i.to_string();
                            let mut golden_str: String = String::from("Hello from ");
                            golden_str.push_str(&i_str);
                            golden_data.push(golden_str);
                        }
                        ans_vec.sort();
                        golden_data.sort();
                        assert_eq!(ans_vec, golden_data);
                    }
                }

                let handles = hypercube::run($dims, test_thread);

                for handle in handles {
                    handle.join().unwrap();
                }
            }
        )*
        };
    }

    macro_rules! hypercube_broadcast_tests {
        ($($name: ident: $dims:expr; $source:expr,)*) => {
        $(
            #[test]
            fn $name() {
                // Setup the test thread.
                fn test_thread(id: usize, n: u32, all_my_tx: HashMap<usize, Sender<String>>, my_rx_clone: Arc<Mutex<HashMap<usize, Receiver<String>>>>) {
                    // Create local copy of HashMap of Receivers
                    let all_rx_local = my_rx_clone;
                    let all_my_rx = all_rx_local.lock().unwrap();
                
                    // Perform a one-to-all broadcast.
                    if id == $source {
                        let msg: String = String::from("Hello");
                        hypercube::broadcast_send(msg, n, id, &all_my_tx);
                    } else {
                        let id_u: u32 = id.try_into().unwrap();
                        let msg = hypercube::broadcast_recv($source, n, id, &all_my_tx, &all_my_rx);

                        // Did we get the actual content?
                        assert_eq!(String::from("Hello"), msg[0..5]);

                        // Is the transmission sequence as expected?
                        let mut actual_seq: Vec<u32> = Vec::new();
                        let mut actual_seq_str = msg.split("->").collect::<Vec<&str>>();
                        actual_seq_str[0] = actual_seq_str[0].split(" ").collect::<Vec<&str>>().last().unwrap();
                        for (i, _) in actual_seq_str.iter().enumerate() {
                            actual_seq.push(actual_seq_str[i].parse::<u32>().unwrap());
                        }
                        let mut expected_seq: Vec<u32> = Vec::new();
                        let virtual_id_u = id_u ^ $source;
                        let mut virtual_prev_src = virtual_id_u;
                        for i in 0..n {
                            let mask = 2u32.pow(i);
                            if virtual_prev_src ^ mask <= virtual_prev_src {
                                let temp = virtual_prev_src ^ mask;
                                expected_seq.push(temp ^ $source);
                                virtual_prev_src = temp;
                            }
                        }
                        expected_seq.reverse();
                        expected_seq.push(id_u);
                        assert_eq!(actual_seq, expected_seq);
                    }
                }

                let handles = hypercube::run($dims, test_thread);
            
                for handle in handles {
                    handle.join().unwrap();
                }
            }
        )*
        };
    }

    // Declare tests to be run.
    hypercube_broadcast_tests! {
        hypercube_broadcast_1_0: 1; 0,
        hypercube_broadcast_2_0: 2; 0,
        hypercube_broadcast_2_3: 2; 3,
        hypercube_broadcast_3_0: 3; 0,
        hypercube_broadcast_3_2: 3; 2,
        hypercube_broadcast_4_0: 4; 0,
        hypercube_broadcast_5_0: 5; 0,
    }

    hypercube_reduction_tests! {
        hypercube_reduction_1_0: 1; 0,
        hypercube_reduction_2_0: 2; 0,
        hypercube_reduction_3_0: 3; 0,
        hypercube_reduction_4_0: 4; 0,
        hypercube_reduction_5_0: 5; 0,
    }
}
