#![allow(dead_code)]

pub mod traceback {
    extern crate base64;

    use std::collections::{HashSet, HashMap};
    use std::sync::{Arc, Mutex};
    use std::{thread, fmt};
    use crate::message::messaging::{MsgReport, Edge};
    use crate::tool::algos;
    use crate::db::{db_tag, db_nbr, db_ik};
    use base64::{decode, encode};

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

    impl fmt::Display for TraceData {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "User: {}, Key: {}", self.uid, &encode(&self.key[..]))
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

    fn db_query_nbrs(vec_uid: &Vec<u32>) -> (Vec<Vec<u32>>, HashMap<u32,[u8;16]>){
        // query nbrs of users
        let map_uid_nbr = db_nbr::query(vec_uid);
        // query ik of users
        let values: Vec<Vec<u32>> = map_uid_nbr.clone().into_values().collect();
        let mut vec_values: Vec<u32> = values.concat();
        vec_values.append(&mut vec_uid.clone());
        let map_id_ik = db_ik::query(&vec_values);

        let mut vec_vec_nbrs = Vec::<Vec<u32>>::new();
        for uid in vec_uid {
            if map_uid_nbr.contains_key(uid) == true {
                let nbrs = map_uid_nbr.get(uid).unwrap();
                vec_vec_nbrs.push(nbrs.clone());
            }
        }
        (vec_vec_nbrs, map_id_ik)
    }

    pub fn par_backward_search(input_msg: &String, md: &TraceData) -> TraceData {
        // let nbrs = redis_pack::query_users_receive(&md.uid);
        let (vec_vec_nbrs, map_id_ik) = db_query_nbrs(&vec![md.uid]);
        let vec_nbrs = vec_vec_nbrs.get(0).unwrap().to_owned();
        let curr_id = Arc::new(md.uid);
        let key = Arc::new(md.key);
        let message = Arc::new(input_msg.clone());
        let par_tags: Arc<Mutex<HashMap<usize, String>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut thread_list = Vec::new();
        for i in 0..vec_nbrs.len() {
            let lock_key = Arc::clone(&key);
            let lock_message = Arc::clone(&message);
            let lock_curr_id = Arc::clone(&curr_id);
            let nbr_id = vec_nbrs.get(i).unwrap();
            let tk = algos::tk_gen(map_id_ik.get(&nbr_id).unwrap(), &lock_curr_id);
            let tags_hmap = par_tags.clone();
            
            let handle = thread::spawn(move || {
                let (bwd_tag,_) = algos::tag_gen(&lock_key, &tk, &lock_message);
                let mut tags = tags_hmap.lock().unwrap();
                tags.insert(i, encode(&bwd_tag[..]));
            });
            thread_list.push(handle);
        }
        for handle in thread_list {
            handle.join().unwrap();
        }
        let tags_hmap = Arc::try_unwrap(par_tags).unwrap().into_inner().unwrap();
        let mut bf_tags_vec: Vec<String> = hmap_to_vec_in_squence(&tags_hmap);
        let bf_result = db_tag::mexists(&mut bf_tags_vec);
        let mut source: TraceData = TraceData::new(0, md.key);

        for i in 0..(bf_result.len()) {
            if *bf_result.get(i).unwrap() == true {
                let nbr_id = vec_nbrs.get(i).unwrap();
                let tk = algos::tk_gen(&map_id_ik.get(&nbr_id).unwrap(), &md.uid);
                let prev_key = algos::prev_key(&md.key, &tk);
                source = TraceData::new(*nbr_id, prev_key);
                // TODO: this break might be a problem when the first user is the source, so we ignore it in here
                break;
            }
        }

        source
    }

