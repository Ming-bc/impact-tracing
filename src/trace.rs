

pub mod traceback {
    extern crate base64;

    use std::collections::{HashSet, HashMap};
    use std::sync::{Arc, Mutex};
    use std::{thread, time};
    use crate::message::messaging::{MsgReport, FwdType, Edge, MsgPacket, Session};
    use crate::tool::algos::{self, store_tag_gen};
    use crate::db::{redis_pack, bloom_filter};
    use base64::{decode, encode};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Clone, Debug)]
    pub struct TraceData {
        pub uid: u32,
        pub key: [u8; 16],
    }

    impl TraceData {
        pub fn new(id: u32, trace_key: [u8; 16]) -> TraceData {
            TraceData { uid: id, key: trace_key }
        }
        pub fn hash(&self) -> String {
            self.uid.to_string() + &encode(&self.key[..])
        }
        pub fn show(&self) {
            println!("User: {}, Key: {}", self.uid, &encode(&self.key[..]));
        }
    }

    fn hmap_to_vec_in_squence<T: std::fmt::Debug + std::clone::Clone> (hmap: &HashMap<usize, T>) -> Vec<T> {
        let mut key_vec: Vec<T> = Vec::new();
        let length = hmap.len();
        for i in 0..length{
            let bind = hmap.get(&i).unwrap();
            key_vec.push(bind.clone());
        }
        key_vec
    }

    fn tag_hmap_to_vec_in_squence<T: std::fmt::Debug + std::clone::Clone> (hmap: &HashMap<usize, T>) -> Vec<T> {
        let mut key_vec: Vec<T> = Vec::new();
        let length = hmap.len();
        for i in 0..length/2{
            let bwd = hmap.get(&(2*i)).unwrap();
            let fwd = hmap.get(&(2*i+1)).unwrap();
            key_vec.push(bwd.clone());
            key_vec.push(fwd.clone());
        }
        key_vec
    }
    
    fn compute_bwd_fwd_tags(uid: &u32, key: &[u8;16], sender: &u32, sid: &String, message: &String) -> ([u8;16], [u8;32], [u8;32]){
        let binding = decode(message).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();
        let bag_key = <&[u8; 16]>::try_from(&decode(sid).unwrap()[..]).unwrap().clone();
        let bwd_tag = store_tag_gen(sender, key, &bag_key, msg_bytes);
        let next_key = algos::next_key(key, &bag_key);
        let fwd_tag = store_tag_gen(uid, &next_key, &bag_key, msg_bytes);
        (next_key, bwd_tag, fwd_tag)
    }

    fn compute_fwd_tags(key: &[u8;16], sender: &u32, sid: &String, message: &String) -> ([u8;16], [u8;32]){
        let binding = decode(message).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();

        let bk = <&[u8; 16]>::try_from(&decode(sid).unwrap()[..]).unwrap().clone();
        let next_key = algos::next_key(key, &bk);
        let tag = store_tag_gen(sender, &next_key, &bk, msg_bytes);
        (next_key, tag)
    }

    pub fn par_backward_search(msg: &str, md: TraceData) -> (TraceData, Vec<TraceData>) {
let part_1 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let sessions = redis_pack::query_users_receive(&md.uid);
let part_2 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let uid = Arc::new(md.uid);
        let key = Arc::new(md.key);
        let message = Arc::new(msg.to_string());
        let par_tags: Arc<Mutex<HashMap<usize, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let par_next_keys: Arc<Mutex<HashMap<usize, [u8;16]>>> = Arc::new(Mutex::new(HashMap::new()));
// TODO: list different parts, and analysis their efficiency.
        let mut thread_list = Vec::new();
        for i in 0..sessions.len() {
            let lock_uid = Arc::clone(&uid);
            let lock_key = Arc::clone(&key);
            let lock_message = Arc::clone(&message);
            let sess = sessions.get(i).unwrap();
            let sid = sess.id.clone();
            let tags_hmap = par_tags.clone();
            let next_key_hmap = par_next_keys.clone();
            let sender = sess.sender;
            
            let handle = thread::spawn(move || {
                let (next_key, bwd_tag, fwd_tag) = compute_bwd_fwd_tags(&lock_uid, &lock_key, &sender, &sid, &lock_message);
                let mut tags = tags_hmap.lock().unwrap();
                tags.insert(2*i, encode(&bwd_tag[..]));
                tags.insert(2*i + 1, encode(&fwd_tag[..]));
                let mut next_keys = next_key_hmap.lock().unwrap();
                next_keys.insert(i, next_key);
            });
            thread_list.push(handle);
        }
        for handle in thread_list {
            handle.join().unwrap();
        }
        let mut tags_hmap = Arc::try_unwrap(par_tags).unwrap().into_inner().unwrap();
        let next_key_hmap = Arc::try_unwrap(par_next_keys).unwrap().into_inner().unwrap();

        let mut bf_tags_vec: Vec<String> = tag_hmap_to_vec_in_squence(&tags_hmap);
        let mut bf_next_key_vec: Vec<[u8; 16]> = hmap_to_vec_in_squence(&next_key_hmap);
let part_3 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let bf_result = bloom_filter::mexists(&mut bf_tags_vec);
let part_4 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut source: TraceData = TraceData::new(0, md.key);
        let mut receivers: Vec<TraceData> = Vec::new();

        for i in 0..(bf_result.len()/2) {
            if *bf_result.get(2*i).unwrap() == true {
                let sess = sessions.get(i).unwrap();
                let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
                let prev_key = algos::prev_key(&md.key, &bk);
                source = TraceData::new(sess.sender, prev_key);
                // TODO: this break might be a problem when the first user is the source, so we ignore it in here
                break;
            }
        }
        
        for i in 0..(bf_result.len()/2)  {
            if *bf_result.get(2*i + 1).unwrap() == true {
                let sess = sessions.get(i).unwrap();
                let next_key = bf_next_key_vec.get(i).unwrap();
                receivers.push(TraceData {uid: sess.sender, key: *next_key})
            }
        }
let part_5 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
// print!("Part 1: {:?}, ", part_2 - part_1);
// print!("Part 2: {:?}, ", part_3 - part_2);
// print!("Part 3: {:?}, ", part_4 - part_3);
// print!("Part 4: {:?}.\n", part_5 - part_4);
        (source, receivers)
    }

    pub fn backward_search(msg: &str, md: TraceData) -> (TraceData, Vec<TraceData>) {
        let sessions = redis_pack::query_users_receive(&md.uid);
        let binding = decode(msg).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();

        let mut bf_tags_vec: Vec<String> = Vec::new();
        let mut bf_next_key_vec: Vec<[u8; 16]> = Vec::new();

        for sess in &sessions {
            // bwd_tag_gen(&sess.sender, &md.uid, &md.key, &sess.id, msg_bytes, &mut tags_tbt, &mut next_key_set);
            let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
            // bwd tag
            let bwd_tag = store_tag_gen(&sess.sender, &md.key, &bk, msg_bytes);
            // fwd tag
            let next_key = algos::next_key(&md.key, &bk);
            let fwd_tag = store_tag_gen(&md.uid, &next_key, &bk, msg_bytes);
            bf_next_key_vec.push(next_key);
            bf_tags_vec.push(encode(&bwd_tag[..]));
            bf_tags_vec.push(encode(&fwd_tag[..]));
        }
        let bf_result = bloom_filter::mexists(&mut bf_tags_vec);
        let mut source: TraceData = TraceData::new(0, md.key);
        let mut receivers: Vec<TraceData> = Vec::new();

        for i in 0..(bf_result.len()/2)  {
            if *bf_result.get(2*i).unwrap() == true {
                let sess = sessions.get(i).unwrap();
                let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
                let prev_key = algos::prev_key(&md.key, &bk);
                source = TraceData::new(sess.sender, prev_key);
                // TODO: this break might be a problem when the first user is the source, so we ignore it in here
                break;
            }
        }
        
        for i in 0..(bf_result.len()/2)  {
            if *bf_result.get(2*i + 1).unwrap() == true {
                let sess = sessions.get(i).unwrap();
                let next_key = bf_next_key_vec.get(i).unwrap();
                receivers.push(TraceData {uid: sess.sender, key: *next_key})
            }
        }
        (source, receivers)
    }

    pub fn par_forward_search(msg: &str, md: &Vec<TraceData>) -> Vec<Vec<TraceData>> {
        let binding = decode(msg).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();
        let mut result: Vec<Vec<TraceData>> = Vec::new();
        let mut users: Vec<u32> = Vec::new();
        for data in md {
            users.push(data.uid);
        }

        let query_sess: Vec<Vec<Session>> = redis_pack::pipe_query_users(&users);
        
        let mut pack_tags_tbt: Vec<Vec<String>> = Vec::new();
        let mut pack_next_key_set: Vec<Vec<[u8; 16]>> = Vec::new();
        for i in 0..query_sess.len() {
            let sessions = query_sess.get(i).unwrap();
            let key = Arc::new(md.get(i).unwrap().key);
            let message = Arc::new(msg.to_string());
            let par_tags: Arc<Mutex<HashMap<usize, String>>> = Arc::new(Mutex::new(HashMap::new()));
            let par_next_keys: Arc<Mutex<HashMap<usize, [u8;16]>>> = Arc::new(Mutex::new(HashMap::new()));
            let mut thread_list = Vec::new();
            for j in 0..sessions.len() {
                let sess = sessions.get(j).unwrap();
                let tags_hmap = par_tags.clone();
                let next_key_hmap = par_next_keys.clone();
                let sender = sess.sender;
                let sid = sess.id.clone();
                let lock_key = Arc::clone(&key);
                let lock_message = Arc::clone(&message);

                let handle = thread::spawn(move || {
                    let (next_key, tag) = compute_fwd_tags(&lock_key, &sender, &sid, &lock_message);
                    let mut next_keys = next_key_hmap.lock().unwrap();
                    next_keys.insert(j, next_key);
                    let mut tags = tags_hmap.lock().unwrap();
                    tags.insert(j, encode(&tag[..]));
                });
                thread_list.push(handle);
            }
            for handle in thread_list {
                handle.join().unwrap();
            }
            let mut tags_hmap = Arc::try_unwrap(par_tags).unwrap().into_inner().unwrap();
            let next_key_hmap = Arc::try_unwrap(par_next_keys).unwrap().into_inner().unwrap();

            let mut tags_tbt: Vec<String> = hmap_to_vec_in_squence(&tags_hmap);
            let mut next_key_set: Vec<[u8; 16]> = hmap_to_vec_in_squence(&next_key_hmap);

            pack_tags_tbt.push(tags_tbt);
            pack_next_key_set.push(next_key_set);
        }

        let query_bool: Vec<Vec<bool>> = bloom_filter::mexists_pack(&pack_tags_tbt);

        for i in 0..query_bool.len() {
            let next_key_set = pack_next_key_set.get(i).unwrap();
            let bf_result = query_bool.get(i).unwrap();
            let sessions = query_sess.get(i).unwrap();

            let mut rcv_result: Vec<TraceData> = Vec::new();
            for j in 0..bf_result.len() {
                if *bf_result.get(j).unwrap() == true {
                    let sess = sessions.get(j).unwrap();
                    let next_key = next_key_set.get(j).unwrap();
                    rcv_result.push(TraceData {uid: sess.receiver, key: *next_key})
                }
            }
            result.push(rcv_result);
        }
        result
    }

    pub fn forward_search(msg: &str, md: &Vec<TraceData>) -> Vec<Vec<TraceData>> {
        let binding = decode(msg).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();
        let mut result: Vec<Vec<TraceData>> = Vec::new();
        let mut users: Vec<u32> = Vec::new();
        for data in md {
            users.push(data.uid);
        }

        let query_sess: Vec<Vec<Session>> = redis_pack::pipe_query_users(&users);
        
        let mut pack_tags_tbt: Vec<Vec<String>> = Vec::new();
        let mut pack_next_key_set: Vec<Vec<[u8; 16]>> = Vec::new();
        let mut sess_index: usize = 0;
        for sessions in &query_sess {
            let mut tags_tbt: Vec<String> = Vec::new();
            let mut next_key_set: Vec<[u8; 16]> = Vec::new();
            for sess in sessions {
                let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
                let next_key = algos::next_key(&md.get(sess_index).unwrap().key, &bk);
                let tag = store_tag_gen(&sess.sender, &next_key, &bk, msg_bytes);

                next_key_set.push(next_key);
                tags_tbt.push(encode(&tag[..]));
            }

            sess_index += 1;
            pack_tags_tbt.push(tags_tbt);
            pack_next_key_set.push(next_key_set);
        }

        let query_bool: Vec<Vec<bool>> = bloom_filter::mexists_pack(&pack_tags_tbt);

        for i in 0..query_bool.len() {
            let next_key_set = pack_next_key_set.get(i).unwrap();
            let bf_result = query_bool.get(i).unwrap();
            let sessions = query_sess.get(i).unwrap();

            let mut rcv_result: Vec<TraceData> = Vec::new();
            for j in 0..bf_result.len() {
                if *bf_result.get(j).unwrap() == true {
                    let sess = sessions.get(j).unwrap();
                    let next_key = next_key_set.get(j).unwrap();
                    rcv_result.push(TraceData {uid: sess.receiver, key: *next_key})
                }
            }
            result.push(rcv_result);
        }
        result
    }

    pub fn tracing(report: MsgReport, snd_start: &u32) -> Vec<Edge>{
        let mut path: Vec<Edge> = Vec::new();
        let mut current_sender= TraceData { uid: *snd_start, key: report.key };
        let mut rcv_set: Vec<TraceData> = Vec::new();
        let mut searched_rcv: HashSet<String> = HashSet::new();

        while (current_sender.uid != 0) | (rcv_set.is_empty() == false) {
            let mut snd_to_rcvs: Vec<TraceData> = Vec::new();
            // Search the previous node of the sender
            if current_sender.uid != 0 {
                let prev_sender: TraceData;
let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                (prev_sender, snd_to_rcvs) = par_backward_search(&report.payload, TraceData { uid: current_sender.uid, key: current_sender.key });
let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
// println!("BwdSearch {} runtime: {:?}", current_sender.uid, trace_end - trace_start);
                if prev_sender.uid != 0 {
                    path.push(Edge::new(prev_sender.uid, current_sender.uid.clone()));
                }
                
                // find searched node
                let mut remove_index: Vec<usize> = Vec::new();
                for i in 0..snd_to_rcvs.len() {
                    let rcv = snd_to_rcvs.get(i).unwrap();
                    if searched_rcv.contains(&rcv.hash()) {
                        remove_index.push(i);
                    }
                    else {
                        path.push(Edge::new(current_sender.uid, snd_to_rcvs.get(i).unwrap().uid))
                    }
                }
                // remove searched node
                for i in &remove_index {
                    snd_to_rcvs.remove(*i);
                }

                searched_rcv.insert(current_sender.hash());
                current_sender = TraceData::from(prev_sender);
            }

            // Search the receivers of the message
            let rcv_len_at_begin = rcv_set.len();
            if rcv_set.is_empty() == false {
                let mut outside_set: Vec<TraceData> = Vec::new();
                let bf_results = par_forward_search(&report.payload, &rcv_set);
                
                for i in 0..bf_results.len() {
                    let mut inside_set: Vec<TraceData> = bf_results.get(i).unwrap().to_vec();
                    let sender = rcv_set.get(i).unwrap();
                    // find searched node
                    let mut remove_index: Vec<usize> = Vec::new();
                    for i in 0..inside_set.len() {
                        let rcv = inside_set.get(i).unwrap();
                        if searched_rcv.contains(&rcv.hash()) {
                            // inside_set.remove(i);
                            remove_index.push(i);
                        }
                    }
                    // remove searched node
                    for i in &remove_index {
                        inside_set.remove(*i);
                    }
                    // if first found, then put in path
                    if inside_set.is_empty() == false {
                        for in_td in &inside_set {
                            path.push(Edge::new(sender.uid, in_td.uid))
                        }
                        outside_set.extend(inside_set);
                    }
                }
                rcv_set.extend(outside_set);
            }
            
            rcv_set.extend(snd_to_rcvs);
            // pop the receivers that already search
            let mut prev_rcv_set = rcv_set;
            rcv_set = prev_rcv_set.split_off(rcv_len_at_begin);
            for user in prev_rcv_set {
                searched_rcv.insert(user.hash());
            }
        }
        path
    }
}


