

pub mod bloom_filter {
    // fpr: 0.000000005, items: 1000000
    use crate::redis::Connection;
    use crate::base64::{encode};

    const BF_IP: &str = "redis://localhost:6379/";

    pub fn connection_redis() -> redis::RedisResult<Connection> {
        let client = redis::Client::open(BF_IP)?;
        let con = client.get_connection()?;
        Ok(con)
    }

    pub fn bf_add(conn: &mut redis::Connection, bytes: &[u8; 32]) -> redis::RedisResult<()> {
        let msg = encode(&bytes[..]);
        let _ : () = redis::cmd("bf.add").arg("newFilter").arg(msg).query(conn)?;
        Ok(())
    }
    
    pub fn bf_exists(conn: &mut redis::Connection, bytes: &[u8; 32]) -> redis::RedisResult<()> {
        let msg = encode(&bytes[..]);
        let _ : () = redis::cmd("bf.exists").arg("newFilter").arg(msg).query(conn)?;
        Ok(())
    }
}

pub mod pack_storage {
    use mongodb::{bson::{doc, Document}, sync::{Client, Collection}};
    use crate::message::messaging::{Session, FwdType};

    const MONGO_IP: &str = "mongodb://localhost:27017/";
    const DB_NAME: &str = "admin";
    const COLLECTION_NAME: &str = "PackStorage";

    pub fn connection_mongodb() -> mongodb::error::Result<()> {
        // Get a handle to the cluster
        let client = Client::with_uri_str(
            MONGO_IP,
        )?;
        // Ping the server to see if you can connect to the cluster
        client
            .database(DB_NAME)
            .run_command(doc! {"ping": 1}, None)?;
        // println!("Connected successfully.");
        // // List the names of the databases in that cluster
        // for db_name in client.list_database_names(None, None)? {
        //     println!("{}", db_name);
        // }
        Ok(())
    }

    pub fn mongo_add(ses: Session) -> mongodb::error::Result<()>  {
        let client = Client::with_uri_str(MONGO_IP)?;
        let db = client.database(DB_NAME);
        let collection = db.collection::<Session>(COLLECTION_NAME);
        let docs = vec![
            ses,
        ];
        collection.insert_many(docs, None)?;
        Ok(())
    }

    // pub fn mongo_query(uid: u32, user_type: FwdType) -> Vec<Session> {
    //     let client = Client::with_uri_str(MONGO_IP)?;
    //     let db = client.database(DB_NAME);
    //     let collection = db.collection::<Session>(COLLECTION_NAME);
        
    //     let role: &str;
    //     match user_type {
    //         FwdType::Send => role = "sender",
    //         FwdType::Receive => role = "receiver",
    //     }
    //     let filter = doc! { role: uid.to_string() };
    // }
}

#[cfg(test)]
mod tests {
    use base64::encode;
    // extern crate test;
    use rand::random;
    use crate::db::bloom_filter::*;
    use crate::db::pack_storage::*;
    use crate::redis::ConnectionLike;
    use crate::message::messaging::Session;

    // fn init_logger() {
    //     //env_logger::init();
    //     let _ = env_logger::builder().is_test(true).try_init();
    // }

    // utils test
    #[test]
    fn bf_is_open() {
        let con = connection_redis().ok().unwrap();
        // 测试是否成功连接Reids
        assert!(con.is_open());
    }

    #[test]
    fn mongo_is_open() {
        assert!(connection_mongodb().is_ok());
    }

    #[test]
    fn bf_add_exists() {
        let bytes = random::<[u8; 32]>();
        let mut conn = connection_redis().ok().unwrap();
        assert!(bf_add(&mut conn, &bytes).is_ok());
        assert!(bf_exists(&mut conn, &bytes).is_ok());
    }

    #[test]
    fn mongo_add_query() {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::build(sid, sender, receiver);

        mongo_add(ses).ok().unwrap();
    }

}