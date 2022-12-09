pub mod display {
    extern crate petgraph;
    
    use crate::{message::messaging::{Edge, Session}, trace::traceback::TraceData};
    use std::{collections::HashMap, io::Write, fs::File};
    use petgraph::graph::{NodeIndex, Graph, Node};
    use petgraph::dot::{Dot, Config};
    use petgraph::algo::astar;

    pub fn refine_user_id(users_id: Vec<u32>, edges: Vec<Edge>) -> (Vec<u32>, Vec<Edge>) {
        let mut users: HashMap<u32, u32> = HashMap::new();
        let mut refined_edges: Vec<Edge> = Vec::new();
        let mut refined_users: Vec<u32> = Vec::new();
        
        for i in 0..(users_id.len()) {
            users.insert(*users_id.get(i).unwrap(), (i+1).try_into().unwrap());
            refined_users.push((i+1).try_into().unwrap());
        }

        for e in edges {
            let sender = users.get(&e.sender).unwrap();
            let receiver = users.get(&e.receiver).unwrap();
            refined_edges.push(Edge::new(*sender, *receiver));
        }
        (refined_users, refined_edges)
    }

    pub fn path_to_dot(path: &Vec<Edge>, start: &u32) {
        let mut graph = Graph::<usize, i32>::new();
        let map = path_to_hmap(path);

        for i in 0..map.len() {
            graph.add_node(i);
        }

        for i in 0..path.len() {
            let sender = path.get(i).unwrap().sender;
            let receiver = path.get(i).unwrap().receiver;
            graph.add_edge(NodeIndex::from(*map.get(&sender).unwrap()), NodeIndex::from(*map.get(&receiver).unwrap()), 0);
        }
        // println!("{:?}",  Dot::with_config(&graph, &[Config::EdgeNoLabel]));

        // Write to file
        let mut f = File::create("python/example.dot").unwrap();
        let output = format!("{}", Dot::with_config(&graph, &[Config::EdgeNoLabel, Config::NodeNoLabel]));
        let _ = f.write_all(&output.as_bytes());
    }

    pub fn path_to_hmap(path: &Vec<Edge>) -> HashMap<u32, u32> {
        let mut sess_map: HashMap<u32, u32> = HashMap::new();
        let mut index: u32 = 0;
        for e in path {
            if sess_map.contains_key(&e.sender) == false {
                sess_map.insert(e.sender, index);
                index += 1;
            }
            if sess_map.contains_key(&e.receiver) == false {
                sess_map.insert(e.receiver, index);
                index += 1;
            }
        }
        sess_map
    }

    pub fn compute_depth(start: &u32, finish: &u32, graph: &Graph<usize, i32>) -> usize {
        // Compute depth
        let finish_node = NodeIndex::from(*finish);
        let start_node = NodeIndex::from(*start);
        let (_, path) = astar(&graph, start_node, |finish| finish == finish_node, |_| 0, |_| 0).expect("Not found a path.");
        path.len()
    }
}