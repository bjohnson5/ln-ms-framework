// Project Modules
use crate::sim_event::SimulationEvent;

// Standard Modules
use std::collections::HashMap;
use std::{thread, time};
use std::sync::mpsc;

// This struct holds all of the events that will take place over the duration of the simulation
#[derive(Clone)]
pub struct SimEventManager {
    pub events: HashMap<u64, Vec<SimulationEvent>> 
}

impl SimEventManager {
    pub fn new() -> Self {
        let event_manager = SimEventManager {
            events: HashMap::new()
        };

        event_manager
    }

    // TODO: This function will need to execute in its own thread
    // It will need to not sleep and to have scheduled executions in its own thread
    // For now, in order to create a proof of concept and demonstrate the project it will simply run in a loop and sleep

    // TODO: This function is based on seconds and runs at real time, eventually it will need to be able to run for
    // longer durations and at a faster-than-real-time rate

    // Send SimulationEvent objects through the event channel at the correct simulation time
    pub fn run(&self, duration: u64, event_channel: mpsc::Sender<SimulationEvent>) {
        println!("SimEventManager:{} -- running SimEventManager for {} seconds", crate::get_current_time(), duration);
        let one_sec = time::Duration::from_secs(1);
        let mut current_sec = 0;
        while current_sec <= duration {
            if self.events.contains_key(&current_sec) {
                // There is at least one event at this time
                let current_events = &self.events[&current_sec];
                let current_events_iter = current_events.iter();
                for e in current_events_iter {
                    event_channel.send(e.clone()).expect("could not send the event");
                }
            }

            current_sec += 1;
            thread::sleep(one_sec);
        }
        event_channel.send(SimulationEvent::SimulationEnded).expect("could not send the simulation ended event");
    }

    // Add a SimulationEvent to the list of events to execute
    pub fn add_event(&mut self, event: SimulationEvent, time: u64) {
        if self.events.contains_key(&time) {
            let current_events = self.events.get_mut(&time).unwrap();
            current_events.push(event);
        } else {
            let mut current_events: Vec<SimulationEvent> = Vec::new();
            current_events.push(event);
            self.events.insert(time, current_events);
        }
    }
}