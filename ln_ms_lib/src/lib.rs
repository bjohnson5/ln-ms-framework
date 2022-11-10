mod ln_node;
mod ln_channel;
mod ln_event;
mod ln_event_manager;
mod ln_sensei_controller;

// Project Modules
use ln_node::LnNode;
use ln_event_manager::LnEventManager;
use ln_channel::LnChannel;
use ln_event::SimulationEvent;
use ln_sensei_controller::LnSenseiController;

// Standard Modules
use std::collections::HashMap;
use std::time::Duration;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool};
use std::process::Command;
use std::thread;
use std::sync::mpsc;

// External Modules
extern crate chrono;
use chrono::Local;
use anyhow::Result;
use tokio::runtime::Builder;
use tokio::sync::broadcast;
use sea_orm::{Database, ConnectOptions};

// Sensei Modules
use migration::{Migrator, MigratorTrait};
use senseicore::{
    chain::{bitcoind_client::BitcoindClient, manager::SenseiChainManager},
    config::SenseiConfig,
    database::SenseiDatabase,
    events::SenseiEvent,
    services::admin::{AdminRequest, AdminResponse, AdminService},
};

// Represents a simuation of a lightning network that runs for a specified duration
// This struct will be the public facing API for users of this library
// A user will define the initial state of the network by adding nodes, channels, events, etc...
// After the LnSimulation is defined it will be run and the events will take place

// NOTE: This project is currently setup as a Proof of Concept and does not connect or simulate any LN operations

pub struct LnSimulation {
    name: String,
    duration: u64,
    em: LnEventManager,
    nodes: HashMap<String, LnNode>,
    channels: Vec<LnChannel>,
}

impl LnSimulation {
    pub fn new(name: String, dur: u64) -> Self {
        let sim = LnSimulation {
            name: name,
            duration: dur, 
            em: LnEventManager::new(),
            nodes: HashMap::new(),
            channels: Vec::new(),
        };

        sim
    }

    pub fn run(&self) -> Result<()> {
        println!("LnSimulation:{} -- running simulation: {} ({} seconds)", get_current_time(), self.name, self.duration);

        // Setup the sensei config
        // TODO: fix the paths, this is ugly
        let this_file = file!();
        let sensei_data_dir = this_file.replace("lib.rs", "sensei_data_dir");
        let sensei_config_file = sensei_data_dir.clone() + "/config.json";
        let mut config = SenseiConfig::from_file(sensei_config_file, None);
        let sqlite_path = sensei_data_dir.clone() + "/" + config.database_url.as_str();
        config.database_url = format!("sqlite://{}?mode=rwc", sqlite_path);
        let stop_signal = Arc::new(AtomicBool::new(false));

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

        // Setup the main runtime
        let simulation_runtime = Builder::new_multi_thread()
            .worker_threads(20)
            .thread_name("simulation")
            .enable_all()
            .build()?;

        // Initialize sensei and the simulation
        simulation_runtime.block_on(async move {
            // Start bitcoin client with nigiri
            println!("starting bitcoin with nigiri...");
            Command::new("sh")
            .arg("-c")
            .arg("nigiri start")
            .output()
            .expect("failed to execute process");

            // Initialize the database
            println!("starting sensei database...");
            let mut db_options = ConnectOptions::new(config.database_url.clone());
            db_options
                .max_connections(100)
                .min_connections(10)
                .connect_timeout(Duration::new(30, 0));
            let db = Database::connect(db_options).await.unwrap();
            Migrator::up(&db, None)
                .await
                .expect("failed to run migrations");
            let database = SenseiDatabase::new(db, sensei_db_runtime_handle.clone());
            database.mark_all_nodes_stopped().await.unwrap();

            // Initialize the bitcoin client
            println!("connecting to bitcoin client...");
            let bitcoind_client = Arc::new(
                BitcoindClient::new(
                    config.bitcoind_rpc_host.clone(),
                    config.bitcoind_rpc_port,
                    config.bitcoind_rpc_username.clone(),
                    config.bitcoind_rpc_password.clone(),
                    tokio::runtime::Handle::current(),
                )
                .await
                .expect("invalid bitcoind rpc config"),
            );

            // Initialize the chain manager
            println!("initializing the sensei chain manager...");
            let chain_manager = Arc::new(
                SenseiChainManager::new(
                    config.clone(),
                    bitcoind_client.clone(),
                    bitcoind_client.clone(),
                    bitcoind_client,
                )
                .await
                .unwrap(),
            );

            // Initialize the admin service
            println!("initializing the sensei admin service...");
            let (event_sender, _event_receiver): (
                broadcast::Sender<SenseiEvent>,
                broadcast::Receiver<SenseiEvent>,
            ) = broadcast::channel(1024);

            let sas = Arc::new(
                AdminService::new(
                    &sensei_data_dir,
                    config.clone(),
                    database,
                    chain_manager,
                    event_sender,
                    tokio::runtime::Handle::current(),
                    stop_signal.clone(),
                )
                .await,
            );

            // Create the initial state of the network (nodes, channels, balances, etc...)
            println!("create nodes...");
            for n in &self.nodes {
                let create_node_req = AdminRequest::CreateNode { 
                    username: String::from(n.0), 
                    alias: String::from(n.0), 
                    passphrase: String::from(n.0), 
                    start: false,
                    entropy: None,
                    cross_node_entropy: None,
                };
                match sas.call(create_node_req).await {
                    Ok(response) => match response {
                        AdminResponse::CreateNode {        
                            pubkey,
                            macaroon: _,
                            listen_addr: _,
                            listen_port: _,
                            id: _,
                            entropy: _,
                            cross_node_entropy: _
                        } => println!("node created successfully with pubkey: {}", pubkey),
                        _ => println!("")
                    },
                    Err(_) => println!("node failed to be created...")
                }
            }

             // TODO: fund the lightning nodes with initial balances
            println!("funding initial balances...");
           
            // TODO: open all initial channels
            println!("create channels...");
            
            // TODO: start the nodes that are marked to be online at the start of the simulation
            println!("starting initial nodes...");

            // Create the channel for threads to communicate over
            // TODO: make this a broadcast channel (many senders and many receivers) so that the analyzer thread also sees events and transaction generator can also send events
            let (event_sender, event_receiver) = mpsc::channel();

            // TODO: Start the TransactionGenerator
            // This thread will generate simulated network traffic
            println!("starting the transaction generator...");

            // TODO: Start the Analyzer
            // This thread will watch the network and gather stats and data about it that will be reported at the end of the sim
            println!("starting the network analyzer...");

            // Start the SenseiController
            println!("starting the sensei controller...");
            //let sensei_admin_service = sas.clone();
            let sensei_controller = Arc::new(LnSenseiController::new(sas, sensei_runtime_handle));
            let sensei_controller_handle = thread::spawn(move || {
                sensei_controller.process_events(event_receiver);
            });

            // Start the EventManager
            // This thread will fire off the events at the specified time in the sim
            println!("starting the event manager...");
            // TODO: currently for simplicity in order to create a proof of concept and demonstrate the use case of this project
            // the duration is denoted in seconds and will run in real time. Eventually this will need to be a longer duration that gets
            // run at faster-than-real-time pace so that long simulations can be modeled.
            // TODO: if this simulation was built to simply spin up a big network and allow manual testing, do not start the event manager

            let event_manager = Arc::new(self.em.clone());
            let d = self.duration;
            let event_manager_handle = thread::spawn(move || {
                event_manager.run(d, event_sender);
            });

            // Wait for all threads to finish and stop the sensei admin service
            match event_manager_handle.join() {
                Ok(()) => println!("event manager stopped..."),
                Err(_) => println!("event manager could not be stopped...")
            }
            match sensei_controller_handle.join() {
                Ok(()) => println!("sensei controller stopped..."),
                Err(_) => println!("sensei controller could not be stopped...")
            }

            // Stop bitcoin client with nigiri
            println!("stopping bitcoin with nigiri...");
            Command::new("sh")
            .arg("-c")
            .arg("nigiri stop --delete")
            .output()
            .expect("failed to execute process");
        });

        // Clean up sensei data after the simulation is done
        fs::remove_file("/home/blake/Projects/ln-ms-framework/ln_ms_lib/src/sensei_data_dir/sensei.db").expect("File delete failed");
        fs::remove_file("/home/blake/Projects/ln-ms-framework/ln_ms_lib/src/sensei_data_dir/sensei.db-shm").expect("File delete failed");
        fs::remove_file("/home/blake/Projects/ln-ms-framework/ln_ms_lib/src/sensei_data_dir/sensei.db-wal").expect("File delete failed");
        fs::remove_dir_all("/home/blake/Projects/ln-ms-framework/ln_ms_lib/src/sensei_data_dir/logs").expect("File delete failed");
        fs::remove_dir_all("/home/blake/Projects/ln-ms-framework/ln_ms_lib/src/sensei_data_dir/regtest").expect("File delete failed");

        Ok(())
    }

