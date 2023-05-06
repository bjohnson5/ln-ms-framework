// Project modules
use crate::sim_event::SimulationEvent;
use crate::sim_node::SimNode;
use crate::sim_channel::SimChannel;
use crate::nigiri_controller;

// Standard modules
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::collections::HashMap;
use std::time::Duration;

// External modules
use tokio::sync::broadcast;

// Sensei modules
use senseicore::services::admin::{AdminRequest, AdminResponse, AdminService};
use senseicore::services::node::{NodeRequest, NodeResponse, OpenChannelRequest, NodeRequestError};
use senseicore::services::PaginationRequest;
use senseicore::node::LightningNode;
use entity::node;
use senseicore::services::PaymentsFilter;
use senseicore::node::HTLCStatus;

/* 
 * This struct processes simulation events and controls the sensei nodes.
 * It listens for the simulation events and then makes calls to the sensei AdminService and LightningNodes
 */
#[derive(Clone)]
pub struct SenseiController {
    sensei_admin_service: Arc<AdminService>,
    sensei_runtime_handle: tokio::runtime::Handle,
    channel_id_map: HashMap<u64, String>
}

impl SenseiController {
    // Create a new SenseiController using the admin service and runtime handle
    pub fn new(admin: Arc<AdminService>, runtime_handle: tokio::runtime::Handle) -> Self {
        let controller = SenseiController {
            sensei_admin_service: admin,
            sensei_runtime_handle: runtime_handle,
            channel_id_map: HashMap::new()
        };

        controller
    }

    // Receive events and make the appropriate calls to the Sensei library
    pub fn process_events(&self, mut event_channel: broadcast::Receiver<SimulationEvent>) {
        // This is the current map of simulation channel ids to sensei channel ids. It is needed to keep track of channels in order to open and close them.
        let mut channel_id_map: HashMap<u64, String> = self.channel_id_map.clone();

        // This is the main thread for processing sim events
        tokio::task::block_in_place(move || {
            self.sensei_runtime_handle.block_on(async move {
                let mut running = true;
                while running {
                    let event = event_channel.recv().await.unwrap();
                    match event {
                        SimulationEvent::StopNodeEvent(name) => {
                            println!("[=== SenseiController === {}] StopNodeEvent for {}", crate::get_current_time(), name);
                            self.stop_node(&name).await;
                        },
                        SimulationEvent::StartNodeEvent(name) => {
                            println!("[=== SenseiController === {}] StartNodeEvent for {}", crate::get_current_time(), name);
                            self.start_node(&name).await;
                        },
                        SimulationEvent::CloseChannelEvent(node_name, id) => {
                            println!("[=== SenseiController === {}] CloseChannelEvent for {}", crate::get_current_time(), id);
                            match channel_id_map.get(&id) {
                                Some(chanid) => {
                                    self.close_channel(node_name, String::from(chanid)).await;
                                },
                                None => {}
                            }
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("[=== SenseiController === {}] OpenChannelEvent for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            match self.open_channel(&channel.src_node, &channel.dest_node, channel.src_balance, channel.dest_balance, channel.id).await {
                                Some(chanid) => {
                                    channel_id_map.insert(channel.id, chanid);
                                },
                                None => {}
                            }
                        },
                        SimulationEvent::TransactionEvent(tx) => {
                            println!("[=== SenseiController === {}] TransactionEvent for {} <-> {}", crate::get_current_time(), tx.src_node, tx.dest_node);
                            match self.get_invoice(tx.dest_node, tx.amount).await {
                                Some(i) => {
                                    self.send_payment(tx.src_node, i).await;
                                },
                                None => println!("Could not get invoice")
                            }
                        },
                        SimulationEvent::NodeStatusEvent(name) => {
                            println!("[=== SenseiController === {}] NodeStatusEvent for {}", crate::get_current_time(), name);
                            self.print_node_status(&name).await;
                        },
                        SimulationEvent::SimulationEndedEvent => {
                            println!("[=== SenseiController === {}] SimulationEndedEvent", crate::get_current_time());
                            self.sensei_admin_service.stop_signal.store(true, Ordering::Release);
                            match self.sensei_admin_service.stop().await {
                                Ok(_) => {},
                                Err(e) => println!("could not stop sensei admin service: {:?}", e)
                            }
                            running = false;
                        },
                    }
                }
            })
        });
    }