#[cfg(test)]
mod tests {
    extern crate base64;
    extern crate test;

    use core::time;
    use std::collections::hash_map::DefaultHasher;
    use std::collections::{HashMap, HashSet};
    use std::hash::{Hasher, Hash};
    use std::thread;

    use base64::encode;
    use rand;
    use test::Bencher;
    
    use crate::db::bloom_filter;
    use crate::message::messaging;
    use crate::tool::algos;
    use crate::db::redis_pack::{self, empty};
    use crate::trace::traceback;
    use crate::visualize::display;

    use crate::message::messaging::{FwdType, MsgPacket, Session, MsgReport, Edge};
    use super::traceback::{TraceData};
    use std::time::{SystemTime, Duration, UNIX_EPOCH};

    const OURS_BRANCH: u32 = 10;

    #[test]
    fn sid_query_test () {
        let length_of_path = 100;
        let num_of_sess_per_user = 10;
        let (mut fwd_sessions, mut search_sessions, _) = path_sess_gen(length_of_path, num_of_sess_per_user);

        let _ = redis_pack::pipe_add(&mut fwd_sessions);
        let sid_map = sess_to_map(&fwd_sessions);
        
        for sess in fwd_sessions {
            let sid_key = hash_from_sender_receiver(&sess.sender, &sess.receiver);
            let query_id = redis_pack::query_sid(&sess.sender, &sess.receiver);
            let map_id = sid_map.get(&sid_key).unwrap();
            assert_eq!(query_id, *map_id);
        }
        let mut conn = redis_pack::get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_bwd_search() {
        // Generate a mock edge at first
        let users: Vec<u32> = vec![1, 2, 3];
        let fake_receivers: Vec<u32> = vec![1, 5, 6, 7];
        let mut sess = mock_rows_line(&users);
        let mut sess_star = mock_rows_star(&fake_receivers);
        sess.extend(sess_star);
        let _ = redis_pack::pipe_add(&mut sess);

        let message = encode(&rand::random::<[u8; 16]>()[..]);
        let sender = users.get(0).unwrap();
        let receiver = users.get(1).unwrap();
        let report_key = new_edge_gen(&message, sender, receiver).key;
        // Bwd Search
        let (result, _) = traceback::backward_search(&message, TraceData::new(*receiver, report_key));
        assert_eq!(result.uid, *sender);
        let (result, _) = traceback::par_backward_search(&message, TraceData::new(*receiver, report_key));

        // let mut conn = redis_pack::connect().unwrap();
        // let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_fwd_search() {
        let (users, keys, message) = path_cases_2();
        let report_key = keys.get(1).unwrap();
        // Search this message from middle node
        let result = traceback::forward_search(&message, &vec![TraceData::new(*users.get(2).unwrap(), *report_key)]);
        assert_eq!(result.get(0).unwrap().get(0).unwrap().uid, *users.get(3).unwrap());
        // let mut conn = redis_pack::connect().unwrap();
        // let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_tracing() {
        // Path case 2: 1-2-3-4-5, 3-6-7, 6-8
        let (users, keys, message) = path_cases_2();
        let start_index: usize = 0;
        let report_key = keys.get(start_index).unwrap();

        // Search this message from middle node
        let msg_path = traceback::tracing(MsgReport::new(*report_key, message), users.get(start_index + 1).unwrap());
        assert_eq!(msg_path.is_empty(), false);

        for edge in &msg_path {
            edge.show();
        }
        // display::vec_to_dot(refined_users, refined_path);

        let mut conn = redis_pack::get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_tracing_in_tree () {
        let depth: u32 = 3;
        let branch_factor: u32 = 3;

        let (fwd_tree_edges, search_tree_size) = tree_edge_compute(depth, branch_factor);
        let (sess, key, message, tree_md) = arbitary_tree_gen(depth, branch_factor);
println!("Gen finish");
        let mut report_index: usize = 0;
        for i in 0..tree_md.len() {
            let md = tree_md.get(i).unwrap();
            if md.uid == fwd_tree_edges {
                report_index = i;
            }
        }
        let report_md = tree_md.get(report_index).unwrap();

        let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = traceback::tracing(MsgReport::new(report_md.key, message.clone()), &report_md.uid);
        let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("Tree runtime: {:?}", trace_end - trace_start);
        assert_eq!(path.len() as u32, fwd_tree_edges);

        let mut db_conn = redis_pack::get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }

    #[test]
    fn test_tracing_in_path_and_tree () {
        let loop_index: usize = 1;
        let depth: u32 = 10;
        let branch_factor: u32 = 3;

        let (fwd_tree_edges, search_tree_size) = tree_edge_compute(depth, branch_factor);
        let path_length: u32 = (search_tree_size / OURS_BRANCH) as u32;

        print!("Tree size: {}; ", fwd_tree_edges - 1);
        print!("Search size: {}; ", search_tree_size - 1);
        print!("Path length: {}; \n", path_length);

        test_tracing_in_abitary_path(&path_length, &loop_index);
        // test_tracing_in_abitary_tree(&depth, &branch_factor, &fwd_tree_edges, &loop_index);
    }

    fn test_tracing_in_abitary_path(path_length: &u32, loop_index: &usize) {
        let (sess, key, message) = arbitary_path_gen(*path_length, OURS_BRANCH);

        for _i in 0..*loop_index {
            let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let path = traceback::tracing(MsgReport::new(key, message.clone()), &sess.receiver);
            let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

            println!("Path runtime: {:?}", trace_end - trace_start);
            assert_eq!(path.len() as u32, path_length - 1);
            thread::sleep(time::Duration::from_millis(1000));
        }
        redis_pack::empty();
    }

    fn test_tracing_in_abitary_tree(depth: &u32, branch_factor: &u32, size: &u32, loop_index: &usize) {
        let gen_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let (sess, key, message, trace_md) = arbitary_tree_gen(*depth, *branch_factor);
        let gen_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("Tree gentime: {:?}", gen_end - gen_start);
        for _i in 0..*loop_index {
            let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let path = traceback::tracing(MsgReport::new(key, message.clone()), &sess.receiver);
            let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            println!("Tree runtime: {:?}", trace_end - trace_start);
    
            assert_eq!(path.len() as u32, *size);
            thread::sleep(time::Duration::from_millis(1000));
        }
        redis_pack::empty();
    }

    fn arbitary_path_gen (length_of_path: u32, num_of_sess_per_user: u32) -> (Session, [u8; 16], String)  {
        // generate sessions
        let (mut fwd_sessions, mut padding_session, mut users) = path_sess_gen(length_of_path, num_of_sess_per_user);

        let _ = redis_pack::pipe_add(&mut fwd_sessions);
        let _ = redis_pack::pipe_add(&mut padding_session);

        // generate forward path
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);

        let first_packet = new_edge_gen(&message, users.get(0).unwrap(), users.get(1).unwrap());
        let sid_map = sess_to_map(&fwd_sessions);
        let mut path_keys = fwd_path_gen(&first_packet.key, &message, &users.clone().split_off(1), &sid_map);

        // return the last session, key and message
        let report_key = path_keys.pop().unwrap();
        let receiver = users.pop().unwrap();
        let sender = users.pop().unwrap();
        let report_sess = Session::new(&0.to_string(), &sender, &receiver);
        
        (report_sess, report_key, message)
    }

    fn arbitary_tree_gen (depth: u32, branch: u32) -> (Edge, [u8; 16], String, Vec<TraceData>)  {
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);

        let (mut sess_tree, mut search_tree, mut users) = tree_sess_gen(depth, branch);

        let sid_map = sess_to_map(&mut sess_tree);

        let _ = redis_pack::pipe_add(&mut sess_tree);
        let _ = redis_pack::pipe_add(&mut search_tree);

        let start = rand::random::<u32>();
        let root = users.get(0).unwrap().clone();
        let mut edge = mock_rows_line(&vec![start, root]);
        let _ = redis_pack::pipe_add(&mut edge);
        let root_packet = new_edge_gen(&message, &start, &root);
        let mut tags: Vec<String> = Vec::new();

        let tree_md = fwd_tree_gen(&root_packet.key, &(1), &root, &message, depth, &branch, &sid_map, &mut tags, &mut users);

        let _ = bloom_filter::madd(&tags);

        (Edge::new(start, root), root_packet.key, message, tree_md)
    }

