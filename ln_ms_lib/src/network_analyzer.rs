// Project modules
use crate::sim_event::SimResultsEvent;
use crate::sim_event::SimulationEvent;

// External modules
use tokio::sync::broadcast;

pub struct NetworkAnalyzer {
    analyzer_runtime_handle: tokio::runtime::Handle
}

impl NetworkAnalyzer {
    // Create a new NetworkAnalyzer
    pub fn new(runtime_handle: tokio::runtime::Handle) -> Self {
        let analyzer = NetworkAnalyzer {
            analyzer_runtime_handle: runtime_handle
        };

        analyzer
    }

    // Receive results events and make the appropriate calls to the Sensei library
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

    pub fn get_sim_results(&self) -> String {
        String::from("test")
    }
}