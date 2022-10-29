mod ln_node;
mod ln_channel;
mod ln_event;
mod ln_event_manager;

use std::collections::HashMap;
use ln_node::LnNode;
use ln_event_manager::LnEventManager;
use ln_channel::LnChannel;
use ln_event::NodeOnlineEvent;
use ln_event::NodeOfflineEvent;

pub struct LnSimulation {
    pub name: String,
    pub start_time: i32,
    pub end_time: i32,
    em: LnEventManager,
    nodes: HashMap<String, LnNode>,
    channels: Vec<LnChannel>
}

impl LnSimulation {
    pub fn new() -> Self {
        let sim = LnSimulation {
            name: String::from("test sim"),
            start_time: 1,
            end_time: 5,
            em: LnEventManager::new(),
            nodes: HashMap::new(),
            channels: Vec::new()
        };

        sim
    }

    pub fn run(&self) -> bool {
        println!("RUN");
        // Create the initial state of the network (nodes, channels, balances, etc...)
        // Start the TransactionGenerator
        // Start the Analyzer
        // Start the EventManager
        self.em.run(self);
        true
    }

    pub fn create_node(&mut self, name: String) {
        println!("Create Node: {}", name);
        let name_key = name.clone();
        let node = LnNode {
            name: name
        };
        self.nodes.insert(name_key, node);
    }

    pub fn create_channel(&mut self, node1_name: String, node2_name: String, amount: i32) {
        println!("Open Channel: {} -> {} for {} sats", node1_name, node2_name, amount);
        let channel1 = LnChannel {
            node1: node1_name,
            node2: node2_name,
            node1_balance: amount,
            node2_balance: 0
        };
        self.channels.push(channel1);
    }

    pub fn create_node_online_event(&mut self, name: String) {
        println!("Add NodeOnlineEvent for: {}", name);
        let event = NodeOnlineEvent {
            node_name: name
        };

        self.em.add_event(Box::new(event));
    }

    pub fn create_node_offline_event(&mut self, name: String) {
        println!("Add NodeOfflineEvent for: {}", name);
        let event = NodeOfflineEvent {
            node_name: name
        };

        self.em.add_event(Box::new(event));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // Setup the simulation
        let mut ln_sim = LnSimulation::new();
        ln_sim.create_node(String::from("blake"));
        ln_sim.create_node(String::from("brianna"));
        ln_sim.create_channel(String::from("blake"), String::from("brianna"), 500);
        ln_sim.create_node_online_event(String::from("blake"));
        ln_sim.create_node_offline_event(String::from("blake"));

        assert_eq!(ln_sim.channels.len(), 1);
        assert_eq!(ln_sim.nodes.len(), 2);
        assert_eq!(ln_sim.em.events.len(), 2);

        // Start the simulation
        let success = ln_sim.run();
        assert_eq!(success, true);
    }
}
