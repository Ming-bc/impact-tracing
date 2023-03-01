pub mod utils {
    extern crate petgraph;
    
    use petgraph::{graph::{NodeIndex, Graph}, visit::IntoNodeReferences};
    use petgraph::dot::{Dot, Config};
    use petgraph::prelude::UnGraph;
    use std::{io::{prelude::*, BufReader}, collections::HashMap, fs::File, hash::Hash};
    use std::fs;

    pub fn rand_state (prob: &f32) -> bool {
        let threshold: u32 = (prob * 1000.0) as u32;
        let coin = rand::random::<u32>() % 1000;
        return coin < threshold
    }

    pub fn import_graph(file_dir: String) -> UnGraph::<usize, ()> {
        let file = fs::File::open(file_dir).unwrap();
        let reader = BufReader::new(file);
        let mut edge_list = Vec::<(u32,u32)>::new();
        for line in reader.lines() {
            let str_line = line.unwrap();
            let items: Vec<&str> = str_line.split(",").collect();
            let edge: (u32, u32) = (items[0].to_string().parse::<u32>().unwrap(), items[1].to_string().parse::<u32>().unwrap());
            edge_list.push(edge);
        }
        UnGraph::<usize, ()>::from_edges(edge_list.into_iter())
    }

    pub fn dedup_in_db_file(input_file_dir: String, output_file_dir: String) {
        let file = fs::File::open(input_file_dir).unwrap();
        let reader = BufReader::new(file);
        let mut edge_list = Vec::<(usize,usize)>::new();
        for line in reader.lines() {
            let str_line = line.unwrap();
            let items: Vec<&str> = str_line.split(",").collect();
            let edge: (usize, usize) = (items[0].to_string().parse::<usize>().unwrap(), items[1].to_string().parse::<usize>().unwrap());
            edge_list.push(edge);
        }
        let simplified_edge_list = dedup_vec_edges(&edge_list);
        
        let mut write_file = fs::File::create(output_file_dir).unwrap();
        for line in simplified_edge_list {
            let str = line.0.to_string() + "," + &line.1.to_string() + "\n";
            write_file.write_all(str.as_bytes()).expect("write failed");
        }
    }

    pub fn dedup_vec_edges (list: &Vec<(usize,usize)>) -> Vec<(usize,usize)> {
        let mut exist_edge = Vec::<(usize,usize)>::new();
        list.iter().for_each(|e| {
            if !exist_edge.contains(e) & !exist_edge.contains(&(e.1, e.0)) {
                exist_edge.push(*e);
            }
        });
        exist_edge
    }

    pub fn vec_to_graph (edges: &Vec::<(usize,usize)>) -> (Graph::<usize,()>, HashMap::<usize, NodeIndex>) {
        let mut nodes = Vec::<usize>::new();
        let mut node_index = HashMap::<usize, NodeIndex>::new();
        let mut fwd_graph = Graph::<usize,()>::new();

        for e in edges {
           (!nodes.contains(&e.0)).then(|| nodes.push(e.0));
           (!nodes.contains(&e.1)).then(|| nodes.push(e.1));
        }

        nodes.iter().for_each(|n| { node_index.insert(*n, fwd_graph.add_node(*n as usize)); });

        for e in edges {
            fwd_graph.add_edge(*node_index.get(&e.0).unwrap(), *node_index.get(&e.1).unwrap(), ());
        }
        (fwd_graph, node_index)
    }

    pub fn hmap_to_graph (hmap_edges: &HashMap<(usize,usize),(usize,usize)>) -> (Graph::<usize,usize>, HashMap::<usize, NodeIndex>) {
        let mut node_index = HashMap::<usize, NodeIndex>::new();
        let mut fwd_graph = Graph::<usize,usize>::new();
        let mut nodes = Vec::<usize>::new();

        hmap_edges.iter().for_each(|e| {
            (!nodes.contains(&e.0.0)).then(|| nodes.push(e.0.0));
            (!nodes.contains(&e.0.1)).then(|| nodes.push(e.0.1));
        });

        nodes.iter().for_each(|n| { node_index.insert(*n, fwd_graph.add_node(*n as usize)); });

        hmap_edges.iter().for_each(|e| {
            fwd_graph.add_edge(*node_index.get(&e.0.0).unwrap(), *node_index.get(&e.0.1).unwrap(), e.1.0);
        });
        (fwd_graph, node_index)
    }

    pub fn fuzzy_value_to_graph (origin_graph: &Graph<usize, usize>, node_values: &HashMap<usize,f64>, edge_values: &HashMap<NodeIndex,f64>) -> Graph::<f64,f64> {
        let mut fuzzy_graph = Graph::<f64,f64>::new();
        let mut hmap_old_new = HashMap::<NodeIndex,NodeIndex>::new();
        for (old_n, k) in origin_graph.node_references() {
            let new_weight = (*k as f64) + (node_values.get(k).unwrap() / 100.0);
            let new_n = fuzzy_graph.add_node(new_weight);
            hmap_old_new.insert(old_n, new_n);
        }
        for e in origin_graph.edge_indices() {
            let (st, end) = origin_graph.edge_endpoints(e).unwrap();
            let edge_weight = match edge_values.contains_key(&st) {
                true => *edge_values.get(&st).unwrap(),
                false =>  -1.0,
            };
            fuzzy_graph.add_edge(*hmap_old_new.get(&st).unwrap(), *hmap_old_new.get(&end).unwrap(), f64::trunc(edge_weight  * 100.0) / 100.0);
        }
        fuzzy_graph
    }

    pub fn graph_to_dot<T: std::fmt::Debug, U: std::fmt::Debug> (g: &Graph<T, U>, dir: String) {
        let mut f = File::create(dir).unwrap();
        let output = format!("{:?}", Dot::with_config(g, &[]));
        let _ = f.write_all(&output.as_bytes());
    }

    pub fn write_val_to_file<T: std::fmt::Debug> (node_value: &HashMap<usize,T>, dir: String) {
        let mut f = File::create(dir).unwrap();
        for (k,v) in node_value {
            let output = format!("{},{:?}\n", k, v);
            let _ = f.write_all(&output.as_bytes());
        }
    }
}

