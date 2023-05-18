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

// External modules
use tokio::sync::broadcast;

// Standard modules
use std::collections::HashMap;

pub struct NetworkAnalyzer {
    analyzer_runtime_handle: tokio::runtime::Handle,
    results: SimResults,
    pub_key_map: HashMap<String, String>
}

impl NetworkAnalyzer {
    // Create a new NetworkAnalyzer
    pub fn new(runtime_handle: tokio::runtime::Handle) -> Self {
        let analyzer = NetworkAnalyzer {
            analyzer_runtime_handle: runtime_handle,
            results: SimResults::new(),
            pub_key_map: HashMap::new()
        };

        analyzer
    }

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

    fn get_dest_node(channels: &Vec<SimChannel>, id: u64) -> String {
        for c in channels {
            if c.id == id {
                return c.dest_node.clone();
            }
        }

        return String::from("");
    }

    // Receive results events and update the results
    pub fn process_events(&mut self, mut results_channel: broadcast::Receiver<SimResultsEvent>) {
        // This is the main thread for processing result events
         tokio::task::block_in_place(move || {
            self.analyzer_runtime_handle.clone().block_on(async move {
                let mut running = true;
                while running {
                    let event = results_channel.recv().await.unwrap();
                    match &event.event {
                        SimulationEvent::StopNodeEvent(name) => {
                            println!("[=== NetworkAnalyzer === {}] StopNodeEvent for {}", crate::get_current_time(), name);
                            if event.success {
                                match self.results.status.nodes.get_mut(name) {
                                    Some(n) => {
                                        n.insert(event.sim_time, false);
                                    },
                                    None => {}
                                }
                            } else {
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::StartNodeEvent(name) => {
                            println!("[=== NetworkAnalyzer === {}] StartNodeEvent for {}", crate::get_current_time(), name);
                            if event.success {
                                match self.results.status.nodes.get_mut(name) {
                                    Some(n) => {
                                        n.insert(event.sim_time, true);
                                    },
                                    None => {}
                                }
                            } else {
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::CloseChannelEvent(channel) => {
                            println!("[=== NetworkAnalyzer === {}] CloseChannelEvent for {}", crate::get_current_time(), channel.id);
                            if event.success {
                                let mut new_list = match self.results.channels.closed_channels.keys().copied().max() {
                                    Some(k) => {
                                        self.results.channels.closed_channels.get(&k).unwrap().clone()
                                    },
                                    None => {
                                        Vec::new()
                                    }
                                };
                                new_list.push(channel.clone());
                                self.results.channels.closed_channels.insert(event.sim_time.clone(), new_list);

                                match self.results.channels.open_channels.keys().copied().max() {
                                    Some(k) => {
                                        let new_open_list = self.results.channels.open_channels.get(&k).unwrap().clone().iter().filter(|&c| c.id != channel.id).cloned().collect();
                                        self.results.channels.open_channels.insert(event.sim_time.clone(), new_open_list);
                                    },
                                    None => {}
                                }

                                // TODO: update onchain balances for the nodes on the ends of this channel
                            } else {
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("[=== NetworkAnalyzer === {}] OpenChannelEvent for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            if event.success {
                                let mut new_open_list = match self.results.channels.open_channels.keys().copied().max() {
                                    Some(k) => {
                                        self.results.channels.open_channels.get(&k).unwrap().clone()
                                    },
                                    None => {
                                        Vec::new()
                                    }
                                };
                                
                                new_open_list.push(channel.clone());
                                self.results.channels.open_channels.insert(event.sim_time.clone(), new_open_list);

                                // TODO: update the onchain balances for the nodes on the ends of this channel
                            } else {
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::TransactionEvent(tx) => {
                            println!("[=== NetworkAnalyzer === {}] TransactionEvent for {} <-> {}", crate::get_current_time(), tx.src_node, tx.dest_node);
                            if event.success {
                                let new_tx: Tx = Tx {
                                    time: event.sim_time.clone(),
                                    success: true,
                                    transaction: tx.clone()
                                };
                                self.results.transactions.txs.push(new_tx);
                            } else {
                                self.results.failed_events.push(event.clone());
                            }
                        },
                        SimulationEvent::PaymentPathSuccessful(path) => {
                            let mut time: Option<u64> = None;
                            for t in &self.results.transactions.txs {
                                if t.transaction.id.is_some() && t.transaction.id.as_ref().unwrap() == &path.payment_id {
                                    time = Some(t.time.clone());
                                }
                            }

                            match time {
                                Some(t) => {
                                    let new_open_list = match self.results.channels.open_channels.keys().copied().max() {
                                        Some(k) => {
                                            self.results.channels.open_channels.get(&k).unwrap().clone()
                                        },
                                        None => {
                                            Vec::new()
                                        }
                                    };

                                    let mut open_list: Vec<SimChannel> = Vec::new();
                                    for c in new_open_list {
                                        for p in &path.path {
                                            let hop_node_name = self.pub_key_map.get(&p.node_pub_key).unwrap();
                                            if c.short_id.is_some() && &c.short_id.unwrap() == &p.short_channel_id {
                                                if hop_node_name == &c.src_node {
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
                                    self.results.channels.open_channels.insert(t.clone(), open_list);

                                    for p in &path.path {
                                        let hop_node_name = self.pub_key_map.get(&p.node_pub_key).unwrap();
                                        match self.results.balance.off_chain.get_mut(hop_node_name) {
                                            Some(hm) => {
                                                let prev_bal = match hm.keys().copied().max() {
                                                    Some(k) => {
                                                        hm.get(&k).unwrap().clone()
                                                    },
                                                    None => {
                                                        0
                                                    }
                                                };

                                                hm.insert(time.unwrap(), prev_bal + p.amount);
                                            },
                                            None => {}
                                        }
                                    }
                                },
                                None => {
                                    println!("transaction not found");
                                }
                            }
                        },
                        SimulationEvent::PaymentFailedEvent(id) => {
                            for mut t in &mut self.results.transactions.txs {
                                if &t.transaction.id.clone().unwrap() == id {
                                    t.transaction.status = SimTransactionStatus::FAILED;
                                }
                            };
                        },
                        SimulationEvent::PaymentSuccessEvent(id, fee) => {
                            let mut current_tx: Option<SimTransaction> = None;
                            let mut time: Option<u64> = None;
                            for mut t in &mut self.results.transactions.txs {
                                if &t.transaction.id.clone().unwrap() == id {
                                    t.transaction.status = SimTransactionStatus::SUCCESSFUL;
                                    current_tx = Some(t.transaction.clone());
                                    time = Some(t.time.clone());
                                }
                            };

                            match current_tx {
                                Some(t) => {
                                    match self.results.balance.off_chain.get_mut(&t.src_node) {
                                        Some(hm) => {
                                            let prev_bal = match hm.keys().copied().max() {
                                                Some(k) => {
                                                    hm.get(&k).unwrap().clone()
                                                },
                                                None => {
                                                    0
                                                }
                                            };

                                            hm.insert(time.unwrap(), prev_bal - t.amount - fee);
                                        },
                                        None => {}
                                    }
                                },
                                None => {}
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

    pub fn get_sim_results(&self) -> SimResults {
        self.results.clone()
    }
}