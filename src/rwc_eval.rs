#![allow(dead_code, unused_imports)]

pub mod rwc_eval {
    use std::{collections::HashMap, time::{SystemTime, UNIX_EPOCH}, fs::File, io::Write, vec};

    use base64::encode;
    use petgraph::{prelude::UnGraph, visit::EdgeRef};

    use crate::{simulation::{sir, utils::{vec_to_graph, dedup_vec_edges}, fuzzy_traceback::{fuzzy_trace_ours, any_leaf, self, degree_analysis}}, message::messaging::{MsgReport, MsgPacket, IdKey, send_packet, Edge}, db::{db_tag, db_ik, db_nbr}, tool::{algos::tk_gen, utils::hash}};
    use crate::trace::traceback;

    #[derive(Debug,PartialEq)]
    pub(crate) enum Dataset {
        CollegeIM,
        EuEmail,
    }

    // Here, we set the starting node manually to ensure consistency in repeated experiments. However, our implementation also holds for random start node.
    pub fn select_dataset(data: &Dataset) -> (String, usize, usize, f32, f32) {
        match data {
            Dataset::CollegeIM => ("./datasets/message.txt".to_string(), 719, 20, 0.05, 0.6),
            Dataset::EuEmail => ("./datasets/email.txt".to_string(), 1, 20, 0.03, 0.6),
        }
    }

    pub fn eval_fuzz_trace_runtime(trace_fpr: &f32, st_node: &usize, s2i: &f32, i2r: &f32, loop_index: &usize, sys_graph: &UnGraph<usize,()>,  fwd_out_dir: &String) -> Vec<f64> {
        let mut record: Vec<Vec<f64>> = Vec::new();
        // 1. init tracing keys
        let map_id_ik = sys_ik_init(&sys_graph);
        for i in 0..*loop_index{
            // 2. generate fwd and fuzz graph
            let (_, fwd_edges, fuzz_edges_hmap) = gen_fwd_fuzz_edges(&sys_graph, st_node, trace_fpr, s2i, i2r);
            let fuzz_edges: Vec<(usize,usize)> = fuzz_edges_hmap.into_iter().map(|(k,_)| k).collect();

            // 2-1. graph analysis
            write_vec_edges_to_file(&fwd_edges, &format!("{}-{}.txt", fwd_out_dir, i));
            let (fwd_graph, _) = vec_to_graph(&fwd_edges);
            let (fuzz_graph, _) = vec_to_graph(&fuzz_edges);
            let (fwd_degree, fuzz_degree) =  (degree_analysis(&fwd_graph, &sys_graph), degree_analysis(&fuzz_graph, &sys_graph));

            // 3. mock sends for fuzz_edges
            let message = "message".to_string() + &i.to_string();

            let (_, first_packet) = frist_pkg(&message, &(*st_node as u32));
            let mut rcv_keys: HashMap<u32,[u8;16]> = HashMap::new();
            let mut expl_user: Vec<u32> = Vec::new();
            recursive_mock_send(&(*st_node as u32), &first_packet.tag_key, &message, &mut expl_user, &fuzz_edges, &map_id_ik, &mut rcv_keys);

            // 4. traceback
            let trace_st_node: u32 = fuzzy_traceback::any_leaf(&fwd_graph) as u32;
            let trace_st_key = rcv_keys.get(&trace_st_node).unwrap();
            
            let t_start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let _ = traceback::tracing(&MsgReport {key: *trace_st_key, payload: message}, &trace_st_node);
            // convert edges to Vec<(usize,usize)>
            let t_end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            // assert_eq!(trace_edges.len()-1, fuzz_edges.len());
            // println!("Real {}, Trace {}", fuzz_edges.len(), trace_edges.len());
            
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
        let _ = db_tag::add(&vec![encode(packet.p_tag)]);
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
        let _ = db_nbr::add(&mut vec![Edge::new(&snd, &root)]);
        let pkg = new_edge_gen(&message, &snd, &root);
        (snd, pkg)
    }

    fn gen_fwd_fuzz_edges(sys_graph: &UnGraph<usize,()>, st_node: &usize, trace_fpr: &f32, s2i: &f32, i2r: &f32) -> (usize, Vec<(usize,usize)>, HashMap<(usize,usize),(usize,usize)>) {
        loop {
            // Generate a forward graph （default is 20 rounds)
            let (infected_edges, node_src) = sir::sir_spread(&20, st_node, s2i, i2r, &sys_graph.clone());
            if infected_edges.len() < 50 {
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
        let mut sys_sess: Vec<Edge> = Vec::new();
        let mut map_id_ik: HashMap<u32, [u8;16]> = HashMap::new();
        for e in sys_edges {
            map_id_ik.insert(e.0 as u32, hash(&e.0.to_string()));
            map_id_ik.insert(e.1 as u32, hash(&e.1.to_string()));
            sys_sess.push(Edge {sid: e.0 as u32, rid: e.1 as u32 })
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
                    let _ = db_tag::add(&vec![encode(packet.p_tag)]);
                    keys.insert(*rid as u32, packet.tag_key); 
                    recursive_mock_send(&(*rid as u32), &packet.tag_key, message, expl_user, edge_list, map_id_ik, keys);
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
    use crate::{simulation::utils::import_graph, rwc_eval::rwc_eval::eval_fuzz_trace_runtime, db::{db_tag, db_ik, db_nbr}};

    fn db_clear() {
        db_nbr::clear();
        db_ik::clear();
        db_tag::clear();
    }

    #[test]
    fn test_trace_time() {
        let (file_dir, st_node, _, _, _) = super::rwc_eval::select_dataset(&super::rwc_eval::Dataset::CollegeIM);
        let sys_graph = import_graph(file_dir);
        // s2i: 0.05, i2r: 0.4-0.9; s2i: 0.03-0.08, i2r: 0.7;
        let s2i_list = vec![vec![0.05], vec![0.03, 0.04, 0.05, 0.06, 0.07, 0.08]];
        let i2r_list = vec![vec![0.4, 0.5, 0.6, 0.7, 0.8, 0.9], vec![0.7]];
        let trace_fpr: f32 = 0.01;
        let loop_index = 1;

        let mut count = 0;
        for b in 0..2 {
            for s2i in s2i_list.get(b).unwrap() {
                for i2r in i2r_list.get(b).unwrap() {
                    db_clear();
                    let output_dir = format!("./output/rwc/{}", count);
                    let record = eval_fuzz_trace_runtime(&trace_fpr, &st_node, s2i, i2r, &loop_index, &sys_graph, &output_dir);
                    println!("S-I-R: {}-{}; Fwd-Fuzz: ({}:{}:{})-({}:{}:{}); Runtime: {}", s2i, i2r, record[0], record[1], record[2], record[3], record[4], record[5], record[6]);
                    count += 1;
                }
            }
        }
        db_clear();
    }
}