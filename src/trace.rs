

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

    // impl TraceData {
    //     pub fn new(id: u32, trace_key: [u8; 16]) -> TraceData {
    //         TraceData { uid: id, key: trace_key }
    //     }
    // }

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

    // fn fwd_search() {

    // }
    // TODO: 重构输入为msg: &[u8]
    pub fn backward_search(msg: String, md: TraceData) -> TraceData {
        let sessions = pack_storage::query_users(md.uid, FwdType::Receive);
        let binding = decode(msg.clone()).unwrap();
        let msg_bytes = <&[u8]>::try_from(&binding[..]).unwrap();
        for sess in &sessions {
            let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
            let b = algos::tag_exists(&md.key, &bk, msg_bytes);
            while b == true {
                let prev_key = algos::prev_key(&md.key, &bk);
                return TraceData { uid: sess.sender, key:  prev_key};
            }
        }
        // return uid = 0, when found not one.
        TraceData { uid: 0, key: md.key }
    }

    pub fn forward_search(msg: String, md: TraceData) -> Vec<TraceData> {
        let mut result = Vec::new();
        let sessions = pack_storage::query_users(md.uid, FwdType::Send);
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
    use crate::trace::traceback::tracing;
    use super::*;
    use super::traceback::{backward_search, forward_search, TraceData};
    use std::mem::{self, MaybeUninit};

    // generate a new edge from a sender to a receiver
    fn new_edge_gen(message: String, sender: u32, receiver: u32) -> MsgPacket {
        let bk = &base64::decode(pack_storage::query_sid(sender.clone(), receiver.clone()).clone()).unwrap()[..];
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let packet = messaging::new_msg(bk_16, message.clone());
        let sess = Session::new( encode(bk_16), sender, receiver);
        while messaging::proc_msg( sess, MsgPacket { key: packet.key, tag: packet.tag, payload: message }) != true {
            panic!("process failed.");
        }
        packet
    }

    // generate a forward edge from a sender to a receiver
    fn fwd_edge_gen(prev_key: [u8; 16], message: String, sender: u32, receiver: u32) {
        let bk = &base64::decode(pack_storage::query_sid(sender, receiver)).unwrap()[..];
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let sess = Session::new( encode(bk_16), sender, receiver);
        let packet = messaging::fwd_msg(&prev_key, bk_16, message, FwdType::Receive);
        while messaging::proc_msg( sess, packet) != true {
            panic!("process failed.");
        }
    }

    // generate a forward path from once send
    fn fwd_path_gen(message: String, users: Vec<u32>) -> [u8; 16] {
        let mut keys: Vec<[u8;16]> = Vec::new();
        let mut sessions: Vec<Session> = Vec::new();
        let mut report_key = MaybeUninit::<[u8; 16]>::uninit();

        // path U1 -> U5
        for i in 0..4 {
            // TODO: mock path
            let t_sender = users.get(i).unwrap();
            let t_receiver = users.get(i+1).unwrap();
            sessions.push(Session::new(0.to_string(), *t_sender, *t_receiver));

            let bk = &base64::decode(pack_storage::query_sid(t_sender.clone(), t_receiver.clone()).clone()).unwrap()[..];
            if i==0 {
                let packet = new_edge_gen(message.clone(), t_sender.clone(), t_receiver.clone());
                keys.push(packet.key);
            } else {
                let prev_key = *keys.get(i-1).unwrap();
                fwd_edge_gen(prev_key, message.clone(), t_sender.clone(), t_receiver.clone());
                let next_key = algos::next_key(&prev_key, <&[u8; 16]>::try_from(bk).unwrap());
                keys.push(next_key);
            }
            
            if i == 1 {
                report_key.write(*keys.get(i).unwrap());
            }
        }
        unsafe {report_key.assume_init()}
    }

    #[test]
    fn test_bwd_search() {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        // Generate a mock path, if doesn't exists.
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = fwd_path_gen(message.clone(), users.clone());
        // Search this message from middle node
        let result = backward_search(message, TraceData {uid: *users.get(2).unwrap(), key: report_key});
        assert_eq!(result.uid, *users.get(1).unwrap());
    }

    #[test]
    fn test_fwd_search() {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        // Generate a mock path, if doesn't exists.
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = fwd_path_gen(message.clone(), users.clone());
        // Search this message from middle node
        let result = forward_search(message, TraceData {uid: *users.get(2).unwrap(), key: report_key});
        assert_eq!(result.get(0).unwrap().uid, *users.get(3).unwrap());
    }

    #[test]
    fn test_tracing() {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        // Generate a mock path, if doesn't exists.
        let msg_bytes = rand::random::<[u8; 16]>();
        let message = encode(&msg_bytes[..]);
        let report_key = fwd_path_gen(message.clone(), users.clone());
        // Search this message from middle node
        let msg_path = tracing(MsgReport { key: report_key, payload: message }, *users.get(2).unwrap());
        if msg_path.is_empty() {
            println!("No path");
        } else {
            for edge in &msg_path {
                edge.show();
            }
            println!();
        }
    }
}