    // Parse a file that contains a definition of a LN topology
    // This definition could be from a project like Polar or from dumping the network information from the mainnet
    pub fn import_network(&self, filename: String) {
        println!("LnSimulation:{} -- importing network definition from {}", get_current_time(), filename);
        // TODO: Implement
    }

    // Create a lightweight node in the simulated network
    pub fn create_node(&mut self, name: String) {
        println!("LnSimulation:{} -- create Node: {}", get_current_time(), name);
        let name_key = name.clone();
        let node = LnNode {
            name: name
        };
        self.nodes.insert(name_key, node);
    }

    // Open a channel between two lightweight nodes in the simulated network
    pub fn create_channel(&mut self, node1_name: String, node2_name: String, amount: i32) {
        println!("LnSimulation:{} -- open Channel: {} -> {} for {} sats", get_current_time(), node1_name, node2_name, amount);
        let channel1 = LnChannel {
            node1: node1_name,
            node2: node2_name,
            node1_balance: amount,
            node2_balance: 0
        };
        self.channels.push(channel1);
    }

    // TODO: There should be a generic create_event function that allows the user to create all supported events
    // NOTE: For simplicity while proving the concept two hard coded event functions will work

    // Create an event that will start up a node in the simulated network
    pub fn create_node_online_event(&mut self, name: String, time: u64) {
        println!("LnSimulation:{} -- add NodeOnlineEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::NodeOnlineEvent(name);
        self.em.add_event(event, time);
    }

    // Create an event that will shut down a node in the simulated network
    pub fn create_node_offline_event(&mut self, name: String, time: u64) {
        println!("LnSimulation:{} -- add NodeOfflineEvent for: {} at {} seconds", get_current_time(), name, time);
        let event = SimulationEvent::NodeOfflineEvent(name);
        self.em.add_event(event, time);
    }
}

pub fn get_current_time() -> String {
    let date = Local::now();
    format!("{}", date.format("[%Y-%m-%d][%H:%M:%S]"))
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
        assert_eq!(success.is_ok(), true);
    }
}
