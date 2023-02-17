pub mod sir {
    extern crate petgraph;
    
    use crate::{message::messaging::{Edge, Session}, trace::traceback::TraceData};
    use petgraph::{graph::{NodeIndex, Graph, Node}, visit::{IntoNodeReferences, IntoNeighbors}, adj::EdgeIndex};
    use petgraph::dot::{Dot, Config};
    use petgraph::prelude::UnGraph;
    use std::{io::{prelude::*, BufReader}, collections::HashMap, fs::File};
    use std::fs;

    #[derive(Debug,PartialEq)]
    pub enum Condition {
        Susceptible,
        Infective,
        Recovered,
    }

    pub fn remove_replicate_in_DB(input_file_dir: String, output_file_dir: String) {
        let file = fs::File::open(input_file_dir).unwrap();
        let reader = BufReader::new(file);
        let mut edge_list = Vec::<(usize,usize)>::new();
        for line in reader.lines() {
            let str_line = line.unwrap();
            let items: Vec<&str> = str_line.split(",").collect();
            let edge: (usize, usize) = (items[0].to_string().parse::<usize>().unwrap(), items[1].to_string().parse::<usize>().unwrap());
            edge_list.push(edge);
        }
        let simplified_edge_list = vec_remove_replicates(&edge_list);
        
        let mut write_file = fs::File::create(output_file_dir).unwrap();
        for line in simplified_edge_list {
            let str = line.0.to_string() + "," + &line.1.to_string() + "\n";
            write_file.write_all(str.as_bytes()).expect("write failed");
        }
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

    pub fn sir_spread(round: &usize, s2i: &usize, i2r: &usize, sys_graph: &UnGraph::<usize, ()>) -> (Graph::<usize,()>, HashMap::<usize, NodeIndex>) {
        // initialization
        let mut node_condition = HashMap::<NodeIndex, Condition>::new();
        let mut edge_condition = Vec::<(NodeIndex,NodeIndex)>::new();
        for n in sys_graph.node_references() {
            node_condition.insert(n.0, Condition::Susceptible);
        }
        // set start node
        if let Some(x) = node_condition.get_mut(&NodeIndex::new(719)) {
            *x = Condition::Infective;
        }

        // Infective, recover or infect others
        for _t in 0..*round {
            let mut tbd_nodes = Vec::<NodeIndex>::new();
            for n in sys_graph.node_references() {
                if *(node_condition.get(&n.0).unwrap()) == Condition::Infective {
                    // Infect neighbors
                    for nbr in sys_graph.neighbors(n.0) {
                        if *(node_condition.get(&nbr).unwrap()) == Condition::Susceptible {
                            if rand_state(s2i) {
                                tbd_nodes.push(nbr.clone());
                                if !edge_condition.contains(&(n.0, nbr)) { edge_condition.push((n.0, nbr));}
                            }
                        }
                        else {
                            if !edge_condition.contains(&(n.0, nbr)) {
                                if rand_state(s2i) { 
                                    // avoid replicates and two-node loop
                                    if !edge_condition.contains(&(nbr, n.0)){ edge_condition.push((n.0, nbr)); }
                                }
                            }
                        }
                    }
                    // Recover?
                    if rand_state(i2r) {
                        *(node_condition.get_mut(&n.0).unwrap()) = Condition::Recovered;
                    }
                }
            }
            for n in tbd_nodes {
                *(node_condition.get_mut(&n).unwrap()) = Condition::Infective;
            }
        }
        let usize_edge_list = node_index_to_usize(edge_condition);
        vec_to_graph(&usize_edge_list)
    }

    fn node_index_to_usize (edges: Vec::<(NodeIndex, NodeIndex)>) -> Vec::<(usize,usize)> {
        let mut conv_edges = Vec::<(usize,usize)>::new();
        for e in edges {
            conv_edges.push((e.0.index(), e.1.index()));
        }
        conv_edges
    }

    pub fn vec_to_graph (edges: &Vec::<(usize,usize)>) -> (Graph::<usize,()>, HashMap::<usize, NodeIndex>) {
        let mut nodes = Vec::<usize>::new();
        let mut node_index = HashMap::<usize, NodeIndex>::new();
        for e in edges {
           (!nodes.contains(&e.0)).then(|| nodes.push(e.0));
           (!nodes.contains(&e.1)).then(|| nodes.push(e.1));
        }
        let mut fwd_graph = Graph::<usize,()>::new();
        for n in nodes { 
            let index = fwd_graph.add_node(n as usize); 
            node_index.insert(n, index);
        }

        for e in edges {
            fwd_graph.add_edge(*node_index.get(&e.0).unwrap(), *node_index.get(&e.1).unwrap(), ());
        }
        (fwd_graph, node_index)
    }

    pub fn rand_state (prob: &usize) -> bool {
        let threshold: usize = prob * 10;
        let coin = rand::random::<usize>() % 10000;
        if coin < (threshold as usize) {return true}
        else {return false}
    }

    pub fn graph_to_dot (g: &Graph<usize, ()>, dir: String) {
        let mut f = File::create(dir).unwrap();
        let output = format!("{:?}", Dot::with_config(g, &[Config::EdgeNoLabel]));
        let _ = f.write_all(&output.as_bytes());
    }

    fn vec_remove_replicates (list: &Vec<(usize,usize)>) -> Vec<(usize,usize)> {
        let mut exist_edge = Vec::<(usize,usize)>::new();
        for i in 0..list.len() {
            let e = list.get(i).unwrap();
            if exist_edge.contains(e) | exist_edge.contains(&(e.1, e.0)) { continue; }
            else { exist_edge.push(*e); }
        }
        exist_edge
    }
}

pub mod fuzzy_traceback {
    use std::collections::HashMap;

    use petgraph::{graph::{NodeIndex, Graph, Node}, visit::{IntoNodeReferences, IntoNeighbors, IntoNeighborsDirected}, Direction};
    use petgraph::dot::{Dot, Config};
    use petgraph::prelude::UnGraph;
    use crate::message::messaging::receive_msg;

    use super::sir::{rand_state, self};

    pub fn fuzz_bfs(full_graph: &UnGraph::<usize,()>, subgraph: &Graph::<usize,()>, full_sub_index_map: &HashMap<usize, NodeIndex>, start_node: &NodeIndex, fpr: &usize) -> (Graph::<usize, ()>, HashMap::<usize, NodeIndex>) {
        let mut curr_nodes = Vec::<NodeIndex>::new();
        let mut next_nodes = Vec::<NodeIndex>::new();
        let mut searched_nodes = Vec::<NodeIndex>::new();
        let mut fuzz_edge_list = Vec::<(usize,usize)>::new();
        curr_nodes.push(*start_node);

        while curr_nodes.len() != 0 {
            for node in &curr_nodes {
                for nbr in full_graph.neighbors(*node) {
                    let mut is_positive = false;
                    if full_sub_index_map.contains_key(&(node.index())) & full_sub_index_map.contains_key(&(nbr.index())) {
                        let node_fwd_index = full_sub_index_map.get(&(node.index())).unwrap();
                        let nbr_fwd_index = full_sub_index_map.get(&(nbr.index())).unwrap();
                        (subgraph.contains_edge(*node_fwd_index, *nbr_fwd_index) | subgraph.contains_edge(*nbr_fwd_index, *node_fwd_index)).then(|| is_positive = true);
                    }
                    else { rand_state(fpr).then(|| is_positive = true); }

                    if is_positive {
                        (!fuzz_edge_list.contains(&(nbr.index(), node.index())) & !fuzz_edge_list.contains(&(node.index(), nbr.index()))).then(|| fuzz_edge_list.push((node.index(), nbr.index())));
                        (!searched_nodes.contains(&nbr)).then(|| next_nodes.push(nbr));
                    }
                }
            }
            searched_nodes.append(&mut curr_nodes);
            curr_nodes.append(&mut next_nodes);
        }
        sir::vec_to_graph(&fuzz_edge_list)
    }

    // divide the traced path into forward path and backward path
    pub fn fuzzy_trace_ours(full_graph: &UnGraph::<usize,()>, subgraph: &Graph::<usize,()>, full_sub_index_map: &HashMap<usize, NodeIndex>, start_node: &NodeIndex, fpr: &usize) -> ((Graph::<usize, ()>, HashMap::<usize, NodeIndex>), (Graph::<usize, ()>, HashMap::<usize, NodeIndex>)) {
        let mut bwd_curr_nodes = Vec::<NodeIndex>::new();
        let mut fwd_curr_nodes = Vec::<NodeIndex>::new();
        let mut bwd_next_nodes = Vec::<NodeIndex>::new();
        let mut fwd_next_nodes = Vec::<NodeIndex>::new();
        let mut bwd_searched_nodes = Vec::<NodeIndex>::new();
        let mut fwd_searched_nodes = Vec::<NodeIndex>::new();
        let mut bwd_edge_list = Vec::<(usize,usize)>::new();
        let mut fwd_edge_list = Vec::<(usize,usize)>::new();

        bwd_curr_nodes.push(*start_node);
        while (bwd_curr_nodes.len() != 0) | (fwd_curr_nodes.len() != 0) {
            fwd_curr_nodes.append(&mut bwd_curr_nodes.clone());
            // backward search
            for node in &bwd_curr_nodes {
                for nbr in full_graph.neighbors(*node) {
                    membership_test(true, full_sub_index_map, &nbr, node, subgraph, fpr, &mut bwd_edge_list, &mut bwd_searched_nodes, &mut bwd_next_nodes);
                }
            }
            bwd_searched_nodes.append(&mut bwd_curr_nodes);
            bwd_curr_nodes.append(&mut bwd_next_nodes);

            // fackward search
            for node in &fwd_curr_nodes {
                for nbr in full_graph.neighbors(*node) {
                    membership_test(false, full_sub_index_map, node, &nbr, subgraph, fpr, &mut fwd_edge_list, &mut fwd_searched_nodes, &mut fwd_next_nodes);
                }
            }
            fwd_searched_nodes.append(&mut fwd_curr_nodes);
            fwd_curr_nodes.append(&mut fwd_next_nodes);
        }
        (sir::vec_to_graph(&bwd_edge_list), sir::vec_to_graph(&fwd_edge_list))
    }

    fn membership_test(is_bwd: bool, sys_fwd_index_map: &HashMap<usize, NodeIndex>, sender: &NodeIndex, receiver: &NodeIndex, fwd_graph: &Graph<usize, ()>, fpr: &usize, edge_list: &mut Vec<(usize, usize)>, searched_nodes: &mut Vec<NodeIndex>, next_nodes: &mut Vec<NodeIndex>) {
        let mut is_positive = false;
        if sys_fwd_index_map.contains_key(&(sender.index())) & sys_fwd_index_map.contains_key(&(receiver.index())) {
            let sender_index = sys_fwd_index_map.get(&(sender.index())).unwrap();
            let receiver_index = sys_fwd_index_map.get(&(receiver.index())).unwrap();
            fwd_graph.contains_edge(*sender_index, *receiver_index).then(|| is_positive = true);
        }
        else { rand_state(fpr).then(|| is_positive = true); }
        
        if is_positive {
            let t_sender = if is_bwd {sender} else {receiver};
            let t_receiver = if is_bwd {receiver} else {sender};

            if !edge_list.contains(&(t_receiver.index(), t_sender.index())) {
                edge_list.push((t_receiver.index(), t_sender.index()));
            }
            (!searched_nodes.contains(&t_sender)).then(|| next_nodes.push(*t_sender));
        }
    }

    pub fn fuzz_value (subgraph: &Graph::<usize,()>, full_graph: &UnGraph::<usize,()>) {
        // 1. Compute FPR of all edges
        // 2. Compute FPR of nodes in backward graph
        // 3. Compute FPR of nodes in forward graph
        // 4. Integrate FPRs of nodes in loop or fwd/bwd graph
    }

    pub fn any_leaf (fwd_graph: &Graph::<usize,()>) -> NodeIndex {
        for n in fwd_graph.node_references() {
            if fwd_graph.neighbors_directed(n.0, Direction::Outgoing).count() == 0 {
                let weight = fwd_graph.node_weight(NodeIndex::from(n.0.index() as u32)).unwrap();
                return NodeIndex::from(*weight as u32)
            }
        }
        return NodeIndex::from(1000000)
    }

    pub fn mean_degree (sub_graph: &Graph::<usize,()>, full_graph: &UnGraph::<usize,()>) -> usize {
        let mut count = 0;
        for n in sub_graph.node_references() {
            count += full_graph.neighbors(NodeIndex::from(*n.1 as u32)).count();
        }
        count / sub_graph.node_count()
    }

}

#[cfg(test)]
mod tests {
    extern crate test;
    use std::{fs::File, io::Write};

    use petgraph::{Graph, Undirected, dot::{Dot, Config}, adj::NodeIndex};
    use crate::simulation::{sir::{self}, fuzzy_traceback::{fuzz_bfs, self, mean_degree, fuzzy_trace_ours}};

    use super::sir::remove_replicate_in_DB;
  
    #[test]
    fn test_remove_replicates() {
        let dir = "./datasets/email-Eu-core-temporal.txt";
        let output_dir = "./python/email.txt";
        remove_replicate_in_DB(dir.to_string(), output_dir.to_string());
    }

    #[test]
    fn test_import_graph() {
        let dir = "./graphs/email.txt";
        let g = sir::import_graph(dir.to_string());
        println!("{:?}, {:?}", g.node_count(), g.edge_count());
    }

    #[test]
    fn test_sir_spread() {
        let dir = "./graphs/message.txt";
        let sys_graph = sir::import_graph(dir.to_string());
println!("{:?}, {:?}", sys_graph.node_count(), sys_graph.edge_count());
        let (fwd_graph, _) = sir::sir_spread(&1, &60, &700, &sys_graph);
println!("{:?}, {:?}", fwd_graph.node_count(), fwd_graph.edge_count());

        sir::graph_to_dot(&fwd_graph, "temp/fwd_graph.dot".to_string());
    }

    #[test]
    fn test_fuzz_bfs() {
        let sys_graph = sir::import_graph("./graphs/message.txt".to_string());
        let (fwd_graph, sys_fwd_map) = sir::sir_spread(&4, &50, &800, &sys_graph.clone());
println!("Forward Graph: node {:?}, edge {:?}, mean degree: {:?}", fwd_graph.node_count(), fwd_graph.edge_count(), mean_degree(&fwd_graph, &sys_graph));
sir::graph_to_dot(&fwd_graph, "./temp/fwd_graph.dot".to_string());
        // start from a leaf node
        let start_node = fuzzy_traceback::any_leaf(&fwd_graph);
        let (fuzz_graph, sys_fuzz_map) = fuzz_bfs(&sys_graph, &fwd_graph, &sys_fwd_map, &start_node, &8);
println!("Fuzzy Graph: node {:?}, edge {:?}, mean degree: {:?}", fuzz_graph.node_count(), fuzz_graph.edge_count(), mean_degree(&fuzz_graph, &sys_graph));
sir::graph_to_dot(&fuzz_graph, "./temp/fuzz_graph.dot".to_string());
    }

    #[test]
    fn test_fuzz_ours() {
        let sys_graph = sir::import_graph("./graphs/message.txt".to_string());
        let (fwd_graph, sys_fwd_map) = sir::sir_spread(&4, &50, &800, &sys_graph.clone());
        // println!("{:?}, {:?}", sys_graph.node_count(), sys_graph.edge_count());
println!("Forward Graph: node {:?}, edge {:?}, mean degree: {:?}", fwd_graph.node_count(), fwd_graph.edge_count(), mean_degree(&fwd_graph, &sys_graph));
sir::graph_to_dot(&fwd_graph, "./temp/fwd_graph.dot".to_string());

    let start_node = fuzzy_traceback::any_leaf(&fwd_graph);
    let ((bwd_fuzz_graph, _), (fwd_fuzz_graph, _)) = fuzzy_trace_ours(&sys_graph, &fwd_graph, &sys_fwd_map, &start_node, &5);
println!("Fuzzy bwd Graph: node {:?}, edge {:?}", bwd_fuzz_graph.node_count(), bwd_fuzz_graph.edge_count());
sir::graph_to_dot(&bwd_fuzz_graph, "./temp/fuzz_bwd_graph.dot".to_string());
println!("Fuzzy fwd Graph: node {:?}, edge {:?}", fwd_fuzz_graph.node_count(), fwd_fuzz_graph.edge_count());
sir::graph_to_dot(&fwd_fuzz_graph, "./temp/fuzz_fwd_graph.dot".to_string());
    }

}