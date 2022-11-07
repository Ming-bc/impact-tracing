

pub mod bloom_filter {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    // fpr: 0.000000005, items: 1000,000
    use redis::Connection;
    use base64::encode;
    use lazy_static::lazy_static;

    const BF_IP: &str = "redis://localhost:6379/";

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
        let _ : () = redis::cmd("bf.add").arg("newFilter").arg(msg).query(&mut conn)?;
        Ok(())
    }
    
    pub fn exists(bytes: &[u8; 32]) -> bool {
        let mut conn = BLOOM.get_connection().unwrap();
        let msg = encode(&bytes[..]);
        redis::cmd("bf.exists").arg("newFilter").arg(msg).query(&mut conn).unwrap()
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

    const REDIS_IP: &str = "redis://localhost:6389/";

    lazy_static! {
        pub static ref BLOOM: redis::Client = create_redis_client();
    }

    fn create_redis_client() -> redis::Client {
        redis::Client::open(REDIS_IP).unwrap()
    }

    pub fn connect() -> redis::RedisResult<Connection> {
        Ok(BLOOM.get_connection()?)
    }

    pub fn add(sess: Session) -> redis::RedisResult<()> {
        let mut conn = BLOOM.get_connection().unwrap();
        // TODO: replace to xor
        let _ : () = conn.hset(sess.sender, sess.receiver, sess.id.clone())?;
        let _ : () = conn.hset(sess.receiver, sess.sender, sess.id)?;
        Ok(())
    }

    pub fn query_sid(sender: &u32, receiver: &u32) -> String {
        let mut conn = BLOOM.get_connection().unwrap();
        conn.hget(*sender, *receiver).unwrap()
    }

    pub fn query_users(uid: &u32, fwd_type: FwdType) -> Vec<Session> {    
        let mut conn = BLOOM.get_connection().unwrap();    
        let users: Vec<String> = conn.hgetall(*uid).unwrap();
        
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
    use test::Bencher;
    use redis::ConnectionLike;
    use crate::db::{bloom_filter, redis_pack};
    use crate::message::messaging::{Session, FwdType};

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
    fn redis_add_query() {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(sid, sender, receiver);

        redis_pack::add(ses).ok().unwrap();
        let mut users = redis_pack::query_users(&sender, FwdType::Send);
        for u in &mut users {
            assert_eq!(receiver, u.receiver);
        }
    }

    #[bench]
    fn bench_bloom_filter_query(b: &mut Bencher) {
        let bytes = random::<[u8; 32]>();
        assert!(bloom_filter::add(&bytes).is_ok());
        b.iter(|| bloom_filter::exists(&bytes));
    }

    #[bench]
    fn bench_redis_query(b: &mut Bencher) {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(sid, sender, receiver);

        redis_pack::add(ses).ok().unwrap();
        b.iter(|| redis_pack::query_users(&sender, FwdType::Send));
        
    }

    #[test]
    fn users_gen_1 () {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301, 902146735, 4206663226, 2261102179];
        mock_rows_line(&users);
    }

    // Generate rows that connects all users in the vector
    pub fn mock_rows_full_connect(users: &Vec<u32>) {
        for i in 0..users.len() {
            for j in i+1..users.len() {
                let bytes = rand::random::<[u8; 16]>();
                let sid = encode(&bytes[..]);

                let ses = Session::new(sid, *users.get(i).unwrap(), *users.get(j).unwrap());
                redis_pack::add(ses).ok().unwrap();
            }
        }
    }

    // Generate rows that connects users as a line
    pub fn mock_rows_line(users: &Vec<u32>) {
        for i in 0..(users.len()-1) {
            let bytes = rand::random::<[u8; 16]>();
            let sid = encode(&bytes[..]);

            let ses = Session::new(sid, *users.get(i).unwrap(), *users.get(i+1).unwrap());
            redis_pack::add(ses).ok().unwrap();
        }
    }

}