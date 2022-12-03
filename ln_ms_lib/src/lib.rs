// Project Modules
mod sim_node;
mod sim_channel;
mod sim_event;
mod sim_event_manager;
mod sim_runtime_graph;
mod sim_utils;
mod sensei_controller;
mod nigiri_controller;

use sim_node::SimNode;
use sim_event_manager::SimEventManager;
use sim_channel::SimChannel;
use sim_event::SimulationEvent;
use sim_runtime_graph::RuntimeNetworkGraph;
use sim_utils::get_current_time;
use sensei_controller::SenseiController;
use nigiri_controller::NigiriController;

// Standard Modules
use std::collections::HashMap;
use std::time::Duration;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;

// External Modules
use anyhow::Result;
use tokio::runtime::Builder;
use tokio::sync::broadcast;
use sea_orm::{Database, ConnectOptions};
use serde_json::Map;

// Sensei Modules
use migration::{Migrator, MigratorTrait};
use senseicore::{
    chain::{bitcoind_client::BitcoindClient, manager::SenseiChainManager},
    config::SenseiConfig,
    database::SenseiDatabase,
    events::SenseiEvent,
    services::admin::AdminService
};

// ==================================================================================================================
// NOTE: This project is currently setup as a Proof of Concept and does not accurately simulate any LN operations yet
// ==================================================================================================================

// This struct is the public facing API for users of this library
// A user will define the initial state of the network by adding nodes, channels, events, etc...
// After the LnSimulation is defined it will be run and the events will take place
pub struct LnSimulation {
    name: String,
    duration: u64,
    num_nodes: u64,
    user_events: HashMap<u64, Vec<SimulationEvent>>,
    user_nodes: HashMap<String, SimNode>,
    user_channels: Vec<SimChannel>,
    network_graph: RuntimeNetworkGraph
}

impl LnSimulation {
    // Create a new Lightning Network Simulation with the specified number of default nodes (num_nodes) that will run for a specified duration (dur)
    pub fn new(name: String, dur: u64, num_nodes: u64) -> Self {
        let sim = LnSimulation {
            name: name,
            duration: dur,
            num_nodes: num_nodes,
            user_events: HashMap::new(),
            user_nodes: HashMap::new(),
            user_channels: Vec::new(),
            network_graph: RuntimeNetworkGraph::new()
        };

        sim
    }

