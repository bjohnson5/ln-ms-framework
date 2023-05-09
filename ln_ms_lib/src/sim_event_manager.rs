// Project Modules
use crate::sim_event::{SimulationEvent, SimEvent};

// Standard Modules
use std::collections::HashMap;
use std::{thread, time};

// External Modules
use tokio::sync::broadcast;

/*
 * This struct holds all of the events that will take place over the duration of the simulation
 * and executes them at the correct time
 */ 
#[derive(Clone)]
pub struct SimEventManager {
    events: HashMap<u64, Vec<SimulationEvent>> 
}

impl SimEventManager {
    pub fn new(sim_events: HashMap<u64, Vec<SimulationEvent>>) -> Self {
        let event_manager = SimEventManager {
            events: sim_events
        };

        event_manager
    }

    /* TODO: 
     * - this function will need to not sleep and to have scheduled executions in its own thread
     * - for now, in order to create a proof of concept and demonstrate the project it will simply run in a loop and sleep
     * - this function is based on seconds and runs at real time, eventually it will need to be changed to a purely event based design
     *   to allow for faster-than-real-time simulation runs
     */
    // Send SimulationEvent objects through the event channel at the correct simulation time
    pub fn run(&self, duration: u64, event_channel: broadcast::Sender<SimEvent>) {
        println!("[=== SimEventManager === {}] Running SimEventManager for {} seconds", crate::get_current_time(), duration);
        let one_sec = time::Duration::from_secs(1);
        let mut current_sec = 0;
        while current_sec <= duration {
            if self.events.contains_key(&current_sec) {
                let current_events = &self.events[&current_sec];
                let current_events_iter = current_events.iter();
                for e in current_events_iter {
                    let sim_event = SimEvent{sim_time: current_sec.clone(), event: e.clone()};
                    event_channel.send(sim_event).expect("could not send the event");
                }
            }

            current_sec += 1;
            thread::sleep(one_sec);
        }
        let sim_event = SimEvent{sim_time: duration, event: SimulationEvent::SimulationEndedEvent};
        event_channel.send(sim_event).expect("could not send the simulation ended event");
    }
}