    // generate a new edge from a sender to a receiver
    pub fn new_edge_gen(message: &str, sender: &u32, receiver: &u32) -> MsgPacket {
        let bk = &base64::decode(redis_pack::query_sid(&sender, &receiver).clone()).unwrap()[..];
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let packet = messaging::new_msg(bk_16, message);
        let sess = Session::new( &encode(bk_16), sender, receiver);
        while messaging::proc_msg( sess, MsgPacket::new(&packet.key, message) ) != true {
            panic!("process failed.");
        }
        packet
    }
    
    // generate a forward edge from a sender to a receiver
    fn fwd_edge_gen(prev_key: [u8; 16], message: &str, sender: &u32, receiver: &u32, bk: &[u8]) -> MsgPacket {
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let sess = Session::new( &encode(bk_16), sender, receiver);
        let packet = messaging::fwd_msg(&prev_key, &vec![*bk_16], message, FwdType::Receive);
        while messaging::proc_msg(sess, MsgPacket::new(&packet.key, message)) != true {
            panic!("process failed.");
        }
        packet
    }

    fn fwd_edge_gen_standalone_write(prev_key: [u8; 16], message: &str, sender: &u32, receiver: &u32, bk: &[u8]) -> (MsgPacket, String) {
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let packet = messaging::fwd_msg(&prev_key, &vec![*bk_16], message, FwdType::Receive);
        let proc_packet = MsgPacket::new(&packet.key, message);
        let proc_tag = algos::proc_tag(sender, bk_16, &proc_packet.tag);
        (packet, encode(proc_tag))
    }

