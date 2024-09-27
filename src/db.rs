#![allow(dead_code,unused_imports)]

pub mod db_tag {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    use redis::Connection;
    use dotenv::dotenv;
    use std::env;

    lazy_static::lazy_static! {
        pub static ref SET: redis::Client = create_redis_set_client();
    }

    pub fn create_redis_set_client() -> redis::Client {
        dotenv().ok();
        let db_tag_ip: String = env::var("DB_TAG_IP").expect("DB_TAG_IP is undefined.");
        redis::Client::open(db_tag_ip).unwrap()
    }

    pub fn get_set_conn() -> redis::RedisResult<Connection> {
        Ok(SET.get_connection()?)
    }

    pub fn add(tags: &Vec<String>) -> redis::RedisResult<()> {
        let mut conn = get_set_conn().unwrap();
        let set_name: String = env::var("DB_TAG_SET_NAME").expect("DB_TAG_SET_NAME is undefined.");
        redis::cmd("SADD").arg(set_name).arg(tags).query(&mut conn)
    }

    pub fn exists(tag: &String) -> bool {
        let mut conn = get_set_conn().unwrap();
        let set_name: String = env::var("DB_TAG_SET_NAME").expect("DB_TAG_SET_NAME is undefined.");
        redis::cmd("SISMEMBER").arg(set_name).arg(tag).query(&mut conn).unwrap()
    }

    pub fn mexists(tags: &Vec<String>) -> Vec<bool> {
        let mut conn = get_set_conn().unwrap();
        let set_name: String = env::var("DB_TAG_SET_NAME").expect("DB_TAG_SET_NAME is undefined.");
        if tags.len() == 0 {
            panic!("keys.len() == 0");
        }
        let result: Vec<bool> = redis::cmd("SMISMEMBER").arg(set_name).arg(tags).query(&mut conn).unwrap();
        result
    }

    pub fn mexists_pack(pack_keys: &Vec<Vec<String>>) -> Vec<Vec<bool>> {
        let mut conn = get_set_conn().unwrap();
        let set_name: String = env::var("DB_TAG_SET_NAME").expect("DB_TAG_SET_NAME is undefined.");
        let mut pipe = redis::pipe();

        for keys in pack_keys {
            if keys.len() == 0 {
                panic!("keys.len() == 0");
            }
            let command = redis::cmd("SMISMEMBER").arg(set_name.clone()).arg(keys.to_owned()).to_owned();
            pipe.add_command(command);
        }
        let result: Vec<Vec<bool>> = pipe.query(&mut conn).unwrap();
        result
    }
    pub fn clear() {
        let mut db_conn = get_set_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
        // let _: () = redis::cmd("BF.RESERVE").arg(set_name).arg(0.000001).arg(10000000).query(&mut db_conn).unwrap();
    }
}

pub mod db_ik {
    extern crate redis;
    extern crate base64;
    extern crate lazy_static;

    use std::collections::HashMap;
    use dotenv::dotenv;
    use std::env;
    use redis::Connection;
    use lazy_static::lazy_static;
    use crate::message::messaging::IdKey;

    lazy_static! {
        pub static ref DB_IK_CONN: redis::Client = create_redis_client();
    }