pub mod sir {
    extern crate petgraph;
    
    use petgraph::{graph::{NodeIndex, Graph}, visit::{IntoNodeReferences}};
    use petgraph::prelude::UnGraph;
    use std::collections::HashMap;

    use super::utils::{rand_state, vec_to_graph};

    #[derive(Debug,PartialEq)]
    enum Condition {
        Susceptible,
        Infective,
        Recovered,
    }

    pub fn sir_spread(round: &usize, s2i: &f32, i2r: &f32, sys_graph: &UnGraph::<usize, ()>) -> (Vec::<(usize,usize)>, HashMap::<usize,usize>) {
        // initialization
        let mut nodes_state = HashMap::<NodeIndex, Condition>::new();
        let mut edges_state = Vec::<(usize,usize)>::new();
        let mut node_msg_source = HashMap::<usize,usize>::new();
        for n in sys_graph.node_references() {
            nodes_state.insert(n.0, Condition::Susceptible);
        }
        // set start node
        let start_index = NodeIndex::new(719);
        if let Some(x) = nodes_state.get_mut(&start_index) {
            *x = Condition::Infective;
        }
        node_msg_source.insert(start_index.index(), usize::MAX);
        // Infective, recover or infect others
        for _t in 0..*round {
            let mut tbd_nodes = Vec::<NodeIndex>::new();
            let infected_nodes: Vec<NodeIndex> = sys_graph.node_indices().filter(|n| *(nodes_state.get(&n).unwrap()) == Condition::Infective).collect();
            infected_nodes.iter().for_each(|n| {
                // Infect neighbors
                for nbr in sys_graph.neighbors(*n) {
                    let mut is_infected = false;
                    match *nodes_state.get(&nbr).unwrap() {
                        Condition::Susceptible => {
                            rand_state(s2i).then(||{
                                tbd_nodes.push(nbr.clone());
                                is_infected = true;
                            });
                        },
                        _ => {
                            vec_edge_exists(&true, &edges_state, &nbr, &n).then(|| is_infected = rand_state(s2i));
                        },
                    }

                    (is_infected & vec_edge_exists(&true, &edges_state, &n, &nbr)).then(|| {
                        edges_state.push((n.index(), nbr.index()));
                        (!node_msg_source.contains_key(&nbr.index())).then(|| {
                            node_msg_source.insert(nbr.index(), n.index());
                        });
                    });
                }
                // Recover?
                rand_state(i2r).then(|| *(nodes_state.get_mut(&n).unwrap()) = Condition::Recovered);
            });
            tbd_nodes.iter().for_each(|&x| *(nodes_state.get_mut(&x).unwrap()) = Condition::Infective);
        }
        (edges_state, node_msg_source)
    }

    pub fn vec_edge_exists(is_directed: &bool, edges_state: &Vec<(usize, usize)>, snd: &NodeIndex, rcv: &NodeIndex) -> bool {
        match is_directed {
            true => !edges_state.contains(&(snd.index(), rcv.index())),
            false => !edges_state.contains(&(snd.index(), rcv.index())) & !edges_state.contains(&(rcv.index(), snd.index())),
        }
    }

}

pub mod fuzzy_traceback {
    use std::{collections::HashMap};

    use petgraph::{graph::{NodeIndex, Graph}, visit::{IntoNodeReferences, NodeRef, IntoNeighbors}, Direction::{self, Incoming, Outgoing}};
    use petgraph::prelude::UnGraph;
    use probability::{distribution::Binomial, prelude::Discrete, source};

    use super::{utils::{rand_state, vec_to_graph, hmap_to_graph}, sir::vec_edge_exists};

    #[derive(Debug, Clone, PartialEq)]
    pub struct TraceMd {
        node: usize,
        prev_node: usize,
    }

    pub fn fuzz_bfs(full_graph: &UnGraph::<usize,()>, subgraph: &Graph::<usize,()>, full_sub_index_map: &HashMap<usize, NodeIndex>, start_node: &usize, fpr: &f32) -> (Graph::<usize, ()>, HashMap::<usize, NodeIndex>) {
        let mut curr_nodes = Vec::<NodeIndex>::new();
        let mut next_nodes = Vec::<NodeIndex>::new();
        let mut searched_nodes = Vec::<NodeIndex>::new();
        let mut fuzz_edge_list = Vec::<(usize,usize)>::new();
        curr_nodes.push(NodeIndex::from(*start_node as u32));

        while curr_nodes.len() != 0 {
            for node in &curr_nodes {
                for nbr in full_graph.neighbors(*node) {
                    let is_positive;
                    if full_sub_index_map.contains_key(&(node.index())) & full_sub_index_map.contains_key(&(nbr.index())) {
                        let node_fwd_index = full_sub_index_map.get(&(node.index())).unwrap();
                        let nbr_fwd_index = full_sub_index_map.get(&(nbr.index())).unwrap();
                        is_positive = subgraph.contains_edge(*node_fwd_index, *nbr_fwd_index) | subgraph.contains_edge(*nbr_fwd_index, *node_fwd_index);
                    }
                    else { is_positive = rand_state(fpr); }
                    is_positive.then(|| {
                        vec_edge_exists(&false, &fuzz_edge_list, &nbr, &node).then(|| fuzz_edge_list.push((node.index(), nbr.index())));
                        (!searched_nodes.contains(&nbr)).then(|| next_nodes.push(nbr));
                    });
                }
            }
            searched_nodes.append(&mut curr_nodes);
            curr_nodes.append(&mut next_nodes);
        }
        vec_to_graph(&fuzz_edge_list)
    }

