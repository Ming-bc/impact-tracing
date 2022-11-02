

pub mod traceback {
    use hmac::digest::impl_write;

    use crate::message::messaging::{MsgReport, FwdType, MsgPacket, Session};
    use crate::tool::utils;
    use crate::db::{bloom_filter, pack_storage};

    struct TraceData {
        uid: u32,
        key: [u8; 16],
    }

    impl TraceData {
        pub fn build(id: u32, trace_key: [u8; 16]) -> TraceData {
            TraceData { uid: id, key: trace_key }
        }
    }

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

    fn bwd_search(msg: String, metadata: TraceData) -> TraceData {
        let sessions = pack_storage::query_users(metadata.uid, FwdType::Receive);
        for i in &sessions {
            
        }
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