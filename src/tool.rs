#![allow(dead_code)]
pub mod utils{
    use aes::Aes128;
    use aes::cipher::{
        BlockEncrypt, BlockDecrypt, KeyInit,
        generic_array::GenericArray,
    };
    use base64::decode;
    use sha3::{Digest, digest::{Update, ExtendableOutput, XofReader}, Sha3_256, Shake128};
    use tiny_keccak::{Kmac, Hasher};

    // input abitray string, output 128bit hash
    pub fn hash(x: &String) -> [u8; 16] {
        let mut y: [u8; 16] = Default::default();
        y.copy_from_slice(&Sha3_256::digest(x).as_slice()[0..16]);
        y
    }

    pub fn hash_shake(x: &String) -> [u8; 16] {
        let mut y: [u8; 16] = Default::default();
        let mut shake = Shake128::default();
        shake.update(&decode(x).unwrap());
        shake.finalize_xof().read(&mut y);
        y
    }

    pub fn hash_shake_6(x: &String) -> [u8; 6] {
        let mut y: [u8; 6] = Default::default();
        let mut shake = Shake128::default();
        shake.update(&decode(x).unwrap());
        shake.finalize_xof().read(&mut y);
        y
    }

    // fn prf(k: &[u8; 16], x: &[u8]) -> [u8; 16] {
    //     let mut y: [u8; 16] = Default::default();
    //     y.copy_from_slice(&Sha3_256::digest(&[k, x].concat()).as_slice()[0..16]);
    //     y
    // }
    
    // pub fn crprf(k: &[u8; 16], x: &[u8]) -> [u8; 32] {
    //     let mut y: [u8; 32] = Default::default();
    //     let mut mac = Hmac::<Sha3_256>::
    //     mac.input(x);
    //     y.copy_from_slice(&mac.result().code().as_slice());
    //     y
    // }

    pub fn crprf(k: &[u8; 16], x: &[u8]) -> [u8; 32] {
        let mut z: [u8; 32] = Default::default();
        let mut kmac = Kmac::v256(k, b"");
        kmac.update(x);
        kmac.finalize(&mut z);
        z
    }

    pub fn encipher(k: &[u8; 16], plaintext: &[u8; 16]) -> [u8; 16] {
        let mut ciphertext: [u8; 16] = Default::default();
        let cipher = Aes128::new(GenericArray::from_slice(k));
        let mut block = GenericArray::clone_from_slice(plaintext);
        cipher.encrypt_block(&mut block);
        ciphertext.copy_from_slice(&block.as_slice());
        ciphertext
    }
    
    pub fn decipher(k: &[u8; 16], ciphertext: &[u8; 16]) -> [u8; 16] {
        let mut plaintext: [u8; 16] = Default::default();
        let cipher = Aes128::new(GenericArray::from_slice(k));
        let mut block = GenericArray::clone_from_slice(ciphertext);
        cipher.decrypt_block(&mut block);
        plaintext.copy_from_slice(&block.as_slice());
        plaintext
    }
    
}

pub mod algos{
    extern crate base64;
    extern crate lazy_static;

    use base64::encode;
    use crate::tool::utils::{hash, crprf, encipher, decipher};
    use crate::db::db_tag;
    use vrf::openssl::{CipherSuite, ECVRF};
    use vrf::VRF;

    pub fn tk_gen(sik: &[u8; 16], rid: &u32) -> [u8; 16] {
        // convert sik and rid to string 
        hash(&(encode(sik) + &rid.to_string()))
    }

    // new_key_gen: generate a ramdom key
    pub fn new_key_gen(tk: &[u8; 16]) -> [u8; 16] {
        let key = rand::random::<[u8; 16]>();
        let new_key = encipher(tk, &key);
        new_key
    }

    // prev_key: generate the prev node's key
    pub fn prev_key(key: &[u8; 16], tk: &[u8; 16]) -> [u8; 16] {
        let old_key = decipher(tk, key);
        old_key
    }

    // next_key: generate the next node's key
    pub fn next_key(key: &[u8; 16], tk: &[u8; 16]) -> [u8; 16] {
        let new_key = encipher(tk, key);
        new_key
    }

    // tag_gen: generate a message tag
    pub fn prf_gen(tag_key: &[u8; 16], message: &String) -> [u8; 32] {
        let hash_msg = hash(message);
        crprf(tag_key, &hash_msg)
    }

