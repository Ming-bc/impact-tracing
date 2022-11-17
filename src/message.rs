pub mod messaging {
    extern crate base64;

    use crate::tool::algos::*;
    // use crate::tool:: utils::*;
    use crate::db::{bloom_filter, redis_pack};
    use base64::decode;
    use serde::{Serialize, Deserialize};

    pub enum FwdType {
        Send,
        Receive,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct MsgPacket {
        pub key: [u8; 16],
        pub tag: [u8; 32],
        pub payload: String, // base64 encode string
    }

    impl MsgPacket {
        pub fn new(tag_key: &[u8; 16], message: &str) -> Self {
            let msg_tag = tag_gen(&tag_key, &decode(message.clone()).unwrap()[..]);
            MsgPacket {
                key: *tag_key,
                tag: msg_tag,
                payload: message.to_string(),
            }
        }

        pub fn vrf_tag(&self) -> bool {
            let tag_hat = tag_gen(&self.key, &decode(self.payload.clone()).unwrap()[..]);
            if tag_hat != self.tag {
                return false;
            }
            true
        }

    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct MsgReport {
        pub key: [u8; 16],
        pub payload: String,
    }

    impl MsgReport {
        pub fn new (report_key: [u8; 16], message: String) -> Self {
            MsgReport { key: report_key, payload: message }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Session {
        pub id: String,
        pub sender: u32,
        pub receiver: u32,
    }

    impl Session {
        pub fn new (sid: String, snd_id: u32, rcv_id: u32) -> Self {
            Session { id: sid, sender: snd_id, receiver: rcv_id }
        }
        
        pub fn show (&self) {
            println!("Sender {}, Receiver {}, ID {}", self.sender, self.receiver, self.id);
        }
    }

    // new_msg:
    pub fn new_msg(bk: &[u8; 16], message: &str) -> MsgPacket {
        let tag_key = new_key_gen(bk);
        MsgPacket::new(&tag_key, message)
    }
    // fwd_msg:
    pub fn fwd_msg(key: &[u8; 16], bk: &Vec<[u8; 16]>, message: &str, fwd_type: FwdType) -> MsgPacket {
        let tag_key:[u8; 16];
        let curr_bk = bk.get(0).unwrap();

        match fwd_type {
            FwdType::Send => {
                let prev_bk = bk.get(1).unwrap();
                let k_p = prev_key(key, prev_bk);
                tag_key = next_key(&k_p, curr_bk)
            },
            FwdType::Receive => tag_key = next_key(key, curr_bk),
        }
        MsgPacket::new(&tag_key, message)
    }

    pub fn send_packet(msg_packet: MsgPacket, session: Session) -> (MsgPacket, Session) {
        // TODO: serilize
        (msg_packet, session)
    }

    // proc_msg:
    pub fn proc_msg(sess: Session, packet: MsgPacket) -> bool {
        let sid = redis_pack::query_sid(&sess.sender, &sess.receiver).clone();
        let bk = <&[u8; 16]>::try_from(&decode(sid).unwrap()[..]).unwrap().clone();
        let store_tag = proc_tag(&sess.sender, &bk, &packet.tag);
        bloom_filter::add(&store_tag).is_ok()
    }

    // pub fn rcv_packet(msg_packet: MsgPacket, session: Session) -> bool {
        
    // }

    // vrf_msg:
    pub fn receive_msg(packet: MsgPacket) -> bool {
        // 1. Decrypts E2EE
        // 2. Compute ^tag
        packet.vrf_tag()
    }

    // report_msg:
    pub fn sub_report(tag_key: &[u8;16], message: &str, sender: u32, receiver: u32) -> (MsgReport, Session) {
        let sid = " ";
        (MsgReport { key: *tag_key, payload: message.to_string()}, Session::new(sid.to_string(), sender, receiver))
    }

    pub fn vrf_report(sess: Session, report: MsgReport) -> bool {
        let bk = &decode(redis_pack::query_sid(&sess.sender, &sess.receiver).clone()).unwrap()[..];

        tag_exists(&sess.sender, &report.key, <&[u8; 16]>::try_from(bk).unwrap(), &decode(report.payload.clone()).unwrap()[..])
    }

    
    
}


#[cfg(test)]
mod tests {
    extern crate base64;
    extern crate test;
    // use rand::random;

    use base64::{encode, decode};
    use test::Bencher;
    use crate::message::messaging::*;
    use crate::tool::algos::*;

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    #[test]
    fn build_verify_tag() {
        let message = rand::random::<[u8; 16]>();
        let msg_str = encode(&message[..]);
        assert_eq!(message, decode(msg_str.clone()).unwrap()[..],
            "Encode or decode failed");
        let tag_key = rand::random::<[u8; 16]>();
        let packet = MsgPacket::new(&tag_key, &msg_str);
        assert!(receive_msg(packet))
    }

    #[test]
    fn snd_rcv_msg() {
        let bk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let packet = new_msg(&bk, &encode(&message[..]));
        // false result test
        let packet_false = MsgPacket {
            payload: packet.payload.clone(),
            key: packet.key,
            tag: tag_gen(&bk, &message),
        };
        assert!(receive_msg(packet));
        assert_ne!(receive_msg(packet_false), true);
    }

    #[test]
    fn report_msg() {
        let snd_id: u32 = 2806396777;
        let rcv_id: u32 = 259328394;

        let message = rand::random::<[u8; 16]>();
        let msg_str = encode(&message[..]);
        let tag_key = rand::random::<[u8; 16]>();
        let packet = MsgPacket::new(&tag_key, &msg_str);
        let sess = Session::new(0.to_string(), snd_id, rcv_id);
        assert!(proc_msg(sess, packet), "Proc failed");
        let (report, sess_sub) = sub_report(&tag_key, &encode(&message[..]), snd_id, rcv_id);
        assert!(vrf_report(sess_sub, report), "Verify failed");
    }

    #[bench]
    fn bench_new_message(b: &mut Bencher) {
        let bk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();

        b.iter(|| new_msg(&bk, &encode(&message[..])));
    }

    #[bench]
    fn bench_forward_message(b: &mut Bencher) {
        let bk = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let msg_str = encode(&message[..]);
        
        b.iter(||fwd_msg(&key, &vec![bk], &msg_str, FwdType::Receive));
    }

    #[bench]
    fn bench_process_message(b: &mut Bencher) {
        let snd_id: u32 = 2806396777;
        let rcv_id: u32 = 259328394;

        let message = rand::random::<[u8; 16]>();
        let msg_str = encode(&message[..]);
        let tag_key = rand::random::<[u8; 16]>();

        b.iter(|| proc_msg(Session::new(0.to_string(), snd_id, rcv_id), MsgPacket::new(&tag_key, &msg_str)));
    }

    #[bench]
    fn bench_receive_message(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();
        let msg_str = encode(&message[..]);
        let tag_key = rand::random::<[u8; 16]>();
        let packet = MsgPacket::new(&tag_key, &msg_str);

        b.iter(||receive_msg(MsgPacket { key: packet.key, tag: packet.tag, payload: msg_str.clone() }));
    }

}