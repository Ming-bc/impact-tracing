

pub mod traceback {
    extern crate base64;

    use std::collections::HashSet;
    use crate::message::messaging::{MsgReport, FwdType, MsgPacket, Session};
    use crate::tool::algos;
    use crate::db::pack_storage;
    use base64::{decode, encode};

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

    pub fn backward_search(msg: &str, md: TraceData) -> TraceData {
        let sessions = pack_storage::query_users(&md.uid, FwdType::Receive);
        let binding = decode(msg).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();
        for sess in &sessions {
            let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
            let b = algos::tag_exists(&md.key, &bk, msg_bytes);
            while b == true {
                let prev_key = algos::prev_key(&md.key, &bk);
                return TraceData::new(sess.sender, prev_key);
            }
        }
        // return uid = 0, when found not one.
        TraceData::new(0, md.key)
    }

    pub fn forward_search(msg: &str, md: TraceData) -> Vec<TraceData> {
        let mut result = Vec::new();
        let sessions = pack_storage::query_users(&md.uid, FwdType::Send);
        let binding = decode(msg).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();
        
        for sess in &sessions {
            let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
            let next_key = algos::next_key(&md.key, &bk);
            let b = algos::tag_exists(&next_key, &bk, msg_bytes);
            if b == true {
                result.push(TraceData {uid: sess.receiver, key: next_key});
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
            // Search the previous node of the sender
            if current_sender.uid != 0 {
                let prev_sender = backward_search(&report.payload, TraceData { uid: current_sender.uid, key: current_sender.key });
                if prev_sender.uid != 0 {
                    path.push(Edge::new(prev_sender.uid, current_sender.uid.clone()));
                }
                // push sender into receiver set
                rcv_set.push(TraceData { uid: current_sender.uid, key: current_sender.key });
                current_sender = TraceData::from(prev_sender);
            }
            // Search the receivers of the message
            let rcv_len_at_begin = rcv_set.len();
            if rcv_set.is_empty() == false {
                let mut outside_set: Vec<TraceData> = Vec::new();
                for out_td in &rcv_set {
                    let mut inside_set = forward_search(&report.payload, TraceData { uid: out_td.uid, key: out_td.key });

                    for i in 0..inside_set.len() {
                        let rcv = inside_set.get(i).unwrap();
                        if searched_rcv.contains(&rcv.uid) {
                            inside_set.remove(i);
                        }
                    }

                    if inside_set.is_empty() == false {
                        for in_td in & inside_set {
                            path.push(Edge::new(out_td.uid, in_td.uid))
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
                searched_rcv.insert(user.uid);
            }
        }
        path
    }
}


#[cfg(test)]
mod tests {
    extern crate base64;

    use std::panic::UnwindSafe;

    use base64::encode;
    use rand;
    
    use crate::message::messaging;
    use crate::tool::algos;
    use crate::db::pack_storage;
    use crate::trace::traceback;
    use crate::visualize::display;

    use crate::message::messaging::{FwdType, MsgPacket, Session, MsgReport};
    use super::traceback::{TraceData};


    #[test]
    fn test_bwd_search() {
        // Generate a mock edge at first
        let sender: u32 = 2806396777;
        let receiver: u32 = 259328394;
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = new_edge_gen(&message, &sender, &receiver).key;
        // Bwd Search
        let result = traceback::backward_search(&message, TraceData::new(receiver, report_key));
        assert_eq!(result.uid, sender);
    }

    #[test]
    fn test_fwd_search() {
        let (users, keys, message) = path_cases_1();
        let report_key = keys.get(1).unwrap();
        // Search this message from middle node
        let result = traceback::forward_search(&message, TraceData::new(*users.get(2).unwrap(), *report_key));
        assert_eq!(result.get(0).unwrap().uid, *users.get(3).unwrap());
    }

    #[test]
    fn test_tracing() {
        // Path case 1: 1-2-3-4-5
        {
            let (users, keys, message) = path_cases_2();
            let report_key = keys.get(1).unwrap();
    
            // Search this message from middle node
            let msg_path = traceback::tracing(MsgReport::new(*report_key, message), users.get(2).unwrap());
            assert_eq!(msg_path.is_empty(), false);
            let refined_path = display::refine_user_id(users, msg_path);
            for edge in &refined_path {
                edge.show();
            }
            println!("");
        }
        
    }

        // generate a new edge from a sender to a receiver
        pub fn new_edge_gen(message: &str, sender: &u32, receiver: &u32) -> MsgPacket {
            let bk = &base64::decode(pack_storage::query_sid(&sender, &receiver).clone()).unwrap()[..];
            let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
            let packet = messaging::new_msg(bk_16, message);
            let sess = Session::new( encode(bk_16), *sender, *receiver);
            while messaging::proc_msg( sess, MsgPacket::new(&packet.key, message) ) != true {
                panic!("process failed.");
            }
            packet
        }
    
        // generate a forward edge from a sender to a receiver
        fn fwd_edge_gen(prev_key: [u8; 16], message: &str, sender: &u32, receiver: &u32) {
            let bk = &base64::decode(pack_storage::query_sid(sender, receiver)).unwrap()[..];
            let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
            let sess = Session::new( encode(bk_16), *sender, *receiver);
            let packet = messaging::fwd_msg(&prev_key, bk_16, message, FwdType::Receive);
            while messaging::proc_msg( sess, packet) != true {
                panic!("process failed.");
            }
        }
    
        fn fwd_path_gen(start_key: &[u8; 16], message: &str, users: Vec<u32>) -> Vec<[u8; 16]> {
            let mut keys: Vec<[u8;16]> = Vec::new();
            let mut sessions: Vec<Session> = Vec::new();
    
            keys.push(*start_key);
    
            for i in 0..(users.len()-1) {
                let sender = users.get(i).unwrap();
                let receiver = users.get(i+1).unwrap();
                let bk = &base64::decode(pack_storage::query_sid(&sender, &receiver).clone()).unwrap()[..];
                sessions.push(Session::new(0.to_string(), *sender, *receiver));
    
                let prev_key = *keys.get(i).unwrap();
                fwd_edge_gen(prev_key, message, &sender, &receiver);
                let next_key = algos::next_key(&prev_key, <&[u8; 16]>::try_from(bk).unwrap());
                keys.push(next_key);
            }
            // TODO: reconstruct new_path_gen
            keys
        }

        // Normal tree: 1-2-3-4-5
        fn path_cases_1() -> (Vec<u32>, Vec<[u8;16]>, String) {
            let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
            let msg_bytes = rand::random::<[u8; 16]>();
            let message = encode(&msg_bytes[..]);

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