pub mod eval {
    extern crate base64;
    extern crate test;

    use std::collections::{HashMap, HashSet};

    use base64::{encode, decode};
    use rand;
    
    use crate::message::messaging;
    use crate::db::{redis_pack, bloom_filter};
    use crate::tool::algos;
    use crate::trace::traceback;

    use crate::message::messaging::{FwdType, MsgPacket, Session, MsgReport, Edge};
    use crate::trace::traceback::{TraceData};
    use std::time::{SystemTime, UNIX_EPOCH};

    const OURS_BRANCH: u32 = 10;

    #[derive(Debug, Clone, Copy)]
    pub enum TreeNodeType {
        ActiveFwd,
        InactiveFwd,
        ActiveUser,
        InactiveUser,
    }

    pub struct TreeNode {
        pub fwd_size: usize,
        pub conn_size: usize,
        pub fwd_type: TreeNodeType,
    }

    impl TreeNode {
        pub fn new(fwd: usize, conn: usize, tp: TreeNodeType) -> Self {
            TreeNode {fwd_size: fwd, conn_size: conn, fwd_type: tp}
        }
        pub fn new_from_tp(tp: TreeNodeType) -> Self {
            let (fwd_size, conn_size) = TreeNode::random_size(tp);
            TreeNode::new(fwd_size, conn_size, tp)
        }

        pub fn random() -> Self {
            let tp: TreeNodeType;
            let rand_type: u16 = rand::random::<u16>() % 100;
            
            match rand_type {
                0..=50 => tp = TreeNodeType::ActiveFwd,
                51..=60 => tp = TreeNodeType::InactiveFwd,
                61..=100 => tp = TreeNodeType::ActiveUser,
                // 16..=100 => tp = TreeNodeType::InactiveUser,
                _ => tp = TreeNodeType::InactiveUser,
            }

            let (fwd_size, conn_size) = TreeNode::random_size(tp);
            TreeNode::new(fwd_size, conn_size, tp)
        }

        fn random_size(tp: TreeNodeType) -> (usize, usize) {
            let fwd_size: usize;
            let conn_size: usize;
            match tp {
                TreeNodeType::ActiveFwd => {
                    conn_size = Self::random_size_val(0, 20);
                    let size = Self::random_size_val(0, 5);
                    if size > conn_size {fwd_size = conn_size} else {fwd_size = size};
                },
                TreeNodeType::InactiveFwd => {
                    conn_size = Self::random_size_val(0, 20);
                    let size = Self::random_size_val(0, conn_size as u16);
                    if size > 2 {fwd_size = size % 2} else {fwd_size = size};
                },
                TreeNodeType::ActiveUser => {
                    conn_size = Self::random_size_val(0, 20);
                    let size = Self::random_size_val(0, conn_size as u16);
                    if size > 1 {fwd_size = 1} else {fwd_size = size};
                },
                TreeNodeType::InactiveUser => {
                    conn_size = Self::random_size_val(0, 10);
                    let size = Self::random_size_val(0, conn_size as u16);
                    // if size > 5 {fwd_size = 5} else {fwd_size = size};
                    fwd_size = size % 3;
                },
            }
            (fwd_size, conn_size)
        }

        fn random_size_val(min: u16, max: u16) -> usize {
            match max - min {
                0 => return min as usize,
                _ => return (min + (rand::random::<u16>() % (max - min))) as usize,
            }
        }

        pub fn show(&self) {
            println!("Fwd: {}, Conn: {}, Type: {:?}", self.fwd_size, self.conn_size, self.fwd_type);
        }
    }

    pub fn fwd_tree_gen (start_key: &[u8; 16], root_id: &u32, root_tp: &TreeNode, message: &str, depth: &u32, depth_limit: &u32) -> (Vec<TraceData>, Vec<Session>, Vec<String>) {
        let mut tree_md: Vec<TraceData> = Vec::new();
        let mut tree_sess: Vec<Session> = Vec::new();
        let mut tree_tag: Vec<String> = Vec::new();
        if depth < depth_limit {
            for i in 0..root_tp.fwd_size {
                let receiver: u32 = rand::random::<u32>();
                let rcv_tp: TreeNode = TreeNode::random();
                let sid = encode(&rand::random::<[u8; 16]>()[..]);
                tree_sess.push(Session::new(&sid, root_id, &receiver));

                let bk = &base64::decode(sid).unwrap()[..];
                let (packet, tag) = fwd_edge_gen(*start_key, message, root_id, &receiver, bk);

                tree_tag.push(tag);
                tree_md.push(TraceData::new(receiver, packet.key));
                tree_sess.extend(fake_receivers(root_id, &(root_tp.conn_size - root_tp.fwd_size)));

                let nxt_depth = *depth + 1;
                let (sub_tree_md, sub_tree_sess, sub_tree_tag) = fwd_tree_gen(&packet.key, &receiver, &rcv_tp, message, &nxt_depth, depth_limit);

                tree_md.extend(sub_tree_md);
                tree_sess.extend(sub_tree_sess);
                tree_tag.extend(sub_tree_tag);
            }
        }
        (tree_md, tree_sess, tree_tag)
    }

