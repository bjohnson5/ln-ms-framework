use lightning::util::events::Event;
use senseicore::hex_utils;
use tokio::sync::broadcast;

use crate::sim_event::{SimResultsEvent, SimulationEvent, SimEvent, SimPaymentPath, PathHop};

pub struct LnEventProcessor {
    ln_event_runtime_handle: tokio::runtime::Handle,
}

impl LnEventProcessor {
    // Create a new NetworkAnalyzer
    pub fn new(runtime_handle: tokio::runtime::Handle) -> Self {
        let proc = LnEventProcessor {
            ln_event_runtime_handle: runtime_handle,
        };

        proc
    }

    // Receive results events and update the results
    pub fn process_events(&self, sim_receivers: Vec<broadcast::Receiver<Event>>, sim_results_sender: broadcast::Sender<SimResultsEvent>, mut sim_event_receiver: broadcast::Receiver<SimEvent>) {
        // This is the main thread for processing events
         tokio::task::block_in_place(move || {
            self.ln_event_runtime_handle.clone().block_on(async move {
                let mut handles= Vec::new();
                for r in sim_receivers {
                    let h = tokio::spawn(LnEventProcessor::node_receive(r, sim_results_sender.clone()));
                    handles.push(h);
                }

                let mut running = true;
                while running {
                    let event = sim_event_receiver.recv().await.unwrap();
                    match &event.event {
                        SimulationEvent::SimulationEndedEvent => {
                            println!("[=== LnEventProcessor === {}] SimulationEndedEvent", crate::get_current_time());
                            running = false;
                        },
                        _ => {
                            //other event
                        }
                    }
                }

                for h in handles {
                    h.abort();
                }
            });           
        });
    }

    async fn node_receive(mut rec: broadcast::Receiver<Event>, sender: broadcast::Sender<SimResultsEvent>) {
        loop {
            let event = match rec.recv().await {
                Ok(e) => {
                    e
                },
                Err(_) => {
                    return;
                }
            };
            match &event {
                Event::PaymentSent{payment_id: id, payment_preimage: _, payment_hash: _, fee_paid_msat: fee} => {
                    match id {
                        Some(i) => {
                            let fee_paid = if fee.is_some() {fee.unwrap()} else {0};
                            let simevent = SimulationEvent::PaymentSuccessEvent(hex_utils::hex_str(&i.0), fee_paid);
                            let e = SimResultsEvent {
                                sim_time: 0,
                                success: true,
                                event: simevent
                            };
                            sender.send(e).expect("could not send the event");
                        },
                        None => {}
                    }
                },
                Event::PaymentFailed{payment_id: id, payment_hash: _} => {
                    let simevent = SimulationEvent::PaymentSuccessEvent(hex_utils::hex_str(&id.0), 0);
                    let e = SimResultsEvent {
                        sim_time: 0,
                        success: true,
                        event: simevent
                    };
                    sender.send(e).expect("could not send the event");
                },
                Event::PaymentPathSuccessful { payment_id: id, payment_hash: _, path: p} => {
                    let mut path: Vec<PathHop> = Vec::new();
                    for hop in p {
                        let path_hop = PathHop {
                            short_channel_id: hop.short_channel_id,
                            amount: hop.fee_msat / 1000,
                            node_pub_key: hop.pubkey.to_string()
                        };
                        path.push(path_hop);
                    }

                    let sim_payment_path = SimPaymentPath {path: path, payment_id: hex_utils::hex_str(&id.0)};

                    let simevent = SimulationEvent::PaymentPathSuccessful(sim_payment_path);
                    let e = SimResultsEvent {
                        sim_time: 0,
                        success: true,
                        event: simevent
                    };
                    sender.send(e).expect("could not send the event");
                }
                _ => {}
            }
        }
    }
}