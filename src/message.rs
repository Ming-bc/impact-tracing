#![allow(dead_code)]

pub mod messaging {
    extern crate base64;

    use std::default;

    use crate::tool::algos::*;
    // use crate::tool:: utils::*;
    use crate::db::{db_tag, db_ik};
    use crate::tool::utils::{hash, encryption, decryption};
    use base64::encode;
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Edge {
        pub sid: u32,
        pub rid: u32,
    }

    impl Edge {
        pub fn new(sid: &u32, rid: &u32) -> Edge {
            Edge { sid: *sid, rid: *rid }
        }
        pub fn show(&self) {
            print!("U{} - U{}, ", self.sid, self.rid);
        }
    }

    #[derive(Debug)]
    pub struct IdKey {
        pub id: u32,
        pub key: [u8; 16],
    }

    impl IdKey {
        pub fn rand_key_gen(id: u32) -> IdKey {
            let key = rand::random::<[u8; 16]>();
            IdKey { id, key }
        }
        pub fn id_as_key_gen (id: u32) -> IdKey {
            let key = hash(&(id).to_string());
            IdKey { id, key }
        }
    }

    #[derive(Serialize,Deserialize,Debug)]
    pub struct MsgPacket {
        pub tag_key: [u8; 16],
        pub epheral_key: [u8; 16],
        pub prf: [u8; 32],
        pub payload: String, // base64 encode string
        pub hk: [u8; 16],
        pub p_tag: [u8; 32],
        pub ct_1: [u8; 32],
        pub ct_2: [u8; 16],
    }

