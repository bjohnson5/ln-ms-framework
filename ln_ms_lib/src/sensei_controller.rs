// Project modules
use crate::sim_event::SimResultsEvent;
use crate::sim_event::SimulationEvent;
use crate::sim_event::SimEvent;
use crate::sim_node::SimNode;
use crate::sim_channel::SimChannel;
use crate::nigiri_controller;
use crate::sim_node_status::SimNodeStatus;
use crate::sim_node_status::SimNodeChannel;
use crate::sim_transaction::SimTransaction;
use crate::sim_transaction::SimTransactionStatus;

// Standard modules
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::collections::HashMap;
use std::time::Duration;

// External modules
use tokio::sync::broadcast;

// Sensei and LDK modules
use lightning::util::events::Event;
use senseicore::services::admin::{AdminRequest, AdminResponse, AdminService};
use senseicore::services::admin::Error;
use senseicore::services::node::{NodeRequest, NodeResponse, OpenChannelRequest, NodeRequestError};
use senseicore::services::PaginationRequest;
use senseicore::node::LightningNode;
use entity::node;

/* 
 * This struct processes simulation events and controls the sensei nodes.
 * It listens for the simulation events and then makes calls to the sensei AdminService and LightningNodes
 */
pub struct SenseiController {
    sensei_admin_service: Arc<AdminService>,
    sensei_runtime_handle: tokio::runtime::Handle,
    channel_id_map: HashMap<u64, String>,
    rev_channel_id_map: HashMap<String, u64>
}

impl SenseiController {
    // Create a new SenseiController using the admin service and runtime handle
    pub fn new(admin: Arc<AdminService>, runtime_handle: tokio::runtime::Handle) -> Self {
        let controller = SenseiController {
            sensei_admin_service: admin,
            sensei_runtime_handle: runtime_handle,
            channel_id_map: HashMap::new(),
            rev_channel_id_map: HashMap::new()
        };

        controller
    }

