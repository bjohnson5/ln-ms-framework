// Project Modules
mod sim_node;
mod sim_channel;
mod sim_transaction;
mod sim_event;
mod sim_event_manager;
mod sim_runtime_graph;
mod sim_utils;
mod sensei_controller;
mod nigiri_controller;

use sim_node::SimNode;
use sim_event_manager::SimEventManager;
use sim_channel::SimChannel;
use sim_transaction::SimTransaction;
use sim_event::SimulationEvent;
use sim_runtime_graph::RuntimeNetworkGraph;
use sim_utils::get_current_time;
use sim_utils::cleanup_sensei;
use sensei_controller::SenseiController;

// Standard Modules
use std::collections::HashMap;
use std::time::Duration;
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

/*
 *    LnSimulation is the public facing API for users of this library.
 *    A user will define the initial state of the network by adding nodes, channels, events, etc...
 *    After the LnSimulation is defined it will be run using the run() function and the events will take place on the defined network
 */ 
pub struct LnSimulation {
    name: String,
    duration: u64,
    num_sim_nodes: u64,
    user_events: HashMap<u64, Vec<SimulationEvent>>,
    user_nodes: HashMap<String, SimNode>,
    user_channels: Vec<SimChannel>,
    network_graph: RuntimeNetworkGraph
}

impl LnSimulation {
    // Create a new Lightning Network Simulation with the specified number of default nodes (num_sim_nodes) that will run for a specified duration (dur)
    pub fn new(name: String, dur: u64, num_sim_nodes: u64) -> Self {
        let sim = LnSimulation {
            name: name,
            duration: dur,
            num_sim_nodes: num_sim_nodes,
            user_events: HashMap::new(),
            user_nodes: HashMap::new(),
            user_channels: Vec::new(),
            network_graph: RuntimeNetworkGraph::new()
        };

        sim
    }