    fn fwd_path_gen(start_key: &[u8; 16], message: &str, users: &Vec<u32>, sids: &HashMap<u64,String>) -> Vec<[u8; 16]> {
        let mut keys: Vec<[u8;16]> = Vec::new();
        let mut tags: Vec<String> = Vec::new();
        let mut sessions: Vec<Session> = Vec::new();

        keys.push(*start_key);

        for i in 0..(users.len()-1) {
            let sender = users.get(i).unwrap();
            let receiver = users.get(i+1).unwrap();
            let sid = sids.get(&hash_from_sender_receiver(sender,receiver)).unwrap().clone();
            let bk = &base64::decode(sid.clone()).unwrap()[..];

            sessions.push(Session::new(&0.to_string(), sender, receiver));

            let prev_key = *keys.get(i).unwrap();
            let (packet, tag) = fwd_edge_gen_standalone_write(prev_key, message, &sender, &receiver, bk);
            keys.push(packet.key);
            tags.push(tag);
        }
        let _ = bloom_filter::madd(&tags);
        keys
    }

    fn fwd_tree_gen(start_key: &[u8; 16], root_index: &u32, root_id: &u32, message: &str, depth: u32, branch: &u32, sids: &HashMap<u64,String>, tags: &mut Vec<String>, users: &Vec<u32>) -> Vec<TraceData> {
        let mut md_tr: Vec<TraceData> = Vec::new();
        if depth != 0 {
            for k in 1..(*branch + 1) as usize {
                let receiver_index = (root_index - 1) * branch + (k as u32) + 1;
                let receiver_id = users.get((receiver_index - 1) as usize).unwrap();
                // let sid = safe_query_sid(root_id, &receiver_id, sids);
                let sid = sids.get(&hash_from_sender_receiver(root_id, receiver_id)).unwrap();
                let bk = &base64::decode(sid.clone()).unwrap()[..];
                let (packet, proc_tag) = fwd_edge_gen_standalone_write(*start_key, message, root_id, &receiver_id, bk);
                // let packet = fwd_edge_gen(*start_key, message, root, &receiver, bk);
                tags.push(proc_tag);
                md_tr.push(TraceData::new(*receiver_id, packet.key));
                let sub_tree_md = fwd_tree_gen(&packet.key, &receiver_index,&receiver_id, message, depth - 1, branch, sids, tags, users);
                md_tr.extend(sub_tree_md);
            }
        }
        md_tr
    }

