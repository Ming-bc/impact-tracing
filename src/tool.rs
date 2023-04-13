#![allow(dead_code)]
pub mod utils{
    use aes::Aes128;
    use aes::cipher::{
        BlockEncrypt, BlockDecrypt, KeyInit,
        generic_array::GenericArray,
    };
    use hmac::{Hmac, Mac};
    use sha3::{Digest, digest::{Update, ExtendableOutput, XofReader}, Sha3_256, Shake128};
    use tiny_keccak::{Kmac, Hasher};
    

    // input abitray string, output 128bit hash
    pub fn hash(x: &[u8]) -> [u8; 16] {
        let mut y: [u8; 16] = Default::default();
        y.copy_from_slice(&Sha3_256::digest(x).as_slice()[0..16]);
        y
    }

    pub fn hash_shake(x: &[u8]) -> [u8; 16] {
        let mut y: [u8; 16] = Default::default();
        let mut shake = Shake128::default();
        shake.update(x);
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

    use base64::encode;
    use crate::tool::utils::{hash, crprf, encipher, decipher};
    use crate::db::bloom_filter;

    // new_key_gen: generate a ramdom key
    pub fn new_key_gen(bk: &[u8; 16]) -> [u8; 16] {
        let key = rand::random::<[u8; 16]>();
        let new_key = encipher(bk, &key);
        new_key
    }

    // prev_key: generate the prev node's key
    pub fn prev_key(key: &[u8; 16], bk: &[u8; 16]) -> [u8; 16] {
        let old_key = decipher(bk, key);
        old_key
    }

    // next_key: generate the next node's key
    pub fn next_key(key: &[u8; 16], bk: &[u8; 16]) -> [u8; 16] {
        let new_key = encipher(bk, key);
        new_key
    }

    // tag_gen: generate a message tag
    pub fn tag_gen(tag_key: &[u8; 16], message: &[u8]) -> [u8; 32] {
        let hash_msg = hash(message);
        crprf(tag_key, &hash_msg)
    }

    // proc_tag: process a tag
    pub fn proc_tag(bk: &[u8; 16], tag: &[u8; 32]) -> [u8; 6] {
        // combine bk and tag in a 48 bytes array
        let mut bk_tag: [u8; 48] = [0; 48];
        let (one, two) = bk_tag.split_at_mut(bk.len());
        one.copy_from_slice(bk);
        two.copy_from_slice(tag);
        
        // return the first 6 bytes of the hash of the bk_tag
        let mut key: [u8; 6] = Default::default();
        key.copy_from_slice(&hash(&bk_tag).as_slice()[0..6]);
        key
    }

    pub fn store_tag_gen(uid: &u32, key: &[u8; 16], bk: &[u8; 16], message: &[u8]) -> [u8; 6] {
        let tag = tag_gen(key, message);
        proc_tag(bk, &tag)
    }

    pub fn tag_exists(uid: &u32, key: &[u8; 16], bk: &[u8; 16], message: &[u8]) -> bool{
        let tag = store_tag_gen(uid, key, bk, message);
        let mut conn = bloom_filter::get_bf_conn().ok().unwrap();
        bloom_filter::exists(&tag)
    }

    pub fn m_tag_exists(tags: &Vec<[u8; 6]>) -> Vec<bool> {
        let mut tag_str: Vec<String> = Vec::new();
        for bytes in tags {
            let bytes_to_str = encode(&bytes[..]);
            tag_str.push(bytes_to_str);
        }
        bloom_filter::mexists(&mut tag_str)
    }

}

#[cfg(test)]
mod tests {
    extern crate test;
  
    use crate::tool::utils::{encipher, decipher, crprf};
    use crate::tool::algos;
    use test::Bencher;
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let bk = rand::random::<[u8; 16]>();
let first = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let new_key = algos::next_key(&key, &bk);
let second = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Runtime {:?}", second - first);
        let old_key = algos::prev_key(&new_key, &bk);
        assert_eq!(key, old_key);
    }

    #[test]
    fn test_tag_gen() {
        let uid = rand::random::<u32>();
        let message = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        let bk = rand::random::<[u8; 16]>();
let first = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        algos::store_tag_gen(&uid, &key, &bk, &message);
let second = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Runtime {:?}", second - first);
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
        let bk = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(algos::next_key(&key, &bk)));
    }

    #[bench]
    fn bench_tag_gen(b: &mut Bencher) {
        let uid = rand::random::<u32>();
        let message = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        let bk = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(algos::store_tag_gen(&uid, &key, &bk, &message)));
    }

    #[bench]
    fn bench_hash(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(utils::hash_shake(&message)));
    }

}