    // Run the Lightning Network Simulation
    pub fn run(&mut self, nigiri: bool) -> Result<()> {
        println!("LnSimulation:{} -- running simulation: {} ({} seconds)", get_current_time(), self.name, self.duration);

        // Setup the sensei config
        // TODO: fix the paths, this is ugly
        let this_file = file!();
        let sensei_data_dir = this_file.replace("lib.rs", "sensei_data_dir");
        let sensei_config_file = sensei_data_dir.clone() + "/config.json";
        let mut config = SenseiConfig::from_file(sensei_config_file.clone(), None);
        let sqlite_path = sensei_data_dir.clone() + "/" + config.database_url.as_str();
        config.database_url = format!("sqlite://{}?mode=rwc", sqlite_path);
        let stop_signal = Arc::new(AtomicBool::new(false));
        let sensei_data_dir_main = sensei_data_dir.clone();

        // Log some initial configuration details
        println!("sensei data: {}", sensei_data_dir);
        println!("sensei config file: {}", sensei_config_file);
        println!("sensei database: {}", config.database_url);
        println!("bitcoind username: {}", config.bitcoind_rpc_username);
        println!("bitcoind password: {}", config.bitcoind_rpc_password);
        println!("bitcoin host: {}", config.bitcoind_rpc_host);

        // Setup the sensei database runtime
        let sensei_db_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("sensei_db")
            .enable_all()
            .build()?;
        let sensei_db_runtime_handle = sensei_db_runtime.handle().clone();

        // Setup the sensei runtime
        let sensei_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("sensei")
            .enable_all()
            .build()?;
        let sensei_runtime_handle = sensei_runtime.handle().clone();

        // Setup the main simulation runtime
        let simulation_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("simulation")
            .enable_all()
            .build()?;

        // Initialize sensei and the simulation
        simulation_runtime.block_on(async move {
            if nigiri {
                // Start bitcoind with nigiri
                println!("starting bitcoind with nigiri...");
                NigiriController::start();
            }

            // Initialize the sensei database
            println!("starting sensei database...");
            let mut sensei_db_options = ConnectOptions::new(config.database_url.clone());
            sensei_db_options.max_connections(100).min_connections(10).connect_timeout(Duration::new(30, 0));
            let sensei_db_conn = Database::connect(sensei_db_options).await.unwrap();
            Migrator::up(&sensei_db_conn, None).await.expect("failed to run migrations");
            let sensei_database = SenseiDatabase::new(sensei_db_conn, sensei_db_runtime_handle.clone());

            // Initialize the bitcoin client
            println!("initializing the sensei bitcoin client...");
            let bitcoind_client = Arc::new(
                BitcoindClient::new(
                    config.bitcoind_rpc_host.clone(),
                    config.bitcoind_rpc_port,
                    config.bitcoind_rpc_username.clone(),
                    config.bitcoind_rpc_password.clone(),
                    tokio::runtime::Handle::current(),
                )
                .await
                .expect("could not connect to to bitcoind"),
            );

            // Initialize the chain manager
            println!("initializing the sensei chain manager...");
            let sensei_chain_manager = Arc::new(
                SenseiChainManager::new(
                    config.clone(),
                    bitcoind_client.clone(),
                    bitcoind_client.clone(),
                    bitcoind_client,
                )
                .await
                .expect("could not initialize the sensei chain manager"),
            );

            // Initialize the admin service
            println!("initializing the sensei admin service...");
            let (sensei_event_sender, _event_receiver): (broadcast::Sender<SenseiEvent>, broadcast::Receiver<SenseiEvent>) = broadcast::channel(1024);
            let sensei_admin_service = Arc::new(
                AdminService::new(
                    &sensei_data_dir,
                    config.clone(),
                    sensei_database,
                    sensei_chain_manager,
                    sensei_event_sender,
                    tokio::runtime::Handle::current(),
                    stop_signal.clone(),
                )
                .await,
            );

            // Create the sensei controller
            let sensei_controller = SenseiController::new(sensei_admin_service, sensei_runtime_handle);

            // TODO: starting lots of sensei nodes is very slow, need to speed this process up by stripping out processes that are not needed for simulations
            // TODO: funding all of the nodes in the network with some initial liquidity will take some research
            // TODO: how much liquidity will each node need to have?
            // TODO: how do we model a realistic LN liquidity distribution?
            // Create the initial state of the network (nodes, channels, balances, etc...)
            println!("initializing simulation network...");
            sensei_controller.initialize_network(&self.user_nodes, &self.user_channels, self.num_nodes, nigiri).await;

            // Set up the initial runtime network graph
            println!("initializing the runtime network graph...");
            self.network_graph.update(&self.user_nodes, &self.user_channels, self.num_nodes);

            // Create the channel for threads to communicate over
            let (sim_event_sender, _): (broadcast::Sender<SimulationEvent>, broadcast::Receiver<SimulationEvent>) = broadcast::channel(1024);
            let sensei_event_receiver = sim_event_sender.subscribe();
            let ln_simulation_receiver = sim_event_sender.subscribe();

            // TODO: Start the TransactionGenerator
            // TODO: Clone the sim_event_sender and pass to transaction generator so that it can send events
            // This thread will generate simulated network traffic
            println!("starting the transaction generator...");

            // TODO: Start the Analyzer
            // TODO: Create anoter event receiver and pass to the analyzer so that it can receive events
            // This thread will watch the network and gather stats and data about it that will be reported at the end of the sim
            println!("starting the network analyzer...");

            // Start the SenseiController
            println!("starting the sensei controller...");
            let sensei_controller_arc = Arc::new(sensei_controller);
            let sensei_controller_handle = thread::spawn(move || {
                sensei_controller_arc.process_events(sensei_event_receiver);
            });

            // Start the EventManager
            // This thread will fire off the events at the specified time in the sim
            println!("starting the event manager...");
            // TODO: currently for simplicity in order to create a proof of concept and demonstrate the use case of this project
            //       the duration is denoted in seconds and will run in real time... eventually this will need to be changed to a purely event driven
            //       time concept that will allow the simulations to be run in a faster-than-real-time mode
            // TODO: allow for a use case where the user wants to spin up a large network and connect a user-controlled node to perform manual testing
            let event_manager = SimEventManager::new(self.user_events.clone());
            let event_manager_arc = Arc::new(event_manager);
            let d = self.duration;
            let event_manager_handle = thread::spawn(move || {
                event_manager_arc.run(d, sim_event_sender);
            });

            // Let the LnSimulation object wait and listen for events
            self.listen(ln_simulation_receiver).await;

            // Wait for all threads to finish and stop the sensei admin service
            match event_manager_handle.join() {
                Ok(()) => println!("event manager stopped..."),
                Err(_) => println!("event manager could not be stopped...")
            }
            match sensei_controller_handle.join() {
                Ok(()) => println!("sensei controller stopped..."),
                Err(_) => println!("sensei controller could not be stopped...")
            }

            // Clear the runtime network graph
            self.network_graph.nodes.clear();
            self.network_graph.channels.clear();

            if nigiri {
                // Stop bitcoind with nigiri
                println!("stopping bitcoind with nigiri...");
                NigiriController::stop();
            }
        });

        // Clean up sensei data after the simulation is done
        fs::remove_file(sensei_data_dir_main.clone() + "/sensei.db").expect("File delete failed");
        fs::remove_file(sensei_data_dir_main.clone() + "/sensei.db-shm").expect("File delete failed");
        fs::remove_file(sensei_data_dir_main.clone() + "/sensei.db-wal").expect("File delete failed");
        fs::remove_dir_all(sensei_data_dir_main.clone() + "/logs").expect("File delete failed");
        fs::remove_dir_all(sensei_data_dir_main.clone() + "/regtest").expect("File delete failed");

        Ok(())
    }