    // divide the traced path into forward path and backward path
    pub fn fuzzy_trace_ours(full_graph: &UnGraph::<usize,()>, subgraph: &Graph::<usize,()>, full_sub_index_map: &HashMap<usize, NodeIndex>, fwd_srcs: &HashMap<usize, usize>, start_node: &usize, fpr: &f32) -> (HashMap::<(usize,usize),(usize,usize)>, HashMap::<(usize,usize),(usize,usize)>, usize) {
        let mut bwd_curr_tmd = Vec::<TraceMd>::new();
        let mut fwd_curr_tmd = Vec::<TraceMd>::new();
        let mut bwd_srched_nodes = Vec::<TraceMd>::new();
        let mut fwd_srched_nodes = Vec::<TraceMd>::new();
        let mut bwd_traced_edges = HashMap::<(usize,usize),(usize,usize)>::new();
        let mut fwd_traced_edges = HashMap::<(usize,usize),(usize,usize)>::new();
        let mut depth: usize = 0;

        // bwd_curr_tmd.push(TraceMd { node: *start_node, tag: *fwd_srcs.get(start_node).unwrap()});
        bwd_curr_tmd.push(TraceMd { node: *start_node, prev_node: usize::MAX});

        while (bwd_curr_tmd.len() != 0) | (fwd_curr_tmd.len() != 0) {
            fwd_curr_tmd.append(&mut bwd_curr_tmd.clone());
            // backward search
            one_round_search(&true, &mut bwd_curr_tmd, full_graph, full_sub_index_map, subgraph, fpr, depth, &mut bwd_traced_edges, &mut bwd_srched_nodes, fwd_srcs);
            // forward search
            one_round_search(&false, &mut fwd_curr_tmd, full_graph, full_sub_index_map, subgraph, fpr, depth, &mut fwd_traced_edges, &mut fwd_srched_nodes, fwd_srcs);
            depth += 1;
        }
        
        (bwd_traced_edges, fwd_traced_edges, depth - 1)
    }

    fn one_round_search(is_bwd: &bool, curr_tmd: &mut Vec<TraceMd>, full_graph: &UnGraph<usize, ()>, id_map: &HashMap<usize, NodeIndex>, subgraph: &Graph<usize, ()>, fpr: &f32, depth: usize, edge_list: &mut HashMap<(usize,usize), (usize,usize)>, searched_tmd: &mut Vec<TraceMd>, fwd_srcs: &HashMap<usize, usize>) {
        let mut next_tmd = Vec::<TraceMd>::new();

        curr_tmd.iter().for_each(|tmd|{
            let node_id = NodeIndex::from(tmd.node as u32);
            full_graph.neighbors(node_id).for_each(|nbr| {
                // membership test
                let (snd_id, rcv_id) = match is_bwd {
                    true => (nbr.index() as usize, node_id.index() as usize),
                    false => (node_id.index() as usize, nbr.index() as usize),
                };

                let mut is_true_positive = false;
                let mut is_false_positive = false;

                if id_map.contains_key(&snd_id) & id_map.contains_key(&rcv_id) {
                    let snd_sub_id = id_map.get(&snd_id).unwrap();
                    let rcv_sub_id = id_map.get(&rcv_id).unwrap();
                    is_true_positive = match is_bwd {
                        true => subgraph.contains_edge(*snd_sub_id, *rcv_sub_id) & (*fwd_srcs.get(&rcv_id).unwrap() == snd_id),
                        false => subgraph.contains_edge(*snd_sub_id, *rcv_sub_id),
                    };
                }
                else { is_false_positive = rand_state(fpr); }
                
                while is_true_positive | is_false_positive {

                    let (t_snd, t_rcv) = match is_bwd {
                        true => (snd_id, rcv_id),
                        false => (rcv_id, snd_id),
                    };
                    (!edge_list.contains_key(&(t_rcv, t_snd))).then(|| {
                        edge_list.insert((t_rcv, t_snd), (depth, tmd.prev_node));
                    });
                    (!searched_tmd.contains(&tmd)).then(|| next_tmd.push(TraceMd { node: t_snd, prev_node: t_rcv }));
                    break;
                }
            });
        });
        searched_tmd.append(curr_tmd);
        curr_tmd.append(&mut next_tmd);
    }

    pub fn calc_fuz_val (fpr: &f64, max_depth: &usize, trace_start_node: &NodeIndex, bwd_edge_list: &HashMap<(usize,usize),(usize,usize)>, fwd_edge_list: &mut HashMap<(usize,usize),(usize,usize)>, full_graph: &UnGraph::<usize,()>) -> (HashMap::<usize, f64>, HashMap::<(usize, usize), f64>) {
        let mut nodes_fuzzy_value = HashMap::<usize, f64>::new();

        let (bwd_graph, _) = hmap_to_graph(bwd_edge_list);
        // remove backward graph from forward graph
        for kv in bwd_edge_list {
            fwd_edge_list.contains_key(&(kv.0.1, kv.0.0)).then(|| fwd_edge_list.remove(&(kv.0.1, kv.0.0)));
        }
        let (fwd_graph, _) = hmap_to_graph(fwd_edge_list);

        // 1. Compute FPR of all edges in bwd_graph and fwd_graph, repectively.
        let bwd_edge_fpr = calc_edge_fpr_with_src(&bwd_edge_list, &bwd_graph, full_graph, fpr);
        let fwd_edge_fpr = calc_edge_fpr_with_src(&fwd_edge_list, &fwd_graph, full_graph, fpr);
// println!("Edge FPR \n{:?}\n", fwd_edge_fpr);
// println!("Edge FPR {:?}\n", fwd_edge_fpr);

        // 2. Compute FPR of nodes in backward & forward graph
        let mut bwd_node_tpr = calc_node_tpr_with_src(&true, max_depth, fpr, &bwd_graph, &full_graph, &bwd_edge_fpr, &fwd_edge_list, &bwd_edge_list);
        bwd_node_tpr.insert((*trace_start_node, NodeIndex::from(usize::MAX as u32)), 1.0);

        let mut fwd_node_tpr = calc_node_tpr_with_src(&false, max_depth, fpr, &fwd_graph, &full_graph, &fwd_edge_fpr, &fwd_edge_list, &bwd_edge_list);
        // TODO:
        let mut fwd_src_node_tpr = calc_fwd_src_node_tpr(&fwd_edge_list, &mut fwd_node_tpr, &fwd_graph);
// println!("Node TPR \n{:?}\n", fwd_node_tpr);

        // // 2-3. sources in forward graph
        // fwd_graph.node_references()
        //     .filter(|(node, _)| 
        //         fwd_graph.neighbors_directed(*node, Incoming).count() == 0)
        //     .for_each(|(node, _)| {
        //         let mut mul_child_fpr: f64 = 1.0;
        //         fwd_graph.neighbors(node).into_iter().for_each(|child|
        //             mul_child_fpr *= 1.0 - fwd_node_tpr.get(&(child,node)).unwrap()
        //         );
        //         let node_tpr = 1.0 - mul_child_fpr;
        //         fwd_node_tpr.insert((node, NodeIndex::from(usize::MAX as u32)), node_tpr);
        //     });

        // // 3. Integrate FPRs of nodes in loop or fwd/bwd graph
        let mut node_tpr = HashMap::<usize, Vec<f64>>::new();

        fwd_node_tpr.into_iter()
            .for_each(|((node,_), tpr)| {
                let node_id = *fwd_graph.node_weight(node).unwrap();
                match node_tpr.contains_key(&node_id) {
                    true => node_tpr.get_mut(&node_id).unwrap().push(tpr),
                    false => {
                        node_tpr.insert(node_id, vec![tpr]);
                    },
                };
            });

        bwd_node_tpr.into_iter()
            .for_each(|((node,_), tpr)| {
                let node_id = *bwd_graph.node_weight(node).unwrap();
                match node_tpr.contains_key(&node_id) {
                    true => node_tpr.get_mut(&node_id).unwrap().push(tpr),
                    false => {
                        node_tpr.insert(node_id, vec![tpr]);
                    },
                };
            });

        node_tpr.into_iter().for_each(|(node, tpr_vec)| {
            let mut fpr: f64 = 1.0;
            tpr_vec.iter().for_each(|tpr| {
                fpr *= 1.0 - tpr;
            });
            nodes_fuzzy_value.insert(node, f64::trunc((1.0 - fpr)  * 10000.0) / 100.0);
        });

        (nodes_fuzzy_value, fwd_edge_fpr)
    }

