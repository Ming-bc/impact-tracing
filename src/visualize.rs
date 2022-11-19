pub mod display {
    extern crate petgraph;
    use petgraph::dot::Dot;

    use crate::message::messaging::Edge;
    use std::collections::HashMap;
    use petgraph::graph::{NodeIndex, Graph};

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

    pub fn vec_to_dot(users_id: Vec<u32>, path: Vec<Edge>) {
        let mut graph = Graph::new();

        for user in users_id {
            graph.add_node(user);
        }
        for i in 0..path.len() {
            let sender = path.get(i).unwrap().sender;
            let receiver = path.get(i).unwrap().receiver;
            graph.add_edge(NodeIndex::from(sender-1), NodeIndex::from(receiver-1), i+1);
        }

        println!("{}", Dot::new(&graph));
    }
}