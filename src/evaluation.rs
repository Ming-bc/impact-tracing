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
                0..=5 => tp = TreeNodeType::ActiveFwd,
                6..=10 => tp = TreeNodeType::InactiveFwd,
                11..=20 => tp = TreeNodeType::ActiveUser,
                21..=100 => tp = TreeNodeType::InactiveUser,
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
                    conn_size = Self::random_size_val(10, 20);
                    fwd_size = Self::random_size_val(5, 15) % 10;
                },
                TreeNodeType::InactiveFwd => {
                    conn_size = Self::random_size_val(10, 20);
                    fwd_size = Self::random_size_val(0, conn_size as u16) % 5;
                },
                TreeNodeType::ActiveUser => {
                    conn_size = Self::random_size_val(5, 15);
                    fwd_size = Self::random_size_val(4, conn_size as u16) % 8;
                },
                TreeNodeType::InactiveUser => {
                    conn_size = Self::random_size_val(0, 15);
                    fwd_size = Self::random_size_val(0, conn_size as u16) % 5;
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

    pub fn fwd_tree_gen (start_key: &[u8; 16], root_id: &u32, root_tp: &TreeNode, message: &str, md_tr: &mut Vec<TraceData>, sess_tr: &mut Vec<Session>, fwd_tree_size: &u32) {
        if md_tr.len() < (fwd_tree_size + 1) as usize {
            for i in 0..root_tp.fwd_size {
                let receiver: u32 = rand::random::<u32>();
                let rcv_tp: TreeNode = TreeNode::random();
                let sid = encode(&rand::random::<[u8; 16]>()[..]);
                sess_tr.push(Session::new(&sid, root_id, &receiver));

                let bk = &base64::decode(sid).unwrap()[..];
                let packet = fwd_edge_gen(*start_key, message, root_id, &receiver, bk);
                md_tr.push(TraceData::new(receiver, packet.key));

                sess_tr.extend(fake_receivers(root_id, &root_tp.conn_size));

                fwd_tree_gen(&packet.key, &receiver, &rcv_tp, message, md_tr, sess_tr, fwd_tree_size);
            }
        }
    }

    fn write_tag_to_bf(sess:Session, packet: MsgPacket) -> bool {
        let sid = sess.id;
        let bk = <&[u8; 16]>::try_from(&decode(sid).unwrap()[..]).unwrap().clone();
        let store_tag = algos::proc_tag(&sess.sender, &bk, &packet.tag);
        bloom_filter::add(&store_tag).is_ok()
    }

    // generate a new edge from a sender to a receiver
    pub fn new_edge_gen(message: &str, sender: &u32, receiver: &u32) -> MsgPacket {
        let sid = base64::encode(&rand::random::<[u8; 16]>()[..]);
        let sess = Session::new(&sid, &sender, &receiver);
        let _ = redis_pack::add(&vec![sess]);

        let bk = &base64::decode(sid).unwrap()[..];
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let sess = Session::new( &encode(bk_16), sender, receiver);
        while write_tag_to_bf( sess, MsgPacket::new(&bk_16, message) ) != true {
            panic!("process failed.");
        }
        messaging::new_msg(bk_16, message)
    }
    
    // generate a forward edge from a sender to a receiver
    fn fwd_edge_gen(prev_key: [u8; 16], message: &str, sender: &u32, receiver: &u32, bk: &[u8]) -> MsgPacket {
        let bk_16 = <&[u8; 16]>::try_from(bk).unwrap();
        let sess = Session::new( &encode(bk_16), sender, receiver);
        let packet = messaging::fwd_msg(&prev_key, &vec![*bk_16], message, FwdType::Receive);
        while write_tag_to_bf(sess, MsgPacket::new(&packet.key, message)) != true {
            panic!("process failed.");
        }
        packet
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
    use crate::db::redis_pack;
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
        let fwd_tree_size: u32 = 500;
        let message = base64::encode(&rand::random::<[u8; 16]>()[..]);
        let start = rand::random::<u32>();
        let root = rand::random::<u32>();
        let root_packet = new_edge_gen(&message, &start, &root);
        println!("TreeGen start");

        let mut tree_md: Vec<TraceData> = Vec::new();
        let mut tree_sess: Vec<Session> = Vec::new();
let gen_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        fwd_tree_gen(&root_packet.key, &root, &TreeNode::new_from_tp(TreeNodeType::ActiveFwd), &message, &mut tree_md, &mut tree_sess, &fwd_tree_size);
println!("FwdTree size: {}", tree_md.len());
println!("SearchTree size: {}", tree_sess.len());
        let _ = redis_pack::pipe_add_auto_cut(&mut tree_sess);
let gen_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Tree gentime: {:?}", gen_end - gen_start);

        let report_md = tree_md.get(tree_md.len()-1).unwrap();

let trace_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let path = traceback::packed_tracing(MsgReport::new(report_md.key, message.clone()), &report_md.uid);
let trace_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
println!("Tree runtime: {:?}", trace_end - trace_start);

        assert_eq!(path.len(), tree_md.len());

        display::path_to_dot(&path);

        let mut db_conn = redis_pack::connect().unwrap();
        let _: () = redis::cmd("FLUSHDB").query(&mut db_conn).unwrap();
    }
}