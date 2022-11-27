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

    pub fn update(&mut self, nodes: &HashMap<String, SimNode>, channels: &Vec<SimChannel>) {
        for (_,n) in nodes {
            self.nodes.push(SimNode { name: String::from(&n.name), running: n.running });
        }
        for c in channels {
            self.channels.push(SimChannel { node1: String::from(&c.node1), node2: String::from(&c.node2), node1_balance: c.node1_balance, node2_balance: c.node2_balance });
        }
    }
}