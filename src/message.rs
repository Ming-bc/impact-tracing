pub mod messaging {
    use crate::tool::algos::*;
    // use crate::tool:: utils::*;
    use crate::db::bloom_filter::*;
    use crate::base64::decode;
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
        fn build(tag_key: &[u8; 16], message: String) -> MsgPacket {
            let msg_tag = tag_gen(&tag_key, &decode(message.clone()).unwrap()[..]);
            MsgPacket {
                key: *tag_key,
                tag: msg_tag,
                payload: message,
            }
        }

        fn vrf_tag(&self) -> bool {
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

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Session {
        pub id: String,
        pub sender: u32,
        pub receiver: u32,
    }

    impl Session {
        pub fn build (sid: String, snd_id: u32, rcv_id: u32) -> Session {
            Session { id: sid, sender: snd_id, receiver: rcv_id }
        }
    }

    // new_msg:
    pub fn new_msg(bk: &[u8; 16], message: String) -> MsgPacket {
        let tag_key = new_key_gen(bk);
        MsgPacket::build(&tag_key, message)
    }
    // fwd_msg:
    pub fn fwd_msg(key: &[u8; 16], bk: &[u8; 16], message: String, fwd_type: FwdType) -> MsgPacket {
        let tag_key:[u8; 16];

        match fwd_type {
            FwdType::Send => tag_key = prev_key(key, bk),
            FwdType::Receive => tag_key = next_key(key, bk),
        }
        MsgPacket::build(&tag_key, message)
    }

    pub fn send_packet(msg_packet: MsgPacket, session: Session) -> (MsgPacket, Session) {
        // TODO: serilize
        (msg_packet, session)
    }

    // proc_msg:
    pub fn proc_msg(bk: &[u8; 16], packet: MsgPacket) -> bool {
        let store_tag = proc_tag(bk, &packet.tag);
        let mut conn = connection_redis().ok().unwrap();
        bf_add(&mut conn, &store_tag).is_ok()
    }

    // pub fn rcv_packet(msg_packet: MsgPacket, session: Session) -> bool {
        
    // }

    // vrf_msg:
    pub fn vrf_msg(packet: MsgPacket) -> bool {
        // 1. Decrypts E2EE
        // 2. Compute ^tag
        packet.vrf_tag()
    }

    // report_msg:
    pub fn sub_report(tag_key: &[u8;16], message: String) -> MsgReport {
        MsgReport { key: *tag_key, payload: message.clone() }
    }

    pub fn vrf_report(bk: &[u8; 16], report: MsgReport) -> bool {
        // TODO: remove bk as input, replace to DB.query
        let tag_prime = tag_gen(&report.key, &decode(report.payload.clone()).unwrap()[..]);
        let tag_hat = tag_gen(bk, &tag_prime);
        let mut conn = connection_redis().ok().unwrap();
        bf_exists(&mut conn, &tag_hat).is_ok()
    }

    
}


#[cfg(test)]
mod tests {
    // extern crate test;
    // use rand::random;
    use crate::message::messaging::*;
    use crate::tool::algos::*;
    use crate::base64::encode;

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    #[test]
    fn snd_rcv_msg() {
        let bk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let packet = new_msg(&bk, encode(&message[..]));
        // false result test
        let packet_false = MsgPacket {
            payload: packet.payload.clone(),
            key: packet.key,
            tag: tag_gen(&bk, &message),
        };
        let b = vrf_msg(packet_false);
        assert!(vrf_msg(packet));
        assert_ne!(b, true);
    }

    #[test]
    fn new_proc_report_msg() {
        let bk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let msg_str = encode(&message[..]);
        let tag_key = rand::random::<[u8; 16]>();
        let msg_tag = tag_gen(&tag_key, &message);
        let packet = MsgPacket {
            payload: msg_str.clone(),
            key: tag_key,
            tag: msg_tag,
        };
        assert!(proc_msg(&bk, packet));
        let report = sub_report(&tag_key, msg_str);
        assert!(vrf_report(&bk, report));
    }


}