
pub mod utils{
    use aes::Aes128;
    use aes::cipher::{
        BlockEncrypt, BlockDecrypt, KeyInit,
        generic_array::GenericArray,
    };
    use hmac::{Hmac, Mac};
    use sha3::{Digest, Sha3_256};
    

    // input abitray string, output 128bit hash
    pub fn hash(x: &[u8]) -> [u8; 16] {
        let mut y: [u8; 16] = Default::default();
        y.copy_from_slice(&Sha3_256::digest(x).as_slice()[0..16]);
        y
    }
    
    // fn prf(k: &[u8; 16], x: &[u8]) -> [u8; 16] {
    //     let mut y: [u8; 16] = Default::default();
    //     y.copy_from_slice(&Sha3_256::digest(&[k, x].concat()).as_slice()[0..16]);
    //     y
    // }
    
    pub fn crprf(k: &[u8; 16], x: &[u8]) -> [u8; 32] {
        let mut y: [u8; 32] = Default::default();
        let mut mac = Hmac::<Sha3_256>::new_varkey(k).unwrap();
        mac.input(x);
        y.copy_from_slice(&mac.result().code().as_slice());
        y
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
    use crate::tool::utils::{hash, crprf, encipher, decipher};

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
        let tag = crprf(tag_key, &hash_msg);
        tag
    }

    // proc_tag: process a tag
    pub fn proc_tag(bk: &[u8; 16], tag: &[u8; 32]) -> [u8; 32] {
        let processed_tag = crprf(bk, tag);
        processed_tag
    }

    // pub fn tag_exists(key: &[u8; 16], bk: &[u8; 16], message: &[u8]) -> bool{
    //     let tag = tag_gen(key, message);
    //     let tag_hat = proc_tag(bk, &tag)
    // }

}

#[cfg(test)]
mod tests {
    // extern crate test;
    // use rand::random;
    use crate::tool::utils::{hash, crprf, encipher, decipher};
    use crate::tool::algos::*;

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
        let new_key = next_key(&key, &bk);
        let old_key = prev_key(&new_key, &bk);
        assert_eq!(key, old_key);
    }

}