    pub fn par_forward_search(input_msg: &String, md: &Vec<TraceData>) -> Vec<Vec<TraceData>> {
        let mut result: Vec<Vec<TraceData>> = Vec::new();
        let users: Vec<u32> = md.into_iter().map(|data| data.uid).collect();
        let (vec_vec_nbrs, map_id_ik) = db_query_nbrs(&users);
        let mut pack_tags_tbt: Vec<Vec<String>> = Vec::new();
        let mut pack_next_key_set: Vec<Vec<[u8; 16]>> = Vec::new();
        for i in 0..vec_vec_nbrs.len() {
            let vec_nbrs = vec_vec_nbrs.get(i).unwrap();
            let curr_uik = &map_id_ik.get(&md.get(i).unwrap().uid).unwrap();
            let key = Arc::new(md.get(i).unwrap().key);
            let message = Arc::new(input_msg.clone());
            let par_tags: Arc<Mutex<HashMap<usize, String>>> = Arc::new(Mutex::new(HashMap::new()));
            let par_next_keys: Arc<Mutex<HashMap<usize, [u8;16]>>> = Arc::new(Mutex::new(HashMap::new()));
            let mut thread_list = Vec::new();
            for j in 0..vec_nbrs.len() {
                let curr_nbr_id = vec_nbrs.get(j).unwrap();
                let tags_hmap = par_tags.clone();
                let next_key_hmap = par_next_keys.clone();
                let tk = algos::tk_gen(&curr_uik, &curr_nbr_id);
                let lock_key = Arc::clone(&key);
                let lock_message = Arc::clone(&message);

                let handle = thread::spawn(move || {
                    let next_key = algos::next_key(&lock_key, &tk);
                    let (tag, _) = algos::tag_gen(&next_key, &tk, &lock_message);
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
            let tags_hmap = Arc::try_unwrap(par_tags).unwrap().into_inner().unwrap();
            let next_key_hmap = Arc::try_unwrap(par_next_keys).unwrap().into_inner().unwrap();

            let tags_tbt: Vec<String> = hmap_to_vec_in_squence(&tags_hmap);
            let next_key_set: Vec<[u8; 16]> = hmap_to_vec_in_squence(&next_key_hmap);

            pack_tags_tbt.push(tags_tbt);
            pack_next_key_set.push(next_key_set);
        }

        let query_bool: Vec<Vec<bool>> = db_tag::mexists_pack(&pack_tags_tbt);
        for i in 0..query_bool.len() {
            let next_key_set = pack_next_key_set.get(i).unwrap();
            let bf_result = query_bool.get(i).unwrap();
            let vec_nbrs = vec_vec_nbrs.get(i).unwrap();

            let mut rcv_result: Vec<TraceData> = Vec::new();
            for j in 0..bf_result.len() {
                if *bf_result.get(j).unwrap() == true {
                    let nbr_id = vec_nbrs.get(j).unwrap();
                    let next_key = next_key_set.get(j).unwrap();
                    rcv_result.push(TraceData {uid: *nbr_id, key: *next_key})
                }
            }
            result.push(rcv_result);
        }
        result
    }

    pub fn tracing(report: &MsgReport, snd_start: &u32) -> Vec<Edge>{
        let mut path: Vec<Edge> = Vec::new();
        let mut current_sender= TraceData { uid: *snd_start, key: report.key };
        let mut rcv_set: Vec<TraceData> = Vec::new();
        let mut searched_rcv: HashSet<String> = HashSet::new();
        rcv_set.push(current_sender.clone());

        while (current_sender.uid != 0) | (rcv_set.is_empty() == false) {
            // Search the previous node of the sender
            if current_sender.uid != 0 {
                let prev_sender: TraceData;
// println!("Backward search: {}", current_sender);
                prev_sender = par_backward_search(&report.payload, &TraceData { uid: current_sender.uid, key: current_sender.key });
                if prev_sender.uid != 0 {
                    path.push(Edge::new(&prev_sender.uid, &current_sender.uid));
                }
                rcv_set.push(current_sender.clone());
                current_sender = TraceData::from(prev_sender);
            }
            // Search the receivers of the message
            let rcv_len_at_begin = rcv_set.len();
            if rcv_set.is_empty() == false {

// println!("\nForward search: ");
// for u in &rcv_set {
//     print!("{}, ", u.uid);
// }
                let mut outside_set: Vec<TraceData> = Vec::new();
                let bf_results = par_forward_search(&&report.payload, &rcv_set);
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
                            path.push(Edge::new(&sender.uid, &in_td.uid))
                        }
                        outside_set.extend(inside_set);
                    }
                }
                rcv_set.extend(outside_set);
            }
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
pub mod tests {
    extern crate base64;
    extern crate test;

    use std::{collections::HashMap, vec};
                
    use base64::encode;
    use rand;
    
    use crate::{db::{db_tag, db_ik, db_nbr}, message::messaging::{self, IdKey}, tool::algos::tk_gen};
    use crate::trace::traceback::{self, TraceData};
    use crate::message::messaging::{MsgPacket, Edge, MsgReport};
    
    const OURS_BRANCH: u32 = 10;

    fn register_users (vec_uid: &Vec<u32>) -> HashMap<u32, [u8; 16]> {
        let mut vec_id_key = Vec::<IdKey>::new();
        for uid in vec_uid {
            // let id_key = IdKey::rand_key_gen(*uid);
            let id_key = IdKey::id_as_key_gen(*uid);
            vec_id_key.push(id_key);
        }
        let _ = db_ik::add(&vec_id_key);
        // convert vec_id_key to hmap
        let map_id_key: HashMap<u32, [u8; 16]> = vec_id_key.into_iter().map(|id_key| (id_key.id, id_key.key)).collect();
        map_id_key
    }

    // Create a forwarding tree: 1-2-3-4-5, 3-6-7, 6-8
    fn create_path_case() -> (Vec<u32>, Vec<[u8;16]>, String) {
        let users: Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let message = "message".to_string();
        let mut sess = mock_nbr_full_connect(&users);

        let _ = db_nbr::add(&mut sess);
        let map_id_ik = register_users(&users);
        // Path 0: 1-2-3-4-5
        let first_packet = new_edge_gen(&message, &1, &2);

        let path_1: Vec<u32> = vec![2, 3, 4, 5, 9, 8];
        let mut keys_1: Vec<[u8; 16]> = fwd_path_gen(&first_packet.tag_key, &message, &path_1, &map_id_ik); 

        // Path 2: 3-6-7
        let path_2: Vec<u32> = vec![3, 6, 7];
        let mut keys_2 = fwd_path_gen(keys_1.get(1).unwrap(), &message, &path_2, &map_id_ik);

        // Path 3: 6-8
        let path_3: Vec<u32> = vec![6, 8];
        let mut keys_3 = fwd_path_gen(keys_2.get(1).unwrap(), &message, &path_3, &map_id_ik);

        keys_1.append(&mut keys_2.split_off(1));
        keys_1.append(&mut keys_3.split_off(1));

        (users, keys_1, message)
    }

    #[test]
    fn test_tracing() {
        // Path case 2: 1-2-3-4-5, 3-6-7, 6-8
        let (users, keys, message) = create_path_case();
        let start_index: usize = 1;
        let report_key = keys.get(start_index).unwrap();

        // Search this message from middle node
        let fwd_graph = traceback::tracing(&MsgReport {key: *report_key, payload: message}, users.get(start_index + 1).unwrap());
        assert_eq!(fwd_graph.is_empty(), false);

        fwd_graph.into_iter().for_each(|e| {
            println!("{} -> {}", e.sid, e.rid);
        });
        // display::vec_to_dot(refined_users, refined_path);
        db_clear();
    }

    fn db_clear() {
        db_nbr::clear();
        db_ik::clear();
        db_tag::clear();
    }

    #[test]
    fn trace_tree () {
        let branch: u32 = 3;
        let depth: u32 = 6;
        let tree_size = calc_tree_size(&depth, &branch);
        let vec_user = (0..tree_size + 1).collect::<Vec<u32>>();
        let map_id_ik = register_users(&vec_user);

        let origin_id = vec_user.get(tree_size as usize).unwrap();
        let root_id = vec_user.get(0).unwrap();
        let first_packet = new_edge_gen(&"message".to_string(), origin_id, &root_id);
        let mut vec_tag = Vec::<String>::new();
        let mut vec_edge = Vec::<Edge>::new();

        mock_tree_recursive(&root_id, &first_packet, &branch, &1, &depth, &"message".to_string(), &map_id_ik, &mut vec_tag, &mut vec_edge);
        let _ = db_tag::add(&vec_tag);
        let _ = db_nbr::add(&vec_edge);

        let path =  traceback::tracing(&MsgReport {key: first_packet.tag_key, payload: "message".to_string()}, &root_id);

        // println!("Path-Tree: {}-{}", path.len(), tree_size - 1);

        assert_eq!(tree_size -1, path.len() as u32);

        db_clear();
    }

    fn mock_tree_recursive(root: &u32, prev_packet: &MsgPacket, branch: &u32, curr_depth: &u32, depth: &u32, message: &String, map_id_ik: &HashMap<u32, [u8;16]>, vec_tag: &mut Vec<String>, vec_edge: &mut Vec<Edge>) {
        if curr_depth < depth {
            for i in 0..*branch {
                let rid = root * branch + i + 1;
                vec_edge.push(Edge::new(root, &rid));
                let packet = fwd_edge_gen(message, root, &rid, prev_packet, map_id_ik);
                vec_tag.push(encode(packet.tag));
                mock_tree_recursive(&rid, &packet, branch, &(curr_depth + 1), depth, message, map_id_ik, vec_tag, vec_edge);
            }
        }
    }

    fn calc_tree_size(depth: &u32, branch: &u32) -> u32 {
        let mut size: u32 = 0;
        for i in 0..*depth {
            size += branch.pow(i);
        }
        size
    }

    // generate a new edge from a sender to a receiver
    fn new_edge_gen(message: &String, sid: &u32, rid: &u32) -> MsgPacket {
        let map_id_ik = db_ik::query(&vec![*sid]);
        let tk = tk_gen(map_id_ik.get(sid).unwrap(), rid);
        let packet = messaging::send_packet(message, &[0;16], &tk);
        let _ = db_tag::add(&vec![encode(packet.tag)]);
        packet
    }

    fn fwd_edge_gen(message: &String, sid: &u32, rid: &u32, prev_packet: &MsgPacket, map_id_ik: &HashMap<u32, [u8;16]>) -> MsgPacket {
        let tk = tk_gen(map_id_ik.get(sid).unwrap(), rid);
        let packet = messaging::send_packet(message, &prev_packet.tag_key, &tk);
        packet
    }

    fn fwd_path_gen(s_tag_key: &[u8; 16], message: &String, users: &Vec<u32>, id_keys: &HashMap<u32,[u8;16]>) -> Vec<[u8; 16]> {
        let mut tag_keys: Vec<[u8;16]> = Vec::new();
        let mut tags: Vec<String> = Vec::new();
        let mut sessions: Vec<Edge> = Vec::new();

        tag_keys.push(*s_tag_key);

        for i in 0..(users.len()-1) {
            let sid = users.get(i).unwrap();
            let rid = users.get(i+1).unwrap();
            let tk = tk_gen(id_keys.get(sid).unwrap(), rid);
            sessions.push(Edge::new( sid, rid));
            let prev_key = *tag_keys.get(i).unwrap();
            let packet = messaging::send_packet(message, &prev_key, &tk);
            tag_keys.push(packet.tag_key);
            tags.push(encode(packet.tag));
        }
        let _ = db_tag::add(&tags);
        tag_keys
    }

    fn path_sess_gen (length: u32, sess_per_user: u32) -> (Vec<Edge>, Vec<Edge>, Vec<u32>) {
        let mut padding_sessions: Vec<Edge> = Vec::new();
        let mut users: Vec<u32> = Vec::new();
        for _i in 0..length {
            // let u_1_index = i + 1;
            users.push(rand::random::<u32>());
            let mut sess_of_user: Vec<u32> = Vec::new();
            sess_of_user.push(rand::random::<u32>());
            for _j in 0..(sess_per_user-1) {
                sess_of_user.push(rand::random::<u32>()); 
            }
            padding_sessions.extend(mock_nbr_star(&sess_of_user));
        }
        let fwd_sessions = mock_nbr_line(&users);
        (fwd_sessions, padding_sessions, users)
    }

    pub fn mock_nbr_line(users: &Vec<u32>) -> Vec<Edge> {
        let mut sessions: Vec<Edge> = Vec::new();
        for i in 0..(users.len()-1) {
            let ses = Edge::new(users.get(i).unwrap(), users.get(i+1).unwrap());
            let vec_ses = vec![ses];
            sessions.extend(vec_ses);
        }
        sessions
    }

    pub fn mock_nbr_star(users: &Vec<u32>) -> Vec<Edge> {
        let mut sessions: Vec<Edge> = Vec::new();
        let central = users.get(0).unwrap();
        for i in 1..users.len() {
            let ses = Edge::new( central, users.get(i).unwrap());
            let vec_ses = vec![ses];
            sessions.extend(vec_ses);
        }
        sessions
    }

    // Generate rows that connects all users in the vector
    pub fn mock_nbr_full_connect(users: &Vec<u32>) -> Vec<Edge> {
        let mut sessions: Vec<Edge> = Vec::new();
        for i in 0..users.len() {
            for j in i+1..users.len() {
                let ses = Edge::new(users.get(i).unwrap(), users.get(j).unwrap());
                sessions.push(ses);
            }
        }
        sessions
    }

}