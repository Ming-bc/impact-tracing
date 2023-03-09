#![allow(dead_code)]

pub mod bloom_filter {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    // fpr: 0.000000005, items: 1000,000
    use redis::Connection;
    use base64::encode;
    use lazy_static::lazy_static;

    const BF_IP: &str = "redis://localhost:6379/";
    const BF_NAME: &str = "newFilter";

    lazy_static! {
        pub static ref BLOOM: redis::Client = create_bloom_filter_client();
    }

    pub fn create_bloom_filter_client() -> redis::Client {
        redis::Client::open(BF_IP).unwrap()
    }

    pub fn get_bf_conn() -> redis::RedisResult<Connection> {
        Ok(BLOOM.get_connection()?)
    }

    pub fn add(bytes: &[u8; 32]) -> redis::RedisResult<()> {
        let mut conn = get_bf_conn().unwrap();
        let msg = encode(&bytes[..]);
        let _ : () = redis::cmd("bf.add").arg(BF_NAME).arg(msg).query(&mut conn)?;
        Ok(())
    }

    pub fn madd(keys: &Vec<String>) -> redis::RedisResult<()> {
        let mut conn = get_bf_conn().unwrap();
        let _ : () = redis::cmd("bf.madd").arg(BF_NAME).arg(keys).query(&mut conn)?;
        Ok(())
    }
    
    pub fn exists(bytes: &[u8; 32]) -> bool {
        let mut conn = get_bf_conn().unwrap();
        let msg = encode(&bytes[..]);
        redis::cmd("bf.exists").arg(BF_NAME).arg(msg).query(&mut conn).unwrap()
    }

    pub fn mexists(keys: &mut Vec<String>) -> Vec<bool> {
        let mut conn = get_bf_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        let mut result: Vec<bool> = Vec::new();
        let mut threshold = 5000;
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

        let mut query_result: Vec<bool> = mexists(&mut query_keys);

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
}

pub mod redis_pack {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    use redis::{Connection, Commands};
    use base64::{encode,decode};
    use lazy_static::lazy_static;
    use crate::message::messaging::{Session, FwdType, Edge};

    const REDIS_IP: &str = "redis://localhost:6389/";

    lazy_static! {
        pub static ref REDIS_CONN: redis::Client = create_redis_client();
    }

    fn create_redis_client() -> redis::Client {
        redis::Client::open(REDIS_IP).unwrap()
    }

    pub fn get_redis_conn() -> redis::RedisResult<Connection> {
        let redis_client = REDIS_CONN.get_connection();
        Ok(redis_client?)
    }

    // write BRANCH users at one time
    pub fn add(sessions: &Vec<Session>) -> redis::RedisResult<()> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        // pipe sender to receivers
        for sess in sessions {
            let sender = sess.sender;
            let receiver = sess.receiver;
            let key = sess.id.clone();
            let command_1 = redis::cmd("HSET").arg(sender).arg(receiver).arg(key.clone()).to_owned();
            let command_2 = redis::cmd("HSET").arg(receiver).arg(sender).arg(key).to_owned();
            pipe.add_command(command_1);
            pipe.add_command(command_2);
        }
        let _ : () = pipe.query(&mut conn)?;
        Ok(())
    }

    fn from_vec_to_u8_array<T>(v: Vec<T>) -> [T; 16] {
        let boxed_slice = v.into_boxed_slice();
        let boxed_array: Box<[T; 16]> = match boxed_slice.try_into() {
            Ok(ba) => ba,
            Err(o) => panic!("Expected a Vec of length {} but it was {}", 16, o.len()),
        };
        *boxed_array
    }

    pub fn empty(){
        let mut db_conn = get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }

    // write BRANCH users at one time
    pub fn add_as_bytes(sessions: &Vec<Session>) -> redis::RedisResult<()> {
        let mut conn = get_redis_conn().unwrap();
        // pipe sender to receivers
        for sess in sessions {
            let sender = sess.sender;
            let receiver = sess.receiver;
            let key: [u8;16] = from_vec_to_u8_array(decode(sess.id.clone()).unwrap());
            redis::cmd("HSET").arg(sender).arg(receiver).arg(&key).query(&mut conn)?;
            redis::cmd("HSET").arg(receiver).arg(sender).arg(&key).query(&mut conn)?;
        }
        Ok(())
    }