    // fn calc_edge_fpr(sub_graph: &Graph<usize, usize>, full_graph: &UnGraph<usize, ()>, fpr: &f64) -> HashMap<NodeIndex, f64> {
    //     let edge_fpr: HashMap::<NodeIndex, f64> = sub_graph
    //         .node_references().into_iter()
    //         .map(|(k,v)| {
    //             let fpr = fpr_from_binom(&sub_graph.neighbors(k).count(), &full_graph.neighbors(NodeIndex::from(*v as u32)).count(), fpr);
    //             (k, fpr)
    //         })
    //         .collect();
    //     edge_fpr
    // }

    fn calc_edge_fpr_with_src(edge_list: &HashMap<(usize,usize),(usize,usize)>, sub_graph: &Graph<usize, usize>, full_graph: &UnGraph<usize, ()>, fpr: &f64) -> HashMap<(usize,usize), f64> {
        // Edge fpr is defined by a tuple ((node, prev_node), fpr)
        let mut edge_fpr = HashMap::<(usize, usize), f64> ::new();

        sub_graph.node_references().into_iter().for_each(|(node, node_wt)| {
            let conn = full_graph.neighbors(NodeIndex::from(*node_wt as u32)).count();
            let mut par_list: Vec<usize> = edge_list.into_iter()
                .filter(|((_,rcv),_)| *rcv == *node_wt)
                .map(|((snd,_),_)| *snd)
                .collect();
            (par_list.len() == 0).then(|| par_list.push(usize::MAX));

            par_list.into_iter().for_each(|par_node_wt| {
                let src_edge_count: usize = if par_node_wt == usize::MAX {
                    sub_graph.neighbors_directed(node, Outgoing).count()
                }
                else {
                    sub_graph.neighbors(node).into_iter().map(|child| {
                        let child_wt = *sub_graph.node_weight(child).unwrap();
                        match edge_list.get(&(*node_wt, child_wt)).unwrap().1 == par_node_wt {
                            true => 1,
                            false => 0,
                        }
                    }).sum()
                };

                let src_fpr = fpr_from_binom(&src_edge_count, &conn, fpr);
                src_fpr.is_nan().then(|| println!("NaN: Node {}; out-degree {}, conn {}", node_wt, src_edge_count, conn));
                edge_fpr.insert((*node_wt, par_node_wt), src_fpr);
            });
        });
        edge_fpr
    }

    // fn integrate_node_tpr(fwd_graph: &Graph<usize, usize>, node_tpr: &mut HashMap<usize, Vec<f64>>, mut fwd_node_tpr: HashMap<NodeIndex, Vec<f64>>) {
    //     fwd_graph.node_references().for_each(|n| {
    //         match node_tpr.contains_key(n.1) {
    //             true => {
    //                 let tpr_list = node_tpr.get_mut(n.1).unwrap();
    //                 tpr_list.append(fwd_node_tpr.get_mut(&n.0).unwrap());
    //             },
    //             false => {
    //                 node_tpr.insert(*n.1, fwd_node_tpr.get_mut(&n.0).unwrap().clone());
    //             },
    //         }
    //     });
    // }

    // fn calc_node_tpr(is_bwd: &bool, max_depth: &usize, graph: &Graph<usize, usize>, edge_fpr: &HashMap<NodeIndex, f64>) -> HashMap<NodeIndex, Vec<f64>> {
    //     let mut node_fpr = HashMap::<NodeIndex, Vec<f64>>::new();
    //     for d in 0..(*max_depth+1) {
    //         graph.edge_indices().for_each(|e| {
    //             if *graph.edge_weight(e).unwrap() == (max_depth - d) {
    //                 let nodes = graph.edge_endpoints(e).unwrap();
    //                 match is_bwd {
    //                     true => calc_bwd_node_tpr(&nodes.0, &nodes.1, &(max_depth - d), &edge_fpr, &mut node_fpr, &graph),
    //                     false => calc_fwd_node_tpr(&nodes.0, &nodes.1, &(max_depth - d), &edge_fpr, &mut node_fpr, &graph),
    //                 }
    //             }
    //         });
    //     }
    //     node_fpr
    // }

