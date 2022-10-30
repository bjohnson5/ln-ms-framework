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

// Represents a simuation of a lightning network that runs for a specified duration
// This struct will be the public facing API for users of this library
// A user will define the initial state of the network by adding nodes, channels, events, etc...
// After the LnSimulation is defined it will be run and the events will take place

// TODO:
// 1. Add the TransactionGenerator thread that will created simulated network traffic
// 2. Add the Analyzer thread that will collect stats and report on the state of the network

pub struct LnSimulation {
    name: String,
    duration: u64,
    em: LnEventManager,
    nodes: HashMap<String, LnNode>,
    channels: Vec<LnChannel>
}

impl LnSimulation {
    pub fn new(name: String, dur: u64) -> Self {
        let sim = LnSimulation {
            name: name,
            duration: dur, 
            em: LnEventManager::new(),
            nodes: HashMap::new(),
            channels: Vec::new()
        };

        sim
    }

    pub fn run(&self) -> bool {
        println!("Running simulation: {} ({} seconds)", self.name, self.duration);

        // TODO:
        // 1. Create the initial state of the network (nodes, channels, balances, etc...)
        //      Start the LN nodes needed for this simulation in the sensei container
        //      Open channels between the LN nodes in the sensei container
        // 2. Start the TransactionGenerator if it is configured
        //      This thread will generate simulated network traffic
        // 3. Start the Analyzer
        //      This thread will watch the network and gather stats and data about it that will be reported at the end of the sim

        // NOTE:
        // Currently for simplicity in order to create a proof of concept and demonstrate the use case of this project
        // the duration is denoted in seconds and will run in real time. Eventually this will need to be a longer duration that gets
        // run at faster-than-real-time pace so that long simulations can be modeled.
        
        // Start the EventManager
        // This thread will fire off the events at the specified time in the sim
        self.em.run(self);

        true
    }

    // Parse a file that contains a definition of a LN topology
    // This definition could be from a project like Polar or from dumping the network information from the mainnet
    pub fn import_network(&self, filename: String) {
        println!("Importing network definition from {}", filename);
    }

    // Create a lightweight node in the simulated network
    pub fn create_node(&mut self, name: String) {
        println!("Create Node: {}", name);
        let name_key = name.clone();
        let node = LnNode {
            name: name
        };
        self.nodes.insert(name_key, node);
    }

    // Open a channel between two lightweight nodes in the simulated network
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

    // Create an event that will start up a node in the simulated network
    pub fn create_node_online_event(&mut self, name: String, time: u64) {
        println!("Add NodeOnlineEvent for: {} at {} seconds", name, time);
        let event = NodeOnlineEvent {
            node_name: name
        };

        self.em.add_event(Box::new(event), time);
    }

    // Create an event that will shut down a node in the simulated network
    pub fn create_node_offline_event(&mut self, name: String, time: u64) {
        println!("Add NodeOfflineEvent for: {} at {} seconds", name, time);
        let event = NodeOfflineEvent {
            node_name: name
        };

        self.em.add_event(Box::new(event), time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // Setup the simulation
        let mut ln_sim = LnSimulation::new(String::from("test"), 10);
        ln_sim.create_node(String::from("blake"));
        ln_sim.create_node(String::from("brianna"));
        ln_sim.create_channel(String::from("blake"), String::from("brianna"), 500);
        ln_sim.create_node_online_event(String::from("blake"), 3);
        ln_sim.create_node_online_event(String::from("brianna"), 3);
        ln_sim.create_node_offline_event(String::from("blake"), 6);
        ln_sim.create_node_offline_event(String::from("brianna"), 8);

        assert_eq!(ln_sim.channels.len(), 1);
        assert_eq!(ln_sim.nodes.len(), 2);
        assert_eq!(ln_sim.em.events.len(), 3);

        // Start the simulation
        let success = ln_sim.run();
        assert_eq!(success, true);
    }
}