    fn create_redis_client() -> redis::Client {
        dotenv().ok();
        let db_ik_ip: String = env::var("DB_IK_IP").expect("DB_IK_IP is undefined.");
        redis::Client::open(db_ik_ip).unwrap()
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
            let ik = base64::encode(user.key);
            let command = redis::cmd("SET").arg(user.id).arg(ik).to_owned();
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
        let id_keys: Vec<String> = pipe.query(&mut conn).unwrap();
        let mut map_id_key = HashMap::<u32,[u8;16]>::new();
        for i in 0..vec_id.len() {
            let q_ik = id_keys.get(i).unwrap();
            let ik: [u8;16] = base64::decode(q_ik).unwrap().try_into().unwrap();
            map_id_key.insert(*vec_id.get(i).unwrap(), ik);
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
    use std::env;

    use redis::Connection;
    use dotenv::dotenv;
    use lazy_static::lazy_static;
    use crate::message::messaging::Edge;

    lazy_static! {
        pub static ref DB_NBR_CONN: redis::Client = create_redis_client();
    }

    fn create_redis_client() -> redis::Client {
        dotenv().ok();
        let db_nbr_ip: String = env::var("DB_NBR_IP").expect("DB_NBR_IP is undefined.");
        redis::Client::open(db_nbr_ip).unwrap()
    }

    pub fn get_redis_conn() -> redis::RedisResult<Connection> {
        let redis_client = DB_NBR_CONN.get_connection();
        Ok(redis_client?)
    }

    // write BRANCH users at one time
    pub fn add(edges: &Vec<Edge>) -> redis::RedisResult<()> {
        let mut conn = get_redis_conn().unwrap();
        let mut pipe = redis::Pipeline::new();

        // pipe sender to receivers
        for e in edges {
            let command_1 = redis::cmd("SADD").arg(e.sid).arg(e.rid).to_owned();
            let command_2 = redis::cmd("SADD").arg(e.rid).arg(e.sid).to_owned();
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
    use crate::db::{db_tag, db_nbr, db_ik};
    use crate::message::messaging::{Edge, IdKey};

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    #[test]
    fn redis_is_open() {
        assert!(db_nbr::get_redis_conn().is_ok());
        assert!(db_ik::get_redis_conn().is_ok());
        assert!(db_tag::get_set_conn().is_ok());
    }

    #[test]
    fn bf_add_exists() {
        let bytes = random::<[u8; 32]>();
        let bytes_2 = random::<[u8; 32]>();
        
        assert!(db_tag::add(&vec![encode(bytes)]).is_ok());
        assert!(db_tag::exists(&encode(bytes)));
        assert_eq!(db_tag::exists(&encode(bytes_2)), false);
    }

    #[test]
    fn bf_madd_mexists() {
        let mut values: Vec<String> = Vec::new();
        for _i in 0..5 {
            values.push(encode(rand::random::<[u8; 32]>()));
        }
        assert!(db_tag::add(&values).is_ok());
        db_tag::mexists(&mut values);
    }

    #[test]
    fn bf_collision_test() {
        let mut input: Vec<String> = Vec::new();
        for _i in 0..10000 {
            input.push(encode(rand::random::<[u8; 32]>()));
        }
        let _ = db_tag::add(&input);
        let mut query: Vec<String> = Vec::new();
        for _i in 0..1000 {
            let q = encode(rand::random::<[u8; 32]>());
            if !input.contains(&q) {
                query.push(q);
            }
        }
        // check true positive
        let result = db_tag::mexists(&mut input);
        for i in 0..result.len() {
            if !(*result.get(i).unwrap()) {
                println!("True nagative: {}", input.get(i).unwrap());
            }
        }
        // check false positive
        let result = db_tag::mexists(&mut query);
        for i in 0..result.len() {
            if *result.get(i).unwrap() {
                println!("False positive: {}", query.get(i).unwrap());
            }
        }
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
        let mut vec_sess = Vec::<Edge>::new();
        for _i in 0..1000 {
            vec_sess.push(Edge { sid: random::<u32>(), rid: random::<u32>() })
        }
        db_nbr::add(&mut vec_sess).ok().unwrap();
        let _ = db_nbr::query(&vec_sess.iter().map(|x| x.sid).collect());
        db_nbr::clear();
    }

    #[test]
    fn bench_db_tag_exist() {
        let bytes = random::<[u8; 6]>();
        let data = vec![encode(bytes)];
        assert!(db_tag::add(&data).is_ok());
        let start = std::time::Instant::now();
        db_tag::exists(&encode(bytes));
        let end = std::time::Instant::now();
        println!("db_tag exist runtime: {:?}", end - start);
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
        let bytes = random::<[u8; 6]>();
        let data = vec![encode(bytes)];
        let start = std::time::Instant::now();
        let _ = db_tag::add(&data).is_ok();
        let end = std::time::Instant::now();
        println!("Query runtime: {:?}", end - start);
    }

    #[bench]
    fn bench_bloom_filter_mexists(b: &mut Bencher) {
        let mut tags: Vec<String> = Vec::new();
        for _i in 0..1 {
            let bytes: [u8; 32] = random::<[u8; 32]>();
            let tag: String = encode(&bytes[..]).clone();
            assert!(db_tag::add(&vec![encode(bytes)]).is_ok());
            tags.push(tag);
        }

        b.iter(|| db_tag::mexists(&mut tags));
    }


}