    fn calc_node_tpr_with_src(is_bwd: &bool, max_depth: &usize, fpr: &f64, sub_graph: &Graph<usize, usize>, full_graph: &UnGraph<usize,()>, edge_fpr: &HashMap<(usize,usize), f64>, fwd_edge_list: &HashMap<(usize, usize), (usize,usize)>, bwd_edge_list: &HashMap<(usize,usize),(usize,usize)>) -> HashMap<(NodeIndex,NodeIndex), f64> {
        let mut node_fpr = HashMap::<(NodeIndex,NodeIndex),f64>::new();
        for d in 0..(*max_depth+1) {
            sub_graph.edge_indices().for_each(|e| {
                if *sub_graph.edge_weight(e).unwrap() == (max_depth - d) {
                    let nodes = sub_graph.edge_endpoints(e).unwrap();
                    match is_bwd {
                        true => calc_bwd_node_tpr_with_src(&nodes.0, &nodes.1, &edge_fpr, &mut node_fpr, &sub_graph, bwd_edge_list),
                        false => calc_fwd_node_tpr_with_src(&nodes.0, &nodes.1, &fpr, &edge_fpr, &mut node_fpr, &sub_graph, full_graph, fwd_edge_list),
                    }
                }
            });
        }
        node_fpr
    }

    fn calc_bwd_node_tpr_with_src(par_node: &NodeIndex, node: &NodeIndex, edge_fpr: &HashMap::<(usize,usize),f64>, hmap_node_tpr: &mut HashMap<(NodeIndex,NodeIndex),f64>, sub_graph: &Graph::<usize,usize>, edge_list: &HashMap<(usize, usize), (usize,usize)>) {
        let (par_node_wt, node_wt) = (sub_graph.node_weight(*par_node).unwrap(), sub_graph.node_weight(*node).unwrap());
        let num_child = sub_graph.neighbors(*node)
            .filter(|nbr| {
                let nbr_wt = sub_graph.node_weight(*nbr).unwrap();
                edge_list.get(&(*node_wt,*nbr_wt)).unwrap().1 == *par_node_wt
            })
            .count();
        let num_bro = sub_graph.neighbors(*par_node).count();
        let gpar_wt = edge_list.get(&(*par_node_wt,*node_wt)).unwrap().1;
        let edge_fpr = edge_fpr.get(&(*par_node_wt, gpar_wt)).expect(&(par_node_wt.to_string() + "_" + &gpar_wt.to_string()));
// println!("Curr-Par-Next: {}-{}-{}; Edge fpr {}, child {}", node_wt, par_node_wt, gpar_wt, edge_fpr, num_child);
        let node_tpr = match num_child {
            0 => (1.0 - *edge_fpr) / (num_bro as f64),
            _ => {
                let mut max_child_fpr: f64 = 0.0;
                sub_graph.neighbors(*node).into_iter()
                    .filter(|child| {
                        let child_wt = *sub_graph.node_weight(*child).unwrap();
                        edge_list.get(&(*node_wt,child_wt)).unwrap().1 == *par_node_wt
                    })
                    .for_each(|child| {
                        (!hmap_node_tpr.contains_key(&(child,*node))).then(||panic!("Curr {:?}, Child {:?}", sub_graph.node_weight(*node), sub_graph.node_weight(child)));
                        let child_fpr = hmap_node_tpr.get(&(child, *node)).unwrap();
                        (*child_fpr > max_child_fpr).then(|| max_child_fpr = *child_fpr);
                    });
                1.0 - *edge_fpr * (1.0 - max_child_fpr)
            }
        };
        hmap_node_tpr.insert((*node, *par_node), node_tpr);
    }

    fn calc_fwd_node_tpr_with_src(par_node: &NodeIndex, node: &NodeIndex, fpr: &f64, edge_fpr: &HashMap::<(usize,usize),f64>, hmap_node_tpr: &mut HashMap<(NodeIndex, NodeIndex),f64>, sub_graph: &Graph::<usize,usize>, full_graph: &UnGraph::<usize,()>, edge_list: &HashMap<(usize, usize), (usize,usize)>) {
        // A) in = 0
        // case 1. curr node is a source
        if sub_graph.neighbors_directed(*node, Incoming).count() == 0 {
            return
        }
        let (par_node_wt, node_wt) = (sub_graph.node_weight(*par_node).unwrap(), sub_graph.node_weight(*node).unwrap());
        let num_child = sub_graph.neighbors(*node)
            .filter(|nbr| {
                let nbr_wt = sub_graph.node_weight(*nbr).unwrap();
                edge_list.get(&(*node_wt,*nbr_wt)).unwrap().1 == *par_node_wt
            })
            .count();
        let gpar_wt = edge_list.get(&(*par_node_wt,*node_wt)).unwrap().1;

        // case 2. its par node is a source
        let edge_fpr: f64 = match sub_graph.neighbors_directed(*par_node, Incoming)
            .into_iter()
            .filter(|gpar| *sub_graph.node_weight(*gpar).unwrap() == gpar_wt)
            .count() == 0 {
                true => {
                    let child_num = edge_list.into_iter()
                        .filter(|((snd,rcv),_)| snd == par_node_wt)
                        .filter(|(_,(_,src))| *src == gpar_wt)
                        .count();
                    let conn = full_graph.neighbors(NodeIndex::from(*par_node_wt as u32)).count();
                    fpr_from_binom(&child_num, &conn, fpr)
                },
                false => *edge_fpr.get(&(*par_node_wt, gpar_wt)).expect(&(node_wt.to_string() + ":" + &par_node_wt.to_string() + "-" + &gpar_wt.to_string())),
            };
        

        // B-1) in(v) \neq 0, out(v) = 0; B-2) in(v) \neq 0, out(v) > 0;
        let node_tpr = match num_child {
            0 => 1.0 - edge_fpr,
            _ => {
                let mut mlp_child_fpr: f64 = 1.0;
                sub_graph.neighbors(*node).into_iter()
                    .filter(|child| {
                        let child_wt = *sub_graph.node_weight(*child).unwrap();
                        edge_list.get(&(*node_wt,child_wt)).unwrap().1 == *par_node_wt
                    })
                    .for_each(|child| {
                        (!hmap_node_tpr.contains_key(&(child,*node))).then(||panic!("Curr {:?}, Child {:?}", sub_graph.node_weight(*node), sub_graph.node_weight(child)));
                        mlp_child_fpr *= 1.0 - hmap_node_tpr.get(&(child,*node)).unwrap();
                    });
                1.0 - edge_fpr * mlp_child_fpr
            }
        };
        hmap_node_tpr.insert((*node, *par_node), node_tpr);
        
    }

