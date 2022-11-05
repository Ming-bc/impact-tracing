

pub mod bloom_filter {
    extern crate redis;
    extern crate base64;

    // fpr: 0.000000005, items: 1000000
    use redis::Connection;
    use base64::encode;

    const BF_IP: &str = "redis://localhost:6379/";

    pub fn connect() -> redis::RedisResult<Connection> {
        let client = redis::Client::open(BF_IP)?;
        let con = client.get_connection()?;
        Ok(con)
    }

    pub fn add(conn: &mut redis::Connection, bytes: &[u8; 32]) -> redis::RedisResult<()> {
        let msg = encode(&bytes[..]);
        let _ : () = redis::cmd("bf.add").arg("newFilter").arg(msg).query(conn)?;
        Ok(())
    }
    
    pub fn exists(conn: &mut redis::Connection, bytes: &[u8; 32]) -> bool {
        let msg = encode(&bytes[..]);
        redis::cmd("bf.exists").arg("newFilter").arg(msg).query(conn).unwrap()
    }
}

pub mod pack_storage {
    use mongodb::{bson::{doc, Document}, sync::{Client, Collection}, options::FindOptions};
    use crate::message::messaging::{Session, FwdType};
    // use crate::futures::stream::{StreamExt, TryStreamExt};

    const MONGO_IP: &str = "mongodb://localhost:27017/";
    const DB_NAME: &str = "admin";
    const COLLECTION_NAME: &str = "PackStorage";

    pub fn connect() -> mongodb::error::Result<()> {
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

    pub fn drop() -> mongodb::error::Result<()>  {
        let client = Client::with_uri_str(MONGO_IP).unwrap();
        let collection = client.database(DB_NAME).collection::<Session>(COLLECTION_NAME);
        collection.drop(None)
    }

    pub fn add(sess: Session) -> mongodb::error::Result<()>  {
        let client = Client::with_uri_str(MONGO_IP)?;
        let collection = client.database(DB_NAME).collection::<Session>(COLLECTION_NAME);
        let docs = vec![
            sess,
        ];
        collection.insert_many(docs, None)?;
        Ok(())
    }

    pub fn query_sid(sender: &u32, receiver: &u32) -> String {
        let client = Client::with_uri_str(MONGO_IP).unwrap();
        let collection = client.database(DB_NAME).collection::<Session>(COLLECTION_NAME);
        let filter = doc! { "sender": sender, "receiver": receiver };
        let cursor = collection.find(filter, None).unwrap();
        let mut sid: String = String::from("value");
        for doc in cursor {
             sid = doc.unwrap().id;
        }
        sid
    }

    pub fn query_users(uid: &u32, user_type: FwdType) -> Vec<Session> {
        let client = Client::with_uri_str(MONGO_IP).unwrap();
        let collection = client.database(DB_NAME).collection::<Session>(COLLECTION_NAME);
        
        let role: &str;
        match user_type {
            FwdType::Send => role = "sender",
            FwdType::Receive => role = "receiver",
        }
        let filter = doc! { role: uid };
        let cursor = collection.find(filter, None).unwrap();
        
        let mut users: Vec<Session> = Vec::new();
        for doc in cursor {
            // println!("{}", doc.unwrap().id);
            let user = Session::from(doc.unwrap());
            users.push(user);
        }
        users
    }
}

#[cfg(test)]
mod tests {
    extern crate base64;
    extern crate rand;
    extern crate redis;

    use base64::encode;
    // extern crate test;
    use rand::random;
    use redis::ConnectionLike;
    use crate::db::{bloom_filter, pack_storage};
    use crate::message::messaging::Session;
    use crate::message::messaging::FwdType;

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
    fn mongo_is_open() {
        assert!(pack_storage::connect().is_ok());
    }

    #[test]
    fn bf_add_exists() {
        let bytes = random::<[u8; 32]>();
        let bytes_2 = random::<[u8; 32]>();
        let mut conn = bloom_filter::connect().ok().unwrap();
        assert!(bloom_filter::add(&mut conn, &bytes).is_ok());
        assert!(bloom_filter::exists(&mut conn, &bytes));
        assert_eq!(bloom_filter::exists(&mut conn, &bytes_2), false);
    }

    #[test]
    fn mongo_add_query() {
        let sender = random::<u32>();
        let receiver = random::<u32>();
        let bytes = rand::random::<[u8; 16]>();
        let sid = encode(&bytes[..]);
        let ses = Session::new(sid, sender, receiver);

        pack_storage::add(ses).ok().unwrap();
        let mut users = pack_storage::query_users(&sender, FwdType::Send);
        for u in &mut users {
            assert_eq!(receiver, u.receiver);
        }
    }

    // Generate 5:5 sender:receiver pair
    fn mongo_mock_rows() {
        let users: Vec<u32> = vec![2806396777, 259328394, 4030527275, 1677240722, 1888975301, 902146735, 4206663226, 2261102179];

        for i in 0..8 {
            for j in i+1..8 {
                let bytes = rand::random::<[u8; 16]>();
                let sid = encode(&bytes[..]);

                let ses = Session::new(sid, *users.get(i).unwrap(), *users.get(j).unwrap());
                pack_storage::add(ses).ok().unwrap();
            }
        }
    }

}