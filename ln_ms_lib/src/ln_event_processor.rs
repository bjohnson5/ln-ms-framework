// Project modules
use crate::sim_event::{SimResultsEvent, SimulationEvent, SimEvent, SimPaymentPath, PathHop};

// External modules
use tokio::sync::broadcast;

// Sensei and LDK modules
use lightning::util::events::Event;
use senseicore::hex_utils;

/*
 * This struct processes LDK events and sends the appropriate simulation results event.
 * Translates LDK events into simulation results
 */
pub struct LnEventProcessor {
    ln_event_runtime_handle: tokio::runtime::Handle,
}

impl LnEventProcessor {
    pub fn new(runtime_handle: tokio::runtime::Handle) -> Self {
        let proc = LnEventProcessor {
            ln_event_runtime_handle: runtime_handle,
        };

        proc
    }

    /*
     * Receive ldk events and update the results. 
     * Each LDK node will have its own sender, so sim_receivers is the list of receivers that correspond to the senders
     */
    pub fn process_events(&self, sim_receivers: Vec<broadcast::Receiver<Event>>, sim_results_sender: broadcast::Sender<SimResultsEvent>, mut sim_event_receiver: broadcast::Receiver<SimEvent>) {
         tokio::task::block_in_place(move || {
            self.ln_event_runtime_handle.clone().block_on(async move {
                // Start a thread for each of the ldk receivers and save the handles
                let mut handles= Vec::new();
                for r in sim_receivers {
                    let h = tokio::spawn(LnEventProcessor::node_receive(r, sim_results_sender.clone()));
                    handles.push(h);
                }

                // Listen for the end of the simulation
                let mut running = true;
                while running {
                    let event = sim_event_receiver.recv().await.unwrap();
                    match &event.event {
                        SimulationEvent::SimulationEndedEvent => {
                            println!("[=== LnEventProcessor === {}] SimulationEndedEvent", crate::get_current_time());
                            running = false;
                        },
                        _ => {
                            // Ignore all other events
                        }
                    }
                }

                // Stop all the receiver threads when the simulation has ended
                for h in handles {
                    h.abort();
                }
            });           
        });
    }

    /*
     * Receives events from ldk and updates the simulation results as needed
     */
    async fn node_receive(mut rec: broadcast::Receiver<Event>, sender: broadcast::Sender<SimResultsEvent>) {
        loop {
            // Listen for events coming from the ldk nodes
            let event = match rec.recv().await {
                Ok(e) => {e},
                Err(_) => {
                    // Stop the thread if the receive fails
                    return;
                }
            };
            match &event {
                Event::PaymentSent{payment_id: pay_id, payment_preimage: _, payment_hash: _, fee_paid_msat: fee} => {
                    match pay_id {
                        Some(id) => {
                            // This payment was successful, send the sim event to the network analyzer with the payment_id and amount
                            let fee_paid = if fee.is_some() { fee.unwrap() / 1000 } else {0};
                            let simevent = SimulationEvent::PaymentSuccessEvent(hex_utils::hex_str(&id.0), fee_paid);
                            let e = SimResultsEvent {
                                sim_time: None,
                                success: true,
                                event: simevent
                            };
                            sender.send(e).expect("could not send the event");
                        },
                        None => { println!("no payment id supplied, not sending event") }
                    }
                },
                Event::PaymentFailed{payment_id: id, payment_hash: _} => {
                    // This payment failed, send the sim event to the network analyzer with the payment id
                    let simevent = SimulationEvent::PaymentSuccessEvent(hex_utils::hex_str(&id.0), 0);
                    let e = SimResultsEvent {
                        sim_time: None,
                        success: true,
                        event: simevent
                    };
                    sender.send(e).expect("could not send the event");
                },
                Event::PaymentPathSuccessful { payment_id: id, payment_hash: _, path: p} => {
                    // This event comes after the PaymentSent event and identifies the path that the payment took
                    let mut path: Vec<PathHop> = Vec::new();
                    for hop in p {
                        let path_hop = PathHop {
                            short_channel_id: hop.short_channel_id,
                            amount: hop.fee_msat / 1000,
                            node_pub_key: hop.pubkey.to_string()
                        };
                        path.push(path_hop);
                    }

                    // Create a SimPaymentPath that will be used to update the simulation results for all the channels and nodes in the path
                    let sim_payment_path = SimPaymentPath {path: path, payment_id: hex_utils::hex_str(&id.0)};

                    let simevent = SimulationEvent::PaymentPathSuccessful(sim_payment_path);
                    let e = SimResultsEvent {
                        sim_time: None,
                        success: true,
                        event: simevent
                    };
                    sender.send(e).expect("could not send the event");
                },
                Event::ChannelClosed { channel_id, ..} => {
                    let simevent = SimulationEvent::CloseChannelSuccessEvent(hex_utils::hex_str(channel_id));
                    let e = SimResultsEvent {
                        sim_time: None,
                        success: true,
                        event: simevent
                    };
                    sender.send(e).expect("could not send the event");
                }
                _ => {
                    // Ignore all other events
                }
            }
        }
    }
}