    pub fn tag_gen(tag_key: &[u8; 16], tk: &[u8; 16], message: &String) -> (Vec<u8>,Vec<u8>) {
        let prf = prf_gen(&tag_key, message);
        let sk = &tk[..].to_vec();
        vrf_prove(&sk,&prf)
    }

    pub fn tag_exists(key: &[u8; 16], tk: &[u8; 16], message: &String) -> bool{
        let (tag, _) = tag_gen( key, tk, message);
        // convert tag to string
        db_tag::exists(&encode(&tag[..]))
    }

    pub fn m_tag_exists(tags: &Vec<[u8; 6]>) -> Vec<bool> {
        let mut tag_str: Vec<String> = Vec::new();
        for bytes in tags {
            let bytes_to_str = encode(&bytes[..]);
            tag_str.push(bytes_to_str);
        }
        db_tag::mexists(&mut tag_str)
    }

    // initialization: 900us; pkgen: 20; prove: 480; verify: 
    pub fn vrf_pkgen(tk: &[u8; 16]) -> (Vec<u8>,Vec<u8>) {
        let mut vrf = ECVRF::from_suite(CipherSuite::P256_SHA256_TAI).unwrap();
        // convert tk to a hex string
        let secret_key = &tk[..].to_vec();
        let public_key = vrf.derive_public_key(&secret_key).unwrap();
        return (secret_key.to_vec(),public_key)
    }

    pub fn vrf_prove(sk: &Vec<u8>, msg: &[u8]) -> (Vec<u8>,Vec<u8>) {
        let mut vrf = ECVRF::from_suite(CipherSuite::P256_SHA256_TAI).unwrap();
        let proof = vrf.prove(&sk, &msg).unwrap();
        let hash = vrf.proof_to_hash(&proof).unwrap();
        return (hash, proof)
    }

    pub fn vrf_verify(pk: &Vec<u8>, alpha: &[u8], proof: &Vec<u8>, result: &Vec<u8>) -> bool {
        let mut vrf = ECVRF::from_suite(CipherSuite::P256_SHA256_TAI).unwrap();
        let beta: Vec<u8> = vrf.verify(&pk, &proof, &alpha).unwrap();
        // check whether beta == result
        return beta == *result
    }

}

#[cfg(test)]
mod tests {
    extern crate test;
  
    use crate::tool::utils::{encipher, decipher, crprf};
    use crate::tool::algos::{self, tag_gen, prf_gen};
    use base64::encode;
    use test::Bencher;
    use vrf::openssl::{CipherSuite, ECVRF};
    use vrf::VRF;
    use std::time::{SystemTime, UNIX_EPOCH, Instant};

    use super::utils;

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    // utils test
    #[test]
    fn enc_dec() {
        let message = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        let ciphertext = encipher(&key,&message);
        let plaintext = decipher(&key,&ciphertext);
        assert_eq!(plaintext, message);
    }

    #[test]
    fn next_prev_key() {
        let key = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();
        let new_key = algos::next_key(&key, &tk);
        let old_key = algos::prev_key(&new_key, &tk);
        assert_eq!(key, old_key);
    }

    #[test]
    fn test_tag_gen() {
        let message = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();
        algos::tag_gen( &key, &tk, &encode(message));
    }

    #[test]
    fn test_vrf() {
        let message = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();
        let (sk,pk) = algos::vrf_pkgen(&tk);
        let (hash, pi) = algos::vrf_prove(&sk, &message);
        assert!(algos::vrf_verify(&pk, &message, &pi, &hash));
    }

    #[bench]
    fn bench_vrf_pkgen(b: &mut Bencher) {
        let tk = rand::random::<[u8; 16]>();
        b.iter(|| test::black_box(algos::vrf_pkgen(&tk)));
    }

    #[bench]
    fn bench_vrf_prove(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();
        let (sk,_) = algos::vrf_pkgen(&tk);
        b.iter(|| test::black_box(algos::vrf_prove(&sk, &message)));
    }

    #[bench]
    fn bench_enc(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        
        b.iter(|| encipher(&key,&message));
    }

    #[bench]
    fn bench_next_key(b: &mut Bencher) {
        let key = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(algos::next_key(&key, &tk)));
    }

    #[bench]
    fn bench_tag_gen(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(algos::tag_gen( &key, &tk, &encode(message))));
    }

    #[bench]
    fn bench_hash(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(utils::hash_shake(&encode(message))));
    }

}