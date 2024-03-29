// Project Modules
mod sim_node;
mod sim_transaction;
mod sim_event;
mod sim_event_manager;
mod sim_runtime_graph;
mod sim_utils;
mod sensei_controller;
mod nigiri_controller;
mod network_analyzer;
mod sim_node_status;
mod ln_event_processor;
pub mod sim_results;
pub mod sim_channel;

use sim_node::SimNode;
use sim_event_manager::SimEventManager;
use sim_channel::SimChannel;
use sim_transaction::SimTransaction;
use sim_event::SimulationEvent;
use sim_event::SimEvent;
use sim_event::SimResultsEvent;
use sim_runtime_graph::RuntimeNetworkGraph;
use sim_utils::get_current_time;
use sim_utils::cleanup_sensei;
use sensei_controller::SenseiController;
use network_analyzer::NetworkAnalyzer;
use sim_results::SimResults;
use sim_transaction::SimTransactionStatus;
use ln_event_processor::LnEventProcessor;

// Standard Modules
use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::sync::Mutex;

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
    pub fn new(name: String, duration: u64, num_sim_nodes: u64) -> Self {
        let sim = LnSimulation {
            name: name,
            duration: duration,
            num_sim_nodes: num_sim_nodes,
            user_events: HashMap::new(),
            user_nodes: HashMap::new(),
            user_channels: Vec::new(),
            network_graph: RuntimeNetworkGraph::new()
        };

        sim
    }

    /*
     * Run the Lightning Network Simulation
     */
    pub fn run(&mut self, nigiri: bool) -> Result<SimResults> {
        println!("[=== LnSimulation === {}] Starting simulation: {} for {} seconds", get_current_time(), self.name, self.duration);
        let d = self.duration.clone();

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

        // Setup the network analyzer main runtime
        let analyzer_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("analyzer")
            .enable_all()
            .build()?;
        let analyzer_runtime_handle = analyzer_runtime.handle().clone();

        // Setup the ln event processor main runtime
        let ln_event_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("ln_event")
            .enable_all()
            .build()?;
        let ln_event_runtime_handle = ln_event_runtime.handle().clone();

        // Setup the sensei database runtime
        let sensei_db_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("sensei_db")
            .enable_all()
            .build()?;
        let sensei_db_runtime_handle = sensei_db_runtime.handle().clone();

        // Setup the bitcoind_client runtime
        let bitcoind_client_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("bitcoind_client")
            .enable_all()
            .build()?;
        let bitcoind_client_runtime_handle = bitcoind_client_runtime.handle().clone();

        // Setup the sensei main runtime
        let sensei_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("sensei")
            .enable_all()
            .build()?;
        let sensei_runtime_handle = sensei_runtime.handle().clone();

        // Setup the sensei admin service runtime
        let sensei_admin_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("sensei_admin_service")
            .enable_all()
            .build()?;
        let sensei_admin_runtime_handle = sensei_admin_runtime.handle().clone();

        // Setup the network graph main runtime
        let network_graph_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("network_graph")
            .enable_all()
            .build()?;
        let network_graph_runtime_handle = network_graph_runtime.handle().clone();

        // Setup the simulation main runtime
        let simulation_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("simulation")
            .enable_all()
            .build()?;

        // Initialize sensei and the simulation
        let sim_results = simulation_runtime.block_on(async move {
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
                    bitcoind_client_runtime_handle,
                ) // TODO: this starts a thread that does not always get stopped, handling the error in sensei for now
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
                    bitcoind_client.clone(),
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
                    sensei_admin_runtime_handle,
                    stop_signal.clone(),
                )
                .await,
            );

            // Create the network analyzer
            let mut network_analyzer = NetworkAnalyzer::new(analyzer_runtime_handle, bitcoind_client.clone());

            // Create the ln event processor
            let ln_event_proc = LnEventProcessor::new(ln_event_runtime_handle);

            // Create the sensei controller
            let mut sensei_controller = SenseiController::new(sensei_admin_service, sensei_runtime_handle);

            // Create the event manager
            let event_manager = SimEventManager::new(self.user_events.clone());

            // Create the initial state of the network (nodes, channels, balances, etc...)
            /* 
             * TODO: Starting lots of sensei nodes is very slow
             * - need to speed this process up by stripping out processes that are not needed for simulations
             * - funding all of the nodes in the network with some initial liquidity will take some research
             * - how much liquidity will each node need to have?
             * - how do we model a realistic LN liquidity distribution?
             */
            println!("[=== LnSimulation === {}] Initializing simulation network", get_current_time());
            let sim_receivers = sensei_controller.initialize_network(&self.user_nodes, &self.user_channels, self.num_sim_nodes, nigiri).await;

            // Set up the initial runtime network graph
            println!("[=== LnSimulation === {}] Initializing the runtime network graph", get_current_time());
            self.network_graph.update(&self.user_nodes, &self.user_channels, self.num_sim_nodes);

            // Set up the network analyzer
            println!("[=== LnSimulation === {}] Initializing the network analyzer", get_current_time());
            network_analyzer.initialize_network(&self.network_graph, &sensei_controller).await;

            // Create the channels for threads to communicate over

            // Used for sending simulation events: Receivers will be sensei controller, runtime graph, ln event processor
            let (sim_event_sender, _): (broadcast::Sender<SimEvent>, broadcast::Receiver<SimEvent>) = broadcast::channel(1024);
            let sensei_event_receiver = sim_event_sender.subscribe();
            let ln_simulation_receiver = sim_event_sender.subscribe();
            let ln_event_sim_receiver = sim_event_sender.subscribe();
            
            // Used for sending results to the network analyzer: Receivers will be network analyzer
            let (sim_results_event_sender, _): (broadcast::Sender<SimResultsEvent>, broadcast::Receiver<SimResultsEvent>) = broadcast::channel(1024);
            let ln_results_event_sender = sim_results_event_sender.clone();
            let network_analyzer_receiver = sim_results_event_sender.subscribe();

            thread::scope(|s| {
                // Start the NetworkAnalyzer
                println!("[=== LnSimulation === {}] Starting the network analyzer", get_current_time());
                let network_analyzer_arc = Arc::new(Mutex::new(&mut network_analyzer));
                let network_analyzer_handle = s.spawn(move || {
                    network_analyzer_arc.lock().unwrap().process_events(network_analyzer_receiver);
                });

                // Start the LnEventProcessor
                println!("[=== LnSimulation === {}] Starting the ln event processor", get_current_time());
                let ln_event_proc_arc = Arc::new(ln_event_proc);
                let ln_event_proc_handle = s.spawn(move || {
                    ln_event_proc_arc.process_events(sim_receivers, ln_results_event_sender, ln_event_sim_receiver);
                });

                // Start the SenseiController
                println!("[=== LnSimulation === {}] Starting the sensei controller", get_current_time());
                let sensei_controller_arc = Arc::new(sensei_controller);
                let sensei_controller_handle = s.spawn(move || {
                    sensei_controller_arc.process_events(sensei_event_receiver, sim_results_event_sender);
                });

                // Start the runtime graph
                println!("[=== LnSimulation === {}] Simulation: {} running", get_current_time(), self.name);
                let network_graph_handle = s.spawn( || {
                    self.listen(ln_simulation_receiver, network_graph_runtime_handle);
                });

                // Start the EventManager
                /*
                 * TODO: Improve the event manager
                 * - currently for simplicity in order to create a proof of concept and demonstrate the use case of this project
                 *   the duration is denoted in seconds and will run in real time... eventually this will need to be changed to a purely event driven
                 *   time concept that will allow the simulations to be run in a faster-than-real-time mode
                 * - allow for a use case where the user wants to spin up a large network and connect a user-controlled node to perform manual testing
                 * - mining, on-chain transactions, block updates all need to be considered, we do not want to wait 10 min for a block to be added
                 *   need to model an event driven on-chain process that will mine blocks at faster than real time.
                 */
                println!("[=== LnSimulation === {}] Starting the event manager", get_current_time());
                let event_manager_arc = Arc::new(event_manager);
                let event_manager_handle = s.spawn(move || {
                    event_manager_arc.run(d, sim_event_sender);
                });

                // Wait for all threads to finish
                match network_analyzer_handle.join() {
                    Ok(()) => println!("[=== LnSimulation === {}] NetworkAnalyzer stopped", get_current_time()),
                    Err(_) => println!("network analyzer could not be stopped...")
                }
                match ln_event_proc_handle.join() {
                    Ok(()) => println!("[=== LnSimulation === {}] LnEventProcessor stopped", get_current_time()),
                    Err(_) => println!("ln event processor could not be stopped...")
                }
                match sensei_controller_handle.join() {
                    Ok(()) => println!("[=== LnSimulation === {}] SenseiController stopped", get_current_time()),
                    Err(_) => println!("sensei controller could not be stopped...")
                }
                match network_graph_handle.join() {
                    Ok(()) => println!("[=== LnSimulation === {}] NetworkGraph stopped", get_current_time()),
                    Err(_) => println!("network graph could not be stopped...")
                }
                match event_manager_handle.join() {
                    Ok(()) => println!("[=== LnSimulation === {}] EventManager stopped", get_current_time()),
                    Err(_) => println!("event manager could not be stopped...")
                }
            });

            // Get the results of the simulation
            let results = network_analyzer.get_sim_results();

            // Clear the runtime network graph
            self.network_graph.nodes.clear();
            self.network_graph.channels.clear();

            // Stop bitcoind with nigiri
            if nigiri {
                println!("[=== LnSimulation === {}] Stopping nigiri bitcoind", get_current_time());
                nigiri_controller::stop();
                nigiri_controller::stop();
            }

            results
        });

        // Clean up sensei data after the simulation is done
        cleanup_sensei(sensei_data_dir_main);

        Ok(sim_results)
    }

    /*
     * Get the current network graph for the simulation
     */
    pub fn get_runtime_network_graph(&self) -> String {
        let serialized_nodes = serde_json::to_string(&self.network_graph.nodes).unwrap();
        let serialized_channels = serde_json::to_string(&self.network_graph.channels).unwrap();
        let mut map = Map::new();
        map.insert(String::from("nodes"), serde_json::Value::String(serialized_nodes));
        map.insert(String::from("channels"), serde_json::Value::String(serialized_channels));
        serde_json::to_string(&map).unwrap()
    }

    /*
     * Parse a file that contains a definition of a LN topology
     * This definition could be from a project like Polar or from dumping the network information from the mainnet (lncli describegraph)
     * The filename param is the json file of the network topology and import_map is the json file that maps nodes to a profile when importing
     * TODO: Implement
     */
    pub fn import_network(&self, filename: String, import_map: String) {
        println!("[=== LnSimulation === {}] Importing network definition from {}, with import map: {}", get_current_time(), filename, import_map);
    }

    /*
     * Export the network to a json file that can be loaded later
     * TODO: Implement
     */
    pub fn export_network(&self, filename: String) {
        println!("[=== LnSimulation === {}] Exporting network definition from {}", get_current_time(), filename);
    }

    /*
     * Parse a file that contains transactions
     * This could be from payment information from the mainnet (lncli fwdinghistory)
     * TODO: Implement
     */
    pub fn import_transactions(&self, filename: String) {
        println!("[=== LnSimulation === {}] Importing transactions from {}", get_current_time(), filename);
    }

    /* 
     * Create a node in the simulated network
     */
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

    /*
     * Create a set of nodes with pre-defined properties
     * TODO: Implement
     */
    pub fn create_node_set(&mut self, number_of_nodes: i32, profile: String) {
        println!("[=== LnSimulation === {}] Create Node Set: {}, {}", get_current_time(), number_of_nodes, profile);
    }

    /*
     * Create a channel between two nodes in the simulated network
     */
    pub fn create_channel(&mut self, src: String, dest: String, amount_sats: u64, id: u64) {
        println!("[=== LnSimulation === {}] Create Channel: {} -> {} for {} sats", get_current_time(), src, dest, amount_sats);
        match self.user_nodes.get(&src) {
            Some(n) => {
                if !n.running {
                    println!("channel not created: source node is not running at the start of the simulation");
                    return;
                }

                if n.initial_balance < amount_sats {
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

        if amount_sats < 20000 {
            println!("channel not created: amount must be greater than 20000");
            return;
        }

        let channel = SimChannel {
            src_node: src,
            dest_node: dest,
            src_balance_sats: amount_sats,
            dest_balance_sats: 0,
            id: id,
            short_id: None,
            run_time_id: None,
            funding_tx: None,
            penalty_reserve_sats: None
        };
        self.user_channels.push(channel.clone());

    }

    /*
     * Create an event that will start up a node in the simulated network
     */
    pub fn create_start_node_event(&mut self, name: String, time: u64) {
        println!("[=== LnSimulation === {}] Add StartNodeEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::StartNodeEvent(name);
        self.add_event(event, time);
    }

    /*
     * Create an event that will shut down a node in the simulated network
     */
    pub fn create_stop_node_event(&mut self, name: String, time: u64) {
        println!("[=== LnSimulation === {}] Add StopNodeEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::StopNodeEvent(name);
        self.add_event(event, time);
    }

    /*
     * Create an event that will open a new channel between two nodes
     */
    pub fn create_open_channel_event(&mut self, src: String, dest: String, amount_sats: u64, time: u64, id: u64) {
        println!("[=== LnSimulation === {}] Add OpenChannelEvent for: {} at {} seconds", get_current_time(), src, time);
        let channel = SimChannel {
            src_node: src, 
            dest_node: dest, 
            src_balance_sats: amount_sats,
            dest_balance_sats: 0,
            id: id,
            short_id: None,
            run_time_id: None,
            funding_tx: None,
            penalty_reserve_sats: None
        };
        let event = SimulationEvent::OpenChannelEvent(channel.clone());
        self.add_event(event, time);
    }

    /*
     * Create an event that will close a channel between two nodes
     */
    pub fn create_close_channel_event(&mut self, node: String, channel_id: u64, time: u64) {
        println!("[=== LnSimulation === {}] Add CloseChannelEvent for: {} at {} seconds", get_current_time(), channel_id, time);
        let event = SimulationEvent::CloseChannelEvent(node, channel_id);
        self.add_event(event, time);
    }

    /*
     * Create a transaction for a given amount between two nodes
     */
    pub fn create_transaction_event(&mut self, src: String, dest: String, amount_sats: u64, time: u64) {
        println!("[=== LnSimulation === {}] Add TransactionEvent for: {} at {} seconds", get_current_time(), src, time);
        let event = SimulationEvent::TransactionEvent(
            SimTransaction {
                id: None,
                src_node: src,
                dest_node: dest,
                amount_sats: amount_sats,
                status: SimTransactionStatus::NONE
            }
        );
        self.add_event(event, time);
    }

    /*
     * Add a SimulationEvent to the list of events to execute
     */
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

    /*
     * Listens for simulation events and updates the runtime network graph
     */
    fn listen(&mut self, mut event_channel: broadcast::Receiver<SimEvent>, runtime_handle: tokio::runtime::Handle) {
        tokio::task::block_in_place(move || {
            runtime_handle.block_on(async move {
                let mut running = true;
                while running {
                    let event = event_channel.recv().await.unwrap();
                    match event.event {
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
                            println!("[=== LnSimulation === {}] CloseChannelEvent, updating network graph", get_current_time());
                            self.network_graph.channels.retain(|c| c.id != id);
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("[=== LnSimulation === {}] OpenChannelEvent, updating network graph", get_current_time());
                            self.network_graph.channels.push(channel);
                        },
                        SimulationEvent::SimulationEndedEvent => {
                            println!("[=== LnSimulation === {}] SimulationEnded", get_current_time());
                            running = false;
                        },
                        _ => {
                            // Ignore all other events
                        }
                    }
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use super::*;

    #[test]
    #[serial]
    fn status_test() {
        let mut ln_sim = LnSimulation::new(String::from("test"), 10, 0);
        
        ln_sim.create_node(String::from("node1"), 0, true);

        ln_sim.create_stop_node_event(String::from("node1"), 2);
        ln_sim.create_start_node_event(String::from("node1"), 6);
        ln_sim.create_stop_node_event(String::from("node1"), 9);

        let sim_results = ln_sim.run(true);
        match sim_results {
            Ok(res) => {
                let node1 = String::from("node1");

                let mut status1 = res.get_node_status(0, &node1);
                assert_eq!(true, status1);

                status1 = res.get_node_status(4, &node1);
                assert_eq!(false, status1);

                status1 = res.get_node_status(7, &node1);
                assert_eq!(true, status1);

                status1 = res.get_node_status(9, &node1);
                assert_eq!(false, status1);
            },
            Err(e) => {
                println!("Test failed due to error: {:?}", e);
                assert_eq!(true, false);
            }
        }
    }

    #[test]
    #[serial]
    fn channel_test() {
        let mut ln_sim = LnSimulation::new(String::from("test"), 10, 0);
        
        ln_sim.create_node(String::from("node1"), 200000, true);
        ln_sim.create_node(String::from("node2"), 0, true);

        ln_sim.create_channel(String::from("node1"), String::from("node2"), 40000, 1);
        ln_sim.create_close_channel_event(String::from("node1"), 1, 5);

        let sim_results = ln_sim.run(true);
        match sim_results {
            Ok(res) => {
                let num_chan_4 = res.get_open_channels(4).unwrap().len();
                assert_eq!(num_chan_4, 1);

                let num_chan_7 = res.get_open_channels(7).unwrap().len();
                assert_eq!(num_chan_7, 0);

                let num_chan_4c = res.get_closed_channels(4);
                assert_eq!(true, num_chan_4c.is_none());

                let num_chan_7c = res.get_closed_channels(7).unwrap().len();
                assert_eq!(num_chan_7c, 1);

                for c in res.get_open_channels(3).unwrap() {
                    assert_eq!(c.src_balance_sats, 39000); // channel balance (40000) - penalty reserve (1000)
                    assert_eq!(c.dest_balance_sats, 0);
                }
            },
            Err(e) => {
                println!("Test failed due to error: {:?}", e);
                assert_eq!(true, false);
            }
        }
    }

    #[test]
    #[serial]
    fn another_channel_test() {
        let mut ln_sim = LnSimulation::new(String::from("test"), 10, 0);
        
        ln_sim.create_node(String::from("node1"), 200000, true);
        ln_sim.create_node(String::from("node2"), 0, true);

        ln_sim.create_open_channel_event(String::from("node1"), String::from("node2"), 40000, 2, 1);
        ln_sim.create_close_channel_event(String::from("node1"), 1, 5);

        let sim_results = ln_sim.run(true);
        match sim_results {
            Ok(res) => {
                let num_chan_4 = res.get_open_channels(4).unwrap().len();
                assert_eq!(num_chan_4, 1);

                let num_chan_7 = res.get_open_channels(7).unwrap().len();
                assert_eq!(num_chan_7, 0);

                let num_chan_4c = res.get_closed_channels(4);
                assert_eq!(true, num_chan_4c.is_none());

                let num_chan_7c = res.get_closed_channels(7).unwrap().len();
                assert_eq!(num_chan_7c, 1);

                for c in res.get_open_channels(3).unwrap() {
                    assert_eq!(c.src_balance_sats, 39000); // channel balance (40000) - penalty reserve (1000)
                    assert_eq!(c.dest_balance_sats, 0);
                }
            },
            Err(e) => {
                println!("Test failed due to error: {:?}", e);
                assert_eq!(true, false);
            }
        }
    }

    /*
     * TODO:    
     * The current configuration of sensei uses the default value for max_inbound_htlc_value_in_flight_percent_of_channel when initiating a new channel or opening a channel from a request
     * The default max_inbound_htlc_value_in_flight_percent_of_channel is 10 meaning that each channel we open can only be used to send 10% of the total channel balance.
     * This can be changed in sensei to allow for larger percentages, and eventually this library will need to allow this value to be configured.
     * For now, we will use the default. So each channel opened can only send 10% of its full capacity.
     */
    #[test]
    #[serial]
    fn direct_payment() {
        let mut ln_sim = LnSimulation::new(String::from("test"), 10, 0);
        
        ln_sim.create_node(String::from("node1"), 200000, true);
        ln_sim.create_node(String::from("node2"), 0, true);

        ln_sim.create_channel(String::from("node1"), String::from("node2"), 40000, 1);
        ln_sim.create_close_channel_event(String::from("node1"), 1, 8);

        ln_sim.create_transaction_event(String::from("node1"), String::from("node2"), 3000, 4);

        ln_sim.create_stop_node_event(String::from("node1"), 9);
        ln_sim.create_stop_node_event(String::from("node2"), 9);

        // Start the simulation
        let sim_results = ln_sim.run(true);
        match sim_results {
            Ok(res) => {
                // check results
                let time_start = 0;
                let time_after_payment = 6;
                let time_after_stop = 10;
                let node1 = String::from("node1");
                let node2 = String::from("node2");

                // balances at start of sim
                let bal1_on = res.get_on_chain_bal(time_start, &node1).unwrap();
                let bal1_off = res.get_off_chain_bal(time_start, &node1).unwrap();
                assert_eq!(bal1_on, 158776); // initial balance (200000) - the amount of the channel (40000) - fees (1224)
                assert_eq!(bal1_off, 40000); // amount of the channel

                let bal2_on = res.get_on_chain_bal(time_start, &node2).unwrap();
                let bal2_off = res.get_off_chain_bal(time_start, &node2).unwrap();
                assert_eq!(bal2_on, 0);
                assert_eq!(bal2_off, 0);

                // balances after payment
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node1).unwrap(), 37000); // initial channel balance (40000) - payment (3000)
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node2).unwrap(), 3000); // initial channel balance (0) + payment (3000)

                // channel balances after payment
                let mut src_balance = 0;
                let mut dest_balance = 0;
                for c in res.get_open_channels(time_after_payment).unwrap() {
                    if c.id == 1 {
                        src_balance = c.src_balance_sats;
                        dest_balance = c.dest_balance_sats;
                        break;
                    }
                }
                assert_eq!(src_balance, 36000); // initial channel balance (40000) - unspendable punishment (1000) - payment (3000)
                assert_eq!(dest_balance, 3000); // initial channel balance (0) + payment (3000)

                // balances after payment and closing the channel
                assert_eq!(res.get_off_chain_bal(time_after_stop, &node1).unwrap(), 0);
                assert_eq!(res.get_off_chain_bal(time_after_stop, &node2).unwrap(), 0);
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node1).unwrap(), 193428); // balance after opening channel (158776) + remaining channel balance (37000) - closing channel on-chain fee (2348)
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node2).unwrap(), 3000); // payment

                // number of transactions in the sim
                assert_eq!(res.get_all_transactions().unwrap().len(), 1);
            },
            Err(e) => {
                // fail the test
                println!("Test failed due to error: {:?}", e);
                assert_eq!(true, false);
            }
        }
    }

    #[test]
    #[serial]
    fn another_direct_payment() {
        let mut ln_sim = LnSimulation::new(String::from("test"), 10, 0);
        
        ln_sim.create_node(String::from("node1"), 200000, true);
        ln_sim.create_node(String::from("node2"), 0, true);

        ln_sim.create_open_channel_event(String::from("node1"), String::from("node2"), 40000, 2, 1);
        ln_sim.create_transaction_event(String::from("node1"), String::from("node2"), 3000, 4);
        ln_sim.create_close_channel_event(String::from("node1"), 1, 6);
        ln_sim.create_stop_node_event(String::from("node1"), 8);
        ln_sim.create_stop_node_event(String::from("node2"), 8);

        // Start the simulation
        let sim_results = ln_sim.run(true);
        match sim_results {
            Ok(res) => {
                // check results
                let time_start = 0;
                let time_open_channel = 3;
                let time_after_payment = 5;
                let time_after_stop = 9;
                let node1 = String::from("node1");
                let node2 = String::from("node2");

                // balances at start of sim
                let bal1_on = res.get_on_chain_bal(time_start, &node1).unwrap();
                let bal1_off = res.get_off_chain_bal(time_start, &node1).unwrap();
                assert_eq!(bal1_on, 200000); // initial balance (200000)
                assert_eq!(bal1_off, 0); // amount of the channel

                let bal2_on = res.get_on_chain_bal(time_start, &node2).unwrap();
                let bal2_off = res.get_off_chain_bal(time_start, &node2).unwrap();
                assert_eq!(bal2_on, 0);
                assert_eq!(bal2_off, 0);
                
                // number open channels at start of sim
                let num_chan = res.get_open_channels(time_start).unwrap().len();
                assert_eq!(num_chan, 0);

                // balances after opening channel
                let bal1_on1 = res.get_on_chain_bal(time_open_channel, &node1).unwrap();
                let bal1_off1 = res.get_off_chain_bal(time_open_channel, &node1).unwrap();
                assert_eq!(bal1_on1, 158776); // initial balance (200000) - the amount of the channel (40000) - fees (1224)
                assert_eq!(bal1_off1, 40000); // amount of the channel

                // balances after payment
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node1).unwrap(), 37000); // initial channel balance (40000) - payment (3000)
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node2).unwrap(), 3000); // initial channel balance (0) + payment (3000)

                // channel balances after payment
                let mut src_balance = 0;
                let mut dest_balance = 0;
                for c in res.get_open_channels(time_after_payment).unwrap() {
                    if c.id == 1 {
                        src_balance = c.src_balance_sats;
                        dest_balance = c.dest_balance_sats;
                        break;
                    }
                }
                assert_eq!(src_balance, 36000); // initial channel balance (40000) - unspendable punishment (1000) - payment (3000)
                assert_eq!(dest_balance, 3000); // initial channel balance (0) + payment (3000)

                // balances after payment and closing the channel
                assert_eq!(res.get_off_chain_bal(time_after_stop, &node1).unwrap(), 0);
                assert_eq!(res.get_off_chain_bal(time_after_stop, &node2).unwrap(), 0);
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node1).unwrap(), 193428); // balance after opening channel (158776) + remaining channel balance (37000) - closing channel on-chain fee (2348)
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node2).unwrap(), 3000); // payment

                // number open channels at end of sim
                assert_eq!(res.get_open_channels(time_after_stop).unwrap().len(), 0);

                // number of transactions in the sim
                assert_eq!(res.get_all_transactions().unwrap().len(), 1);
            },
            Err(e) => {
                // fail the test
                println!("Test failed due to error: {:?}", e);
                assert_eq!(true, false);
            }
        }
    }

    #[test]
    #[serial]
    fn forward_payment() {
        let mut ln_sim = LnSimulation::new(String::from("test"), 10, 5);
        
        ln_sim.create_node(String::from("node1"), 200000, true);
        ln_sim.create_node(String::from("node2"), 0, true);
        ln_sim.create_node(String::from("node3"), 200000, true);

        ln_sim.create_channel(String::from("node1"), String::from("node3"), 40000, 1);
        ln_sim.create_close_channel_event(String::from("node1"), 1, 8);

        ln_sim.create_channel(String::from("node3"), String::from("node2"), 40000, 2);
        ln_sim.create_close_channel_event(String::from("node3"), 2, 8);

        ln_sim.create_transaction_event(String::from("node1"), String::from("node2"), 3000, 4);

        ln_sim.create_stop_node_event(String::from("node1"), 9);
        ln_sim.create_stop_node_event(String::from("node2"), 9);
        ln_sim.create_stop_node_event(String::from("node3"), 9);

        // Start the simulation
        let sim_results = ln_sim.run(true);
        match sim_results {
            Ok(res) => {
                // check results
                let time_start = 0;
                let time_after_payment = 6;
                let time_after_stop = 10;
                let node1 = String::from("node1");
                let node2 = String::from("node2");
                let node3 = String::from("node3");

                // balances at start of sim
                let bal1_on = res.get_on_chain_bal(time_start, &node1).unwrap();
                let bal1_off = res.get_off_chain_bal(time_start, &node1).unwrap();
                assert_eq!(bal1_on, 158776); // initial balance (200000) - the amount of the channel (40000) - fees (1224)
                assert_eq!(bal1_off, 40000);

                let bal2_on = res.get_on_chain_bal(time_start, &node2).unwrap();
                let bal2_off = res.get_off_chain_bal(time_start, &node2).unwrap();
                assert_eq!(bal2_on, 0);
                assert_eq!(bal2_off, 0);
                
                let bal3_on = res.get_on_chain_bal(time_start, &node3).unwrap();
                let bal3_off = res.get_off_chain_bal(time_start, &node3).unwrap();
                assert_eq!(bal3_on, 158776); // initial balance (200000) - the amount of the channel (40000) - fees (1224)
                assert_eq!(bal3_off, 40000);

                // number open channels at start of sim
                let num_chan = res.get_open_channels(time_start).unwrap().len();
                assert_eq!(num_chan, 2);

                // balances after payment
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node1).unwrap(), 36999); // 3000 sent for the payment and 1 sat for a fee
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node2).unwrap(), 3000); // receives 3000 sat payment
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node3).unwrap(), 40001); // gains 1 sat for forwarding the payment on behalf of node1

                // channel balances after payment
                let mut src_balance1 = 0;
                let mut dest_balance1 = 0;
                let mut src_balance2 = 0;
                let mut dest_balance2 = 0;
                for c in res.get_open_channels(time_after_payment).unwrap() {
                    if c.id == 1 {
                        src_balance1 = c.src_balance_sats;
                        dest_balance1 = c.dest_balance_sats;
                    } else if c.id == 2 {
                        src_balance2 = c.src_balance_sats;
                        dest_balance2 = c.dest_balance_sats;
                    }
                }
                assert_eq!(src_balance1, 35999); // channel balance (40000) - unspendable punishment (1000) - payment (3000) - fee (1)
                assert_eq!(dest_balance1, 3001); // payment (3000) + fee (1)

                assert_eq!(src_balance2, 36000); // channel balance (40000) - unspendable punishment (1000) - payment (3000)
                assert_eq!(dest_balance2, 3000); // payment (3000)

                // on chain balances after payment and closing the channel
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node1).unwrap(), 193427); // previous balance (158776) + remaining channel balance (35999) + punishment (1000) - closing channel on-chain fee (2348)
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node2).unwrap(), 3000); // payment (3000)
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node3).unwrap(), 196429); // previous balance (158776) + remaining channel balance (3001) + remaining channel balance (36000) + punishment (1000) - closing channel on-chain fee (2348)

                // number open channels at end of sim
                assert_eq!(res.get_open_channels(time_after_stop).unwrap().len(), 0);

                // number of transactions in the sim
                assert_eq!(res.get_all_transactions().unwrap().len(), 1);
            },
            Err(e) => {
                // fail the test
                println!("Test failed due to error: {:?}", e);
                assert_eq!(true, false);
            }
        }
    }

    #[test]
    #[serial]
    fn multiple_path_payment() {
        let mut ln_sim = LnSimulation::new(String::from("test"), 30, 5);
        
        ln_sim.create_node(String::from("node1"), 200000, true);
        ln_sim.create_node(String::from("node2"), 0, true);
        ln_sim.create_node(String::from("node3"), 200000, true);
        ln_sim.create_node(String::from("node4"), 200000, true);

        ln_sim.create_channel(String::from("node1"), String::from("node2"), 40000, 1);
        ln_sim.create_close_channel_event(String::from("node1"), 1, 20);

        ln_sim.create_channel(String::from("node1"), String::from("node3"), 40000, 2);
        ln_sim.create_close_channel_event(String::from("node1"), 2, 20);

        ln_sim.create_channel(String::from("node1"), String::from("node4"), 40000, 3);
        ln_sim.create_close_channel_event(String::from("node1"), 3, 20);

        ln_sim.create_channel(String::from("node4"), String::from("node2"), 40000, 4);
        ln_sim.create_close_channel_event(String::from("node4"), 4, 20);

        ln_sim.create_channel(String::from("node3"), String::from("node2"), 40000, 5);
        ln_sim.create_close_channel_event(String::from("node3"), 5, 20);

        ln_sim.create_transaction_event(String::from("node1"), String::from("node2"), 9000, 10);

        ln_sim.create_stop_node_event(String::from("node1"), 25);
        ln_sim.create_stop_node_event(String::from("node2"), 25);
        ln_sim.create_stop_node_event(String::from("node3"), 25);
        ln_sim.create_stop_node_event(String::from("node4"), 25);

        // Start the simulation
        let sim_results = ln_sim.run(true);
        match sim_results {
            Ok(res) => {
                // check results
                let time_start = 0;
                let time_after_payment = 15;
                let time_after_stop = 28;
                let node1 = String::from("node1");
                let node2 = String::from("node2");
                let node3 = String::from("node3");
                let node4 = String::from("node4");

                // balances at start of sim
                let bal1_on = res.get_on_chain_bal(time_start, &node1).unwrap();
                let bal1_off = res.get_off_chain_bal(time_start, &node1).unwrap();
                assert!(bal1_on < 77000); // 200000 - 3 channels (40000x3=120000) - 3 open fees
                assert_eq!(bal1_off, 120000);

                let bal2_on = res.get_on_chain_bal(time_start, &node2).unwrap();
                let bal2_off = res.get_off_chain_bal(time_start, &node2).unwrap();
                assert_eq!(bal2_on, 0);
                assert_eq!(bal2_off, 0);
                
                let bal3_on = res.get_on_chain_bal(time_start, &node3).unwrap();
                let bal3_off = res.get_off_chain_bal(time_start, &node3).unwrap();
                assert!(bal3_on < 160000); // 200000 - 1 channel (40000)
                assert_eq!(bal3_off, 40000);
                
                let bal4_on = res.get_on_chain_bal(time_start, &node4).unwrap();
                let bal4_off = res.get_off_chain_bal(time_start, &node4).unwrap();
                assert!(bal4_on < 160000); // 200000 - 1 channel (40000)
                assert_eq!(bal4_off, 40000);
                
                // number open channels at start of sim
                let num_chan = res.get_open_channels(time_start).unwrap().len();
                assert_eq!(num_chan, 5);

                // balances after payment
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node1).unwrap(), 110998); // 120000 - payment (9000) - fees (2)
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node2).unwrap(), 9000); // payment
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node3).unwrap(), 40001); // balance + fee (1)
                assert_eq!(res.get_off_chain_bal(time_after_payment, &node4).unwrap(), 40001); // balance + fee (1)

                // channel balances after payment
                let mut src_balance1 = 0;
                let mut dest_balance1 = 0;
                let mut src_balance2 = 0;
                let mut dest_balance2 = 0;
                let mut src_balance3 = 0;
                let mut dest_balance3 = 0;
                let mut src_balance4 = 0;
                let mut dest_balance4 = 0;
                let mut src_balance5 = 0;
                let mut dest_balance5 = 0;
                for c in res.get_open_channels(time_after_payment).unwrap() {
                    if c.id == 1 {
                        src_balance1 = c.src_balance_sats;
                        dest_balance1 = c.dest_balance_sats;
                    }
                    if c.id == 2 {
                        src_balance2 = c.src_balance_sats;
                        dest_balance2 = c.dest_balance_sats;
                    }
                    if c.id == 3 {
                        src_balance3 = c.src_balance_sats;
                        dest_balance3 = c.dest_balance_sats;
                    }
                    if c.id == 4 {
                        src_balance4 = c.src_balance_sats;
                        dest_balance4 = c.dest_balance_sats;
                    }
                    if c.id == 5 {
                        src_balance5 = c.src_balance_sats;
                        dest_balance5 = c.dest_balance_sats;
                    }
                }
                assert_eq!(src_balance1, 39000 - 4000);
                assert_eq!(dest_balance1, 4000);
                assert!(src_balance2 == (39000 - 2003) || src_balance2 == (39000 - 2999));
                assert!(dest_balance2 == 2003 || dest_balance2 == 2999);
                assert!(src_balance3 == (39000 - 2999) || src_balance3 == (39000 - 2003));
                assert!(dest_balance3 == 2999 || dest_balance3 == 2003);
                assert!(src_balance4 == (39000 - 2998) || src_balance4 == (39000 - 2002));
                assert!(dest_balance4 == 2998 || dest_balance4 == 2002);
                assert!(src_balance5 == (39000 - 2002) || src_balance5 == (39000 - 2998));
                assert!(dest_balance5 == 2002 || dest_balance5 == 2998);

                // on chain balances after payment and closing the channel
                assert!(res.get_on_chain_bal(time_after_stop, &node1).unwrap() < 185000); // on chain balance + off chain balance (110998) - 3 closing fees
                assert_eq!(res.get_on_chain_bal(time_after_stop, &node2).unwrap(), 9000); // payment
                assert!(res.get_on_chain_bal(time_after_stop, &node3).unwrap()< 200000); // on chain balance + off chain balance (40001) - closing fee (2348)
                assert!(res.get_on_chain_bal(time_after_stop, &node4).unwrap()< 200000); // on chain balance + off chain balance (40001) - closing fee (2348)

                // status of each node at the end of the sim
                assert_eq!(res.get_node_status(time_after_stop, &node1), false);
                assert_eq!(res.get_node_status(time_after_stop, &node2), false);
                assert_eq!(res.get_node_status(time_after_stop, &node3), false);
                assert_eq!(res.get_node_status(time_after_stop, &node4), false);

                // number open channels at end of sim
                assert_eq!(res.get_open_channels(time_after_stop).unwrap().len(), 0);

                // number of transactions in the sim
                assert_eq!(res.get_all_transactions().unwrap().len(), 1);
            },
            Err(e) => {
                // fail the test
                println!("Test failed due to error: {:?}", e);
                assert_eq!(true, false);
            }
        }
    }
}
