

pub mod traceback {
    use hmac::digest::impl_write;

    use crate::message::messaging::{MsgReport, FwdType, MsgPacket, Session};
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
        result
    }

    // pub fn tracing(report: MsgReport, sess: Session) {
    //     let mut path: Vec<Edge> = Vec::new();
    //     let mut snd_set: Vec<TraceData> = Vec::new();
    //     let mut rcv_set: Vec<TraceData> = Vec::new();
    //     snd_set.push(TraceData { uid: sess.sender, key: report.key });

    //     while (snd_set.len() != 0) & (rcv_set.len() != 0) {
    //         let snd_len = snd_set.len();
    //         let rcv_len = rcv_set.len();
            
    //         let t_snd_set = 
    //     }
    // }
}


#[cfg(test)]
mod tests {
    use base64::encode;
    use rand;
    use crate::message::messaging;
    use crate::message::messaging::{FwdType, MsgPacket, Session, MsgReport};
    use crate::base64;
    use crate::db::pack_storage;
    use super::*;
    use super::traceback::{backward_search, forward_search, TraceData};
    use std::mem::{self, MaybeUninit};

    
    // Path: U1 -> U2 -> U3 -> U4 -> U5, where U3 is the start point
    fn mock_path_1(message: String) -> [u8; 16] {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        let mut datas: Vec<MsgPacket> = Vec::new();
        let mut sessions: Vec<Session> = Vec::new();
        let mut report_key = MaybeUninit::<[u8; 16]>::uninit();

        for i in 0..4 {
            // TODO: mock path
            let sid: String = String::from(" ");
            let sess = Session::new(sid, *users.get(i).unwrap(), *users.get(i+1).unwrap());
            sessions.push(sess);

            let t_sess = Session::new(message.clone(), users.get(i).unwrap().clone(), users.get(i+1).unwrap().clone());
            let bk = &base64::decode(pack_storage::query_sid(t_sess).clone()).unwrap()[..];
            let packet: MsgPacket;
            if i==0 {
                packet = messaging::new_msg(<&[u8; 16]>::try_from(bk).unwrap(), message.clone());
            } else {
                let prev_packet = datas.get(i-1).unwrap();
                packet = messaging::fwd_msg(&prev_packet.key, <&[u8; 16]>::try_from(bk).unwrap(), prev_packet.payload.clone(), messaging::FwdType::Receive);
            }
            datas.push(packet);
            if i == 1 {
                report_key.write(datas.get(i).unwrap().key);
            }

let t_sess = sessions.get(i).unwrap();
let proc_sess = Session::new(t_sess.id.clone(), t_sess.sender, t_sess.receiver);
let t_data =  datas.get(i).clone().unwrap();
let proc_data = MsgPacket::new(&t_data.key, t_data.payload.clone());

            while messaging::proc_msg( proc_sess, proc_data) != true {
                panic!("process failed.");
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
        let report_key = mock_path_1(message.clone());
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
        let report_key = mock_path_1(message.clone());
        // Search this message from middle node
        let result = forward_search(message, TraceData {uid: *users.get(2).unwrap(), key: report_key});
        assert_eq!(result.get(0).unwrap().uid, *users.get(3).unwrap());
    }
}