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
        let mut edge_list = Vec::<(u32,u32)>::new();
        for line in reader.lines() {
            let str_line = line.unwrap();
            let items: Vec<&str> = str_line.split(",").collect();
            let edge: (u32, u32) = (items[0].to_string().parse::<u32>().unwrap(), items[1].to_string().parse::<u32>().unwrap());
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

    pub fn sir_spread(round: usize, s2i: usize, i2r: usize, sys_graph: UnGraph::<usize, ()>) -> (Graph::<usize,()>, HashMap::<u32, NodeIndex>) {
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
        for _t in 0..round {
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
        let u32_edge_list = node_index_to_u32(edge_condition);
        vec_to_graph(&u32_edge_list)
    }

    fn node_index_to_u32 (edges: Vec::<(NodeIndex, NodeIndex)>) -> Vec::<(u32,u32)> {
        let mut conv_edges = Vec::<(u32,u32)>::new();
        for e in edges {
            conv_edges.push((e.0.index() as u32, e.1.index() as u32));
        }
        conv_edges
    }

    pub fn vec_to_graph (edges: &Vec::<(u32,u32)>) -> (Graph::<usize,()>, HashMap::<u32, NodeIndex>) {
        let mut nodes = Vec::<u32>::new();
        let mut node_index = HashMap::<u32, NodeIndex>::new();
        for e in edges {
            if !nodes.contains(&e.0) {nodes.push(e.0)}
            if !nodes.contains(&e.1) {nodes.push(e.1)}
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

    pub fn rand_state (prob: usize) -> bool {
        let coin = rand::random::<usize>() % 1000;
        if coin < prob {return true}
        else {return false}
    }

    pub fn graph_to_dot (g: &Graph<usize, ()>, dir: String) {
        let mut f = File::create(dir).unwrap();
        let output = format!("{:?}", Dot::with_config(g, &[Config::EdgeNoLabel]));
        let _ = f.write_all(&output.as_bytes());
    }

    fn vec_remove_replicates (list: &Vec<(u32,u32)>) -> Vec<(u32,u32)> {
        let mut exist_edge = Vec::<(u32,u32)>::new();
        for i in 0..list.len() {
            let e = list.get(i).unwrap();
            if exist_edge.contains(e) | exist_edge.contains(&(e.1, e.0)) {
                continue;
            }
            else { exist_edge.push(*e); }
        }
        exist_edge
    }
}

pub mod fuzz_trace {
    use std::collections::HashMap;

    use petgraph::{graph::{NodeIndex, Graph, Node}, visit::{IntoNodeReferences, IntoNeighbors, IntoNeighborsDirected}, Direction};
    use petgraph::dot::{Dot, Config};
    use petgraph::prelude::UnGraph;
    use super::sir::{rand_state, self};


    pub fn fuzz_bfs(sys_graph: UnGraph::<usize,()>, fwd_graph: Graph::<usize,()>, sys_fwd_index_map: HashMap<u32, NodeIndex>, start_node: NodeIndex, fpr: usize) -> (Graph::<usize, ()>, HashMap::<u32, NodeIndex>) {
        let mut curr_nodes = Vec::<NodeIndex>::new();
        let mut next_nodes = Vec::<NodeIndex>::new();
        let mut searched_nodes = Vec::<NodeIndex>::new();
        let mut fuzz_edge_list = Vec::<(u32,u32)>::new();
        curr_nodes.push(start_node);

        while curr_nodes.len() != 0 {
            for node in &curr_nodes {
                for nbr in sys_graph.neighbors(*node) {
                    let mut is_positive = false;
                    if sys_fwd_index_map.contains_key(&(node.index() as u32)) & sys_fwd_index_map.contains_key(&(nbr.index() as u32)) {
                        let node_fwd_index = sys_fwd_index_map.get(&(node.index() as u32)).unwrap();
                        let nbr_fwd_index = sys_fwd_index_map.get(&(nbr.index() as u32)).unwrap();
                        if fwd_graph.contains_edge(*node_fwd_index, *nbr_fwd_index) | fwd_graph.contains_edge(*nbr_fwd_index, *node_fwd_index) { is_positive = true; }
                    }
                    else { if rand_state(fpr) { is_positive = true;}}

                        if is_positive {
                            if !fuzz_edge_list.contains(&(nbr.index() as u32, node.index() as u32)) & !fuzz_edge_list.contains(&(node.index() as u32, nbr.index() as u32)) {
                                fuzz_edge_list.push((node.index() as u32, nbr.index() as u32));
                            }
                            if !searched_nodes.contains(&nbr) { next_nodes.push(nbr); }
                        }
                }
            }
            searched_nodes.append(&mut curr_nodes);
            curr_nodes.append(&mut next_nodes);
        }
        sir::vec_to_graph(&fuzz_edge_list)
    }

    pub fn any_leaf (fwd_graph: &Graph::<usize,()>) -> NodeIndex {
        for n in fwd_graph.node_references() {
            if fwd_graph.neighbors_directed(n.0, Direction::Outgoing).count() == 0 {
                let weight = fwd_graph.node_weight(NodeIndex::from(n.0.index() as u32)).unwrap();
                return NodeIndex::from(*weight as u32)
            }
        }
        return NodeIndex::from(5000)
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use std::{fs::File, io::Write};

    use petgraph::{Graph, Undirected, dot::{Dot, Config}, adj::NodeIndex};
    use crate::simulation::{sir::{self}, fuzz_trace::{fuzz_bfs, self}};

    use super::sir::remove_replicate_in_DB;
  
    #[test]
    fn test_remove_replicates() {
        let dir = "./datasets/email-Eu-core-temporal.txt";
        let output_dir = "./python/email.txt";
        remove_replicate_in_DB(dir.to_string(), output_dir.to_string());
    }

    #[test]
    fn test_import_graph() {
        let dir = "./graphs/message.txt";
        let g = sir::import_graph(dir.to_string());
        println!("{:?}, {:?}", g.node_count(), g.edge_count());
    }

    #[test]
    fn test_sir_spread() {
        let dir = "./graphs/message.txt";
        let sys_graph = sir::import_graph(dir.to_string());
println!("{:?}, {:?}", sys_graph.node_count(), sys_graph.edge_count());
        let (fwd_graph, _) = sir::sir_spread(1, 6, 70, sys_graph);
println!("{:?}, {:?}", fwd_graph.node_count(), fwd_graph.edge_count());

        sir::graph_to_dot(&fwd_graph, "python/fwd_graph.dot".to_string());
    }

    #[test]
    fn test_fuzz_bfs() {
        let sys_graph = sir::import_graph("./python/message.txt".to_string());
        let (fwd_graph, sys_fwd_map) = sir::sir_spread(3, 50, 700, sys_graph.clone());
        // println!("{:?}, {:?}", sys_graph.node_count(), sys_graph.edge_count());
println!("{:?}, {:?}", fwd_graph.node_count(), fwd_graph.edge_count());
sir::graph_to_dot(&fwd_graph, "python/fwd_graph.dot".to_string());
// println!("{:?}", sys_fwd_map);
        // start from a leaf node
        let start_node = fuzz_trace::any_leaf(&fwd_graph);
        let (fuzz_graph, sys_fuzz_map) = fuzz_bfs(sys_graph, fwd_graph, sys_fwd_map, start_node, 5);
println!("{:?}, {:?}", fuzz_graph.node_count(), fuzz_graph.edge_count());
sir::graph_to_dot(&fuzz_graph, "python/fuzz_graph.dot".to_string());
// println!("{:?}", sys_fuzz_map);
    }
}