    fn tree_sess_gen(depth: u32, branch: u32) -> (Vec<Session>, Vec<Session>, Vec<u32>) {
        let mut sess_tree: Vec<Session> = Vec::new();
        let mut search_tree: Vec<Session> = Vec::new();
        let mut fwd_users: Vec<u32> = Vec::new();
        let (fwd_tree_edges, _) = tree_edge_compute(depth, branch);
        // generate forward tree users
        for _i in 0..(fwd_tree_edges + 1) {
            fwd_users.push(rand::random::<u32>());
        }
        // generate users and store to db
        let mut depth_count = depth;
        while depth_count > 0 {
            for i in 0..(branch.pow(depth - depth_count)) {
                let mut sender_index: u32 = 0;
                for k in 0..(depth - depth_count) {
                    sender_index += branch.pow(k);
                }
                sender_index += i;
                let sender = fwd_users.get(sender_index as usize).unwrap();
                for j in 1..(branch+1) {
                    let sid = encode(&rand::random::<[u8; 16]>()[..]);
                    let receiver_index = sender_index * branch + j;
                    let receiver = fwd_users.get(receiver_index as usize).unwrap();
                    sess_tree.push(Session::new(&sid, sender, receiver));
                }
            }
            depth_count = depth_count - 1;
        }
        // generate fake edges
        let mut processed_user: HashSet<u32> = HashSet::new();
        if branch > OURS_BRANCH {
            panic!("Tree branch is larger than ours branch factor!")
        }
        else {
            let mut mock_rcv_start = sess_tree.len() as u32 + 10;
            let leaf_start_index: u32 = sess_tree.len() as u32 - (branch).pow(depth) + 2;
            for sess in &sess_tree {
                // root nodes
                if sess.sender < leaf_start_index {
                    if processed_user.contains(&sess.sender) == false {
                        search_tree.extend(fake_receivers(&sess.sender, &(OURS_BRANCH - branch - 1), &mock_rcv_start));
                        mock_rcv_start += OURS_BRANCH - branch - 1;
                    }
                }
                // leaf nodes
                if sess.receiver >= leaf_start_index {
                    search_tree.extend(fake_receivers(&sess.receiver, &(OURS_BRANCH - 1), &mock_rcv_start));
                    mock_rcv_start += OURS_BRANCH - 1;
                }
                processed_user.insert(sess.sender);
            }
        }
        (sess_tree, search_tree, fwd_users)
    }