    impl MsgPacket {
        pub fn new(tag_key: &[u8; 16], message: &String, prf: &[u8;32]) -> Self {
            MsgPacket {
                tag_key: *tag_key, // 128 bits aes output
                prf: *prf, // 256 bits hash output
                epheral_key: rand::random::<[u8; 16]>(), // 128 bits aes key
                payload: message.clone(), 
                hk: Default::default(),
                p_tag: Default::default(),
                ct_1: Default::default(),
                ct_2: Default::default(),
            }
        }
        pub fn new_with_ek(tag_key: &[u8; 16], message: &String, prf: &[u8;32], ek: &[u8;16], ct: &[u8;48], p_tag: &[u8;32]) -> Self {
            MsgPacket {
                tag_key: *tag_key, // 128 bits aes output
                prf: *prf, // 256 bits hash output
                epheral_key: *ek, // 128 bits aes key
                payload: message.clone(), 
                hk: Default::default(),
                p_tag: *p_tag,
                // gen ct_1 and ct_2
                ct_1: ct[..32].try_into().unwrap(),
                ct_2: ct[32..].try_into().unwrap(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct MsgReport {
        pub key: [u8; 16],
        pub payload: String,
    }

    pub fn send_packet(message: &String, prev_key: &[u8; 16], tk: &[u8; 16]) -> MsgPacket {
        // if tk is null then generate a new tag key
        let tag_key = if prev_key == &[0; 16] {
            new_key_gen(tk)
        } else {
            next_key(prev_key, tk)
        };
        let t: [u8; 32] = prf_gen(&tag_key, message);
        let ek: [u8; 16] = rand::random::<[u8; 16]>();
        let ct: [u8; 48] = encryption(&ek, &t);
        let hk: [u8; 16] = hk_gen(&tk);
        let p_tag = tag_proc(&t, &hk);
        MsgPacket::new_with_ek(&tag_key, message, &t, &ek, &ct, &p_tag)
    }

    // proc_msg:
    pub fn plt_proc_packet(sess: &Edge, packet: &mut MsgPacket) {
        let map_id_key = db_ik::query(&vec![sess.sid]);
        let ik = map_id_key.get(&sess.sid).unwrap();
        let tk = tk_gen(ik, &sess.rid);
        let hk = hk_gen(&tk);
        packet.hk = hk;
    }

    pub fn store_tag(packet: &mut MsgPacket) {
        let _ = db_tag::add(&vec![encode(packet.prf)]);
    }

    // vrf_msg:
    pub fn receive_packet(packet: &MsgPacket) -> bool {
        // 1. Decrypts E2EE
        // 2. Compute prf = F_k(m)
        let prf = prf_gen(&packet.tag_key, &packet.payload);
        // copy ct_1 and ct_2 to ct
        let mut ct: [u8; 48] = [0;48];
        let (one, two) = ct.split_at_mut(packet.ct_1.len());
        one.copy_from_slice(&packet.ct_1);
        two.copy_from_slice(&packet.ct_2);
        let tag = decryption(&packet.epheral_key, &ct);
        // 3. Verify tag
        prf == tag
    }

    // report_msg:
    pub fn submit_report(tag_key: &[u8;16], message: &String, sess: &Edge) -> (MsgReport, Edge) {
        (MsgReport { key: *tag_key, payload: message.clone()}, sess.clone())
    }

    pub fn verify_report(sess: &Edge, report: &MsgReport) -> bool {
        let map_id_key = db_ik::query(&vec![sess.sid]);
        let ik = map_id_key.get(&sess.sid).unwrap();
        let tk = tk_gen(ik, &sess.rid);
        tag_exists( &report.key, &tk, &report.payload)
    }
    
}


#[cfg(test)]
mod tests {
    extern crate base64;
    extern crate test;
    // use rand::random;

    use std::time::{Instant, Duration};

    use aes_gcm::{Aes128Gcm, KeyInit};
    use base64::{encode, decode};
    use test::Bencher;
    use crate::db::{db_tag, db_nbr, db_ik};
    use crate::message::messaging::*;
    use crate::tool::algos::*;
    use aes_gcm::{
        aead::{Aead, AeadCore, OsRng},
        Aes256Gcm, Nonce, Key // Or `Aes128Gcm`
    };
    use double_ratchet_2::ratchet::Ratchet;
    use serde;

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    #[test]
    fn snd_rcv_msg() {
        let tk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let prev_key = rand::random::<[u8; 16]>();
        
        let packet = send_packet(&encode(message), &prev_key, &tk);
        assert!(receive_packet(&packet));
    }

    #[test]
    fn test_send_packet() {
        let tk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let prev_key = rand::random::<[u8; 16]>();
        send_packet(&encode(message), &prev_key, &tk);
    }

    #[bench]
    fn bench_send_message_e2e(b: &mut Bencher) {
        let tk = rand::random::<[u8; 16]>();
        let mut message = encode(rand::random::<[u8; 16]>());
        for _i in 0..63 {
            message.push_str(&encode(rand::random::<[u8; 16]>()));
        }
        let prev_key = rand::random::<[u8; 16]>();
        let enc_pkt = send_packet(&message, &prev_key, &tk);

        let sk = [1; 32];
        let (mut bob_ratchet, public_key) = Ratchet::init_bob(sk);
        let mut alice_ratchet = Ratchet::init_alice(sk, public_key);
        let enc_string: String = serde_json::to_string(&enc_pkt).unwrap();
        b.iter(|| test::black_box(alice_ratchet.ratchet_encrypt(&enc_string.as_bytes().to_vec(), b"none")));

        // let (header, encrypted, nonce) = alice_ratchet.ratchet_encrypt(&enc_string.as_bytes().to_vec(), b"none");
        // let decrypted = bob_ratchet.ratchet_decrypt(&header, &encrypted, &nonce, b"none");
        // // deserialize decrypted to pkt
        // let dec_string = String::from_utf8(decrypted).unwrap();
        // let _: MsgPacket = serde_json::from_str(&dec_string).unwrap();
        // assert_eq!(enc_string, dec_string);
    }

    #[bench]
    fn bench_send_message(b: &mut Bencher) {
        let tk = rand::random::<[u8; 16]>();
        let mut message = encode(rand::random::<[u8; 16]>());
        for _i in 0..63 {
            message.push_str(&encode(rand::random::<[u8; 16]>()));
        }
        let prev_key = rand::random::<[u8; 16]>();
        b.iter(|| send_packet(&message, &prev_key, &tk));
    }

    #[bench]
    fn bench_receive_message(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();
        let packet = send_packet(&encode(message),&[0;16], &tk);

        b.iter(|| receive_packet(&packet));
    }

    #[test]
    fn report_msg() {
        let sid: u32 = rand::random::<u32>();
        let rid: u32 = rand::random::<u32>();
        let message = rand::random::<[u8; 16]>();

        let ik: [u8; 16] = rand::random::<[u8; 16]>();
        let tk: [u8; 16] = tk_gen(&ik, &rid);
        let tag_key = new_key_gen(&tk);
        let sess = Edge::new( &sid, &rid);
        let tag = proc_tag_gen(&tag_key, &tk, &encode(message));

        let _ = db_ik::add(&vec![IdKey {id: sess.sid, key: ik}]);
        let _ = db_nbr::add(&vec![sess.clone()]);
        let _ = db_tag::add(&vec![encode(tag)]);

        let (report, sess_sub) = submit_report(&tag_key, &encode(message), &sess);
        assert!(verify_report(&sess_sub, &report), "Verify failed");
    }

// Test messaging runtime
// -------------------------------------------------------------------------------------------------

    #[test]
    fn test_plt_proc() {
        let mut count: Duration = Default::default();
        let loop_count = 100;
        for _ in 0..loop_count {
            let tk = rand::random::<[u8; 16]>();
            let uid = rand::random::<u32>();
            let id_key = IdKey::rand_key_gen(uid);
            db_ik::add(&vec![id_key]).ok();
    
            let st = Instant::now();
            let _ = hk_gen(&tk);
            db_ik::query(&vec![uid]);
            let et = st.elapsed();
            count += et;
        }
        println!("Average time: {:?}", count/loop_count);
    }

    #[bench]
    fn bench_process_tag(b: &mut Bencher) {
        let key = Aes128Gcm::generate_key(OsRng);
        let cipher = Aes128Gcm::new(&key);
        let nounce = Aes128Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nounce, b"plaintext".as_ref()).unwrap();
        b.iter(|| cipher.decrypt(&nounce, ciphertext.as_ref()).unwrap());
    }
}