    // Receive events and make the appropriate calls to the Sensei library
    pub fn process_events(&self, mut event_channel: broadcast::Receiver<SimEvent>, output_channel: broadcast::Sender<SimResultsEvent>) {
        // This is the current map of simulation channel ids to sensei channel ids. It is needed to keep track of channels in order to open and close them.
        let mut channel_id_map: HashMap<u64, String> = self.channel_id_map.clone();
        let mut rev_channel_id_map: HashMap<String, u64> = self.rev_channel_id_map.clone();

        // This is the main thread for processing sim events
        tokio::task::block_in_place(move || {
            self.sensei_runtime_handle.block_on(async move {
                let mut running = true;
                while running {
                    let event = event_channel.recv().await.unwrap();
                    match &event.event {
                        SimulationEvent::StopNodeEvent(name) => {
                            println!("[=== SenseiController === {}] StopNodeEvent for {}", crate::get_current_time(), name);
                            let success: bool;
                            match self.stop_node(name).await {
                                Ok(()) => {
                                    success = true;
                                },
                                Err(e) => {
                                    println!("could not stop node: {:?}", e);
                                    success = false;
                                }
                            }

                            // Tell the network analyzer that this node has been stopped or failed to stop at this time
                            let sim_event = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: success, event: event.event.clone()};
                            output_channel.send(sim_event).expect("could not send the event");
                        },
                        SimulationEvent::StartNodeEvent(name) => {
                            println!("[=== SenseiController === {}] StartNodeEvent for {}", crate::get_current_time(), name);
                            let success: bool;
                            match self.start_node(name).await {
                                Ok(()) => {
                                    success = true;
                                },
                                Err(e) => {
                                    println!("could not start node: {:?}", e);
                                    success = false;
                                }
                            }

                            // Tell the network analyzer that this node has been started or failed to start at this time
                            let sim_event = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: success, event: event.event.clone()};
                            output_channel.send(sim_event).expect("could not send the event");
                        },
                        SimulationEvent::CloseChannelEvent(channel) => {
                            println!("[=== SenseiController === {}] CloseChannelEvent for {}", crate::get_current_time(), channel.id);
                            let success: bool;
                            match channel_id_map.get(&channel.id) {
                                Some(chanid) => {
                                    match self.close_channel(&channel.src_node, String::from(chanid)).await {
                                        Ok(()) => {
                                            success = true;
                                        },
                                        Err(e) => {
                                            println!("could not close channel: {:?}", e);
                                            success = false;
                                        }                                        
                                    }
                                },
                                None => {
                                    println!("could not find channel.");
                                    success = false;
                                }
                            }

                            // Tell the network analyzer that this channel has closed or failed to close at this time
                            let sim_event = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: success, event: event.event.clone()};
                            output_channel.send(sim_event).expect("could not send the event");
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("[=== SenseiController === {}] OpenChannelEvent for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            match self.open_channel(&channel.src_node, &channel.dest_node, channel.src_balance, channel.dest_balance, channel.id).await {
                                Ok(chanid) => {
                                    // Establish the relationship between sensei channel id and sim channel id for the new channel
                                    channel_id_map.insert(channel.id, chanid.clone());
                                    rev_channel_id_map.insert(chanid.clone(), channel.id);
                                    
                                    // Get the node status from sensei and set the short id that was assigned to this channel
                                    let node_status = self.get_node_status(&channel.src_node).await;
                                    match node_status {
                                        Some(status) => {
                                            // Create a new SimChannel with the same values as the "channel" variable and the short id of the "SimNodeChannel" that we got from sensei
                                            let simchan = SimChannel {
                                                id: channel.id.clone(),
                                                short_id: match status.get_channel(channel.id) {
                                                    Some(sc) => {
                                                        sc.short_id
                                                    },
                                                    None => {None}
                                                },
                                                src_node: channel.src_node.clone(),
                                                dest_node: channel.dest_node.clone(),
                                                src_balance: channel.src_balance,
                                                dest_balance: channel.dest_balance.clone()
                                            };

                                            // Tell the network analyzer that this channel was opened and pass the new channel object to use
                                            let channel_event = SimulationEvent::OpenChannelEvent(simchan);
                                            let sim_event = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: true, event: channel_event.clone()};
                                            output_channel.send(sim_event).expect("could not send the event");
                                        },
                                        None => { println!("node status not found, not updating the network analyzer.") }
                                    }
                                },
                                Err(e) => {
                                    println!("could not open channel: {:?}", e);
                                    
                                    // Tell the network analyzer that this channel failed to open
                                    let sim_event = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: false, event: event.event.clone()};
                                    output_channel.send(sim_event).expect("could not send the event");
                                }
                            }
                        },
                        SimulationEvent::TransactionEvent(tx) => {
                            println!("[=== SenseiController === {}] TransactionEvent for {} <-> {}", crate::get_current_time(), tx.src_node, tx.dest_node);
                            match self.get_invoice(&tx.dest_node, tx.amount).await {
                                Some(i) => {
                                    // Attempt to send a payment
                                    match self.send_payment(&tx.src_node, i).await {
                                        Ok(id) => {
                                            // Payment was sent and now we can set the payment id of this SimTransaction
                                            let transaction = SimTransaction {
                                                id: Some(id.clone()),
                                                src_node: tx.src_node.clone(),
                                                dest_node: tx.dest_node.clone(),
                                                amount: tx.amount,
                                                status: SimTransactionStatus::PENDING
                                            };

                                            // Tell the network analyzer that we sent this payment successfully (it still might fail to get received though, so it is PENDING)
                                            let transaction_event = SimulationEvent::TransactionEvent(transaction);
                                            let sim_event_src = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: true, event: transaction_event};
                                            output_channel.send(sim_event_src).expect("could not send the event");
                                        },
                                        Err(e) => {
                                            println!("could not send payment: {:?}", e);
                                            
                                            // Tell the network analyzer that this transaction event failed
                                            let sim_even_src = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: false, event: event.event.clone()};
                                            output_channel.send(sim_even_src).expect("could not send the event");
                                        }
                                    }
                                },
                                None => {
                                    println!("could not get invoice");

                                    // Tell the network analyzer that this transaction event failed
                                    let sim_even_src = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: false, event: event.event.clone()};
                                    output_channel.send(sim_even_src).expect("could not send the event");
                                }
                            }
                        },
                        SimulationEvent::SimulationEndedEvent => {
                            println!("[=== SenseiController === {}] SimulationEndedEvent", crate::get_current_time());
                            self.sensei_admin_service.stop_signal.store(true, Ordering::Release);
                            match self.sensei_admin_service.stop().await {
                                Ok(_) => {},
                                Err(e) => println!("could not stop sensei admin service: {:?}", e)
                            }

                            // Tell the network analyzer that the simulation has ended and that it should stop
                            let sim_event = SimResultsEvent{sim_time: Some(event.sim_time.clone()), success: true, event: event.event.clone()};
                            output_channel.send(sim_event).expect("could not send the event");
                            running = false;
                        },
                        _ => {
                            // Ignore all other events
                        }
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
    pub async fn initialize_network(&mut self, nodes: &HashMap<String, SimNode>, channels: &Vec<SimChannel>, num_nodes: u64, nigiri: bool) -> Vec<broadcast::Receiver<Event>> {
        println!("[=== SenseiController === {}] Creating simulation nodes", crate::get_current_time());
        let mut sim_receivers = Vec::new();
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
                        match self.get_sensei_node(&node_name).await {
                            Ok(node) => {
                                // TODO: fund the simulation node with the configured amount. 1 BTC is a place holder for now.
                                if nigiri {
                                    let node_req = NodeRequest::GetUnusedAddress {};
                                    match node.call(node_req).await {
                                        Ok(r) => {
                                            match r {
                                                NodeResponse::GetUnusedAddress { address } => {
                                                    nigiri_controller::fund_address(address, 1_000_000_000);
                                                },
                                                _ => println!("not an expected response from GetUnusedAddress")
                                            }
                                        }
                                        Err(e) => println!("error getting unused address: {:?}", e),
                                    }
                                }
                                let r = node.sim_sender.subscribe();
                                sim_receivers.push(r);
                            },
                            Err(e) => {
                                println!("node not found: {}", e);
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
                        match self.get_sensei_node(&n.0).await {
                            Ok(node) => {
                                if n.1.initial_balance != 0 && nigiri {
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
                                }
                                let r = node.sim_sender.subscribe();
                                sim_receivers.push(r);
                            },
                            Err(e) => {
                                println!("node not found: {}", e);
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
                Ok(chanid) => {
                    // Establish the relationship between sensei channel id and sim channel id for the new channel
                    self.channel_id_map.insert(c.id, chanid.clone());
                    self.rev_channel_id_map.insert(chanid.clone(), c.id);
                },
                Err(e) => {
                    println!("failed to open channel: {:?}", e);
                }
            }
        }

        // The sensei chain manager updates once a second. We need to wait and make sure all commitment txs are seen by the chain manager.
        tokio::time::sleep(Duration::from_secs(2)).await;
            
        // Stop the nodes that are not marked running at the start of the simulation
        println!("[=== SenseiController === {}] Setting the initial state of each node", crate::get_current_time());
        for n in nodes {
            if !n.1.running {
                match self.stop_node(n.0).await {
                    Ok(()) => {},
                    Err(e) => {
                        println!("could not stop node: {:?}", e);
                    }
                }
            }
        }

        sim_receivers
    }

    // Get a nodes total balance and channels by name
    pub async fn get_node_status(&self, name: &String) -> Option<SimNodeStatus> {
        let mut status = SimNodeStatus::new();

        match self.get_sensei_node(name).await {
            Ok(node) => {
                status.pub_key = node.get_pubkey();

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

                status.balance.total = balance;
                status.balance.onchain = onchain;
                status.balance.offchain = channel;

                // CHANNELS
                let mut has_more = true;
                let mut page_num = 0;
                while has_more {
                    let request = PaginationRequest {
                        page: page_num.clone(),
                        take: 5,
                        query: None,
                    };
                    match node.list_channels(request) {
                        Ok((c, r)) => {
                            for chan in c.into_iter() {
                                status.channels.push(SimNodeChannel::new(self.rev_channel_id_map.get(&chan.channel_id).unwrap().clone(), chan.short_channel_id,  chan.confirmations_required.unwrap(), chan.is_usable, chan.is_public, 
                                    chan.is_outbound, chan.balance_msat, chan.outbound_capacity_msat, chan.inbound_capacity_msat, chan.is_channel_ready));
                            }
                            has_more = r.has_more;
                        },
                        Err(e) => {
                            println!("could not get channels: {:?}", e);
                            return None;
                        }
                    }
                    page_num = page_num + 1;
                }

                Some(status)
            },
            _ => {
                println!("node not found");
                None
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
    async fn stop_node(&self, name: &String) -> Result<(), Error> {
        match self.get_sensei_node_model(name).await {
            Some(model) => {
                let id = String::from(model.id);
                let stop_node_req = AdminRequest::StopNode {
                    pubkey: id.clone(),
                };
                match self.sensei_admin_service.call(stop_node_req).await {
                    Ok(AdminResponse::StopNode {}) => { Ok(()) },
                    Err(e) => Err(e),
                    _ => Err(Error::Generic(String::from("unexpected response from stop node")))
                }
            },
            None => {
                Err(Error::Generic(String::from("node not found in the database")))
            }
        }
    }

    // Start a sensei node
    async fn start_node(&self, name: &String) -> Result<(), Error> {
        match self.get_sensei_node_model(name).await {
            Some(model) => {
                let id = String::from(model.id);
                let start_node_req = AdminRequest::StartNode {
                    pubkey: id.clone(),
                    passphrase: name.clone(),
                };
                match self.sensei_admin_service.call(start_node_req).await {
                    Ok(AdminResponse::StartNode { macaroon: _ }) => { Ok(()) },
                    Err(e) => Err(e),
                    _ => Err(Error::Generic(String::from("unexpected response from stop node")))
                }
            },
            None => {
                Err(Error::Generic(String::from("node not found in the database")))
            }
        }
    }

    // Close a sensei channel
    async fn close_channel(&self, node_name: &String, id: String) -> Result<(), NodeRequestError> {
        match self.get_sensei_node(node_name).await {
            Ok(node) => {
                let close_chan = NodeRequest::CloseChannel {
                    channel_id: String::from(&id),
                    force: false,
                };

                match node.call(close_chan).await {
                    Ok(NodeResponse::CloseChannel {}) => {
                        //TODO: mining should be on a separate thread and continually generating new blocks. That will simulate accurate channel opening... you have to wait until the funding tx is included in a block
                        nigiri_controller::mine();
                        Ok(())
                    },
                    Err(e) => Err(e),
                    _ => Err(NodeRequestError::Sensei(String::from("unexpected response from close channel")))
                }
            },
            _ => Err(NodeRequestError::Sensei(String::from("node not found in the database")))
        }
    }

    // Open a sensei channel
    async fn open_channel(&self, src_node_name: &String, dest_node_name: &String, src_amount: u64, dest_amount: u64, id: u64) -> Result<String, Error> {
        let dest_pubkey: String;
        let dest_connection: String;
        match self.get_sensei_node_model(dest_node_name).await {
            Some(model) => {
                dest_connection = String::from(model.listen_addr) + ":" + &model.listen_port.to_string();
            },
            None => {
                return Err(Error::Generic(String::from("dest node not found")))
            }
        }

        match self.get_sensei_node(dest_node_name).await {
            Ok(node) => {
                dest_pubkey = node.get_pubkey();
            },
            Err(e) => {
                return Err(Error::Generic(String::from("dest node not found: ") + &String::from(e)))
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
                                return Ok(String::from(chanid));
                            },
                            None => {
                                return Err(Error::Generic(String::from("could not open channel for: ") + src_node_name + " " + dest_node_name));
                            }
                        }
                    },
                    Err(NodeRequestError::Sensei(e)) => {
                        return Err(Error::Generic(String::from("could not open channel for: ") + src_node_name + " " + dest_node_name + " " + &e));
                    },
                    _ => {
                        return Err(Error::Generic(String::from("unexpected response from open channel")));
                    }
                }
            },
            Err(e) => {
                return Err(Error::Generic(String::from("src node not found: ") + e));
            }
        }
    }

    // Create and return an invoice string for a node
    async fn get_invoice(&self, node_name: &String, amount: u64) -> Option<String> {
        match self.get_sensei_node(node_name).await {
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
    async fn send_payment(&self, node_name: &String, invoice: String) -> Result<String, Error> {
        match self.get_sensei_node(node_name).await {
            Ok(node) => {
                let payment_req = NodeRequest::SendPayment {
                    invoice: invoice
                };
                match node.call(payment_req).await {
                    Ok(NodeResponse::SendPayment {id}) => {
                        return Ok(id);
                    },
                    Err(NodeRequestError::Sensei(e)) => {
                        return Err(Error::Generic(e));
                    },
                    _ => {
                        return Err(Error::Generic(String::from("unexpected response from send payment")));
                    }
                }
            },
            Err(e) => {
                return Err(Error::Generic(String::from(e)));
            }
        }
    }
}