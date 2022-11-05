

pub mod traceback {
    use std::collections::HashSet;

    use hmac::digest::impl_write;

    use crate::message::messaging::{MsgReport, FwdType, MsgPacket, Session, fwd_msg};
    use crate::tool::{algos, utils};
    use crate::db::{bloom_filter, pack_storage};
    use crate::base64::{decode, encode};

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
            print!("{} - {}, ", self.sender, self.receiver);
        }
    }

    // pub fn display_path(users: Vec<u32>, path: Vec<Edge>) {

    // }

    pub fn backward_search(msg: String, md: TraceData) -> TraceData {
        let sessions = pack_storage::query_users(&md.uid, FwdType::Receive);
        let binding = decode(msg.clone()).unwrap();
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

    pub fn forward_search(msg: String, md: TraceData) -> Vec<TraceData> {
        let mut result = Vec::new();
        let sessions = pack_storage::query_users(&md.uid, FwdType::Send);
        let binding = decode(msg.clone()).unwrap();
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

    pub fn tracing(report: MsgReport, snd_start: u32) -> Vec<Edge>{
        let mut path: Vec<Edge> = Vec::new();
        let mut current_sender= TraceData { uid: snd_start, key: report.key };
        let mut rcv_set: Vec<TraceData> = Vec::new();
        let mut searched_rcv: HashSet<u32> = HashSet::new();

        while (current_sender.uid != 0) | (rcv_set.is_empty() == false) {
            // Search the previous node of the sender
            if current_sender.uid != 0 {
                let prev_sender = backward_search(report.payload.clone(), TraceData { uid: current_sender.uid, key: current_sender.key });
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
                    let mut inside_set = forward_search(report.payload.clone(), TraceData { uid: out_td.uid, key: out_td.key });

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
    use base64::encode;
    use rand;
    use crate::message::messaging;
    use crate::message::messaging::{FwdType, MsgPacket, Session, MsgReport};
    use crate::tool::algos;
    use crate::base64;
    use crate::db::pack_storage;
    use crate::trace::traceback;
    use super::traceback::{TraceData};
    use std::mem::{MaybeUninit};

    #[test]
    fn test_bwd_search() {
        // Generate a mock edge at first
        let sender: u32 = 2806396777;
        let receiver: u32 = 259328394;
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = new_edge_gen(&message, &sender, &receiver).key;
        // Bwd Search
        let result = traceback::backward_search(message, TraceData::new(receiver, report_key));
        assert_eq!(result.uid, sender);
    }

    #[test]
    fn test_fwd_search() {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        // Generate a mock path, if doesn't exists.
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = new_path_gen(&message, users.clone());
        // Search this message from middle node
        let result = traceback::forward_search(message, TraceData::new(*users.get(2).unwrap(), report_key));
        assert_eq!(result.get(0).unwrap().uid, *users.get(3).unwrap());
    }

    #[test]
    fn test_tracing() {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        // Generate a mock path, if doesn't exists.
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = new_path_gen(&message, users.clone());
        // Search this message from middle node
        let msg_path = traceback::tracing(MsgReport::new(report_key, message), *users.get(2).unwrap());
        if msg_path.is_empty() {
            println!("No path");
        } else {
            for edge in &msg_path {
                edge.show();
            }
            println!();
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
    
        fn fwd_path_gen(start_key: &[u8; 16], message: &str, users: Vec<u32>) -> [u8; 16] {
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
            *keys.get(1).unwrap()
        }
    
        // generate a forward path from the source to some receivers
        fn new_path_gen(message: &str, users: Vec<u32>) -> [u8; 16] {
            let mut keys: Vec<[u8;16]> = Vec::new();
            let mut sessions: Vec<Session> = Vec::new();
            let mut report_key = MaybeUninit::<[u8; 16]>::uninit();
    
            let first_packet = new_edge_gen(message, users.get(0).unwrap(), users.get(1).unwrap());
            fwd_path_gen(&first_packet.key, message, users.clone().split_off(1))
        }

}