    pub fn query_sid_as_bytes(sender: &u32, receiver: &u32) -> String {
        let mut conn = get_redis_conn().unwrap();
        let mut key: [u8; 16] = Default::default();
        let mut result: Vec<u8> = conn.hget(sender, receiver).unwrap();
        key.copy_from_slice(&result);
        encode(key)
    }

    pub fn pipe_add(sess:&mut Vec<Session>) -> redis::RedisResult<()> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();
        let mut threshold = 100000;

        while sess.len() > threshold {
            let length = sess.len();
            let store_sess = sess.split_off(length - threshold);
            let _ = add(&store_sess);
        }
        add(sess)
    }

    pub fn query_sid(sender: &u32, receiver: &u32) -> String {
        let mut conn = get_redis_conn().unwrap();
        conn.hget(*sender, *receiver).unwrap()
    }

    pub fn pipe_query_sid(edges: &Vec<Edge>) -> Vec<String> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();
        for sess in edges {
            let command = redis::cmd("HGET").arg(sess.sender).arg(sess.receiver).to_owned();
            pipe.add_command(command);
        }
        let result = pipe.query(&mut conn).unwrap();
        result
    }

    pub fn query_users(uid: &u32, fwd_type: FwdType) -> Vec<Session> {    
        let mut conn = get_redis_conn().unwrap();
        let users: Vec<String> = conn.hgetall(*uid).unwrap();
        // TODO: optimize
        let mut sessions: Vec<Session> = Vec::new();
        for i in 0..(users.len()/2) {
            let sid = users.get(2*i+1).unwrap();
            let sender: u32;
            let receiver: u32;
            
            match fwd_type {
                FwdType::Receive => {
                    sender = users.get(2*i).unwrap().parse().unwrap();
                    receiver = *uid;
                }
                FwdType::Send => {
                    sender = *uid;
                    receiver = users.get(2*i).unwrap().parse().unwrap();
                }
            }
            sessions.push(Session::new(&sid, &sender, &receiver));
        }
        sessions
    }

    pub fn query_users_receive(uid: &u32) -> Vec<Session> {    
        let mut conn = get_redis_conn().unwrap();
        let users: Vec<String> = conn.hgetall(*uid).unwrap();
        // TODO: optimize
        let mut sessions: Vec<Session> = Vec::new();
        for i in 0..(users.len()/2) {
            let sid = users.get(2*i+1).unwrap();
            let sender = users.get(2*i).unwrap().parse().unwrap();
            let receiver= *uid;
            sessions.push(Session::new(&sid, &sender, &receiver));
        }
        sessions
    }
// TODO: auto cut in 100000
    pub fn pipe_query_users(uids: &Vec<u32>) -> Vec<Vec<Session>> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        for uid in uids {
            let command = redis::cmd("HGETALL").arg(uid).to_owned();
            pipe.add_command(command);
        }
        let query_result: Vec<Vec<String>> = pipe.query(&mut conn).unwrap();

        let mut result: Vec<Vec<Session>> = Vec::new();
        for i in 0..query_result.len() {
            let uid_key = query_result.get(i).unwrap();
            let mut uid_result: Vec<Session> = Vec::new();
            for j in 0..(uid_key.len()/2) {
                let sid = uid_key.get(2*j+1).unwrap();
                let sender: u32 = *uids.get(i).unwrap();
                let receiver: u32 = uid_key.get(2*j).unwrap().parse().unwrap();
                uid_result.push(Session::new(&sid, &sender, &receiver));
            }
            result.push(uid_result);
        }
        result
    }

}

#[cfg(test)]
pub mod tests {
    extern crate base64;
    extern crate rand;
    extern crate redis;
    extern crate test;

