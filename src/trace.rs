

pub mod traceback {
    use hmac::digest::impl_write;

    use crate::message::messaging::{MsgReport, FwdType, MsgPacket, Session};
    use crate::tool::{algos, utils};
    use crate::db::{bloom_filter, pack_storage};
    use crate::base64::{decode};

    struct TraceData {
        uid: u32,
        key: [u8; 16],
    }

    // impl TraceData {
    //     pub fn build(id: u32, trace_key: [u8; 16]) -> TraceData {
    //         TraceData { uid: id, key: trace_key }
    //     }
    // }

    struct Edge {
        sender: u32,
        receiver: u32,
    }

    impl Edge {
        pub fn build(snd_id: u32, rcv_id: u32) -> Edge {
            Edge { sender: snd_id, receiver: rcv_id }
        }
        pub fn show(&self) {
            print!("{} - {} ", self.sender, self.receiver);
        }
    }

    // fn fwd_search() {

    // }

    fn bwd_search(msg: String, md: TraceData) -> TraceData {
        let sessions = pack_storage::query_users(md.uid, FwdType::Receive);
        for sess in &sessions {
            let bk = <&[u8; 16]>::try_from(&decode(sess.id.clone()).unwrap()[..]).unwrap().clone();
            let b = algos::tag_exists(&md.key, &bk, msg.as_bytes());
            while b == true {
                let prev_key = algos::prev_key(&md.key, &bk);
                return TraceData { uid: sess.sender, key:  prev_key};
            }
        }
        let empty_uid:u32 = 0;
        TraceData { uid: empty_uid, key: md.key }
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

    
    // Path: U1 -> U2 -> U3 -> U4 -> U5, where U3 is the start point
    #[test]
    fn mock_path_1() {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301];
        let message = rand::random::<[u8; 16]>();
        let msg_str = base64::encode(&message[..]);
        let mut datas: Vec<MsgPacket> = Vec::new();
        let mut sessions: Vec<Session> = Vec::new();

        for i in 0..3 {
            // TODO: mock path
            let sid: String = String::from(" ");
            let sess = Session::build(sid, *users.get(i).unwrap(), *users.get(i+1).unwrap());
println!("{:?}",sess);
            sessions.push(sess);

let t_sess = Session::build(encode(message), users.get(i).unwrap().clone(), users.get(i+1).unwrap().clone());
            let bk = &base64::decode(pack_storage::query_sid(t_sess).clone()).unwrap()[..];
            let mut packet: MsgPacket;
            if i==0 {
                packet = messaging::new_msg(<&[u8; 16]>::try_from(bk).unwrap(), msg_str.clone());
            } else {
                let prev_packet = datas.get(i-1).unwrap();
                packet = messaging::fwd_msg(&prev_packet.key, <&[u8; 16]>::try_from(bk).unwrap(), prev_packet.payload.clone(), messaging::FwdType::Receive);
            }
println!("{:?}",packet);
            datas.push(packet);
println!("Length of sessions: {}", sessions.len());
println!("Length of datas: {}", datas.len());

let t_sess = sessions.get(i).unwrap();
let proc_sess = Session::build(t_sess.id.clone(), t_sess.sender, t_sess.receiver);
let t_data =  datas.get(i).clone().unwrap();
let proc_data = MsgPacket::build(&t_data.key, t_data.payload.clone());

            while messaging::proc_msg( proc_sess, proc_data) != true {
                panic!("process failed.");
            }
        }
    }

    #[test]
    fn test_bwd_search() {
        
    }
}