    fn calc_fwd_src_node_tpr(edge_list: &HashMap<(usize,usize),(usize,usize)>, hmap_node_tpr: &mut HashMap<(NodeIndex,NodeIndex),f64>, sub_graph: &Graph::<usize,usize>) {
        // Find sources in the graph
        // 1-1. in(v) = 0
        let mut sources_1: Vec<(NodeIndex,usize)> = sub_graph.node_references().into_iter()
            .filter(|(node, _)| 
                sub_graph.neighbors_directed(*node, Incoming).count() == 0
            )
            .map(|(node, _)| 
                (node, usize::MAX)
            )
            .collect();
        
        // 1-2. in(v) \neq 0, but no real parents
        let mut sources_2 = Vec::<(NodeIndex,usize)>::new();
        sub_graph.node_references().for_each(|(node, node_wt)| {
            let mut src_list: Vec<usize> = sub_graph.neighbors(node)
                .map(|child| {
                    let child_wt = sub_graph.node_weight(child).unwrap();
                    edge_list.get(&(*node_wt, *child_wt)).unwrap().1
                })
                .collect();
            src_list.sort();
            src_list.dedup();
            
            src_list.into_iter().for_each(|src| {
                if sub_graph.neighbors_directed(node, Incoming)
                    .into_iter()
                    .filter(|par| *sub_graph.node_weight(*par).unwrap() == src)
                    .count() == 0 {
                        sources_2.push((node, src));
                    }
            });
        });
        sources_1.append(&mut sources_2);
        sources_1.sort();
        sources_1.dedup();

        sources_1.into_iter().for_each(|(node, src)| {
            let node_wt = sub_graph.node_weight(node).unwrap();
            let num_child = sub_graph.neighbors(node)
            .filter(|nbr| {
                let nbr_wt = sub_graph.node_weight(*nbr).unwrap();
                edge_list.get(&(*node_wt,*nbr_wt)).unwrap().1 == src
            })
            .count();

            let node_tpr = match num_child {
                0 => f64::NAN,
                _ => {
                    let mut mlp_child_fpr: f64 = 1.0;
                    sub_graph.neighbors(node).into_iter()
                        .filter(|child| {
                            let child_wt = *sub_graph.node_weight(*child).unwrap();
                            edge_list.get(&(*node_wt,child_wt)).unwrap().1 == src
                        })
                        .for_each(|child| {
                            (!hmap_node_tpr.contains_key(&(child, node))).then(||panic!("Curr {:?}, Child {:?}", sub_graph.node_weight(node), sub_graph.node_weight(child)));
                            mlp_child_fpr *= 1.0 - hmap_node_tpr.get(&(child, node)).unwrap();
                        });
                    1.0 - mlp_child_fpr
                }
            };
            (!node_tpr.is_nan()).then(|| hmap_node_tpr.insert((node, NodeIndex::from(usize::MAX as u32)), node_tpr));
        });
    }

    // fn calc_bwd_node_tpr(par_node: &NodeIndex, node: &NodeIndex, weight: &usize, hmap_edge_fpr: &HashMap::<NodeIndex,f64>, hmap_node_tpr: &mut HashMap<NodeIndex,Vec<f64>>, sub_graph: &Graph::<usize,usize>) {
    //     let num_child = weighted_child_num(sub_graph, node, weight);
    //     let num_bro = sub_graph.neighbors(*par_node).count();
    //     let edge_fpr = hmap_edge_fpr.get(par_node).unwrap();
    //     let node_tpr = match num_child {
    //         0 => (1.0 - *edge_fpr) / (num_bro as f64),
    //         _ => {
    //             let mut max_child_fpr: f64 = 0.0;
    //             for child in sub_graph.neighbors(*node) {
    //                 let conn_edge = sub_graph.find_edge(*node, child).unwrap();
    //                 if *sub_graph.edge_weight(conn_edge).unwrap() == (weight + 1) {
    //                     (!hmap_node_tpr.contains_key(&child)).then(||panic!("Curr {:?}, Child {:?}", sub_graph.node_weight(*node), sub_graph.node_weight(child)));
    //                     let child_fpr = hmap_node_tpr.get(&child).unwrap().last().unwrap();
    //                     (*child_fpr > max_child_fpr).then(|| max_child_fpr = *child_fpr);
    //                 }
    //             }
    //             1.0 - *edge_fpr * (1.0 - max_child_fpr)
    //         }
    //     };
    //     hmap_safe_insert(hmap_node_tpr, node, node_tpr);
    // }

    // fn calc_fwd_node_tpr(par_node: &NodeIndex, node: &NodeIndex, weight: &usize, hmap_edge_fpr: &HashMap::<NodeIndex,f64>, hmap_node_tpr: &mut HashMap<NodeIndex,Vec<f64>>, sub_graph: &Graph::<usize,usize>) {
    //     let num_child = weighted_child_num(sub_graph, node, weight);
    //     let edge_fpr = hmap_edge_fpr.get(par_node).unwrap();
    //     // 1) in(v) \neq 0, out(v) = 0; 2) in(v) \neq 0, out(v) > 0;
    //     let node_tpr = match num_child {
    //         0 => 1.0 - *edge_fpr,
    //         _ => {
    //             let mut mlp_child_fpr: f64 = 1.0;
    //             for child in sub_graph.neighbors(*node) {
    //                 let conn_edge = sub_graph.find_edge(*node, child).unwrap();
    //                 if *sub_graph.edge_weight(conn_edge).unwrap() == (weight + 1) {
    //                     (!hmap_node_tpr.contains_key(&child)).then(||panic!("Curr {:?}, Child {:?}", sub_graph.node_weight(*node), sub_graph.node_weight(child)));
    //                     mlp_child_fpr *= 1.0 - hmap_node_tpr.get(&child).unwrap().last().unwrap();
    //                 }
    //             }
    //             1.0 - (*edge_fpr) * mlp_child_fpr
    //         }
    //     };
    //     hmap_safe_insert(hmap_node_tpr, node, node_tpr);
    // }