    // Create and fund all the initial nodes in the network
    /*
     * TODO: This function is slow because creating sensei nodes is slow
     * - needs to be re-worked to speed up if the simulation framework is going to allow for large networks
     */
    pub async fn initialize_network(&mut self, nodes: &HashMap<String, SimNode>, channels: &Vec<SimChannel>, num_nodes: u64, nigiri: bool) {
        println!("[=== SenseiController === {}] Creating simulation nodes", crate::get_current_time());
        let num = num_nodes + 1;
        for number in 1..num {
            let node_name = String::from("simnode")+&number.to_string();
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
                    AdminResponse::CreateNode { .. } => {
                        // TODO: fund the simulation node with the configured amount. 500 is a place holder for now.
                        if nigiri {
                            match self.get_sensei_node(&node_name).await {
                                Ok(node) => {
                                    let node_req = NodeRequest::GetUnusedAddress {};
                                    match node.call(node_req).await {
                                        Ok(r) => {
                                            match r {
                                                NodeResponse::GetUnusedAddress { address } => {
                                                    nigiri_controller::fund_address(address, 500);
                                                },
                                                _ => println!("not an expected response from GetUnusedAddress")
                                            }
                                        }
                                        Err(e) => println!("error getting unused address: {:?}", e),
                                    }
                                },
                                Err(e) => {
                                    println!("node not found: {}", e);
                                }
                            }
                        }
                    },
                    _ => println!("no response from create node request")
                },
                Err(e) => println!("node failed to be created: {:?}", e)
            }
        }
        
        // Create and fund the user nodes, start them all in order to setup channels and fund the on chain wallets
        println!("[=== SenseiController === {}] Creating user defined nodes", crate::get_current_time());
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
                            match self.get_sensei_node(&n.0).await {
                                Ok(node) => {
                                    let node_req = NodeRequest::GetUnusedAddress {};
                                    match node.call(node_req).await {
                                        Ok(r) => {
                                            match r {
                                                NodeResponse::GetUnusedAddress { address } => {
                                                    nigiri_controller::fund_address(address, n.1.initial_balance);
                                                },
                                                _ => println!("not an expected response from GetUnusedAddress")
                                            }
                                        }
                                        Err(e) => println!("error getting unused address: {:?}", e),
                                    }
                                },
                                Err(e) => {
                                    println!("node not found: {}", e);
                                }
                            }
                        }
                    },
                    _ => println!("no response from create node request")
                },
                Err(e) => println!("node failed to be created: {:?}", e)
            }
        }

        // The sensei chain manager updates once a second. We need to wait and make sure all nodes are funded and the chain manager is aware of new blocks.
        // The chain manager needs to be up to date before trying to open channels.
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        println!("[=== SenseiController === {}] Creating channels", crate::get_current_time());
        for c in channels {
            match self.open_channel(&c.src_node, &c.dest_node, c.src_balance, c.dest_balance, c.id).await {
                Some(chanid) => {
                    self.channel_id_map.insert(c.id, chanid);
                },
                None => {
                    println!("failed to open channel")
                }
            }
        }
            
        // Stop the nodes that are not marked running at the start of the simulation
        println!("[=== SenseiController === {}] Setting the initial state of each node", crate::get_current_time());
        for n in nodes {
            if !n.1.running {
                self.stop_node(n.0).await;
            }
        }
    }

    // Get a nodes total balance, payments, and channels by name
    // TODO: This should return a data structure with node status
    async fn print_node_status(&self, name: &String) {
        match self.get_sensei_node(name).await {
            Ok(node) => {
                // BALANCE
                let balance_req = NodeRequest::GetBalance {};
                let balance_resp = node.call(balance_req).await.unwrap();
                let (balance, onchain, channel) = match balance_resp {
                    senseicore::services::node::NodeResponse::GetBalance {
                        onchain_balance_sats,
                        channel_balance_msats,
                        ..
                    } => {
                        let total = onchain_balance_sats + (channel_balance_msats / 1000);
                        (total, onchain_balance_sats, (channel_balance_msats / 1000))
                    }
                    _ => (0, 0, 0)
                };

                println!("Balance: {} {} {}", balance, onchain, channel);

                // PAYMENTS
                let pagination = PaginationRequest {
                    page: 0,
                    take: 1,
                    query: None,
                };
                let filter = PaymentsFilter {
                    status: Some(HTLCStatus::Succeeded.to_string()),
                    origin: None,
                };
                
                let (payments, _pagination) = node
                .database
                .list_payments_sync(node.id.clone(), pagination, filter)
                .unwrap();

                // TODO: loop over all payments instead of just printing the first one
                if payments.len() > 0 {
                    println!("Payments: {} {} {} ", payments.len(), payments[0].status, payments[0].amt_msat.unwrap() / 1000);
                }

                // CHANNELS
                let channels = node.list_channels(PaginationRequest {
                    page: 0,
                    take: 5,
                    query: None,
                })
                .unwrap()
                .0;

                // TODO: loop over all channels instead of just printing the first one
                if channels.len() > 0 {
                    println!("Channels: {} {} {} {} {} {} {} {} {}", channels[0].confirmations_required.unwrap(), channels.len().to_string(), channels[0].is_usable, channels[0].is_public, channels[0].is_outbound, channels[0].balance_msat, channels[0].outbound_capacity_msat, channels[0].inbound_capacity_msat, channels[0].is_channel_ready);
                }
            },
            _ => {
                println!("node not found");
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
                        Err("could not get node from directory")
                    }
                }
            },
            None => {
                Err("could not get node from database")
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
                println!("could not get node from database");
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
                match self.sensei_admin_service.call(stop_node_req).await {
                    Ok(AdminResponse::StopNode {}) => {},
                    Err(e) => println!("could not stop node: {} {:?}", id, e),
                    _ => println!("unexpected response from stop node")
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
                match self.sensei_admin_service.call(start_node_req).await {
                    Ok(AdminResponse::StartNode { macaroon: _ }) => {},
                    Err(e) => println!("could not start node: {} {:?}", id, e),
                    _ => println!("unexpected response from start node")
                }
            },
            None => {
                println!("node not found in the database {}", name);
            }
        }
    }

    // Close a sensei channel
    async fn close_channel(&self, node_name: String, id: String) {
        match self.get_sensei_node(&node_name).await {
            Ok(node) => {
                let close_chan = NodeRequest::CloseChannel {
                    channel_id: String::from(&id),
                    force: false,
                };

                match node.call(close_chan).await {
                    Ok(NodeResponse::CloseChannel {}) => {
                        //TODO: mining should be on a separate thread and continually generating new blocks. That will simulate accurate channel opening... you have to wait until the funding tx is included in a block
                        nigiri_controller::mine();
                    },
                    Err(e) => println!("could not close channel: {:?}", e),
                    _ => println!("unexpected response from close channel")
                }
            },
            _ => println!("src node not found")
        }
    }

    // Open a sensei channel
    async fn open_channel(&self, src_node_name: &String, dest_node_name: &String, src_amount: u64, dest_amount: u64, id: u64) -> Option<String> {
        let dest_pubkey: String;
        let dest_connection: String;
        match self.get_sensei_node_model(dest_node_name).await {
            Some(model) => {
                dest_connection = String::from(model.listen_addr) + ":" + &model.listen_port.to_string();
            },
            None => {
                println!("dest node not found");
                return None;
            }
        }

        match self.get_sensei_node(dest_node_name).await {
            Ok(node) => {
                dest_pubkey = node.get_pubkey();
            },
            Err(e) => {
                println!("dest node not found: {}", e);
                return None;
            }
        }

        match self.get_sensei_node(src_node_name).await {
            Ok(node) => {
                let mut open_requests: Vec<OpenChannelRequest> = Vec::new();
                let open_chan_req = OpenChannelRequest {
                    counterparty_pubkey: dest_pubkey.clone(),
                    amount_sats: src_amount+dest_amount,
                    public: true,
                    scid_alias: None,
                    custom_id: Some(id),
                    push_amount_msats: Some(dest_amount*1000),
                    counterparty_host_port: Some(dest_connection),
                    forwarding_fee_proportional_millionths: None,
                    forwarding_fee_base_msat: None,
                    cltv_expiry_delta: None,
                    max_dust_htlc_exposure_msat: None,
                    force_close_avoidance_max_fee_satoshis: None
                };
                open_requests.push(open_chan_req);
                let open_chan = NodeRequest::OpenChannels {
                    requests: open_requests
                };
                match node.call(open_chan).await {
                    Ok(NodeResponse::OpenChannels {requests: _, results: r}) => {
                        match &r[0].channel_id {
                            Some(chanid) => {
                                //TODO: mining should be on a separate thread and continually generating new blocks. That will simulate accurate channel opening... you have to wait until the funding tx is included in a block
                                nigiri_controller::mine();
                                return Some(String::from(chanid));
                            },
                            None => {
                                println!("could not open channel for : {} -> {}", src_node_name, dest_node_name);
                                return None;
                            }
                        }
                    },
                    Err(NodeRequestError::Sensei(e)) => {
                        println!("could not open channel for : {} -> {} {}", src_node_name, dest_node_name, e);
                        return None;
                    },
                    _ => {
                        println!("unexpected response from open channel");
                        return None;
                    }
                }
            },
            Err(e) => {
                println!("src node not found: {}", e);
                return None;
            }
        }
    }

    // Create and return an invoice string for a node
    async fn get_invoice(&self, node_name: String, amount: u64) -> Option<String> {
        match self.get_sensei_node(&node_name).await {
            Ok(node) => {
                let invoice_req = NodeRequest::GetInvoice {
                    amt_msat: amount * 1000,
                    description: String::from(""),
                };
                match node.call(invoice_req).await {
                    Ok(NodeResponse::GetInvoice {invoice: i}) => return Some(i),
                    Err(e) => {
                        println!("could not create invoice: {:?}", e);
                        return None;
                    },
                    _ => {
                        println!("unexpected response from get invoice");
                        return None;
                    }
                }
            },
            Err(e) => {
                println!("node not found: {}", e);
                return None;
            }
        }
    }

    // Pay an invoice from a node
    async fn send_payment(&self, node_name: String, invoice: String) {
        match self.get_sensei_node(&node_name).await {
            Ok(node) => {
                let payment_req = NodeRequest::SendPayment {
                    invoice: invoice
                };
                match node.call(payment_req).await {
                    Ok(NodeResponse::SendPayment {}) => {},
                    Err(NodeRequestError::Sensei(e)) => println!("Could not send payment. {}", e),
                    _ => println!("unexpected response from send payment")
                }
            },
            Err(e) => {
                println!("node not found: {}", e);
            }
        }
    }
}