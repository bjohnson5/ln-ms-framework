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

// Sensei modules
use senseicore::chain::bitcoind_client::BitcoindClient;

// Standard modules
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/*
 * Processes results and creates the SimResults object that will get returned after the simulation ends
 */
pub struct NetworkAnalyzer {
    analyzer_runtime_handle: tokio::runtime::Handle,
    results: SimResults,
    pub_key_map: HashMap<String, String>,
    bitcoind_client: Arc<BitcoindClient>,
    finalized_closed_channels: Vec<String>
}

impl NetworkAnalyzer {
    pub fn new(runtime_handle: tokio::runtime::Handle, bitcoind_client: Arc<BitcoindClient>) -> Self {
        let analyzer = NetworkAnalyzer {
            analyzer_runtime_handle: runtime_handle,
            results: SimResults::new(),
            pub_key_map: HashMap::new(),
            bitcoind_client: bitcoind_client,
            finalized_closed_channels: Vec::new()
        };

        analyzer
    }

    /*
     * Set up the initial state of the network at sim time = 0
     */
    pub async fn initialize_network(&mut self, network: &RuntimeNetworkGraph, sensei_controller: &SenseiController) {
        self.results.channels.open_channels.insert(0, Vec::new());
        self.results.event_times.push(0);

        // Initialize balances and status
        for n in &network.nodes {
            self.results.balance.on_chain.insert(n.name.clone(), HashMap::new());
            self.results.balance.off_chain.insert(n.name.clone(), HashMap::new());
            self.results.status.nodes.insert(n.name.clone(), HashMap::new());
            let status = sensei_controller.get_node_status(&n.name, None, None).await;
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
                                short_id: c.short_id.clone(),
                                run_time_id: Some(c.run_time_id),
                                funding_tx: c.funding_tx
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
                    if event.sim_time.is_some() && !self.results.event_times.contains(&event.sim_time.unwrap()){
                        self.results.event_times.push(event.sim_time.unwrap());
                    }
                    // Match on the SimulationEvent
                    match &event.event {
                        SimulationEvent::StopNodeEvent(name) => {
                            println!("[=== NetworkAnalyzer === {}] StopNodeEvent for {}", crate::get_current_time(), name);
                            if event.success {
                                // The node was successfully stopped, update the status in the results at the event sim time
                                match self.results.status.nodes.get_mut(name) {
                                    Some(n) => {
                                        n.insert(event.sim_time.unwrap(), false);
                                    },
                                    None => {}
                                }
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
                                        n.insert(event.sim_time.unwrap(), true);
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
                                let chan = self.get_last_open_channel_status(channel.id);
                                match chan {
                                    Some(c) => {
                                        self.update_close_channel_results(event.sim_time.unwrap().clone(), &c);
                                    },
                                    None => {}
                                }
                            } else {
                                // The channel failed to close, add the event to the list of failed events
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::CloseChannelSuccessEvent(id) => {
                            println!("[=== NetworkAnalyzer === {}] CloseChannelSuccessEvent for {}", crate::get_current_time(), id);
                            let chan = self.get_last_closed_channel_status(id.clone());
                            match chan {
                                Some((c, t)) => {
                                    self.update_off_chain_balance(t, &c.src_node, c.src_balance + 1000, true);
                                    self.update_off_chain_balance(t, &c.dest_node, c.dest_balance, true);

                                    let mut pending_channel_closes: Vec<(SimChannel, u64)> = Vec::new();
                                    pending_channel_closes.push((c.clone(), t));

                                    while !pending_channel_closes.is_empty() {
                                        let closes = pending_channel_closes.clone();
                                        pending_channel_closes.clear();
                                        for (c, t) in closes {                                            
                                            match self.get_closing_fees(c.src_balance, c.funding_tx.clone()).await {
                                                Some(fee) => {
                                                    self.update_on_chain_balance(t, &c.src_node, c.src_balance + 1000 - fee.0, true);
                                                    self.update_on_chain_balance(t, &c.dest_node, c.dest_balance - fee.1, true);
                                                },
                                                None => {
                                                    pending_channel_closes.push((c, t));
                                                }
                                            }
                                        }
                                        tokio::time::sleep(Duration::from_secs(1)).await;
                                    }
                                    self.finalized_closed_channels.push(id.clone());
                                },
                                None => {}
                            }
                        }
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("[=== NetworkAnalyzer === {}] OpenChannelEvent for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            if event.success {
                                self.update_open_channel_results(event.sim_time.unwrap().clone(), channel);
                                self.update_off_chain_balance(event.sim_time.unwrap().clone(), &channel.src_node, channel.src_balance + 1000, false);
                                self.update_off_chain_balance(event.sim_time.unwrap().clone(), &channel.dest_node, channel.dest_balance, false);

                                let src_open_fee = self.get_open_fees(channel.funding_tx.clone()).await;
                                self.update_on_chain_balance(event.sim_time.unwrap().clone(), &channel.src_node, src_open_fee + channel.src_balance + channel.dest_balance + 1000, false);
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
                                    time: event.sim_time.unwrap().clone(),
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
                            let payment_amount: u64 = path.path.last().unwrap().amount;
                            for transaction in &self.results.transactions.txs {
                                match &transaction.transaction.id {
                                    Some(id) => {
                                        if id == &path.payment_id {
                                            time_option = Some(transaction.time.clone());
                                            break;
                                        }
                                    },
                                    None => {}
                                }
                            }

                            let time: u64;
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
                            self.update_open_channel_balances(time.clone(), prev_open_list, &path.path, payment_amount);

                            // Update the node balances along the path
                            for p in &path.path {
                                let hop_node_name = self.pub_key_map.get(&p.node_pub_key).unwrap().clone();
                                self.update_off_chain_balance(time.clone(), &hop_node_name, p.amount, false);
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
                            let mut time: u64 = 0;
                            for mut t in &mut self.results.transactions.txs {
                                if &t.transaction.id.clone().unwrap() == id {
                                    // Set the status to successful
                                    t.transaction.status = SimTransactionStatus::SUCCESSFUL;
                                    current_tx = Some(t.transaction.clone());
                                    time = t.time.clone();
                                }
                            };

                            // Update the offchain balance for the source node (sending node)
                            match current_tx {
                                Some(t) => {
                                    self.update_off_chain_balance(time, &t.src_node, t.amount + fee, true)
                                },
                                None => {
                                    println!("Transaction for this success event was not found");
                                }
                            }
                        },
                        SimulationEvent::SimulationEndedEvent => {
                            running = false;
                        }
                    }
                }
            })
        });
    }

    /*
     * Get the simulation results
     */
    pub fn get_sim_results(&self) -> SimResults {
        self.results.clone()
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
                    hm.insert(time, prev_bal - amount);
                } else {
                    hm.insert(time, prev_bal + amount);
                }
                
            },
            None => {
                println!("node not found");
            }
        }
    }

    /*
     * Update the on chain balance for a node at a certain time
     */
    fn update_on_chain_balance(&mut self, time: u64, node: &String, amount: u64, close: bool) {
        match self.results.balance.on_chain.get_mut(node) {
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
                if close {
                    hm.insert(time, prev_bal + amount);
                } else {
                    hm.insert(time, prev_bal - amount);
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
    fn update_open_channel_balances(&mut self, time: u64, previous_list: Vec<SimChannel>, path: &Vec<PathHop>, payment_amount: u64) {
        let mut open_list: Vec<SimChannel> = Vec::new();
        let mut updated: bool;
        // Loop through the previous list of open channels and find the channel that is being used
        for prev_channel in previous_list {
            updated = false;
            for node in path {
                let amount: u64;
                if payment_amount == node.amount {
                    amount = payment_amount;
                } else {
                    amount = payment_amount + node.amount;
                }
                let hop_node_name = self.pub_key_map.get(&node.node_pub_key).unwrap();
                if prev_channel.short_id.is_some() && &prev_channel.short_id.unwrap() == &node.short_channel_id {
                    // If the node for this hop is the source node of the channel then increase the src balance and decrease the dest balance
                    if hop_node_name == &prev_channel.src_node {
                        let new_chan = SimChannel {
                            id: prev_channel.id,
                            src_node: prev_channel.src_node.clone(),
                            dest_node: prev_channel.dest_node.clone(),
                            short_id: prev_channel.short_id,
                            run_time_id: prev_channel.run_time_id.clone(),
                            dest_balance: prev_channel.dest_balance - amount,
                            src_balance: prev_channel.src_balance + amount,
                            funding_tx: prev_channel.funding_tx.clone()
                        };
                        open_list.push(new_chan);
                        updated = true;
                    }

                    // If the node for this hop is the destination node of the channel then increase the dest balance and decrease the src balance
                    if hop_node_name == &prev_channel.dest_node {
                        let new_chan = SimChannel {
                            id: prev_channel.id,
                            src_node: prev_channel.src_node.clone(),
                            dest_node: prev_channel.dest_node.clone(),
                            short_id: prev_channel.short_id,
                            run_time_id: prev_channel.run_time_id.clone(),
                            dest_balance: prev_channel.dest_balance + amount,
                            src_balance: prev_channel.src_balance - amount,
                            funding_tx: prev_channel.funding_tx.clone()
                        };
                        open_list.push(new_chan);
                        updated = true;
                    }
                }
            }

            if !updated {
                open_list.push(prev_channel.clone());
            }
        }

        // Add the new list to the open channels
        self.results.channels.open_channels.insert(time, open_list);
    }

    async fn get_open_fees(&self, funding_tx: Option<String>) -> u64 {
        (self.bitcoind_client.get_tx_fees(funding_tx.clone().unwrap()).await * 100000000.0).round() as u64
    }

    async fn get_closing_fees(&self, src_chan_balance: u64, funding_tx: Option<String>) -> Option<(u64, u64)> {
        match self.bitcoind_client.find_input(funding_tx.clone().unwrap()).await {
            Some(closing_tx) => {
                let src_bal = src_chan_balance as f64 / 100000000.0;
                if self.bitcoind_client.is_output_value(closing_tx.clone(), src_bal).await {
                    Some((0, (self.bitcoind_client.get_tx_fees(closing_tx.clone()).await * 100000000.0) as u64))
                } else {
                    Some(((self.bitcoind_client.get_tx_fees(closing_tx.clone()).await * 100000000.0).round() as u64, 0))
                }
            },
            None => {
                None
            }
        }
    }

    fn get_last_closed_channel_status(&self, id: String) -> Option<(SimChannel, u64)> {
        if self.finalized_closed_channels.contains(&id) {
            return None;
        }

        let mut time: u64 = 0;
        let prev_open_list = match self.results.channels.closed_channels.keys().copied().max() {
            Some(k) => {
                time = k.clone();
                self.results.channels.closed_channels.get(&k).unwrap().clone()
            },
            None => {
                Vec::new()
            }
        };

        for c in prev_open_list {
            if c.run_time_id.is_some() && c.run_time_id.clone().unwrap() == id {
                return Some((c.clone(), time));
            }
        }

        return None;
    }

    fn get_last_open_channel_status(&self, id: u64) -> Option<SimChannel> {
        let prev_open_list = match self.results.channels.open_channels.keys().copied().max() {
            Some(k) => {
                self.results.channels.open_channels.get(&k).unwrap().clone()
            },
            None => {
                Vec::new()
            }
        };

        for c in prev_open_list {
            if c.id == id {
                return Some(c.clone());
            }
        }

        return None;
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
}