    fn fake_receivers (sender: &u32, num: &u32, start: &u32) -> Vec<Session> {
        let mut sess:Vec<Session> = Vec::new();
        for i in 0..*num {
            // let receiver = start + 1;
            let receiver: u32 = rand::random::<u32>();
            let sid = encode(&rand::random::<[u8; 16]>()[..]);
            sess.push(Session::new(&sid, sender, &receiver));
        }
        sess
    }

    fn path_sess_gen (length: u32, sess_per_user: u32) -> (Vec<Session>, Vec<Session>, Vec<u32>) {
        let mut padding_sessions: Vec<Session> = Vec::new();
        let mut users: Vec<u32> = Vec::new();
        for _i in 0..length {
            // let u_1_index = i + 1;
            users.push(rand::random::<u32>());
            let mut sess_of_user: Vec<u32> = Vec::new();
            sess_of_user.push(rand::random::<u32>());
            for _j in 0..(sess_per_user-1) {
                // let u_2 = rand::random::<u32>();
                // let u_2_index = length + u_1_index + j + 1;
                sess_of_user.push(rand::random::<u32>()); 
            }
            padding_sessions.extend(mock_rows_star(&sess_of_user));
        }
        let fwd_sessions = mock_rows_line(&users);
        (fwd_sessions, padding_sessions, users)
    }