    fn write_tag_to_bf(sess:Session, packet: MsgPacket){
        let store_tag = store_tag_gen(sess, packet);

        let op = bloom_filter::madd(&vec![store_tag.clone()]);
        match op.is_ok() {
            true => {}
            false => {
                println!("False tag: {}", store_tag);
                panic!("process failed.");
            }
        }
    }

    fn store_tag_gen(sess:Session, packet: MsgPacket) -> String {
        let sid = sess.id;
        let bk = <&[u8; 16]>::try_from(&decode(sid).unwrap()[..]).unwrap().clone();
        encode(algos::proc_tag(&sess.sender, &bk, &packet.tag))
    }
    // generate a new edge from a sender to a receiver
    pub fn new_edge_gen(message: &str, sender: &u32, receiver: &u32) -> MsgPacket {
        let sid = base64::encode(&rand::random::<[u8; 16]>()[..]);
        let sess = Session::new(&sid, &sender, &receiver);
        let _ = redis_pack::pipe_add(&mut vec![sess]);

        let bk = &base64::decode(sid).unwrap()[..];
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let sess = Session::new( &encode(bk_16), sender, receiver);
        write_tag_to_bf( sess, MsgPacket::new(&bk_16, message) );
        messaging::new_msg(bk_16, message)
    }
    
    // generate a forward edge from a sender to a receiver
    fn fwd_edge_gen(prev_key: [u8; 16], message: &str, sender: &u32, receiver: &u32, bk: &[u8]) -> (MsgPacket, String) {
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let sess = Session::new( &encode(bk_16), sender, receiver);
        let packet = messaging::fwd_msg(&prev_key, &vec![*bk_16], message, FwdType::Receive);
        let tag = store_tag_gen(sess, MsgPacket::new(&packet.key, message));
        (packet, tag)
    }

    fn fake_receivers (sender: &u32, num: &usize) -> Vec<Session> {
        let mut sess:Vec<Session> = Vec::new();
        for i in 0..*num {
            let receiver = rand::random::<u32>();
            let sid = encode(&rand::random::<[u8; 16]>()[..]);
            sess.push(Session::new(&sid, sender, &receiver));
        }
        sess
    }

}

#[cfg(test)]
mod tests {
    extern crate base64;

    use crate::evaluation::eval::*;
    use crate::trace::traceback::{self, TraceData};
    use crate::message::messaging::{MsgReport, Session};
    use crate::db::{redis_pack,bloom_filter};
    use std::time::{SystemTime, Duration, UNIX_EPOCH};
    use crate::visualize::display;

    #[test]
    fn test_random_node () {
        for i in 0..10 {
            TreeNode::random().show();
        }
    }

    #[test]
    fn test_fwd_tree_gen () {
        let tree_depth: u32 = 50;
        let message = base64::encode(&rand::random::<[u8; 16]>()[..]);
        let start = rand::random::<u32>();
        let root = rand::random::<u32>();
        let root_packet = new_edge_gen(&message, &start, &root);

let gen_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let (tree_md, mut tree_sess, tree_tag) = fwd_tree_gen(&root_packet.key, &root, &TreeNode::new_from_tp(TreeNodeType::ActiveFwd), &message, &(0 as u32), &tree_depth);
let gen_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Tree gentime: {:?}", gen_end - gen_start);
println!("FwdTree size: {}\nSearchTree size: {}", tree_md.len(), tree_sess.len());

        let _ = redis_pack::pipe_add(&mut tree_sess);
        let _ = bloom_filter::madd(&tree_tag);

        let report_md = tree_md.get(tree_md.len()-1).unwrap();
let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = traceback::tracing(MsgReport::new(report_md.key, message.clone()), &report_md.uid);
let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Tree runtime: {:?}", trace_end - trace_start);
        assert_eq!(tree_md.len(), path.len());

        display::path_to_dot(&path, &tree_depth);

        let mut db_conn = redis_pack::get_redis_conn().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }
}