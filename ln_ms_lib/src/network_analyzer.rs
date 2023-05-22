// Project modules
use crate::sim_event::SimResultsEvent;
use crate::sim_event::SimulationEvent;
use crate::sim_results::SimResults;
use crate::sim_results::Tx;
use crate::sim_runtime_graph::RuntimeNetworkGraph;
use crate::sensei_controller::SenseiController;
use crate::sim_channel::SimChannel;
use crate::sim_transaction::SimTransactionStatus;
use crate::sim_transaction::SimTransaction;
use crate::sim_event::PathHop;

// External modules
use tokio::sync::broadcast;

// Standard modules
use std::collections::HashMap;

/*
 * Processes results and creates the SimResults object that will get returned after the simulation ends
 */
pub struct NetworkAnalyzer {
    analyzer_runtime_handle: tokio::runtime::Handle,
    results: SimResults,
    pub_key_map: HashMap<String, String>
}

impl NetworkAnalyzer {
    pub fn new(runtime_handle: tokio::runtime::Handle) -> Self {
        let analyzer = NetworkAnalyzer {
            analyzer_runtime_handle: runtime_handle,
            results: SimResults::new(),
            pub_key_map: HashMap::new()
        };

        analyzer
    }

    /*
     * Set up the initial state of the network at sim time = 0
     */
    pub async fn initialize_network(&mut self, network: &RuntimeNetworkGraph, sensei_controller: &SenseiController) {
        self.results.channels.open_channels.insert(0, Vec::new());

        // Initialize balances and status
        for n in &network.nodes {
            self.results.balance.on_chain.insert(n.name.clone(), HashMap::new());
            self.results.balance.off_chain.insert(n.name.clone(), HashMap::new());
            self.results.status.nodes.insert(n.name.clone(), HashMap::new());
            let status = sensei_controller.get_node_status(&n.name).await;
            match status {
                Some(s) => {
                    self.pub_key_map.insert(s.pub_key, n.name.clone());
                    self.results.balance.on_chain.get_mut(&n.name).unwrap().insert(0, s.balance.onchain);
                    self.results.balance.off_chain.get_mut(&n.name).unwrap().insert(0, s.balance.offchain);
                    self.results.status.nodes.get_mut(&n.name).unwrap().insert(0, n.running);
                    // Initialize channels
                    for c in s.channels {
                        if c.is_outbound {
                            let sc = SimChannel {
                                src_node: n.name.clone(),
                                dest_node: NetworkAnalyzer::get_dest_node(&network.channels, c.id),
                                src_balance: c.outbound_capacity / 1000,
                                dest_balance: c.inbound_capacity / 1000,
                                id: c.id.clone(),
                                short_id: c.short_id.clone()
                            };
                            self.results.channels.open_channels.get_mut(&0).unwrap().push(sc);
                        }
                    }
                },
                None => {}
            }
        }
    }

