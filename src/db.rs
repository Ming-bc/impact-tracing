

pub mod bloom_filter {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    // fpr: 0.000000005, items: 1000,000
    use redis::Connection;
    use base64::encode;
    use lazy_static::lazy_static;

    const BF_IP: &str = "redis://localhost:6379/";
    const BF_NAME: &str = "filter_1";

    lazy_static! {
        pub static ref BLOOM: redis::Client = create_bloom_filter_client();
    }

    pub fn create_bloom_filter_client() -> redis::Client {
        redis::Client::open(BF_IP).unwrap()
    }

    pub fn connect() -> redis::RedisResult<Connection> {
        Ok(BLOOM.get_connection()?)
    }

    pub fn add(bytes: &[u8; 32]) -> redis::RedisResult<()> {
        let mut conn = BLOOM.get_connection().unwrap();
        let msg = encode(&bytes[..]);
        let _ : () = redis::cmd("bf.add").arg(BF_NAME).arg(msg).query(&mut conn)?;
        Ok(())
    }

    pub fn madd(keys: &Vec<String>) -> redis::RedisResult<()> {
        let mut conn = BLOOM.get_connection().unwrap();
        let _ : () = redis::cmd("bf.madd").arg(BF_NAME).arg(keys).query(&mut conn)?;
        Ok(())
    }
    
    pub fn exists(bytes: &[u8; 32]) -> bool {
        let mut conn = BLOOM.get_connection().unwrap();
        let msg = encode(&bytes[..]);
        redis::cmd("bf.exists").arg(BF_NAME).arg(msg).query(&mut conn).unwrap()
    }

    pub fn mexists(keys: &Vec<String>) -> Vec<bool> {
        let mut conn = BLOOM.get_connection().unwrap();
        redis::cmd("bf.mexists").arg(BF_NAME).arg(keys).query(&mut conn).unwrap()
    }
}

pub mod redis_pack {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    use redis::{Connection, Commands};
    use base64::{encode,decode};
    use lazy_static::lazy_static;
    use crate::message::messaging::{Session, FwdType};
    use std::time::Duration;
    use std::thread;

    const REDIS_IP: &str = "redis://localhost:6389/";

    lazy_static! {
        pub static ref REDIS: redis::Client = create_redis_client();
    }

    fn create_redis_client() -> redis::Client {
        redis::Client::open(REDIS_IP).unwrap()
    }

    pub fn connect() -> redis::RedisResult<Connection> {
        Ok(REDIS.get_connection()?)
    }

    pub fn add(sessions: &Vec<Session>) -> redis::RedisResult<()> {
        for sess in sessions {
            let mut conn = REDIS.get_connection().unwrap();
            let _ : () = conn.hset(sess.sender, sess.receiver, sess.id.clone())?;
            let _ : () = conn.hset(sess.receiver, sess.sender, sess.id.clone())?;
        }
        Ok(())
    }

