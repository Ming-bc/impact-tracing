#![allow(dead_code)]

pub mod messaging {
    extern crate base64;

    use crate::tool::algos::*;
    // use crate::tool:: utils::*;
    use crate::db::{db_tag, db_nbr, db_ik};
    use crate::tool::utils::hash;
    use base64::{decode, encode};
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
        pub tag: [u8; 32],
        pub vrf_pi: Vec<u8>,
        pub vrf_pk: Vec<u8>,
        pub payload: String, // base64 encode string
    }

    impl MsgPacket {
        pub fn new(tag_key: &[u8; 16], message: &String, pi: &Vec<u8>, hash: &[u8;32]) -> Self {
            MsgPacket {
                tag_key: *tag_key, // 128 bits aes output
                tag: *hash, // 256 bits hash output
                vrf_pi: pi.to_vec(), // 641 bits (64 bytes) vrf proof
                epheral_key: rand::random::<[u8; 16]>(), // 128 bits aes key
                payload: message.clone(), 
                vrf_pk: Vec::new(), // 33 bytes public key
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
        let (hash,pi) = tag_gen(&tag_key, tk, message);
        // convert hash to [u8; 32]
        let mut hash_arr: [u8; 32] = Default::default();
        hash_arr.copy_from_slice(&hash[..]);
        MsgPacket::new(&tag_key, message, &pi, &hash_arr)
    }

    // proc_msg:
    pub fn plt_proc_packet(sess: &Edge, packet: &mut MsgPacket) {
        let map_id_key = db_ik::query(&vec![sess.sid]);
        let ik = map_id_key.get(&sess.sid).unwrap();
        let tk = tk_gen(ik, &sess.rid);
        let (_,pk) = vrf_pkgen(&tk);
        packet.vrf_pk = pk;
    }

    pub fn store_tag(packet: &mut MsgPacket) {
        let _ = db_tag::add(&vec![encode(packet.tag)]);
    }

    // vrf_msg:
    pub fn receive_packet(packet: &MsgPacket) -> bool {
        // 1. Decrypts E2EE
        // 2. Compute prf = F_k(m)
        let prf = prf_gen(&packet.tag_key, &packet.payload);
        // 3. Verify tag
        return vrf_verify(&packet.vrf_pk, &prf.to_vec(), &packet.vrf_pi, &packet.tag.to_vec());
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

    use std::time::{SystemTime, UNIX_EPOCH};

    use aes_gcm::{Aes128Gcm, KeyInit};
    use base64::{encode, decode};
    use test::Bencher;
    use vrf::VRF;
    use vrf::openssl::{ECVRF, CipherSuite};
    use crate::db::{db_tag, db_nbr, db_ik};
    use crate::message::messaging::*;
    use crate::tool::algos::*;
    use aes_gcm::{
        aead::{Aead, AeadCore, OsRng},
        Aes256Gcm, Nonce, Key // Or `Aes128Gcm`
    };

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    fn proc_msg(tk: &[u8;16], packet: &mut MsgPacket) {
        (_,packet.vrf_pk) = vrf_pkgen(tk);
    }

    #[test]
    fn snd_rcv_msg() {
        let tk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let prev_key = rand::random::<[u8; 16]>();
        
        let mut packet = send_packet(&encode(message), &prev_key, &tk);
        proc_msg(&tk, &mut packet);
        assert!(receive_packet(&packet));
    }

    #[bench]
    fn bench_send_message(b: &mut Bencher) {
        let tk = rand::random::<[u8; 16]>();
        let message = rand::random::<[u8; 16]>();
        let prev_key = rand::random::<[u8; 16]>();
        b.iter(|| send_packet(&encode(message), &prev_key, &tk));
    }

    #[bench]
    fn bench_receive_message(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();
        let mut packet = send_packet(&encode(message),&[0;16], &tk);
        let (_,pk) = vrf_pkgen(&tk);
        packet.vrf_pk = pk;

        b.iter(|| receive_packet(&packet));
    }

    #[test]
    fn report_msg() {
        let sid: u32 = rand::random::<u32>();
        let rid: u32 = rand::random::<u32>();
        let message = rand::random::<[u8; 16]>();

        let ik: [u8; 16] = rand::random::<[u8; 16]>();
        let tk = tk_gen(&ik, &rid);
        let tag_key = new_key_gen(&tk);
        let sess = Edge::new( &sid, &rid);
        let (tag,_) = tag_gen(&tag_key, &tk, &encode(message));

        let _ = db_ik::add(&vec![IdKey {id: sess.sid, key: ik}]);
        let _ = db_nbr::add(&vec![sess.clone()]);
        let _ = db_tag::add(&vec![encode(tag)]);

        let (report, sess_sub) = submit_report(&tag_key, &encode(message), &sess);
        assert!(verify_report(&sess_sub, &report), "Verify failed");
    }

// Test messaging runtime
// -------------------------------------------------------------------------------------------------

    fn test_send(message: &String, prev_key: &[u8; 16], tk: &[u8; 16], vrf_instance: &mut ECVRF) {
        let tag_key = next_key(prev_key, tk);
        // let tag_key = new_key_gen(tk);
        let prf = prf_gen(&tag_key, message);
        let sk = &tk[..].to_vec();
        let pi = vrf_instance.prove(&sk, &prf.to_vec()).unwrap();
        let hash = vrf_instance.proof_to_hash(&pi).unwrap();
        // convert hash to [u8; 32]
        let mut hash_arr: [u8; 32] = Default::default();
        hash_arr.copy_from_slice(&hash[..]);
        MsgPacket::new(&tag_key, message, &pi, &hash_arr);
    }

    #[bench]
    fn bench_send(b: &mut Bencher) {
        let message = encode(rand::random::<[u8; 16]>());
        let prev_key = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();
        let mut vrf = ECVRF::from_suite(CipherSuite::P256_SHA256_TAI).unwrap();
        b.iter(|| test_send(&message, &prev_key, &tk, &mut vrf));
    }

    #[bench]
    fn bench_process_message(b: &mut Bencher) {
        let tk = rand::random::<[u8; 16]>();
        let mut vrf = ECVRF::from_suite(CipherSuite::P256_SHA256_TAI).unwrap();
        let secret_key = &tk[..].to_vec();
        b.iter(|| vrf.derive_public_key(&secret_key).unwrap());
    }

    #[bench]
    fn bench_process_tag(b: &mut Bencher) {
        let key = Aes128Gcm::generate_key(OsRng);
        let cipher = Aes128Gcm::new(&key);
        let nounce = Aes128Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nounce, b"plaintext".as_ref()).unwrap();
        b.iter(|| cipher.decrypt(&nounce, ciphertext.as_ref()).unwrap());
    }

    #[bench]
    fn bench_receive_tag(b: &mut Bencher) {
        let mut vrf = ECVRF::from_suite(CipherSuite::P256_SHA256_TAI).unwrap();
        let message = rand::random::<[u8; 16]>().to_vec();
        let tk = rand::random::<[u8; 16]>();
        let secret_key = &tk[..].to_vec();
        let public_key = vrf.derive_public_key(&secret_key).unwrap();
        let proof = vrf.prove(&secret_key, &message).unwrap();

        b.iter(|| vrf.verify(&public_key, &proof, &message).unwrap());
        // b.iter(|| vrf.prove(&secret_key, &message));
    }
}