    // Run the Lightning Network Simulation
    pub fn run(&mut self, nigiri: bool) -> Result<()> {
        println!("[=== LnSimulation === {}] Starting simulation: {} for {} seconds", get_current_time(), self.name, self.duration);

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
        println!("[=== LnSimulation === {}] Configuration:", get_current_time());
        println!("  sensei data: {}", sensei_data_dir);
        println!("  sensei config file: {}", sensei_config_file);
        println!("  sensei database: {}", config.database_url);
        println!("  bitcoind username: {}", config.bitcoind_rpc_username);
        println!("  bitcoind password: {}", config.bitcoind_rpc_password);
        println!("  bitcoin host: {}", config.bitcoind_rpc_host);

        // Setup the sensei database runtime
        let sensei_db_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("sensei_db")
            .enable_all()
            .build()?;
        let sensei_db_runtime_handle = sensei_db_runtime.handle().clone();

        // Setup the sensei main runtime
        let sensei_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("sensei")
            .enable_all()
            .build()?;
        let sensei_runtime_handle = sensei_runtime.handle().clone();

        // Setup the simulation main runtime
        let simulation_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("simulation")
            .enable_all()
            .build()?;

        // Initialize sensei and the simulation
        simulation_runtime.block_on(async move {
            // Start bitcoind with nigiri
            if nigiri {
                println!("[=== LnSimulation === {}] Starting bitcoind with nigiri:", get_current_time());
                nigiri_controller::start();
            }

            // Initialize the sensei database
            println!("[=== LnSimulation === {}] Starting sensei database", get_current_time());
            let mut sensei_db_options = ConnectOptions::new(config.database_url.clone());
            sensei_db_options
                .max_connections(100)
                .min_connections(10)
                .connect_timeout(Duration::new(30, 0));
            let sensei_db_conn = Database::connect(sensei_db_options)
                .await
                .expect("unable to connect to sensei database");
            Migrator::up(&sensei_db_conn, None)
                .await
                .expect("unable to run migrations");
            let sensei_database = SenseiDatabase::new(sensei_db_conn, sensei_db_runtime_handle);

            // Initialize the bitcoin client
            println!("[=== LnSimulation === {}] Initializing the sensei bitcoind client", get_current_time());
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
            println!("[=== LnSimulation === {}] Initializing the sensei chain manager", get_current_time());
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
            println!("[=== LnSimulation === {}] Initializing the sensei admin service", get_current_time());
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
            let mut sensei_controller = SenseiController::new(sensei_admin_service, sensei_runtime_handle);

            // Create the initial state of the network (nodes, channels, balances, etc...)
            /* 
             * TODO: Starting lots of sensei nodes is very slow
             * - need to speed this process up by stripping out processes that are not needed for simulations
             * - funding all of the nodes in the network with some initial liquidity will take some research
             * - how much liquidity will each node need to have?
             * - how do we model a realistic LN liquidity distribution?
             */
            println!("[=== LnSimulation === {}] Initializing simulation network", get_current_time());
            sensei_controller.initialize_network(&self.user_nodes, &self.user_channels, self.num_sim_nodes, nigiri).await;

            // Set up the initial runtime network graph
            println!("[=== LnSimulation === {}] Initializing the runtime network graph", get_current_time());
            self.network_graph.update(&self.user_nodes, &self.user_channels, self.num_sim_nodes);

            // Create the channel for threads to communicate over
            let (sim_event_sender, _): (broadcast::Sender<SimulationEvent>, broadcast::Receiver<SimulationEvent>) = broadcast::channel(1024);
            let sensei_event_receiver = sim_event_sender.subscribe();
            let ln_simulation_receiver = sim_event_sender.subscribe();

            /* 
             * TODO: Start the TransactionGenerator
             * - Clone the sim_event_sender and pass to transaction generator so that it can send events
             * - this thread will generate simulated network traffic
             */
            println!("[=== LnSimulation === {}] Starting the transaction generator", get_current_time());

            /* 
             * TODO: Start the Analyzer
             * - create another event receiver and pass to the analyzer so that it can receive events
             * - this thread will watch the network and gather stats and data about it that will be reported at the end of the sim
            */
            println!("[=== LnSimulation === {}] Starting the network analyzer", get_current_time());

            // Start the SenseiController
            println!("[=== LnSimulation === {}] Starting the sensei controller", get_current_time());
            let sensei_controller_arc = Arc::new(sensei_controller);
            let sensei_controller_handle = thread::spawn(move || {
                sensei_controller_arc.process_events(sensei_event_receiver);
            });

            // Start the EventManager
            /*
             * TODO: Improve the event manager
             * - currently for simplicity in order to create a proof of concept and demonstrate the use case of this project
             *   the duration is denoted in seconds and will run in real time... eventually this will need to be changed to a purely event driven
             *   time concept that will allow the simulations to be run in a faster-than-real-time mode
             * - allow for a use case where the user wants to spin up a large network and connect a user-controlled node to perform manual testing
             */
            println!("[=== LnSimulation === {}] Starting the event manager", get_current_time());
            let event_manager = SimEventManager::new(self.user_events.clone());
            let event_manager_arc = Arc::new(event_manager);
            let d = self.duration;
            let event_manager_handle = thread::spawn(move || {
                event_manager_arc.run(d, sim_event_sender);
            });

            // Let the LnSimulation object wait and listen for events
            println!("[=== LnSimulation === {}] Simulation: {} running", get_current_time(), self.name);
            self.listen(ln_simulation_receiver).await;

            // Wait for all threads to finish and stop the sensei admin service
            match event_manager_handle.join() {
                Ok(()) => println!("[=== LnSimulation === {}] Event manager stopped", get_current_time()),
                Err(_) => println!("event manager could not be stopped...")
            }
            match sensei_controller_handle.join() {
                Ok(()) => println!("[=== LnSimulation === {}] Sensei controller stopped", get_current_time()),
                Err(_) => println!("sensei controller could not be stopped...")
            }

            // Clear the runtime network graph
            self.network_graph.nodes.clear();
            self.network_graph.channels.clear();

            // Stop bitcoind with nigiri
            if nigiri {
                println!("[=== LnSimulation === {}] Stopping nigiri bitcoind", get_current_time());
                nigiri_controller::stop();
                nigiri_controller::stop();
            }
        });

        // Clean up sensei data after the simulation is done
        cleanup_sensei(sensei_data_dir_main);

        Ok(())
    }

    // Get the current network graph for the simulation
    pub fn get_runtime_network_graph(&self) -> String {
        let serialized_nodes = serde_json::to_string(&self.network_graph.nodes).unwrap();
        let serialized_channels = serde_json::to_string(&self.network_graph.channels).unwrap();
        let mut map = Map::new();
        map.insert(String::from("nodes"), serde_json::Value::String(serialized_nodes));
        map.insert(String::from("channels"), serde_json::Value::String(serialized_channels));
        serde_json::to_string(&map).unwrap()
    }