    // write BRANCH users at one time
    pub fn pipe_add(sessions: &Vec<Session>) -> redis::RedisResult<()> {
        let client = redis::Client::open(REDIS_IP).unwrap();
        let mut conn = client.get_connection().unwrap();
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

    pub fn pipe_add_auto_cut(sess:&mut Vec<Session>) -> redis::RedisResult<()> {
        let mut conn = REDIS.get_connection().unwrap();
        let mut pipe = redis::Pipeline::new();
        let mut threshold = 100000;

        while sess.len() > threshold {
            let length = sess.len();
            let store_sess = sess.split_off(length - threshold);
            let _ = pipe_add(&store_sess);
        }
        pipe_add(sess)
    }

    pub fn query_sid(sender: &u32, receiver: &u32) -> String {
        let mut conn = REDIS.get_connection().unwrap();
        // conn.hget(*sender, *receiver).unwrap()
// println!("sender {}, receiver {}", sender, receiver);
        conn.hget(*receiver, *sender).unwrap()
    }

    // pub fn safe_query_sid(sender: &u32, receiver: &u32) -> String {
    //     let mut conn = REDIS.get_connection().unwrap();
    //     // conn.hget(*sender, *receiver).unwrap()
    //     let ret: String;
    //     let redis_result = conn.hget(*receiver, *sender);
        
    //     if redis_result.is_err() {
    //         let bytes = rand::random::<[u8; 16]>();
    //         let sid = encode(&bytes[..]);
    //         let ses = Session::new(sid.clone(), *sender, *receiver);

    //         add(&vec![ses]).ok().unwrap();
    //         ret = sid;
    //     }
    //     else {
    //         ret = redis_result.unwrap();
    //     }
    //     ret
    // }

    pub fn query_users(uid: &u32, fwd_type: FwdType) -> Vec<Session> {    
        let mut conn = REDIS.get_connection().unwrap();    
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
            sessions.push(Session::new(sid.clone(), sender, receiver));
        }
        sessions
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
    use serde::de::value;
    use test::Bencher;
    use redis::ConnectionLike;
    use crate::db::{bloom_filter, redis_pack};
    use crate::message::messaging::{Session, FwdType};

    use super::redis_pack::query_users;

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    // utils test
    #[test]
    fn bf_is_open() {
        let con = bloom_filter::connect().ok().unwrap();
        // 测试是否成功连接Reids
        assert!(con.is_open());
    }

    #[test]
    fn redis_is_open() {
        assert!(redis_pack::connect().is_ok());
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
        bloom_filter::mexists(&values);
    }


    #[test]
    fn redis_add_query() {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(sid, sender, receiver);

        redis_pack::add(&vec![ses]).ok().unwrap();
        let mut users = redis_pack::query_users(&sender, FwdType::Send);
        for u in &mut users {
            assert_eq!(receiver, u.receiver);
        }
    }

    #[test]
    fn test_mock_star() {
        let mut users: Vec<u32> = Vec::new();
        let length: i32 = 80000;
        for i in 0..length {
            let id = random::<u32>();
            users.push(id);
        }
        mock_rows_star(&users);
        let result = query_users(users.get(0).unwrap(), FwdType::Send);
        assert_eq!(result.len(), (length-2) as usize);
        // for sess in result {
        //     sess.show();
        // }
    }

    #[bench]
    fn bench_bloom_filter_query(b: &mut Bencher) {
        let bytes = random::<[u8; 32]>();
        assert!(bloom_filter::add(&bytes).is_ok());
        b.iter(|| bloom_filter::exists(&bytes));
    }

    #[bench]
    fn bench_bloom_filter_mexists(b: &mut Bencher) {
        let mut tags: Vec<String> = Vec::new();
        for i in 0..10 {
            let bytes: [u8; 32] = random::<[u8; 32]>();
            let tag: String = encode(&bytes[..]).clone();
            assert!(bloom_filter::add(&bytes).is_ok());
            tags.push(tag);
        }

        b.iter(|| bloom_filter::mexists(&tags));
    }

    #[bench]
    fn bench_redis_query(b: &mut Bencher) {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(sid, sender, receiver);

        redis_pack::add(&vec![ses]).ok().unwrap();

        b.iter(|| redis_pack::query_users(&sender, FwdType::Send));
    }

    #[bench]
    fn bench_redis_query_sid(b: &mut Bencher) {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(sid, sender, receiver);

        redis_pack::add(&vec![ses]).ok().unwrap();
        b.iter(|| redis_pack::query_sid(&sender, &receiver));
    }

    // #[test]
    // fn users_gen_1 () {
    //     let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301, 902146735, 4206663226, 2261102179];
    //     mock_rows_full_connect(&users);
    // }

    // Generate rows that connects all users in the vector
    pub fn mock_rows_full_connect(users: &Vec<u32>) {
        for i in 0..users.len() {
            for j in i+1..users.len() {
                let bytes = rand::random::<[u8; 16]>();
                let sid = encode(&bytes[..]);

                let ses = Session::new(sid, *users.get(i).unwrap(), *users.get(j).unwrap());
                redis_pack::add(&vec![ses]).ok().unwrap();
            }
        }
    }

    // Generate rows that connects users as a line
    pub fn mock_rows_line(users: &Vec<u32>) -> Vec<Session> {
        let mut sessions: Vec<Session> = Vec::new();
        for i in 0..(users.len()-1) {
            let bytes = rand::random::<[u8; 16]>();
            let sid = encode(&bytes[..]);

            let ses = Session::new(sid, *users.get(i).unwrap(), *users.get(i+1).unwrap());
            let vec_ses = vec![ses];
            redis_pack::add(&vec_ses).ok().unwrap();
            sessions.extend(vec_ses);
        }
        // redis_pack::pipe_add(sessions).ok().unwrap();
        sessions
    }

    pub fn mock_rows_star(users: &Vec<u32>) -> Vec<Session> {
        let mut sessions: Vec<Session> = Vec::new();
        let central = users.get(0).unwrap();
        for i in 1..users.len() {
            let bytes = rand::random::<[u8; 16]>();
            let sid = encode(&bytes[..]);

            let ses = Session::new(sid, *central, *users.get(i).unwrap());
            let vec_ses = vec![ses];
            
            redis_pack::add(&vec_ses).ok().unwrap();
            sessions.extend(vec_ses);
        }
        // redis_pack::pipe_add(sessions).ok().unwrap();
        sessions
    }

}