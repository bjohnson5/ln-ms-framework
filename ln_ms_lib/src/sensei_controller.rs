// Project modules
use crate::sim_event::SimulationEvent;
use crate::sim_node::SimNode;
use crate::sim_channel::SimChannel;
use crate::nigiri_controller;

// Standard modules
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::collections::HashMap;

// External modules
use tokio::sync::broadcast;

// Sensei modules
use senseicore::services::admin::{AdminRequest, AdminResponse, AdminService};
use senseicore::services::node::{NodeRequest, NodeResponse, NodeRequest::OpenChannels, OpenChannelRequest};
use senseicore::node::LightningNode;
use entity::node;

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

    // TODO: implement all other events
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
                            println!("SenseiController:{} -- running a CloseChannel event for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            // TODO: implement
                            //self.close_channel().await;
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("SenseiController:{} -- running an OpenChannel event for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            // TODO: implement
                            //self.open_channel(&channel.src_node, &channel.dest_node, channel.src_balance, channel.dest_balance).await;
                        },
                        SimulationEvent::TransactionEvent(tx) => {
                            println!("LnSimulation:{} -- running a TransactionEvent for {} <-> {}", crate::get_current_time(), tx.src_node, tx.dest_node);
                            // TODO: implement
                            // TODO: create an invoice from dest_node (NodeRequest::GetInvoice), send payment from src_node (NodeRequest::SendPayment)
                        }
                        SimulationEvent::SimulationEndedEvent => {
                            println!("SenseiController:{} -- Simulation has ended", crate::get_current_time());
                            self.sensei_admin_service.stop_signal.store(true, Ordering::Release);
                            let _res = self.sensei_admin_service.stop().await;                          
                            running = false;
                        },
                    }
                }
            })
        });
    }

    // TODO: this function is slow because creating sensei nodes is slow, needs to be re-worked to speed up if the simulation framework is going to allow for large networks
    // Create and fund all the initial nodes in the network
    pub async fn initialize_network(&self, nodes: &HashMap<String, SimNode>, _channels: &Vec<SimChannel>, num_nodes: u64, nigiri: bool) {
        // TODO: allow the user to configure these default nodes (example: how much initial balance?)
        // Start the specified number of default nodes that are already in the sensei database
        let num = num_nodes + 1;
        for number in 1..num {
            let node_name = String::from("node")+&number.to_string();
            let create_node_req = AdminRequest::CreateNode { 
                username: node_name.clone(), 
                alias: node_name.clone(), 
                passphrase: node_name.clone(), 
                start: true,
                entropy: None,
                cross_node_entropy: None,
            };
            match self.sensei_admin_service.call(create_node_req).await {
                Ok(response) => match response {
                    AdminResponse::CreateNode {
                        ..
                    } => {
                        if nigiri {
                            match self.get_sensei_node(&node_name).await {
                                Ok(node) => {
                                    let node_req = NodeRequest::GetUnusedAddress {};
                                    let address_resp = node.call(node_req).await.unwrap();
                                    match address_resp {
                                        senseicore::services::node::NodeResponse::GetUnusedAddress { address } => {
                                            nigiri_controller::fund_address(address, 500);
                                        }
                                        _ => println!("error getting unused address"),
                                    }
                                },
                                _ => {
                                    println!("node not found");
                                }
                            }
                        }
                    },
                    _ => println!("no response from create node request")
                },
                Err(_) => println!("node failed to be created")
            }
        }
        
        // Create and fund the user nodes, start them all in order to setup channels and fund the on chain wallets
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
                        if n.1.initial_balance != 0 && nigiri {
                            match self.get_sensei_node(n.0).await {
                                Ok(node) => {
                                    let node_req = NodeRequest::GetUnusedAddress {};
                                    let address_resp = node.call(node_req).await.unwrap();
                                    match address_resp {
                                        senseicore::services::node::NodeResponse::GetUnusedAddress { address } => {
                                            nigiri_controller::fund_address(address, n.1.initial_balance);
                                        }
                                        _ => println!("error getting unused address"),
                                    }
                                },
                                _ => {
                                    println!("node not found");
                                }
                            }
                        }
                    },
                    _ => println!("no response from create node request")
                },
                Err(_) => println!("node failed to be created")
            }
        }

        println!("create channels...");
        // TODO: implement
        //for c in channels {
        //    self.open_channel(&c.src_node, &c.dest_node, c.src_balance, c.dest_balance).await;
        //}
            
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
    async fn get_sensei_node(&self, name: &String) -> Result<Arc<LightningNode>, &str> {
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
    async fn get_sensei_node_model(&self, name: &String) -> Option<node::Model> {
        let db_node = self.sensei_admin_service
        .database
        .get_node_by_username(name)
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
    async fn stop_node(&self, name: &String) {
        match self.get_sensei_node_model(name).await {
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
    async fn start_node(&self, name: &String) {
        match self.get_sensei_node_model(name).await {
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
                println!("node not found in the database {}", name);
            }
        }
    }

    // Close a sensei channel
    async fn _close_channel(&self) {
        // TODO: implement
    }

    // Open a sensei channel
    async fn _open_channel(&self, src_node_name: &String, dest_node_name: &String, src_amount: u64, dest_amount: u64) {
        let dest_id: String;
        match self.get_sensei_node_model(dest_node_name).await {
            Some(model) => {
                dest_id = String::from(model.id);
            },
            None => {
                println!("dest node not found");
                return;
            }
        }

        match self.get_sensei_node(src_node_name).await {
            Ok(node) => {
                let mut open_requests: Vec<OpenChannelRequest> = Vec::new();
                let open_chan_req = OpenChannelRequest {
                    counterparty_pubkey: dest_id.clone(),
                    amount_sats: src_amount+dest_amount,
                    public: true,
                    scid_alias: None,
                    custom_id: None,
                    push_amount_msats: Some(dest_amount/1000),
                    counterparty_host_port: None,
                    forwarding_fee_proportional_millionths: None,
                    forwarding_fee_base_msat: None,
                    cltv_expiry_delta: None,
                    max_dust_htlc_exposure_msat: None,
                    force_close_avoidance_max_fee_satoshis: None
                };
                open_requests.push(open_chan_req);
                let open_chan = OpenChannels {
                    requests: open_requests
                };
                let open_response = node.call(open_chan).await;
                match open_response {
                    Ok(NodeResponse::OpenChannels {requests: _, results: _}) => {},
                    _ => println!("could not open channel for : {} -> {}", src_node_name, dest_node_name)
                }
            },
            _ => {
                println!("src node not found");
            }
        }
    }
}