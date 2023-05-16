use std::collections::HashMap;

// Project modules
use crate::sim_event::SimResultsEvent;
use crate::sim_event::SimulationEvent;
use crate::sim_results::SimResults;
use crate::sim_runtime_graph::RuntimeNetworkGraph;
use crate::sensei_controller::SenseiController;
use crate::sim_channel::SimChannel;

// External modules
use tokio::sync::broadcast;

pub struct NetworkAnalyzer {
    analyzer_runtime_handle: tokio::runtime::Handle,
    results: SimResults
}

impl NetworkAnalyzer {
    // Create a new NetworkAnalyzer
    pub fn new(runtime_handle: tokio::runtime::Handle) -> Self {
        let analyzer = NetworkAnalyzer {
            analyzer_runtime_handle: runtime_handle,
            results: SimResults::new()
        };

        analyzer
    }

    pub async fn initialize_network(&mut self, network: &RuntimeNetworkGraph, sensei_controller: &SenseiController) {
        // Initialize balances and status
        self.results.balance.on_chain.insert(0, HashMap::new());
        self.results.balance.off_chain.insert(0, HashMap::new());
        self.results.status.nodes.insert(0, HashMap::new());
        for n in &network.nodes {
            let status = sensei_controller.get_node_status(&n.name).await;
            match status {
                Some(s) => {
                    self.results.balance.on_chain.get_mut(&0).unwrap().insert(n.name.clone(), s.balance.onchain);
                    self.results.balance.off_chain.get_mut(&0).unwrap().insert(n.name.clone(), s.balance.offchain);
                    self.results.status.nodes.get_mut(&0).unwrap().insert(n.name.clone(), n.running);
                },
                None => {}
            }
        }

        // Initialize channels
        self.results.channels.open_channels.insert(0, Vec::new());
        for c in &network.channels {
            let sc = SimChannel {
                src_node: c.src_node.clone(),
                dest_node: c.dest_node.clone(),
                src_balance: c.src_balance.clone(),
                dest_balance: c.dest_balance.clone(),
                id: c.id.clone()
            };
            self.results.channels.open_channels.get_mut(&0).unwrap().push(sc);
        }
    }

    // Receive results events and update the results
    pub fn process_events(&self, mut results_channel: broadcast::Receiver<SimResultsEvent>) {
        // This is the main thread for processing result events
         tokio::task::block_in_place(move || {
            self.analyzer_runtime_handle.block_on(async move {
                let mut running = true;
                while running {
                    let event = results_channel.recv().await.unwrap();
                    match event.event {
                        SimulationEvent::StopNodeEvent(name) => {
                            println!("[=== NetworkAnalyzer === {}] StopNodeEvent for {}", crate::get_current_time(), name);
                            if event.success {
                                //success
                            } else {
                                //fail
                            }
                        },
                        SimulationEvent::StartNodeEvent(name) => {
                            println!("[=== NetworkAnalyzer === {}] StartNodeEvent for {}", crate::get_current_time(), name);
                            if event.success {
                                //success
                            } else {
                                //fail
                            }
                        },
                        SimulationEvent::CloseChannelEvent(channel) => {
                            println!("[=== NetworkAnalyzer === {}] CloseChannelEvent for {}", crate::get_current_time(), channel.id);
                            if event.success {
                                //success
                            } else {
                                //fail
                            }
                        },
                        SimulationEvent::OpenChannelEvent(channel) => {
                            println!("[=== NetworkAnalyzer === {}] OpenChannelEvent for {} <-> {}", crate::get_current_time(), channel.src_node, channel.dest_node);
                            if event.success {
                                //success
                            } else {
                                //fail
                            }
                        },
                        SimulationEvent::TransactionEvent(tx) => {
                            println!("[=== NetworkAnalyzer === {}] TransactionEvent for {} <-> {}", crate::get_current_time(), tx.src_node, tx.dest_node);
                            if event.success {
                                //success
                            } else {
                                //fail
                            }
                        },
                        SimulationEvent::SimulationEndedEvent => {
                            running = false;
                            if event.success {
                                //success
                            } else {
                                //fail
                            }
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