    // fn weighted_child_num(sub_graph: &Graph<usize, usize>, node: &NodeIndex, weight: &usize) -> usize {
    //     let mut num_child: usize = 0;
    //     for nbr in sub_graph.edges(*node){
    //         (*nbr.weight() == (weight + 1)).then(|| num_child += 1);
    //     }
    //     num_child
    // }

    // fn calc_fwd_src_node_tpr(node: &NodeIndex, hmap_node_tpr: &mut HashMap<NodeIndex,Vec<f64>>, sub_graph: &Graph::<usize,usize>) {
    //     let num_parent = sub_graph.neighbors_directed(*node, Incoming).count();
    //     if num_parent == 0 {
    //         let mut mul_child_fpr: f64 = 1.0;
    //         sub_graph.neighbors(*node).into_iter().for_each(|child|
    //             mul_child_fpr *= 1.0 - hmap_node_tpr.get(&child).unwrap().last().unwrap()
    //         );
    //         let node_tpr = 1.0 - mul_child_fpr;
    //         hmap_safe_insert(hmap_node_tpr, node, node_tpr);
    //     }
    // }

    fn fpr_from_binom(out_degree: &usize, nbh: &usize, fpr: &f64) -> f64 {
        if *out_degree > 0 {
            let binom_distrib = Binomial::new(*nbh, *fpr);

            let total_prob: f64 = (0..(out_degree + 1)).map(|i| binom_distrib.mass(i)).sum();
            let fpr: f64 = (0..(out_degree + 1)).map(|i| ((i as f64) / (*out_degree as f64)) * (binom_distrib.mass(i) / total_prob)).sum();

            (fpr == 0.0).then(|| println!("Edge fpr = 0; Outdegee {}, NBH {}", out_degree, nbh));
            return fpr
        }
        return 0.0
    }

    fn hmap_safe_insert(hmap_node_fpr: &mut HashMap<NodeIndex, Vec<f64>>, node: &NodeIndex, node_tpr: f64) {
        match hmap_node_fpr.contains_key(node) {
            true => hmap_node_fpr.get_mut(node).unwrap().push(node_tpr),
            false => {hmap_node_fpr.insert(*node, vec![node_tpr]);},
        }
    }

    pub fn any_leaf (fwd_graph: &Graph::<usize,()>) -> usize {
        for n in fwd_graph.node_references() {
            if fwd_graph.neighbors_directed(n.0, Direction::Outgoing).count() == 0 {
                let weight = fwd_graph.node_weight(NodeIndex::from(n.0.index() as u32)).unwrap();
                return *weight
            }
        }
        return 1000000
    }

    pub fn degree_analysis (sub_graph: &Graph::<usize,()>, full_graph: &UnGraph::<usize,()>) -> f64 {
        let mut vec_degree = Vec::<i32>::new();
        for n in sub_graph.node_references() {
            vec_degree.push(full_graph.neighbors(NodeIndex::from(*n.1 as u32)).count() as i32);
        }
        // mean degree
        let sum: i32 = Iterator::sum(vec_degree.iter());
        let mean = f64::from(sum) / (vec_degree.len() as f64);
        // max
        // let max = Iterator::max(vec_degree.iter()).unwrap();
        f64::trunc(mean)
    }

    pub fn fuz_val_analysis(node_fuzzy_value: &HashMap<usize,f64>, real_nodes: &HashMap<usize,NodeIndex>) -> HashMap::<usize,usize> {
        let group:usize = 5;
        let mut node_vec_id = HashMap::<usize,usize>::new();
        let mut all: Vec<usize> = vec![0,0,0,0,0];
        let mut real: Vec<usize> = vec![0,0,0,0,0];
        for (n,v) in node_fuzzy_value {
            let mut vec_id: usize = 0;
            match *v {
                0.0..=95.0 => vec_id = 5,
                95.0..=99.0 => vec_id = 4,
                99.0..=99.95 => vec_id = 3,
                99.95..=99.99 => vec_id = 2,
                99.99..=100.0 => vec_id = 1,
                _ => (),
            }
            real_all_count(&(group - vec_id), &n, &mut real, &mut all, &real_nodes, &mut node_vec_id);
        }
        println!("{:?} {:?}", real, all);
        node_vec_id
    }

    fn real_all_count(vec_id: &usize, node_id: &usize, real_count: &mut Vec<usize>, all: &mut Vec<usize>, real_nodes: &HashMap<usize, NodeIndex>, node_vec_id: &mut HashMap::<usize,usize>) {
        *all.get_mut(*vec_id).unwrap() += 1;
        real_nodes.contains_key(node_id).then(|| {
            *real_count.get_mut(*vec_id).unwrap() += 1;
            node_vec_id.insert(*node_id, *vec_id);
        });
    }

}

#[cfg(test)]
mod tests {
    extern crate test;
    use std::{fs::File, io::Write, collections::HashMap};

    use petgraph::{Graph, Undirected, dot::{Dot, Config}, adj::NodeIndex, visit::IntoNodeReferences};
    use crate::simulation::{sir::{self}, fuzzy_traceback::{fuzz_bfs, self, degree_analysis, fuzzy_trace_ours, calc_fuz_val, fuz_val_analysis}, utils::{import_graph, graph_to_dot, fuzzy_value_to_graph, write_val_to_file, hmap_to_graph, vec_to_graph}};

    use super::utils::dedup_in_db_file;
  
    #[test]
    fn test_remove_replicates() {
        let dir = "./datasets/email-Eu-core-outputoral.txt";
        let output_dir = "./python/email.txt";
        dedup_in_db_file(dir.to_string(), output_dir.to_string());
    }

