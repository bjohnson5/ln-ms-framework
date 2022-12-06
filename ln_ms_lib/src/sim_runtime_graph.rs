// Project Modules
use crate::sim_node::SimNode;
use crate::sim_channel::SimChannel;

// Standard Modules
use std::collections::HashMap;

pub struct RuntimeNetworkGraph {
    pub nodes: Vec<SimNode>,
    pub channels: Vec<SimChannel>
}

impl RuntimeNetworkGraph {
    pub fn new() -> Self {
        let network_graph = RuntimeNetworkGraph {
            nodes: Vec::new(),
            channels: Vec::new()
        };

        network_graph
    }

    pub fn update(&mut self, nodes: &HashMap<String, SimNode>, channels: &Vec<SimChannel>, num_nodes: u64) {
        for (_,n) in nodes {
            self.nodes.push(SimNode { name: String::from(&n.name), initial_balance: n.initial_balance, running: n.running });
        }
        
        for c in channels {
            self.channels.push(SimChannel { src_node: String::from(&c.src_node), dest_node: String::from(&c.dest_node), src_balance: c.src_balance, dest_balance: c.dest_balance });
        }

        let num = num_nodes + 1;
        for number in 1..num {
            let node_name = String::from("node")+&number.to_string();
            self.nodes.push(SimNode { name: String::from(&node_name), initial_balance: 0, running: true });
        }
    }
}