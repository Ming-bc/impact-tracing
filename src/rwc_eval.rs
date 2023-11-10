#![allow(dead_code)]

pub mod rwc_eval {
    use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}, fs::File, io::Write, vec};

    use base64::encode;
    use petgraph::{prelude::UnGraph, visit::EdgeRef};

    use crate::{simulation::{sir, utils::{vec_to_graph, dedup_vec_edges}, fuzzy_traceback::{fuzzy_trace_ours, any_leaf, self, degree_analysis}}, message::messaging::{MsgReport, MsgPacket, Session, IdKey, send_packet}, db::{bloom_filter, db_ik, db_nbr}, tool::{algos::tk_gen, utils::hash}};
    use crate::trace::traceback;

    pub fn eval_fuzz_trace_runtime(trace_fpr: &f32, s2i: &f32, i2r: &f32, loop_index: &usize, sys_graph: &UnGraph<usize,()>,  fwd_out_dir: &String) -> Vec<f64> {
        let mut record: Vec<Vec<f64>> = Vec::new();
        // 1. init tracing keys
        let map_id_ik = sys_ik_init(&sys_graph);
        for i in 0..*loop_index{
            // 2. generate fwd and fuzz graph
            let (_, fwd_edges, fuzz_edges_hmap) = gen_fwd_fuzz_edges(&sys_graph, trace_fpr, s2i, i2r);
            let fuzz_edges: Vec<(usize,usize)> = fuzz_edges_hmap.into_iter().map(|(k,_)| k).collect();
println!("Forward edges: {}", fwd_edges.len());
for (snd, rcv) in &fwd_edges {
    print!("{}-{}, ", snd, rcv);
}
            // 2-1. graph analysis
            write_vec_edges_to_file(&fwd_edges, &format!("{}-{}.txt", fwd_out_dir, i));
            let (fwd_graph, _) = vec_to_graph(&fwd_edges);
            let (fuzz_graph, _) = vec_to_graph(&fuzz_edges);
            let (fwd_degree, fuzz_degree) =  (degree_analysis(&fwd_graph, &sys_graph), degree_analysis(&fuzz_graph, &sys_graph));

            // 3. mock sends for fuzz_edges
            let trace_st_node: u32 = fuzzy_traceback::any_leaf(&fwd_graph) as u32;
            let message = "message".to_string();
            let root: u32 = 719;

            let (_, first_packet) = frist_pkg(&message, &root);
            let mut rcv_keys: HashMap<u32,[u8;16]> = HashMap::new();
            let mut expl_user: Vec<u32> = Vec::new();
            recursive_mock_send(&root, &first_packet.key, &message, &mut expl_user, &fuzz_edges, &map_id_ik, &mut rcv_keys);

            // 4. traceback
            let trace_st_key = rcv_keys.get(&trace_st_node).unwrap();
            let t_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let edges = traceback::tracing(&MsgReport {key: *trace_st_key, payload: message}, &trace_st_node);

println!("\nTraced edges: {}", edges.len());
for e in &edges {
    print!("{}-{}, ", e.sender, e.receiver);
}

// convert edges to Vec<(usize,usize)>
let vec_edges: Vec<(usize,usize)> = edges.into_iter().map(|e| (e.sender as usize, e.receiver as usize)).collect();
println!("\nDiff edges: {}", diff_edges(&fwd_edges, &vec_edges).len());
for (snd, rcv) in &diff_edges(&fwd_edges, &vec_edges) {
    print!("{}-{}, ", snd, rcv);
}
            let t_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            
            // 5. record as fwd_nodes, fwd_edges, fuzz_nodes, fuzz_edges, runtime
            record.push(vec![fwd_graph.node_count() as f64, fwd_edges.len() as f64, fwd_degree as f64,  fuzz_graph.node_count() as f64, fuzz_edges.len() as f64, fuzz_degree as f64, (t_end.as_millis() - t_start.as_millis()) as f64]);
        }
        matrix_aver(&record)
    }

    fn matrix_aver(record: &Vec<Vec<f64>>) -> Vec<f64> {
        // compute the average of each column
        let mut avg: Vec<f64> = Vec::new();
        for i in 0..record[0].len() {
            let mut sum: f64 = 0.0;
            for j in 0..record.len() {
                sum += record[j][i];
            }
            avg.push(f64::trunc((sum / record.len() as f64)  * 100.0) / 100.0);
        }
        avg
    }

    pub fn new_edge_gen(message: &String, sid: &u32, rid: &u32) -> MsgPacket {
        let map_id_ik = db_ik::query(&vec![*sid]);
        let tk = tk_gen(map_id_ik.get(sid).unwrap(), rid);
        let packet = send_packet(message, &[1;16], &tk);
        let _ = bloom_filter::add(&vec![encode(packet.tag)]);
        packet
    }

    fn diff_edges(vec1: &Vec<(usize,usize)>, vec2: &Vec<(usize,usize)>) -> Vec<(usize,usize)> {
        let mut diff: Vec<(usize,usize)> = Vec::new();
        for e in vec2 {
            if !vec1.contains(e) {
                diff.push(*e);
            }
        }
        diff
    }

    fn frist_pkg(message: &String, root: &u32) -> (u32, MsgPacket) {
        let snd: u32 = 30000;
        let sik = hash(&snd.to_string());
        let _ = db_ik::add(&vec![IdKey{id: snd, key: sik}]);
        let _ = db_nbr::add(&mut vec![Session::new(&snd, &root)]);
        let pkg = new_edge_gen(&message, &snd, &root);
        (snd, pkg)
    }

    fn gen_fwd_fuzz_edges(sys_graph: &UnGraph<usize,()>, trace_fpr: &f32, s2i: &f32, i2r: &f32) -> (usize, Vec<(usize,usize)>, HashMap<(usize,usize),(usize,usize)>) {
        loop {
            // Generate a forward graph （default is 20 rounds)
            let (infected_edges, node_src) = sir::sir_spread(&20, s2i, i2r, &sys_graph.clone());
if (infected_edges.len() < 10) | (infected_edges.len() > 100) {
    continue;
}
            let (fwd_graph, fwd_to_sys_id_map) = vec_to_graph(&infected_edges);
            // 2. Run fuzzy traceback to generate the fuzzy forward graph
            let start_node = any_leaf(&fwd_graph);
            let (bwd_traced_edges, mut fwd_traced_edges, _) = fuzzy_trace_ours(&sys_graph, &fwd_graph, &fwd_to_sys_id_map, &node_src, &start_node, &trace_fpr);
            bwd_traced_edges.into_iter()
                .filter(|((snd, rcv), _)| {
                    !fwd_traced_edges.contains_key(&(*rcv, *snd))
                })
                .collect::<HashMap<(usize,usize),(usize,usize)>>()
                .into_iter().for_each(|(k,v)| {
                    fwd_traced_edges.insert((k.1, k.0), v);
            });
            return (start_node, infected_edges, fwd_traced_edges);
        }
    }

    fn sys_ik_init(sys_graph: &UnGraph<usize, ()>) -> HashMap<u32, [u8;16]> {
        let raw_sys_edges: Vec<(usize,usize)> = sys_graph.edge_references().map(|e| (e.source().index(), e.target().index())).collect();
        let sys_edges = dedup_vec_edges(&raw_sys_edges);
        let mut sys_sess: Vec<Session> = Vec::new();
        let mut map_id_ik: HashMap<u32, [u8;16]> = HashMap::new();
        for e in sys_edges {
            map_id_ik.insert(e.0 as u32, hash(&e.0.to_string()));
            map_id_ik.insert(e.1 as u32, hash(&e.1.to_string()));
            sys_sess.push(Session {sid: e.0 as u32, rid: e.1 as u32 })
        }

        // convert map_id_ik to Vec<IdKey>
        let mut id_ik: Vec<IdKey> = Vec::new();
        for (id, ik) in map_id_ik.clone() {
            id_ik.push(IdKey {id: id as u32, key: ik});
        }
        let _= db_ik::add(&id_ik);
        let _= db_nbr::add(&sys_sess);
        map_id_ik
    }

    fn recursive_mock_send(root: &u32, key: &[u8; 16], message: &String, expl_user: &mut Vec<u32>, edge_list: &Vec<(usize,usize)>, map_id_ik: &HashMap<u32,[u8;16]>, keys: &mut HashMap<u32,[u8;16]>) {
        match expl_user.contains(root) {
            false => {
                expl_user.push(*root);
                edge_list.into_iter()
                .filter(|(sid, _)| (*sid as u32) == *root)
                .for_each(|(sid,rid)| {
                    let tk = tk_gen(map_id_ik.get(&(*sid as u32)).unwrap(), &(*rid as u32));
                    let packet = send_packet(message, key, &tk);
                    let _ = bloom_filter::add(&vec![encode(packet.tag)]);
                    keys.insert(*rid as u32, packet.key); 
                    recursive_mock_send(&(*rid as u32), &packet.key, message, expl_user, edge_list, map_id_ik, keys);
                })
            },
            true => ()
        }
    }

    fn write_vec_edges_to_file(edges: &Vec<(usize,usize)>, out_dir: &String) {
        let mut file = File::create(out_dir).unwrap();
        for (snd, rcv) in edges {
            let output = format!("{},{}\n", snd, rcv);
            let _ = file.write_all(&output.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{simulation::utils::import_graph, rwc_eval::rwc_eval::eval_fuzz_trace_runtime, db::{bloom_filter, db_ik, db_nbr}};

    fn db_clear() {
        db_nbr::clear();
        db_ik::clear();
        bloom_filter::clear();
    }

    #[test]
    fn test_gen_fwd_fuzz_edges() {
        // let sys_graph = import_graph("./graphs/message.txt".to_string());
        let sys_graph = import_graph("./graphs/email.txt".to_string());
        // s2i: 0.05, i2r: 0.4-0.9; s2i: 0.03-0.08, i2r: 0.7;
        // let s2i_list = vec![0.05];
        // let i2r_list = vec![0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        // let s2i_list = vec![0.03, 0.04, 0.05, 0.06, 0.07, 0.08];
        // let i2r_list = vec![0.7];
        let s2i_list = vec![0.013];
        let i2r_list = vec![0.7];
        let trace_fpr: f32 = 0.0;
        let loop_index = 1;

        let mut count = 0;
        for s2i in s2i_list {
            for i2r in &i2r_list {
                let output_dir = format!("./output/rwc/{}", count);
                let record = eval_fuzz_trace_runtime(&trace_fpr, &s2i, i2r, &loop_index, &sys_graph, &output_dir);
                println!("\nS-I-R: {}-{}; Fwd-Fuzz: ({}:{}:{})-({}:{}:{}); Runtime: {}", s2i, i2r, record[0], record[1], record[2], record[3], record[4], record[5], record[6]);
                count += 1;
                db_clear();
            }
        }
    }

}