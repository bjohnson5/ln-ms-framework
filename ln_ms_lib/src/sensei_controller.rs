// Project modules
use crate::sim_event::SimulationEvent;

// Standard modules
use std::sync::Arc;
use std::sync::atomic::{Ordering};
use std::sync::mpsc;

// Sensei modules
use senseicore::services::admin::{AdminRequest, AdminResponse, AdminService};
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

    // The process_events function will receive events and make the appropriate calls to the Sensei Admin Service
    pub fn process_events(&self, event_channel: mpsc::Receiver<SimulationEvent>) {
        tokio::task::block_in_place(move || {
            self.sensei_runtime_handle.block_on(async move { 
                for event in event_channel {
                    match event {
                        SimulationEvent::NodeOfflineEvent(name) => {
                            println!("SenseiController:{} -- running a NodeOffline event for {}", crate::get_current_time(), name);
                            match self.get_sensei_node(name.clone()).await {
                                Some(model) => {
                                    let id = String::from(model.id);
                                    let stop_node_req = AdminRequest::StopNode {
                                        pubkey: id.clone(),
                                    };
                                    let stop_node_resp = self.sensei_admin_service.call(stop_node_req).await;
                                    match stop_node_resp {
                                        Ok(AdminResponse::StopNode {}) => {
                                            println!("Stopped node: {}", id);
                                        },
                                        _ => println!("Could not stop node: {}", id)
                                    }
                                },
                                None => {
                                    println!("Node not found in the database");
                                }
                            }
                        },
                        SimulationEvent::NodeOnlineEvent(name) => {
                            println!("SenseiController:{} -- running a NodeOnline event for {}", crate::get_current_time(), name);
                            match self.get_sensei_node(name.clone()).await {
                                Some(model) => {
                                    let id = String::from(model.id);
                                    let start_node_req = AdminRequest::StartNode {
                                        pubkey: id.clone(),
                                        passphrase: name,
                                    };
                                    let start_node_resp = self.sensei_admin_service.call(start_node_req).await;
                                    match start_node_resp {
                                        Ok(AdminResponse::StartNode { macaroon: _ }) => {
                                            println!("Started node: {}", id);
                                        },
                                        _ => println!("Could not start node: {}", id)
                                    }
                                },
                                None => {
                                    println!("Node not found in the database");
                                }
                            }
                        },
                        SimulationEvent::SimulationEnded => {
                            println!("SenseiController:{} -- Simulation has ended", crate::get_current_time());
                            self.sensei_admin_service.stop_signal.store(true, Ordering::Release);
                            let _res = self.sensei_admin_service.stop().await;                          
                            break;
                        }
                    }
                }
            })
        });
    }

    // Gets a sensei node from the database by username and returns an Option (None if the node was not found in the database)
    pub async fn get_sensei_node(&self, username: String) -> Option<node::Model> {
        let db_node = self.sensei_admin_service
        .database
        .get_node_by_username(&username)
        .await;
        match db_node {
            Ok(option) => {
                option
            },
            Err(_) => {
                println!("Error getting node from database");
                None
            }
        }
    }
}