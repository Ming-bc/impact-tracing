#![allow(dead_code)]

pub mod bloom_filter {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    // fpr: 0.000000005, items: 1000,000
    use redis::Connection;
    const BF_IP: &str = "redis://localhost:6379/";
    const BF_NAME: &str = "newFilter";

    lazy_static::lazy_static! {
        pub static ref BLOOM: redis::Client = create_bloom_filter_client();
    }

    pub fn create_bloom_filter_client() -> redis::Client {
        redis::Client::open(BF_IP).unwrap()
    }

    pub fn get_bf_conn() -> redis::RedisResult<Connection> {
        Ok(BLOOM.get_connection()?)
    }

    pub fn add(tags: &Vec<String>) -> redis::RedisResult<()> {
        let mut conn = get_bf_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        for t in tags {
            let command = redis::cmd("bf.add").arg(BF_NAME).arg(t).to_owned();
            pipe.add_command(command);
        }
        let _ : () = pipe.query(&mut conn)?;
        Ok(())
    }
    
    pub fn exists(tag: &String) -> bool {
        let mut conn = get_bf_conn().unwrap();

        redis::cmd("bf.exists").arg(BF_NAME).arg(tag).query(&mut conn).unwrap()
    }

    pub fn mexists(keys: &mut Vec<String>) -> Vec<bool> {
        let mut conn = get_bf_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        let mut result: Vec<bool> = Vec::new();
        let threshold = 5000;
        while keys.len() > threshold {
            let store_keys = keys.split_off( keys.len() - threshold);
            let command = redis::cmd("bf.mexists").arg(BF_NAME).arg(store_keys).to_owned();
            pipe.add_command(command);
        }
        if keys.len() > 0 {
            let command = redis::cmd("bf.mexists").arg(BF_NAME).arg(keys.to_owned()).to_owned();
            pipe.add_command(command);
        }

        let query_result_vec: Vec<Vec<bool>> = pipe.query(&mut conn).unwrap();

        for i in 0..query_result_vec.len() {
            result.extend(query_result_vec.get(query_result_vec.len() - i - 1).unwrap());
        }
        result
    }

    pub fn mexists_pack(pack_keys: &Vec<Vec<String>>) -> Vec<Vec<bool>> {
        let mut pack_result: Vec<Vec<bool>> = Vec::new();
        let mut pack_keys_length: Vec<usize> = Vec::new();
        let mut query_keys: Vec<String> = Vec::new();

        for keys in pack_keys {
            pack_keys_length.push(keys.len());
            for k in keys {
                query_keys.push(k.to_string());
            }
        }

        let query_result: Vec<bool> = mexists(&mut query_keys);

        let mut count_bool: usize = 0;
        for i in 0..pack_keys_length.len(){
            let mut result: Vec<bool> = Vec::new();
            for j in 0..*pack_keys_length.get(i).unwrap() {
                result.push(*query_result.get(count_bool + j).unwrap());
            }
            count_bool += *pack_keys_length.get(i).unwrap();
            pack_result.push(result);
        }

        pack_result
    }

    pub fn clear() {
        let mut db_conn = get_bf_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }
}

pub mod db_ik {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    use std::collections::HashMap;

    use redis::Connection;
    use lazy_static::lazy_static;
    use crate::message::messaging::IdKey;

    const DB_IK_IP: &str = "redis://localhost:6400/";

    lazy_static! {
        pub static ref DB_IK_CONN: redis::Client = create_redis_client();
    }

    fn create_redis_client() -> redis::Client {
        redis::Client::open(DB_IK_IP).unwrap()
    }

    pub fn get_redis_conn() -> redis::RedisResult<Connection> {
        let redis_client = DB_IK_CONN.get_connection();
        Ok(redis_client?)
    }

    // write BRANCH users at one time
    pub fn add(vec_id_key: &Vec<IdKey>) -> redis::RedisResult<()> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        for user in vec_id_key {
            let command = redis::cmd("SET").arg(user.id).arg(&user.key).to_owned();
            pipe.add_command(command);
        }
        let _ : () = pipe.query(&mut conn)?;
        Ok(())
    }

    pub fn query(vec_id: &Vec<u32>) -> HashMap<u32,[u8;16]> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        for id in vec_id {
            let command = redis::cmd("GET").arg(id).to_owned();
            pipe.add_command(command);
        }
        let id_keys: Vec<Vec<u8>> = pipe.query(&mut conn).unwrap();

        let mut map_id_key = HashMap::<u32,[u8;16]>::new();
        for i in 0..vec_id.len() {
            let mut id_key: [u8;16] = Default::default();
            // convert vec<u8> to [u8;16]
            id_key.copy_from_slice(id_keys.get(i).unwrap());
            map_id_key.insert(*vec_id.get(i).unwrap(), id_key);
        }
        map_id_key
    }

    pub fn clear() {
        let mut db_conn = get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }
}

pub mod db_nbr {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    use std::collections::HashMap;

    use redis::Connection;
    use lazy_static::lazy_static;
    use crate::message::messaging::Session;

    const DB_NBR_IP: &str = "redis://localhost:6401/";

    lazy_static! {
        pub static ref DB_NBR_CONN: redis::Client = create_redis_client();
    }

    fn create_redis_client() -> redis::Client {
        redis::Client::open(DB_NBR_IP).unwrap()
    }

    pub fn get_redis_conn() -> redis::RedisResult<Connection> {
        let redis_client = DB_NBR_CONN.get_connection();
        Ok(redis_client?)
    }