    fn sess_to_map (sessions: &Vec<Session>) -> HashMap<u64, String> {
        let mut sid_map: HashMap<u64, String>  = HashMap::new();
        for sess in sessions {
            let key = hash_from_sender_receiver(&sess.sender, &sess.receiver);
            sid_map.insert(key, sess.id.clone());
        }
        if sid_map.len() != sessions.len() {
            panic!("HashMap {} doesn't equal to sessions {}!", sid_map.len(), sessions.len());
        }
        sid_map
    }

    fn hash_from_sender_receiver (sender: &u32, receiver: &u32) -> u64 {
        let sr_string = sender.to_string() + &receiver.to_string();
        let mut s = DefaultHasher::new();
        sr_string.hash(&mut s);
        s.finish()
    }

    fn tree_edge_compute (depth: u32, branch: u32) -> (u32, u32) {
        let mut fwd_tree_edges: u32 = 0;
        let mut search_tree_size: u32 = 1;
        for i in 0..(depth + 1) {
            fwd_tree_edges += branch.pow(i);
            if i == 0 {
                search_tree_size += OURS_BRANCH - branch - 1;
            }
            else if i == depth {
                search_tree_size += (OURS_BRANCH - 1) * branch.pow(i);
            }
            else {
                search_tree_size += branch.pow(i) * (OURS_BRANCH - branch - 1);
            }
        }
        (fwd_tree_edges, search_tree_size)
    }

