

pub mod traceback {
    extern crate base64;

    use std::collections::HashSet;
    use crate::message::messaging::{MsgReport, FwdType, MsgPacket, Session};
    use crate::tool::algos::{self, store_tag_gen};
    use crate::db::{redis_pack, bloom_filter};
    use base64::{decode, encode};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct TraceData {
        pub uid: u32,
        pub key: [u8; 16],
    }

    impl TraceData {
        pub fn new(id: u32, trace_key: [u8; 16]) -> TraceData {
            TraceData { uid: id, key: trace_key }
        }
    }

    pub struct Edge {
        pub sender: u32,
        pub receiver: u32,
    }

    impl Edge {
        pub fn new(snd_id: u32, rcv_id: u32) -> Edge {
            Edge { sender: snd_id, receiver: rcv_id }
        }
        pub fn show(&self) {
            print!("U{} - U{}, ", self.sender, self.receiver);
        }
    }

    pub fn backward_search(msg: &str, md: TraceData) -> (TraceData, Vec<TraceData>) {
        let sessions = redis_pack::query_users(&md.uid, FwdType::Receive);
        let binding = decode(msg).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();

        let mut tags_tbt: Vec<String> = Vec::new();
        let mut next_key_set: Vec<[u8; 16]> = Vec::new(); 

// let time0 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        for sess in &sessions {
            // bwd tag
            let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
            let bwd_tag = store_tag_gen(&sess.sender, &md.key, &bk, msg_bytes);
            // fwd tag
            let next_key = algos::next_key(&md.key, &bk);
            let fwd_tag = store_tag_gen(&md.uid, &next_key, &bk, msg_bytes);

            next_key_set.push(next_key);
            tags_tbt.push(encode(&bwd_tag[..]));
            tags_tbt.push(encode(&fwd_tag[..]));
        }

// let time1 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
// println!("Part 1 runtime: {:?}", time1 - time0);
        let bf_result = bloom_filter::mexists(&tags_tbt);
        let mut source: TraceData = TraceData::new(0, md.key);
        let mut receivers: Vec<TraceData> = Vec::new();

        for i in 0..(bf_result.len()/2)  {
            if *bf_result.get(2*i).unwrap() == true {
                let sess = sessions.get(i).unwrap();
                let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
                let prev_key = algos::prev_key(&md.key, &bk);
                source = TraceData::new(sess.sender, prev_key);
                break;
            }
        }