    // write BRANCH users at one time
    pub fn add(sessions: &Vec<Session>) -> redis::RedisResult<()> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        // pipe sender to receivers
        for sess in sessions {
            let sender = sess.sid;
            let receiver = sess.rid;
            let command_1 = redis::cmd("SADD").arg(sender).arg(receiver).to_owned();
            let command_2 = redis::cmd("SADD").arg(receiver).arg(sender).to_owned();
            pipe.add_command(command_1);
            pipe.add_command(command_2);
        }
        let _ : () = pipe.query(&mut conn)?;
        Ok(())
    }

    pub fn query(vec_uid: &Vec<u32>) -> HashMap<u32,Vec<u32>> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        vec_uid.into_iter().for_each(|uid| {
            let command = redis::cmd("SMEMBERS").arg(uid).to_owned();
            pipe.add_command(command);
        });
        let result: Vec<Vec<u32>> = pipe.query(&mut conn).unwrap();
        // combine vec_uid and result to HashMap HashMap<u32,Vec<u32>>
        let mut map_uid_nbr: HashMap<u32,Vec<u32>> = HashMap::new();
        for i in 0..vec_uid.len() {
            map_uid_nbr.insert(*vec_uid.get(i).unwrap(), result.get(i).unwrap().to_vec());
        }
        map_uid_nbr
    }

    pub fn clear() {
        let mut db_conn = get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }

}

#[cfg(test)]
pub mod tests {
    extern crate base64;
    extern crate rand;
    extern crate redis;
    extern crate test;

    use base64::encode;
    // extern crate test;
    use rand::random;
    use test::Bencher;
    use redis::ConnectionLike;
    use crate::db::{bloom_filter, db_nbr, db_ik};
    use crate::message::messaging::{Session, IdKey};

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    // utils test
    #[test]
    fn bf_is_open() {
        let con = bloom_filter::get_bf_conn().ok().unwrap();
        // 测试是否成功连接Reids
        assert!(con.is_open());
    }

    #[test]
    fn redis_is_open() {
        assert!(db_nbr::get_redis_conn().is_ok());
    }

    #[test]
    fn bf_add_exists() {
        let bytes = random::<[u8; 32]>();
        let bytes_2 = random::<[u8; 32]>();
        
        assert!(bloom_filter::add(&vec![encode(bytes)]).is_ok());
        assert!(bloom_filter::exists(&encode(bytes)));
        assert_eq!(bloom_filter::exists(&encode(bytes_2)), false);
    }

    #[test]
    fn bf_madd_mexists() {
        let mut values: Vec<String> = Vec::new();
        for _i in 0..5 {
            values.push(encode(rand::random::<[u8; 32]>()));
        }
        assert!(bloom_filter::add(&values).is_ok());
        bloom_filter::mexists(&mut values);
    }

    #[test]
    fn db_ik_add_query() {
        let mut vec_id_key = Vec::new();
        for _i in 0..1000 {
            let id_key = IdKey::rand_key_gen(random::<u32>());
            vec_id_key.push(id_key);
        }
        db_ik::add(&vec_id_key).ok();
        let map_id_key = db_ik::query(&vec_id_key.iter().map(|x| x.id).collect());
        for i in 0..vec_id_key.len() {
            let uid = vec_id_key.get(i).unwrap().id;
            let ukey = vec_id_key.get(i).unwrap().key;
            assert_eq!(*map_id_key.get(&uid).unwrap(), ukey);
        }
        db_ik::clear();
    }

    #[test]
    fn db_nbr_add_query() {
        let mut vec_sess = Vec::<Session>::new();
        for _i in 0..1000 {
            vec_sess.push(Session { sid: random::<u32>(), rid: random::<u32>() })
        }
        db_nbr::add(&mut vec_sess).ok().unwrap();
        let _ = db_nbr::query(&vec_sess.iter().map(|x| x.sid).collect());
        db_nbr::clear();
    }

    #[bench]
    fn bench_bloom_filter_exist(b: &mut Bencher) {
        let bytes = random::<[u8; 32]>();
        let data = vec![encode(bytes)];
        assert!(bloom_filter::add(&data).is_ok());
        b.iter(|| bloom_filter::exists(&encode(bytes)));
    }

    #[test]
    fn bench_db_ik_query() {
        let id = random::<u32>();
        let id_key = IdKey::rand_key_gen(id);
        db_ik::add(&vec![id_key]).ok();
    let start = std::time::Instant::now();
        // b.iter(|| db_ik::query(&vec![id]));
        db_ik::query(&vec![id]);
    let end = std::time::Instant::now();
    println!("Query runtime: {:?}", end - start);
    }

    #[test]
    fn bench_bloom_filter_add() {
        let bytes = random::<[u8; 32]>();
        let data = vec![encode(bytes)];
        let start = std::time::Instant::now();
        let _ = bloom_filter::add(&data).is_ok();
        let end = std::time::Instant::now();
        println!("Query runtime: {:?}", end - start);
    }

    #[bench]
    fn bench_bloom_filter_mexists(b: &mut Bencher) {
        let mut tags: Vec<String> = Vec::new();
        for _i in 0..1 {
            let bytes: [u8; 32] = random::<[u8; 32]>();
            let tag: String = encode(&bytes[..]).clone();
            assert!(bloom_filter::add(&vec![encode(bytes)]).is_ok());
            tags.push(tag);
        }

        b.iter(|| bloom_filter::mexists(&mut tags));
    }


}