    // Parse a file that contains a definition of a LN topology
    // This definition could be from a project like Polar or from dumping the network information from the mainnet (lncli describegraph)
    // The filename param is the json file of the network topology and import_map is the json file that maps nodes to a profile when importing
    // TODO: Implement
    pub fn import_network(&self, filename: String, import_map: String) {
        println!("[=== LnSimulation === {}] Importing network definition from {}, with import map: {}", get_current_time(), filename, import_map);
    }

    // Export the network to a json file that can be loaded later
    // TODO: Implement
    pub fn export_network(&self, filename: String) {
        println!("[=== LnSimulation === {}] Exporting network definition from {}", get_current_time(), filename);
    }

    // Parse a file that contains transactions
    // This could be from payment information from the mainnet (lncli fwdinghistory)
    // TODO: Implement
    pub fn import_transactions(&self, filename: String) {
        println!("[=== LnSimulation === {}] Importing transactions from {}", get_current_time(), filename);
    }

    // Create a node in the simulated network
    pub fn create_node(&mut self, name: String, initial_balance: u64, running: bool) {
        println!("[=== LnSimulation === {}] Create Node: {}", get_current_time(), name);
        let name_key = name.clone();
        let node = SimNode {
            name: name,
            initial_balance: initial_balance,
            running: running
        };
        self.user_nodes.insert(name_key, node);
    }

    // Create a set of nodes with pre-defined properties
    // TODO: Implement
    pub fn create_node_set(&mut self, number_of_nodes: i32, profile: String) {
        println!("[=== LnSimulation === {}] Create Node Set: {}, {}", get_current_time(), number_of_nodes, profile);
    }

    // Create a channel between two nodes in the simulated network
    pub fn create_channel(&mut self, src: String, dest: String, amount: u64, id: u64) {
        println!("[=== LnSimulation === {}] Create Channel: {} -> {} for {} sats", get_current_time(), src, dest, amount);
        match self.user_nodes.get(&src) {
            Some(n) => {
                if !n.running {
                    println!("channel not created: source node is not running at the start of the simulation");
                    return;
                }

                if n.initial_balance < amount {
                    println!("channel not created: source node does not have enough of an initial balance to open this channel");
                    return;
                }
            },
            None => {
                println!("channel not created: destination node not found with the given name");
                return;
            }
        }
        
        match self.user_nodes.get(&dest) {
            Some(n) => {
                if !n.running {
                    println!("channel not created: destination node is not running at the start of the simulation");
                    return;
                }
            },
            None => {
                println!("channel not created: destination node not found with the given name");
                return;
            }
        }

        if amount < 20000 {
            println!("channel not created: amount must be greater than 20000");
            return;
        }

        let channel = SimChannel {
            src_node: src,
            dest_node: dest,
            src_balance: amount,
            dest_balance: 0,
            id: id
        };
        self.user_channels.push(channel);
    }