    /*
     * Receive results events and update the results
     */
    pub fn process_events(&mut self, mut results_channel: broadcast::Receiver<SimResultsEvent>) {
         tokio::task::block_in_place(move || {
            self.analyzer_runtime_handle.clone().block_on(async move {
                let mut running = true;
                while running {
                    let event = results_channel.recv().await.unwrap();
                    // Match on the SimulationEvent
                    match &event.event {
                        SimulationEvent::StopNodeEvent(name) => {
                            println!("[=== NetworkAnalyzer === {}] StopNodeEvent for {}", crate::get_current_time(), name);
                            if event.success {
                                // The node was successfully stopped, update the status in the results at the event sim time
                                match self.results.status.nodes.get_mut(name) {
                                    Some(n) => {
                                        n.insert(event.sim_time, false);
                                    },
                                    None => {}
                                }

                                // TODO: this will force close all the channels for this node, need to update those channels and nodes in the results... onchain balances included
                            } else {
                                // The node failed to stop, add the event to the list of failed events
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::StartNodeEvent(name) => {
                            println!("[=== NetworkAnalyzer === {}] StartNodeEvent for {}", crate::get_current_time(), name);
                            if event.success {
                                // The node was successfully started, update the status in the results at the event sim time
                                match self.results.status.nodes.get_mut(name) {
                                    Some(n) => {
                                        n.insert(event.sim_time, true);
                                    },
                                    None => {}
                                }
                            } else {
                                // The node failed to start, add the event to the list of failed events
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::CloseChannelEvent(channel) => {
                            println!("[=== NetworkAnalyzer === {}] CloseChannelEvent for {}", crate::get_current_time(), channel.id);
                            if event.success {
                                self.update_close_channel_results(event.sim_time.clone(), channel);
                                // TODO: update onchain balances for the nodes on the ends of this channel
                            } else {
                                // The channel failed to close, add the event to the list of failed events
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("[=== NetworkAnalyzer === {}] OpenChannelEvent for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            if event.success {
                                self.update_open_channel_results(event.sim_time.clone(), channel);
                                // TODO: update the onchain balances for the nodes on the ends of this channel
                            } else {
                                // The channel failed to open, add the event to the list of failed events
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::TransactionEvent(tx) => {
                            println!("[=== NetworkAnalyzer === {}] TransactionEvent for {} <-> {}", crate::get_current_time(), tx.src_node, tx.dest_node);
                            if event.success {
                                // The payment was sent, add it to the list of transactions (it will be updated later with the details if it is successful)
                                let new_tx: Tx = Tx {
                                    time: event.sim_time.clone(),
                                    transaction: tx.clone()
                                };
                                self.results.transactions.txs.push(new_tx);
                            } else {
                                // The payment failed to send, add the event to the list of failed events
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::PaymentPathSuccessful(path) => {
                            // A payment that was sent went through successfully and was received
                            // Update the channel and node balances at the time of this payment
                            // Get the time that this transaction was sent
                            let mut time_option: Option<u64> = None;
                            for transaction in &self.results.transactions.txs {
                                match transaction.transaction.id {
                                    Some(id) => {
                                        if id == &path.payment_id {
                                            time_option = Some(transaction.time.clone());
                                            break;
                                        }
                                    },
                                    None => {}
                                }
                            }
                            let time: u64 = 0;
                            match time_option {
                                Some(t) => {
                                    time = t;
                                }
                                None => {
                                    println!("transaction not found");
                                    return;
                                }
                            }

                            // Update the channel balances along the path
                            let prev_open_list = match self.results.channels.open_channels.keys().copied().max() {
                                Some(k) => {
                                    self.results.channels.open_channels.get(&k).unwrap().clone()
                                },
                                None => {
                                    Vec::new()
                                }
                            };
                            self.update_open_channel_balances(time, prev_open_list, &path.path);

                            // Update the node balances along the path
                            for p in &path.path {
                                let hop_node_name = self.pub_key_map.get(&p.node_pub_key).unwrap();
                                self.update_off_chain_balance(time, hop_node_name, p.amount, false);
                            }
                        },
                        SimulationEvent::PaymentFailedEvent(id) => {
                            // A payment that was sent did not go through successfully, set the status to FAILED
                            // Do not update any balances
                            for mut t in &mut self.results.transactions.txs {
                                if &t.transaction.id.clone().unwrap() == id {
                                    t.transaction.status = SimTransactionStatus::FAILED;
                                }
                            };
                        },
                        SimulationEvent::PaymentSuccessEvent(id, fee) => {
                            // A payment that was sent went through successfully and was received
                            // Get the transaction that this success event corresponds too and the time it was sent
                            let mut current_tx: Option<SimTransaction> = None;
                            let mut time: Option<u64> = None;
                            for mut t in &mut self.results.transactions.txs {
                                if &t.transaction.id.clone().unwrap() == id {
                                    // Set the status to successful
                                    t.transaction.status = SimTransactionStatus::SUCCESSFUL;
                                    current_tx = Some(t.transaction.clone());
                                    time = Some(t.time.clone());
                                }
                            };

                            // Update the offchain balance for the source node (sending node)
                            match current_tx {
                                Some(t) => {
                                    self.update_off_chain_balance(time.clone(), &t.src_node, t.amount + fee, true)
                                },
                                None => {
                                    println!("Transaction for this success event was not found");
                                }
                            }
                        },
                        SimulationEvent::SimulationEndedEvent => {
                            running = false;
                        },
                    }
                }
            })
        });
    }

    /*
     * Update the results for closing a channel. Add the channel to the list of closed channels and remove it from the list of open channels
     */
    fn update_close_channel_results(&mut self, time: u64, channel: &SimChannel) {
        // Get the most current list of closed channels or a new list if none exist yet
        let mut new_list = match self.results.channels.closed_channels.keys().copied().max() {
            Some(k) => {
                self.results.channels.closed_channels.get(&k).unwrap().clone()
            },
            None => {
                Vec::new()
            }
        };
        // Add the channel to the list of closed channels
        new_list.push(channel.clone());
        self.results.channels.closed_channels.insert(time, new_list);

        // Remove the channel from the list of open channels
        match self.results.channels.open_channels.keys().copied().max() {
            Some(k) => {
                let new_open_list = self.results.channels.open_channels.get(&k).unwrap().clone().iter().filter(|&c| c.id != channel.id).cloned().collect();
                self.results.channels.open_channels.insert(time, new_open_list);
            },
            None => {}
        }
    }

    /*
     * Update the results for opening a channel. Add the new channel to the list of open channels 
     */
    fn update_open_channel_results(&mut self, time: u64, channel: &SimChannel) {
        // Get the most current list of open channels or a new list if none exist yet
        let mut new_open_list = match self.results.channels.open_channels.keys().copied().max() {
            Some(k) => {
                self.results.channels.open_channels.get(&k).unwrap().clone()
            },
            None => {
                Vec::new()
            }
        };
        
        // Add the new channel
        new_open_list.push(channel.clone());
        self.results.channels.open_channels.insert(time, new_open_list);
    }

    /*
     * Update the off chain balance for a node at a certain time
     */
    fn update_off_chain_balance(&mut self, time: u64, node: &String, amount: u64, sent: bool) {
        match self.results.balance.off_chain.get_mut(node) {
            Some(hm) => {
                // Get the previous balance for this node
                let prev_bal = match hm.keys().copied().max() {
                    Some(k) => {
                        hm.get(&k).unwrap().clone()
                    },
                    None => {
                        0
                    }
                };

                // If this node is sending a payment subtract the amount from the previous balance, otherwise add it.
                if sent {
                    hm.insert(time.unwrap(), prev_bal - amount);
                } else {
                    hm.insert(time.unwrap(), prev_bal + amount);
                }
                
            },
            None => {
                println!("node not found");
            }
        }
    }

    /*
     * Update the channel balances along a payment path
     */
    fn update_open_channel_balances(&mut self, time: u64, previous_list: Vec<SimChannel>, path: &Vec<PathHop>) {
        let mut open_list: Vec<SimChannel> = Vec::new();
        // Loop through the previous list of open channels and find the channel that is being used
        for prev_channel in previous_list {
            for node in path {
                let hop_node_name = self.pub_key_map.get(&node.node_pub_key).unwrap();
                if prev_channel.short_id.is_some() && &prev_channel.short_id.unwrap() == &node.short_channel_id {
                    // If the node for this hop is the source node of the channel then increase the src balance and decrease the dest balance
                    if hop_node_name == &prev_channel.src_node {
                        let new_chan = SimChannel {
                            id: c.id,
                            src_node: c.src_node.clone(),
                            dest_node: c.dest_node.clone(),
                            short_id: c.short_id,
                            dest_balance: c.dest_balance - p.amount,
                            src_balance: c.src_balance + p.amount
                        };
                        open_list.push(new_chan);
                    }

                    // If the node for this hop is the destination node of the channel then increase the dest balance and decrease the src balance
                    if hop_node_name == &c.dest_node {
                        let new_chan = SimChannel {
                            id: c.id,
                            src_node: c.src_node.clone(),
                            dest_node: c.dest_node.clone(),
                            short_id: c.short_id,
                            dest_balance: c.dest_balance + p.amount,
                            src_balance: c.src_balance - p.amount
                        };
                        open_list.push(new_chan);
                    }
                }
            }
        }

        // Add the new list to the open channels
        self.results.channels.open_channels.insert(t.clone(), open_list);
    }

    /* 
     * Get the destination node name for a channel
     */
    fn get_dest_node(channels: &Vec<SimChannel>, id: u64) -> String {
        for c in channels {
            if c.id == id {
                return c.dest_node.clone();
            }
        }

        return String::from("");
    }

    /*
     * Get the simulation results
     */
    pub fn get_sim_results(&self) -> SimResults {
        self.results.clone()
    }
}