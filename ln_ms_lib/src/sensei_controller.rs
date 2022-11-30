// Project modules
use crate::sim_event::SimulationEvent;
use crate::sim_node::SimNode;
use crate::nigiri_controller::NigiriController;

// Standard modules
use std::sync::Arc;
use std::sync::atomic::{Ordering};
use tokio::sync::broadcast;
use std::collections::HashMap;

// Sensei modules
use senseicore::services::admin::{AdminRequest, AdminResponse, AdminService};
use senseicore::services::node::NodeRequest;
use entity::node;
use senseicore::node::LightningNode;

// This struct holds the Sensei Admin Service and will process simulation events by controlling the Sensei nodes in the network

#[derive(Clone)]
pub struct SenseiController {
    sensei_admin_service: Arc<AdminService>,
    sensei_runtime_handle: tokio::runtime::Handle
}

impl SenseiController {
    pub fn new(admin: Arc<AdminService>, runtime_handle: tokio::runtime::Handle) -> Self {
        let controller = SenseiController {
            sensei_admin_service: admin,
            sensei_runtime_handle: runtime_handle
        };

        controller
    }

    // The process_events function will receive events and make the appropriate calls to the Sensei Admin Service
    pub fn process_events(&self, mut event_channel: broadcast::Receiver<SimulationEvent>) {
        tokio::task::block_in_place(move || {
            self.sensei_runtime_handle.block_on(async move {
                let mut running = true;
                while running {
                    let event = event_channel.recv().await.unwrap();
                    match event {
                        SimulationEvent::NodeOfflineEvent(name) => {
                            println!("SenseiController:{} -- running a NodeOffline event for {}", crate::get_current_time(), name);
                            self.stop_node(&name).await;
                        },
                        SimulationEvent::NodeOnlineEvent(name) => {
                            println!("SenseiController:{} -- running a NodeOnline event for {}", crate::get_current_time(), name);
                            self.start_node(&name).await;
                        },
                        SimulationEvent::CloseChannelEvent(channel) => {
                            println!("SenseiController:{} -- running a CloseChannel event for {} <-> {}", crate::get_current_time(), channel.node1, channel.node2);
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("SenseiController:{} -- running an OpenChannel event for {} <-> {}", crate::get_current_time(), channel.node1, channel.node2);
                        },
                        SimulationEvent::SimulationEnded => {
                            println!("SenseiController:{} -- Simulation has ended", crate::get_current_time());
                            self.sensei_admin_service.stop_signal.store(true, Ordering::Release);
                            let _res = self.sensei_admin_service.stop().await;                          
                            running = false;
                        }
                    }
                }
            })
        });
    }

    // Create and fund all the initial nodes in the network
    pub async fn initialize_network(&self, nodes: &HashMap<String, SimNode>) {
        for n in nodes {
            let create_node_req = AdminRequest::CreateNode { 
                username: String::from(n.0), 
                alias: String::from(n.0), 
                passphrase: String::from(n.0), 
                start: true,
                entropy: None,
                cross_node_entropy: None,
            };
            match self.sensei_admin_service.call(create_node_req).await {
                Ok(response) => match response {
                    AdminResponse::CreateNode {
                        ..
                    } => {
                        match self.get_sensei_node(n.0).await {
                            Ok(node) => {
                                let node_req = NodeRequest::GetUnusedAddress {};
                                let address_resp = node.call(node_req).await.unwrap();
                                match address_resp {
                                    senseicore::services::node::NodeResponse::GetUnusedAddress { address } => {
                                        NigiriController::fund_address(address, n.1.initial_balance);
                                    }
                                    _ => println!("error getting unused address"),
                                }
                            },
                            _ => {
                                println!("node not found");
                            }
                        }
                    },
                    _ => println!("no response from create node request")
                },
                Err(_) => println!("node failed to be created")
            }
        }

        // TODO: open all initial channels
        println!("create channels...");
            
        // Stop the nodes that are not marked running at the start of the simulation
        println!("setting initial node states (start/stop)...");
        for n in nodes {
            if !n.1.running {
                self.stop_node(n.0).await;
            }
        }
    }

    // Get a nodes total balance by name
    pub async fn _get_node_balance(&self, name: &String) -> u64 {
        match self.get_sensei_node(name).await {
            Ok(node) => {
                let balance_req = NodeRequest::GetBalance {};
                let balance_resp = node.call(balance_req).await.unwrap();
                let balance = match balance_resp {
                    senseicore::services::node::NodeResponse::GetBalance {
                        onchain_balance_sats,
                        channel_balance_msats,
                        ..
                    } => {
                        let total = onchain_balance_sats + channel_balance_msats / 1000;
                        total
                    }
                    _ => 0
                };

                balance
            },
            _ => {
                println!("node not found");
                0
            }
        }
    }

    // Gets a sensei node from the node directory
    pub async fn get_sensei_node(&self, name: &String) -> Result<Arc<LightningNode>, &str> {
        match self.get_sensei_node_model(name).await {
            Some(model) => {
                let node_directory = self.sensei_admin_service.node_directory.lock().await;
                match node_directory.get(&model.id) {
                    Some(Some(node_handle)) => {
                        Ok(node_handle.node.clone())
                    },
                    _ => {
                        Err("error getting node from directory")
                    }
                }
            },
            None => {
                Err("error getting node from database")
            }
        }
    }

    // Gets a sensei node from the database by username and returns an Option (None if the node was not found in the database)
    pub async fn get_sensei_node_model(&self, name: &String) -> Option<node::Model> {
        let db_node = self.sensei_admin_service
        .database
        .get_node_by_username(&name)
        .await;
        match db_node {
            Ok(option) => {
                option
            },
            _ => {
                println!("error getting node from database");
                None
            }
        }
    }

    // Stop a sensei node
    pub async fn stop_node(&self, name: &String) {
        match self.get_sensei_node_model(&name).await {
            Some(model) => {
                let id = String::from(model.id);
                let stop_node_req = AdminRequest::StopNode {
                    pubkey: id.clone(),
                };
                let stop_node_resp = self.sensei_admin_service.call(stop_node_req).await;
                match stop_node_resp {
                    Ok(AdminResponse::StopNode {}) => {},
                    _ => println!("could not stop node: {}", id)
                }
            },
            None => {
                println!("node not found in the database");
            }
        }
    }

    // Start a sensei node
    pub async fn start_node(&self, name: &String) {
        match self.get_sensei_node_model(&name).await {
            Some(model) => {
                let id = String::from(model.id);
                let start_node_req = AdminRequest::StartNode {
                    pubkey: id.clone(),
                    passphrase: name.clone(),
                };
                let start_node_resp = self.sensei_admin_service.call(start_node_req).await;
                match start_node_resp {
                    Ok(AdminResponse::StartNode { macaroon: _ }) => {},
                    _ => println!("could not start node: {}", id)
                }
            },
            None => {
                println!("node not found in the database");
            }
        }
    }
}