    // Listens for simulation events and updates the runtime network graph
    pub async fn listen(&mut self, mut event_channel: broadcast::Receiver<SimulationEvent>) {
        let mut running = true;
        while running {
            let event = event_channel.recv().await.unwrap();
            match event {
                SimulationEvent::NodeOfflineEvent(name) => {
                    println!("LnSimulation:{} -- NodeOfflineEvent, updating network graph", get_current_time());
                    for node in &mut self.network_graph.nodes {
                        if node.name == name {
                            node.running = false;
                            break;
                        }
                    }
                },
                SimulationEvent::NodeOnlineEvent(name) => {
                    println!("LnSimulation:{} -- NodeOnlineEvent, updating network graph", get_current_time());
                    for node in &mut self.network_graph.nodes {
                        if node.name == name {
                            node.running = true;
                            break;
                        }
                    }
                },
                SimulationEvent::CloseChannelEvent(channel) => {
                    println!("LnSimulation:{} -- CloseChannelEvent, updating network graph", get_current_time());
                    self.network_graph.channels.retain(|c| c.node1 != channel.node1 && c.node2 != channel.node2);
                },
                SimulationEvent::OpenChannelEvent(channel) => {
                    println!("LnSimulation:{} -- OpenChannelEvent, updating network graph", get_current_time());
                    self.network_graph.channels.push(channel);
                },
                SimulationEvent::SimulationEnded => {
                    println!("LnSimulation:{} -- SimulationEnded", get_current_time());
                    running = false;
                }
            }
        }
    }

    // Get the current network graph for the simulation
    pub fn get_runtime_network_graph(&self) -> String {
        println!("LnSimulation:{} -- get current runtime network graph", get_current_time());
        let serialized_nodes = serde_json::to_string(&self.network_graph.nodes).unwrap();
        let serialized_channels = serde_json::to_string(&self.network_graph.channels).unwrap();
        let mut map = Map::new();
        map.insert(String::from("nodes"), serde_json::Value::String(serialized_nodes));
        map.insert(String::from("channels"), serde_json::Value::String(serialized_channels));
        serde_json::to_string(&map).unwrap()
    }