        for i in 0..(bf_result.len()/2)  {
            if *bf_result.get(2*i + 1).unwrap() == true {
                let sess = sessions.get(i).unwrap();
                let next_key = next_key_set.get(i).unwrap();
                receivers.push(TraceData {uid: sess.sender, key: *next_key})
            }
        }
        // return uid = 0, when found not one.
        (source, receivers)
    }

    pub fn forward_search(msg: &str, md: TraceData) -> Vec<TraceData> {
        let mut result = Vec::new();
        let sessions = redis_pack::query_users(&md.uid, FwdType::Send);
        let binding = decode(msg).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();

        let mut tags_tbt: Vec<String> = Vec::new();
        let mut next_key_set: Vec<[u8; 16]> = Vec::new();
let time0 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        for sess in &sessions {
            let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
            let next_key = algos::next_key(&md.key, &bk);
            let tag = store_tag_gen(&sess.sender, &next_key, &bk, msg_bytes);

            next_key_set.push(next_key);
            tags_tbt.push(encode(&tag[..]));
        }
let time1 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
// println!("Fwd search part 1 runtime: {:?}", time1 - time0);

        let bf_result = bloom_filter::mexists(&tags_tbt);
        for i in 0..bf_result.len() {
            if *bf_result.get(i).unwrap() == true {
                let sess = sessions.get(i).unwrap();
                let next_key = next_key_set.get(i).unwrap();
                result.push(TraceData {uid: sess.receiver, key: *next_key})
            }
        }

        // return empty vector, when found not one.
        result
    }

    pub fn tracing(report: MsgReport, snd_start: &u32) -> Vec<Edge>{
        let mut path: Vec<Edge> = Vec::new();
        let mut current_sender= TraceData { uid: *snd_start, key: report.key };
        let mut rcv_set: Vec<TraceData> = Vec::new();
        let mut searched_rcv: HashSet<u32> = HashSet::new();

        while (current_sender.uid != 0) | (rcv_set.is_empty() == false) {
            let mut snd_to_rcvs: Vec<TraceData> = Vec::new();
            // Search the previous node of the sender
            if current_sender.uid != 0 {
                let prev_sender: TraceData;
// let gen_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                (prev_sender, snd_to_rcvs) = backward_search(&report.payload, TraceData { uid: current_sender.uid, key: current_sender.key });
// let gen_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
// println!("Backward search runtime: {:?}", gen_end - gen_start);
                if prev_sender.uid != 0 {
                    path.push(Edge::new(prev_sender.uid, current_sender.uid.clone()));
                }

                
                // searched or not
                for i in 0..snd_to_rcvs.len() {
                    let rcv = snd_to_rcvs.get(i).unwrap();
                    if searched_rcv.contains(&rcv.uid) {
                        snd_to_rcvs.remove(i);
                    }
                    else {
                        path.push(Edge::new(current_sender.uid, snd_to_rcvs.get(i).unwrap().uid))
                    }
                }
                searched_rcv.insert(current_sender.uid);
                current_sender = TraceData::from(prev_sender);
            }

            // Search the receivers of the message
            let rcv_len_at_begin = rcv_set.len();
            if rcv_set.is_empty() == false {
                let mut outside_set: Vec<TraceData> = Vec::new();
                for out_td in &rcv_set {
// let gen_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    let mut inside_set = forward_search(&report.payload, TraceData { uid: out_td.uid, key: out_td.key });
// let gen_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
// println!("Forward search runtime: {:?}", gen_end - gen_start);
                    // searched or not ?
                    for i in 0..inside_set.len() {
                        let rcv = inside_set.get(i).unwrap();
                        if searched_rcv.contains(&rcv.uid) {
                            inside_set.remove(i);
                        }
                    }
                    // if first found, then put in path TODO: reconstruct
                    if inside_set.is_empty() == false {
                        for in_td in & inside_set {
                            path.push(Edge::new(out_td.uid, in_td.uid))
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
                searched_rcv.insert(user.uid);
            }
        }
        path
    }
}


#[cfg(test)]
mod tests {
    extern crate base64;
    extern crate test;

    use std::f32::consts::E;
    use std::panic::UnwindSafe;

    use base64::encode;
    use rand;
    use test::Bencher;
    
    use crate::message::messaging;
    use crate::tool::algos;
    use crate::db::redis_pack;
    use crate::trace::traceback;
    use crate::visualize::display;
    use crate::db::tests::{mock_rows_line, mock_rows_star, mock_rows_full_connect};

    use crate::message::messaging::{FwdType, MsgPacket, Session, MsgReport};
    use super::traceback::{TraceData};
    use std::time::{SystemTime, Duration, UNIX_EPOCH};
    use std::thread;

    const OURS_BRANCH: u32 = 10;


    #[test]
    fn test_bwd_search() {
        // Generate a mock edge at first
        let users: Vec<u32> = vec![2806396777, 259328394];
        let sender: u32 = 2806396777;
        let receiver: u32 = 259328394;
        mock_rows_full_connect(&users);

        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = new_edge_gen(&message, &sender, &receiver).key;
        // Bwd Search
        let (result, _) = traceback::backward_search(&message, TraceData::new(receiver, report_key));
        assert_eq!(result.uid, sender);

        // let mut conn = redis_pack::connect().unwrap();
        // let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_fwd_search() {
        let (users, keys, message) = path_cases_1();
        let report_key = keys.get(1).unwrap();
        // Search this message from middle node
        let result = traceback::forward_search(&message, TraceData::new(*users.get(2).unwrap(), *report_key));
        assert_eq!(result.get(0).unwrap().uid, *users.get(3).unwrap());

        // let mut conn = redis_pack::connect().unwrap();
        // let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_tracing() {
        // Path case 1: 1-2-3-4-5
        // Path case 2: 1-2-3-4-5, 3-6-7, 6-8
        let (users, keys, message) = path_cases_2();
        let report_key = keys.get(1).unwrap();

        // Search this message from middle node
        let msg_path = traceback::tracing(MsgReport::new(*report_key, message), users.get(2).unwrap());
        assert_eq!(msg_path.is_empty(), false);

        let (refined_users, refined_path) = display::refine_user_id(users, msg_path);
        for edge in &refined_path {
            edge.show();
        }
        println!("");

        display::vec_to_dot(refined_users, refined_path);

        // let mut conn = redis_pack::connect().unwrap();
        // let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_tracing_in_abitary_path() {
        let path_length: u32 = 23619;
        let branch_factor: u32 = OURS_BRANCH;
        
let gen_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let (sess, key, message) = arbitary_path_gen(path_length, branch_factor);
let gen_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Gentime: {:?}", gen_end - gen_start);

let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = traceback::tracing(MsgReport::new(key, message.clone()), &sess.receiver);
let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

println!("Runtime: {:?}", trace_end - trace_start);
        assert_eq!(path.len() as u32, path_length - 1);

        let mut conn = redis_pack::connect().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[test]
    fn test_tracing_in_abitary_tree() {
        let depth: u32 = 6;
        let branch_factor: u32 = 3;
        let mut fwd_tree_edges: u32 = 1;
        let mut search_tree_size: u32 = 1;
        for i in 0..(depth + 1) {
            fwd_tree_edges += branch_factor.pow(i);
            if (i == depth) | (i == 0) {
                search_tree_size += 9 * branch_factor.pow(i);
            }
            else {
                search_tree_size += branch_factor.pow(i) * (10 - branch_factor - 1);
            }
        }
        println!("Forward tree size is {}", fwd_tree_edges - 1);
        println!("Search tree size is {}", search_tree_size - 1);

let gen_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let (sess, key, message) = arbitary_tree_gen(depth, branch_factor);
let gen_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Gentime: {:?}", gen_end - gen_start);

let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = traceback::tracing(MsgReport::new(key, message.clone()), &sess.receiver);
let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

println!("Runtime: {:?}", trace_end - trace_start);
        assert_eq!(path.len() as u32, fwd_tree_edges - 1);


        let mut conn = redis_pack::connect().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[bench]
    fn bench_bwd_search(b: &mut Bencher) {
        let users: Vec<u32> = vec![2806396777, 259328394];
        let sender: u32 = 2806396777;
        let receiver: u32 = 259328394;
        mock_rows_full_connect(&users);

        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = new_edge_gen(&message, &sender, &receiver).key;
        // Bwd Search
        b.iter(|| traceback::backward_search(&message, TraceData::new(receiver, report_key)));

        // let mut conn = redis_pack::connect().unwrap();
        // let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    #[bench]
    fn bench_fwd_search(b: &mut Bencher) {
        let (users, keys, message) = path_cases_1();
        let report_key = keys.get(1).unwrap();
        // Search this message from middle node
        b.iter(|| traceback::forward_search(&message, TraceData::new(*users.get(2).unwrap(), *report_key)));
    }

    #[bench]
    fn bench_tracing_in_abitary_path(b: &mut Bencher) {
        let path_length: u32 = 100;
        let branch_factor: u32 = 10;
        let (sess, key, message) = arbitary_path_gen(path_length, branch_factor);

        b.iter(|| traceback::tracing(MsgReport::new(key, message.clone()), &sess.receiver));

        let mut conn = redis_pack::connect().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut conn).unwrap();
    }

    // generate a new edge from a sender to a receiver
    pub fn new_edge_gen(message: &str, sender: &u32, receiver: &u32) -> MsgPacket {
        let bk = &base64::decode(redis_pack::query_sid(&sender, &receiver).clone()).unwrap()[..];
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let packet = messaging::new_msg(bk_16, message);
        let sess = Session::new( encode(bk_16), *sender, *receiver);
        while messaging::proc_msg( sess, MsgPacket::new(&packet.key, message) ) != true {
            panic!("process failed.");
        }
        packet
    }
    
    // generate a forward edge from a sender to a receiver
    fn fwd_edge_gen(prev_key: [u8; 16], message: &str, sender: &u32, receiver: &u32) -> MsgPacket {
        let bk = &base64::decode(redis_pack::query_sid(sender, receiver)).unwrap()[..];
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let sess = Session::new( encode(bk_16), *sender, *receiver);
        let packet = messaging::fwd_msg(&prev_key, bk_16, message, FwdType::Receive);
        while messaging::proc_msg(sess, MsgPacket::new(&packet.key, message)) != true {
            panic!("process failed.");
        }
        packet
    }

    fn fwd_path_gen(start_key: &[u8; 16], message: &str, users: Vec<u32>) -> Vec<[u8; 16]> {
        let mut keys: Vec<[u8;16]> = Vec::new();
        let mut sessions: Vec<Session> = Vec::new();

        keys.push(*start_key);

        for i in 0..(users.len()-1) {
            let sender = users.get(i).unwrap();
            let receiver = users.get(i+1).unwrap();

            let sid = redis_pack::query_sid(&sender, &receiver);
            let bk = &base64::decode(sid.clone()).unwrap()[..];

            sessions.push(Session::new(0.to_string(), *sender, *receiver));

            let prev_key = *keys.get(i).unwrap();
            fwd_edge_gen(prev_key, message, &sender, &receiver);
            let next_key = algos::next_key(&prev_key, <&[u8; 16]>::try_from(bk).unwrap());
            keys.push(next_key);

// TODO: avoid query error
if i % 100 == 0 {
    thread::sleep(Duration::from_millis(50));
}
        }
        // TODO: reconstruct new_path_gen
        keys
    }

    pub fn redis_mock_rows_line(users: &Vec<u32>) -> Vec<Session> {
        let mut sessions: Vec<Session> = Vec::new();
        for i in 0..(users.len()-1) {
            let bytes = rand::random::<[u8; 16]>();
            let sid = encode(&bytes[..]);

            let ses = Session::new(sid, *users.get(i).unwrap(), *users.get(i+1).unwrap());
            let vec_ses = vec![ses];
            sessions.extend(vec_ses);
        }
        sessions
    }

    pub fn redis_mock_rows_star(users: &Vec<u32>) -> Vec<Session> {
        let mut sessions: Vec<Session> = Vec::new();
        let central = users.get(0).unwrap();
        for i in 1..users.len() {
            let bytes = rand::random::<[u8; 16]>();
            let sid = encode(&bytes[..]);

            let ses = Session::new(sid, *central, *users.get(i).unwrap());
            let vec_ses = vec![ses];
            sessions.extend(vec_ses);
        }
        sessions
    }

    // generate a path
    fn arbitary_path_gen (length_of_path: u32, num_of_sess_per_user: u32) -> (Session, [u8; 16], String)  {
        // generate sessions
        let mut search_sessions: Vec<Session> = Vec::new();
        let mut users: Vec<u32> = Vec::new();
        for i in 0..length_of_path {
            // let u_1 = rand::random::<u32>();
            let u_1 = i * 10 + 1;
            users.push(u_1);
            let mut sess_of_user: Vec<u32> = Vec::new();
            sess_of_user.push(u_1);
            for j in 0..(num_of_sess_per_user-1) {
                // let u_2 = rand::random::<u32>();
                let u_2 = u_1 + j + 1;
                sess_of_user.push(u_2);
            }
            // mock_rows_star(&sess_of_user);
            search_sessions.extend(redis_mock_rows_star(&sess_of_user));
        }
        // mock_rows_line(&users);
        search_sessions.extend(redis_mock_rows_line(&users));
        
        let _ = redis_pack::pipe_add_auto_cut(&mut search_sessions);

        // generate forward path
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);

        let first_packet = new_edge_gen(&message, users.get(0).unwrap(), users.get(1).unwrap());
        let mut path_keys = fwd_path_gen(&first_packet.key, &message, users.clone().split_off(1));

        // return the last session, key and message
        let report_key = path_keys.pop().unwrap();
        let receiver = users.pop().unwrap();
        let sender = users.pop().unwrap();
        let report_sess = Session::new(0.to_string(), sender, receiver);
        
        (report_sess, report_key, message)
    }

    fn arbitary_tree_gen (depth: u32, branch: u32) -> (Session, [u8; 16], String)  {
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);

        let start = rand::random::<u32>();
        let root = rand::random::<u32>();
        mock_rows_line(&vec![start, root]);
        let root_packet = new_edge_gen(&message, &start, &root);
        fwd_tree_gen(&root_packet.key, &root, &message, depth, &branch);

        (Session::new(0.to_string(), start, root), root_packet.key, message)
    }
        
    // -> (Vec<u32>, Vec<[u8; 16]>)
    fn fwd_tree_gen(start_key: &[u8; 16], root: &u32, message: &str, depth: u32, branch: &u32) {
        if depth != 0 {
            let mut users: Vec<u32> = Vec::new();
            users.push(*root);
            for i in 0..*branch {
                let u = rand::random::<u32>();
                users.push(u);
            }
            // add fake users to fill sessions per user
            if branch > &OURS_BRANCH {
                panic!("Tree branch is larger than ours branch factor!")
            }
            else {
                for j in 0..(OURS_BRANCH - *branch) {
                    let u = rand::random::<u32>();
                    users.push(u);
                }
            }
thread::sleep(Duration::from_millis(10));
            // write bk to db
            mock_rows_star(&users);
            let nodes = users.split_off(1);
            
            let mut fwd_keys: Vec<[u8; 16]> = Vec::new();
            for k in 0..*branch as usize {
// println!("depth {}, branch {}", depth, i);
                let receiver = nodes.get(k).unwrap();
                let packet = fwd_edge_gen(*start_key, message, root, receiver);
                fwd_keys.push(packet.key);
                let _ = fwd_tree_gen(&packet.key, receiver, message, depth - 1, branch);
            }
        }
    }

    // Normal tree: 1-2-3-4-5
    fn path_cases_1() -> (Vec<u32>, Vec<[u8;16]>, String) {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);

        mock_rows_full_connect(&users);

        // generate the first edge of a path: 1-2
        let first_packet = new_edge_gen(&message, users.get(0).unwrap(), users.get(1).unwrap());
        // generate a forward path: 2-3-4-5
        let mut path_keys = fwd_path_gen(&first_packet.key, &message, users.clone().split_off(1));
        (users, path_keys, message)
    }

    // More complex tree: 1-2-3-4-5, 3-6-7, 6-8
    fn path_cases_2() -> (Vec<u32>, Vec<[u8;16]>, String) {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301, 902146735, 4206663226, 2261102179];
        // Generate a mock path, if doesn't exists.
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        mock_rows_full_connect(&users);

        // Path 0: 1-2
        let first_packet = new_edge_gen(&message, users.get(0).unwrap(), users.get(1).unwrap());

        // Path 1: 2-3-4-5
        let users_path_1: Vec<u32> = vec![259328394, 4030527275, 1677240722, 1888975301];
        let start_key_1 = &first_packet.key;
        let mut keys_1 = fwd_path_gen(&start_key_1, &message, users_path_1);

        // Path 2: 3-6-7
        let users_path_2: Vec<u32> = vec![4030527275, 902146735, 4206663226];
        let start_key_2 = &keys_1.get(1).unwrap();
        let mut keys_2 = fwd_path_gen(&start_key_2, &message, users_path_2);

        // Path 3: 6-8
        let users_path_2: Vec<u32> = vec![902146735, 2261102179];
        let start_key_2 = &keys_2.get(1).unwrap();
        let mut keys_3 = fwd_path_gen(&start_key_2, &message, users_path_2);

        keys_1.append(&mut keys_2.split_off(1));
        keys_1.append(&mut keys_3.split_off(1));

        (users, keys_1, message)
    }

}