pub mod display {
    use crate::trace::traceback::{Edge};
    use std::collections::HashMap;

    pub fn refine_user_id(users_id: Vec<u32>, edges: Vec<Edge>) -> Vec<Edge> {
        let mut users: HashMap<u32, u32> = HashMap::new();
        let mut refined_edges: Vec<Edge> = Vec::new();
        
        for i in 0..(users_id.len()) {
            users.insert(*users_id.get(i).unwrap(), (i+1).try_into().unwrap());
        }

        for e in edges {
            let sender = users.get(&e.sender).unwrap();
            let receiver = users.get(&e.receiver).unwrap();
            refined_edges.push(Edge::new(*sender, *receiver));
        }
        refined_edges
    }
}