    // Parse a file that contains a definition of a LN topology
    // This definition could be from a project like Polar or from dumping the network information from the mainnet
    pub fn import_network(&self, filename: String) {
        println!("LnSimulation:{} -- importing network definition from {}", get_current_time(), filename);
        // TODO: Implement
    }

    // Export the network to a json file that can be loaded later
    pub fn export_network(&self, filename: String) {
        println!("LnSimulation:{} -- exporting network definition from {}", get_current_time(), filename);
        // TODO: Implement
    }

    // Create a lightweight node in the simulated network
    pub fn create_node(&mut self, name: String, initial_balance: i32, running: bool) {
        println!("LnSimulation:{} -- create Node: {}", get_current_time(), name);
        let name_key = name.clone();
        let node = SimNode {
            name: name,
            initial_balance: initial_balance,
            running: running
        };
        self.user_nodes.insert(name_key, node);
    }

    // Open a channel between two lightweight nodes in the simulated network
    pub fn create_channel(&mut self, node1_name: String, node2_name: String, amount: i32) {
        println!("LnSimulation:{} -- open Channel: {} -> {} for {} sats", get_current_time(), node1_name, node2_name, amount);
        let channel1 = SimChannel {
            node1: node1_name,
            node2: node2_name,
            node1_balance: amount,
            node2_balance: 0
        };
        self.user_channels.push(channel1);
    }

    // TODO: There should be a generic create_event function that allows the user to create all supported events
    // NOTE: For simplicity while proving the concept two hard coded event functions will work

    // Create an event that will start up a node in the simulated network
    pub fn create_node_online_event(&mut self, name: String, time: u64) {
        println!("LnSimulation:{} -- add NodeOnlineEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::NodeOnlineEvent(name);
        self.add_event(event, time);
    }

    // Create an event that will shut down a node in the simulated network
    pub fn create_node_offline_event(&mut self, name: String, time: u64) {
        println!("LnSimulation:{} -- add NodeOfflineEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::NodeOfflineEvent(name);
        self.add_event(event, time);
    }

    // Create an event that will open a new channel between two nodes
    pub fn create_open_channel_event(&mut self, name: String, name2: String, amount: i32, time: u64) {
        println!("LnSimulation:{} -- add OpenChannelEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::OpenChannelEvent(SimChannel{node1: name, node2: name2, node1_balance: amount, node2_balance: 0});
        self.add_event(event, time);
    }

    // Create an event that will close a channel between two nodes
    pub fn create_close_channel_event(&mut self, name: String, name2: String, amount: i32, time: u64) {
        println!("LnSimulation:{} -- add CloseChannelEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::CloseChannelEvent(SimChannel{node1: name, node2: name2, node1_balance: amount, node2_balance: 0});
        self.add_event(event, time);
    }

    // Add a SimulationEvent to the list of events to execute
    pub fn add_event(&mut self, event: SimulationEvent, time: u64) {
        if self.user_events.contains_key(&time) {
            let current_events = self.user_events.get_mut(&time).unwrap();
            current_events.push(event);
        } else {
            let mut current_events: Vec<SimulationEvent> = Vec::new();
            current_events.push(event);
            self.user_events.insert(time, current_events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // Setup the simulation
        let mut ln_sim = LnSimulation::new(String::from("test"), 10, 5);
        
        ln_sim.create_node(String::from("blake"), 500, false);
        ln_sim.create_node(String::from("brianna"), 600, false);
        ln_sim.create_node(String::from("brooks"), 700, false);
        ln_sim.create_node(String::from("clay"), 800, false);
        ln_sim.create_channel(String::from("blake"), String::from("brianna"), 500);
        ln_sim.create_node_online_event(String::from("blake"), 3);
        ln_sim.create_node_online_event(String::from("brianna"), 3);
        ln_sim.create_node_offline_event(String::from("blake"), 6);
        ln_sim.create_node_offline_event(String::from("brianna"), 8);
        
        assert_eq!(ln_sim.user_channels.len(), 1);
        assert_eq!(ln_sim.user_nodes.len(), 4);
        assert_eq!(ln_sim.user_events.len(), 3);
        
        // Start the simulation
        let success = ln_sim.run(true);
        assert_eq!(success.is_ok(), true);
    }
}
