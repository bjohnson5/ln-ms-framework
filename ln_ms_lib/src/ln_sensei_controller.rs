use crate::ln_event::SimulationEvent;

use std::sync::Arc;
use std::sync::atomic::{Ordering};
use std::sync::mpsc;

use senseicore::services::admin::{AdminRequest, AdminResponse, AdminService};
use entity::node;

#[derive(Clone)]
pub struct LnSenseiController {
    sensei_admin_service: Arc<AdminService>,
    sensei_runtime_handle: tokio::runtime::Handle
}

impl LnSenseiController {
    pub fn new(sas: Arc<AdminService>, runtime_handle: tokio::runtime::Handle) -> Self {
        let sc = LnSenseiController {
            sensei_admin_service: sas,
            sensei_runtime_handle: runtime_handle
        };

        sc
    }

    pub fn process_events(&self, event_channel: mpsc::Receiver<SimulationEvent>) {
        tokio::task::block_in_place(move || {
            self.sensei_runtime_handle.block_on(async move { 
                for event in event_channel {
                    match event {
                        SimulationEvent::NodeOfflineEvent(name) => {
                            println!("LnSenseiController:{} -- running a NodeOffline event for {}", crate::get_current_time(), name);
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
                            println!("LnSenseiController:{} -- running a NodeOnline event for {}", crate::get_current_time(), name);
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
                            println!("LnSenseiController:{} -- Simulation has ended", crate::get_current_time());
                            self.sensei_admin_service.stop_signal.store(true, Ordering::Release);
                            let _res = self.sensei_admin_service.stop().await;                          
                            break;
                        }
                    }
                }
            })
        });
    }

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