    pub fn mock_rows_line(users: &Vec<u32>) -> Vec<Session> {
        let mut sessions: Vec<Session> = Vec::new();
        for i in 0..(users.len()-1) {
            let bytes = rand::random::<[u8; 16]>();
            let sid = encode(&bytes[..]);

            let ses = Session::new(&sid, users.get(i).unwrap(), users.get(i+1).unwrap());
            let vec_ses = vec![ses];
            sessions.extend(vec_ses);
        }
        sessions
    }

    pub fn mock_rows_star(users: &Vec<u32>) -> Vec<Session> {
        let mut sessions: Vec<Session> = Vec::new();
        let central = users.get(0).unwrap();
        for i in 1..users.len() {
            let bytes = rand::random::<[u8; 16]>();
            let sid = encode(&bytes[..]);

            let ses = Session::new(&sid, central, users.get(i).unwrap());
            let vec_ses = vec![ses];
            sessions.extend(vec_ses);
        }
        sessions
    }

    // Generate rows that connects all users in the vector
    pub fn mock_rows_full_connect(users: &Vec<u32>) -> Vec<Session> {
        let mut sessions: Vec<Session> = Vec::new();
        for i in 0..users.len() {
            for j in i+1..users.len() {
                let bytes = rand::random::<[u8; 16]>();
                let sid = encode(&bytes[..]);

                let ses = Session::new(&sid, users.get(i).unwrap(), users.get(j).unwrap());
                sessions.push(ses);
            }
        }
        sessions
    }

    // More complex tree: 1-2-3-4-5, 3-6-7, 6-8
    fn path_cases_2() -> (Vec<u32>, Vec<[u8;16]>, String) {
        let users: Vec<u32> = vec![3254, 4538, 1029, 5798, 6379, 789, 12, 9987];
        let message = encode(&rand::random::<[u8; 16]>()[..]);
        let mut sess = mock_rows_full_connect(&users);

        let _ = redis_pack::pipe_add(&mut sess);
        let sid_map = sess_to_map(&sess);

        // Path 0: 1-2-3-4-5
        let first_packet = new_edge_gen(&message, users.get(0).unwrap(), users.get(1).unwrap());

        let path_1: Vec<u32> = vec![4538, 1029, 5798, 6379];
        let mut keys_1 = fwd_path_gen(&first_packet.key, &message, &path_1, &sid_map);

        // Path 2: 3-6-7
        let path_2: Vec<u32> = vec![1029, 789, 12];
        let mut keys_2 = fwd_path_gen(keys_1.get(1).unwrap(), &message, &path_2, &sid_map);

        // Path 3: 6-8
        let path_3: Vec<u32> = vec![789, 9987];
        let mut keys_3 = fwd_path_gen(keys_2.get(1).unwrap(), &message, &path_3, &sid_map);

        keys_1.append(&mut keys_2.split_off(1));
        keys_1.append(&mut keys_3.split_off(1));

        (users, keys_1, message)
    }


}