    use base64::{encode, decode};
    // extern crate test;
    use rand::random;
    use test::Bencher;
    use redis::ConnectionLike;
    use crate::db::{bloom_filter, redis_pack};
    use crate::message::messaging::{Session, FwdType, Edge};
    use std::time::{SystemTime, Duration, UNIX_EPOCH};

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
        assert!(redis_pack::get_redis_conn().is_ok());
    }

    #[test]
    fn bf_add_exists() {
        let bytes = random::<[u8; 32]>();
        let bytes_2 = random::<[u8; 32]>();
        
        assert!(bloom_filter::add(&bytes).is_ok());
        assert!(bloom_filter::exists(&bytes));
        assert_eq!(bloom_filter::exists(&bytes_2), false);
    }

    #[test]
    fn bf_madd_mexists() {
        let mut values: Vec<String> = Vec::new();
        for i in 0..5 {
            let bytes = rand::random::<[u8; 32]>();
            let key = encode(&bytes[..]);
            values.push(key);
        }
        assert!(bloom_filter::madd(&values).is_ok());
        bloom_filter::mexists(&mut values);
    }


    #[test]
    fn redis_add_query() {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(&sid, &sender, &receiver);

        redis_pack::pipe_add(&mut vec![ses]).ok().unwrap();
        let mut users = redis_pack::query_users(&sender, FwdType::Send);
        for u in &mut users {
            assert_eq!(receiver, u.receiver);
        }
    }

    #[test]
    fn test_bf_pipe_mexists() {
        let mut pack_val: Vec<Vec<String>> = Vec::new();
        for j in 0..3 {
            let mut values: Vec<String> = Vec::new();
            for i in 0..3 {
                let bytes = rand::random::<[u8; 32]>();
                let key = encode(&bytes[..]);
                values.push(key);
            }
            assert!(bloom_filter::madd(&values).is_ok());
            pack_val.push(values);
        }

        pack_val.push(vec![12.to_string(), 15.to_string(), 13.to_string()]);
        
        let result = bloom_filter::mexists_pack(&pack_val);
println!("{:?}", result);
    }

    #[bench]
    fn bf_pipe_mexists(b: &mut Bencher) {
        let mut pack_val: Vec<Vec<String>> = Vec::new();
        for j in 0..3 {
            let mut values: Vec<String> = Vec::new();
            for i in 0..3 {
                let bytes = rand::random::<[u8; 32]>();
                let key = encode(&bytes[..]);
                values.push(key);
            }
            assert!(bloom_filter::madd(&values).is_ok());
            pack_val.push(values);
        }
        
        b.iter(|| bloom_filter::mexists_pack(&pack_val));
    }
    
    #[test]
    fn test_redis_pipe_query_sid() {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(&sid, &sender, &receiver);

        redis_pack::pipe_add(&mut vec![ses]).ok().unwrap();
        let result = redis_pack::pipe_query_sid(&vec!(Edge::new(sender, receiver)));
        for res in result {
            assert_eq!(sid, res);
        }
    }

    #[bench]
    fn bench_bloom_filter_exist(b: &mut Bencher) {
        let bytes = random::<[u8; 32]>();
        assert!(bloom_filter::add(&bytes).is_ok());
        b.iter(|| bloom_filter::exists(&bytes));
    }

    #[bench]
    fn bench_bloom_filter_mexists(b: &mut Bencher) {
        let mut tags: Vec<String> = Vec::new();
        for i in 0..1 {
            let bytes: [u8; 32] = random::<[u8; 32]>();
            let tag: String = encode(&bytes[..]).clone();
            assert!(bloom_filter::add(&bytes).is_ok());
            tags.push(tag);
        }

        b.iter(|| bloom_filter::mexists(&mut tags));
    }

    #[bench]
    fn bench_redis_query(b: &mut Bencher) {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(&sid, &sender, &receiver);

        redis_pack::pipe_add(&mut vec![ses]).ok().unwrap();

        b.iter(|| redis_pack::query_users(&sender, FwdType::Send));

        let mut db_conn = redis_pack::get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }

    #[bench]
    fn bench_redis_query_sid(b: &mut Bencher) {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(&sid, &sender, &receiver);

        redis_pack::pipe_add(&mut vec![ses]).ok().unwrap();
        b.iter(|| redis_pack::query_sid(&sender, &receiver));
    }

    #[test]
    fn bench_redis_query_sid_as_bytes() {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(&sid, &sender, &receiver);

        redis_pack::add(&mut vec![ses]).ok().unwrap();

        let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("Set {}, get {}", sid, redis_pack::query_sid_as_bytes(&sender, &receiver));
        let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        println!("Query runtime: {:?}", trace_end - trace_start);

        redis_pack::empty();
    }

}