    // Create an event that will start up a node in the simulated network
    pub fn create_start_node_event(&mut self, name: String, time: u64) {
        println!("[=== LnSimulation === {}] Add StartNodeEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::StartNodeEvent(name);
        self.add_event(event, time);
    }

    // Create an event that will shut down a node in the simulated network
    pub fn create_stop_node_event(&mut self, name: String, time: u64) {
        println!("[=== LnSimulation === {}] Add StopNodeEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::StopNodeEvent(name);
        self.add_event(event, time);
    }

    // Create an event that will open a new channel between two nodes
    pub fn create_open_channel_event(&mut self, src: String, dest: String, amount: u64, time: u64, id: u64) {
        println!("[=== LnSimulation === {}] Add OpenChannelEvent for: {} at {} seconds", get_current_time(), src, time);
        let event = SimulationEvent::OpenChannelEvent(
            SimChannel{
                src_node: src, 
                dest_node: dest, 
                src_balance: amount, 
                dest_balance: 0,
                id: id
            }
        );
        self.add_event(event, time);
    }

    // Create an event that will close a channel between two nodes
    pub fn create_close_channel_event(&mut self, node_name: String, id: u64, time: u64) {
        println!("[=== LnSimulation === {}] Add CloseChannelEvent for: {} at {} seconds", get_current_time(), id, time);
        let event = SimulationEvent::CloseChannelEvent(node_name, id);
        self.add_event(event, time);
    }

    // Create a transaction for a given amount between two nodes
    pub fn create_transaction_event(&mut self, src: String, dest: String, amount: u64, time: u64) {
        println!("[=== LnSimulation === {}] Add TransactionEvent for: {} at {} seconds", get_current_time(), src, time);
        let event = SimulationEvent::TransactionEvent(
            SimTransaction{
                src_node: src,
                dest_node: dest,
                amount: amount
            }
        );
        self.add_event(event, time);
    }

    // Create an event to tell the sensei controller to print the status of a node (mainly for testing purposes)
    pub fn create_node_status_event(&mut self, node_name: String, time: u64) {
        println!("[=== LnSimulation === {}] Add NodeStatusEvent for: {} at {} seconds", get_current_time(), node_name, time);
        let event = SimulationEvent::NodeStatusEvent(node_name);
        self.add_event(event, time);
    }

    // Add a SimulationEvent to the list of events to execute
    fn add_event(&mut self, event: SimulationEvent, time: u64) {
        if self.user_events.contains_key(&time) {
            let current_events = self.user_events.get_mut(&time).unwrap();
            current_events.push(event);
        } else {
            let mut current_events: Vec<SimulationEvent> = Vec::new();
            current_events.push(event);
            self.user_events.insert(time, current_events);
        }
    }

    // Listens for simulation events and updates the runtime network graph
    async fn listen(&mut self, mut event_channel: broadcast::Receiver<SimulationEvent>) {
        let mut running = true;
        while running {
            let event = event_channel.recv().await.unwrap();
            match event {
                SimulationEvent::StopNodeEvent(name) => {
                    println!("[=== LnSimulation === {}] NodeOfflineEvent, updating network graph", get_current_time());
                    for node in &mut self.network_graph.nodes {
                        if node.name == name {
                            node.running = false;
                            break;
                        }
                    }
                },
                SimulationEvent::StartNodeEvent(name) => {
                    println!("[=== LnSimulation === {}] NodeOnlineEvent, updating network graph", get_current_time());
                    for node in &mut self.network_graph.nodes {
                        if node.name == name {
                            node.running = true;
                            break;
                        }
                    }
                },
                SimulationEvent::CloseChannelEvent(_, id) => {
                    println!("[=== LnSimulation === {}]CloseChannelEvent, updating network graph", get_current_time());
                    self.network_graph.channels.retain(|c| c.id != id);
                },
                SimulationEvent::OpenChannelEvent(channel) => {
                    println!("[=== LnSimulation === {}] OpenChannelEvent, updating network graph", get_current_time());
                    self.network_graph.channels.push(channel);
                },
                SimulationEvent::SimulationEndedEvent => {
                    println!("[=== LnSimulation === {}]SimulationEnded", get_current_time());
                    running = false;
                },
                _ => {
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // Setup the simulation
        let mut ln_sim = LnSimulation::new(String::from("test"), 60, 5);
        
        ln_sim.create_node(String::from("node1"), 60000, true);
        ln_sim.create_node(String::from("node2"), 40000, true);
        ln_sim.create_channel(String::from("node1"), String::from("node2"), 50000, 55);

        ln_sim.create_node_status_event(String::from("node1"), 10);
        ln_sim.create_node_status_event(String::from("node2"), 10);

        /*
         * TODO:
         * We can only send 10% of the total channel balance... WTF??
         * The routing algo always says:
         * ERROR: failed to find route: Failed to find a sufficient route to the given destination
         * The routing path is found from the destination to the source, and the destination always has 10% capacity, why??
         */
        ln_sim.create_transaction_event(String::from("node1"), String::from("node2"), 5000, 13);

        ln_sim.create_node_status_event(String::from("node1"), 20);
        ln_sim.create_node_status_event(String::from("node2"), 20);

        ln_sim.create_close_channel_event(String::from("node1"), 55, 30);

        ln_sim.create_node_status_event(String::from("node1"), 35);
        ln_sim.create_node_status_event(String::from("node2"), 35);

        ln_sim.create_stop_node_event(String::from("node1"), 40);
        ln_sim.create_stop_node_event(String::from("node2"), 50);
        
        // Start the simulation
        let success = ln_sim.run(true);
        assert_eq!(success.is_ok(), true);
    }
}