    #[test]
    fn test_import_graph() {
        let dir = "./graphs/email.txt";
        let g = import_graph(dir.to_string());
        println!("{:?}, {:?}", g.node_count(), g.edge_count());
    }

    #[test]
    fn test_sir_spread() {
        let dir = "./graphs/message.txt";
        let sys_graph = import_graph(dir.to_string());
        println!("{:?}, {:?}", sys_graph.node_count(), sys_graph.edge_count());
        let (infected_edges, msg_source) = sir::sir_spread(&10, &0.05, &0.6, &sys_graph.clone());
        let (fwd_graph, sys_fwd_map) = vec_to_graph(&infected_edges);
        println!("{:?}, {:?}", fwd_graph.node_count(), fwd_graph.edge_count());

        graph_to_dot(&fwd_graph, "output/fwd_graph.dot".to_string());
    }

    #[test]
    fn test_fuzz_bfs() {
        let sys_graph = import_graph("./graphs/message.txt".to_string());
        let (infected_edges, msg_source) = sir::sir_spread(&10, &0.05, &0.6, &sys_graph.clone());
        let (fwd_graph, sys_fwd_map) = vec_to_graph(&infected_edges);
println!("Forward Graph: node {:?}, edge {:?}, mean degree: {:?}", fwd_graph.node_count(), fwd_graph.edge_count(), degree_analysis(&fwd_graph, &sys_graph));
graph_to_dot(&fwd_graph, "./output/fwd_graph.dot".to_string());
        // start from a leaf node
        let start_node = fuzzy_traceback::any_leaf(&fwd_graph);
        let (fuzz_graph, _) = fuzz_bfs(&sys_graph, &fwd_graph, &sys_fwd_map, &start_node, &0.01);
println!("Fuzzy Graph: node {:?}, edge {:?}, mean degree: {:?}", fuzz_graph.node_count(), fuzz_graph.edge_count(), degree_analysis(&fuzz_graph, &sys_graph));
graph_to_dot(&fuzz_graph, "./output/fuzz_graph.dot".to_string());
    }

    #[test]
    fn test_fuzz_ours() {
        let sys_graph = import_graph("./graphs/message.txt".to_string());
        let trace_fpr: f32 = 0.01;

        loop {
            // 1.Generate a forward graph that start in node 719 by SIR algorithm
            let (infected_edges, node_src) = sir::sir_spread(&20, &0.05, &0.7, &sys_graph.clone());
            if infected_edges.len() == 0 {
                continue;
            }
            let (fwd_graph, sys_fwd_map) = vec_to_graph(&infected_edges);

            println!("Forward Graph: node {:?}, edge {:?}, mean degree: {:?}", fwd_graph.node_count(), fwd_graph.edge_count(), degree_analysis(&fwd_graph, &sys_graph));
            graph_to_dot(&fwd_graph, "./output/fwd_graph.dot".to_string());
            graph_to_dot(&fwd_graph, "../Traceability-Evaluation/graphs/fwd_graph.dot".to_string());

            // 2. Run fuzzy traceback to generate the fuzzy forward graph
            let start_node = fuzzy_traceback::any_leaf(&fwd_graph);
            let (mut bwd_traced_edges, mut fwd_traced_edges, max_depth) = fuzzy_trace_ours(&sys_graph, &fwd_graph, &sys_fwd_map, &node_src, &start_node, &trace_fpr);

            let (fuz_bwd_graph, fuz_bwd_id_map) = hmap_to_graph(&bwd_traced_edges);
            let (fuz_fwd_graph, _) = hmap_to_graph(&fwd_traced_edges);

            println!("Fuzzy bwd Graph: node {:?}, edge {:?}", fuz_bwd_graph.node_count(), fuz_bwd_graph.edge_count());
            graph_to_dot(&fuz_bwd_graph, "./output/fuzz_bwd_graph.dot".to_string());
            println!("Fuzzy fwd Graph: node {:?}, edge {:?}", fuz_fwd_graph.node_count(), fuz_fwd_graph.edge_count());
            graph_to_dot(&fuz_fwd_graph, "./output/fuzz_fwd_graph.dot".to_string());

            // 3. Compute fuzzy values of nodes in fuzzy graph by the membership function
            let start_in_bwd_graph = fuz_bwd_id_map.get(&start_node).unwrap();
            let (node_tpr, edge_fpr) = calc_fuz_val(&(trace_fpr as f64), &max_depth, start_in_bwd_graph, &mut bwd_traced_edges, &mut fwd_traced_edges, &sys_graph);
            println!("Fuzzy value node count: {}", node_tpr.len());

            // 3-1. insert deleted edges from bwd_edges to fwd_edges
            bwd_traced_edges.into_iter().filter(|((snd, rcv), _)| {
                    !fwd_traced_edges.contains_key(&(*rcv, *snd))
                })
                .collect::<HashMap<(usize,usize),(usize,usize)>>()
                .into_iter().for_each(|(k,v)| {
                    fwd_traced_edges.insert((k.1, k.0), v);
            });
            let (full_fuz_graph, _) = hmap_to_graph(&fwd_traced_edges);
            println!("Full fuzzy Graph: node {:?}, edge {:?}", full_fuz_graph.node_count(), full_fuz_graph.edge_count());

            // let fuzzy_graph = fuzzy_value_to_graph(&fuz_fwd_graph, &node_tpr, &edge_fpr);
            // graph_to_dot(&fuzzy_graph, "./output/fuzz_graph.dot".to_string());

            // 4. Output the max fuzzy value node to a file for analysis
            let max_tpr_nodes = fuz_val_analysis(&node_tpr, &sys_fwd_map);
            write_val_to_file(&max_tpr_nodes, "../Traceability-Evaluation/graphs/values.txt".to_string());
            let sys_fwd_map_usize: HashMap<usize,usize> = sys_fwd_map.iter().map(|(k,v)|
                (v.index(), *k)).collect();
            write_val_to_file(&sys_fwd_map_usize, "../Traceability-Evaluation/graphs/index.txt".to_string());
            break;
        }
    }

}