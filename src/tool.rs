#![allow(dead_code,unused_imports)]
pub mod utils{
    use aes::Aes128;
    use aes_gcm::{
        aead::{Aead, AeadCore, OsRng},
        Aes256Gcm, Nonce, Key // Or `Aes128Gcm`
    };
    use aes::cipher::{
        BlockEncrypt, BlockDecrypt, KeyInit,
        generic_array::GenericArray,
    };
    use base64::{decode, encode};
    use sha3::{Digest, digest::{Update, ExtendableOutput, XofReader}, Sha3_256, Shake128};
    use tiny_keccak::{Kmac, Hasher};

    // input abitray string, output 128bit hash
    pub fn hash(x: &String) -> [u8; 16] {
        let mut y: [u8; 16] = Default::default();
        y.copy_from_slice(&Sha3_256::digest(x).as_slice()[0..16]);
        y
    }

    pub fn hash_array_32(x: &[u8]) -> [u8; 32] {
        let mut y: [u8; 32] = Default::default();
        y.copy_from_slice(&Sha3_256::digest(x).as_slice()[0..32]);
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

    pub fn encryption(k: &[u8; 16], plaintext: &[u8; 32]) -> [u8; 48] {
        let mut ct: [u8; 48] = [0;48];
        // hash k to 32 bytes
        let mut hash_k: [u8; 32] = Default::default();
        hash_k.copy_from_slice(&Sha3_256::digest(k).as_slice()[0..32]);

        let key = Key::<Aes256Gcm>::from_slice(&hash_k);
        let nonce = Nonce::from_slice(b"unique nonce"); // 96-bits; unique per message
        let cipher = Aes256Gcm::new(key);
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).unwrap();
        ct.copy_from_slice(&ciphertext);
        ct
    }

    pub fn decryption(k: &[u8; 16], ciphertext: &[u8; 48]) -> [u8; 32] {
        let mut p: [u8; 32] = Default::default();
        // hash k to 32 bytes
        let mut hash_k: [u8; 32] = Default::default();
        hash_k.copy_from_slice(&Sha3_256::digest(k).as_slice()[0..32]);
        
        let key = Key::<Aes256Gcm>::from_slice(&hash_k);
        let nonce = Nonce::from_slice(b"unique nonce"); // 96-bits; unique per message
        let cipher = Aes256Gcm::new(key);
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).unwrap();
        p.copy_from_slice(&plaintext);
        p
    } 
}

pub mod algos{
    extern crate base64;
    extern crate lazy_static;

    use base64::encode;
    use crate::tool::utils::{hash, crprf, encipher, decipher};
    use crate::db::db_tag;

    use super::utils::hash_array_32;

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

    pub fn hk_gen(tk: &[u8; 16]) -> [u8; 16] {
        // hash the tk
        hash(&encode(tk))
    }

    pub fn tag_proc(t: &[u8; 32], hk: &[u8; 16]) -> [u8; 32] {
        // concat tag and hk
        let mut proc_tag: [u8; 48] = [0; 48];
        let (one, two) = proc_tag.split_at_mut(hk.len());
        one.copy_from_slice(hk);
        two.copy_from_slice(t);
        hash_array_32(&proc_tag)
    }

    pub fn proc_tag_gen(tag_key: &[u8; 16], tk: &[u8; 16], message: &String) -> Vec<u8> {
        let t = prf_gen(&tag_key, message);
        let hk = hk_gen(&tk);
        tag_proc(&t, &hk).to_vec()
    }

    pub fn tag_exists(key: &[u8; 16], tk: &[u8; 16], message: &String) -> bool{
        let tag = proc_tag_gen( key, tk, message);
        // convert tag to string
        db_tag::exists(&encode(&tag[..]))
    }

    pub fn tag_mexists(tags: &Vec<[u8; 6]>) -> Vec<bool> {
        let mut tag_str: Vec<String> = Vec::new();
        for bytes in tags {
            let bytes_to_str = encode(&bytes[..]);
            tag_str.push(bytes_to_str);
        }
        db_tag::mexists(&mut tag_str)
    }

}

#[cfg(test)]
mod tests {
    extern crate test;
  
    use crate::tool::utils::{encipher, decipher, crprf};
    use crate::tool::algos::{self, proc_tag_gen, prf_gen};
    use base64::encode;
    use test::Bencher;
    use std::time::{SystemTime, UNIX_EPOCH, Instant};

    use super::utils::{self, encryption};

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
        algos::proc_tag_gen( &key, &tk, &encode(message));
    }

    #[test]
    fn test_encryption_decryption() {
        let message = rand::random::<[u8; 32]>();
        let key = rand::random::<[u8; 16]>();
        let ciphertext = encryption(&key, &message);
        let plaintext = utils::decryption(&key, &ciphertext);
        assert!(plaintext == message);
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
    fn bench_proc_tag_gen(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();
        let key = rand::random::<[u8; 16]>();
        let tk = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(algos::proc_tag_gen( &key, &tk, &encode(message))));
    }

    #[bench]
    fn bench_hash(b: &mut Bencher) {
        let message = rand::random::<[u8; 16]>();

        b.iter(|| test::black_box(utils::hash_shake(&encode(message))));
    }

}