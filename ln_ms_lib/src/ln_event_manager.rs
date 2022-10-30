use crate::ln_event::LnEvent;
use crate::LnSimulation;
use std::collections::HashMap;
use std::{thread, time};

// This struct holds all of the events that will take place over the duration of the simulation
pub struct LnEventManager {
    pub events: HashMap<u64, Vec<Box<dyn LnEvent>>> 
}

impl LnEventManager {
    pub fn new() -> Self {
        let em = LnEventManager {
            events: HashMap::new()
        };

        em
    }

    // Run the EventManager thread using the borrowed LnSimulation object to borrow the LnNode that each event is for

    // TODO: This function will need to execute in its own thread
    // It will need to not sleep and to have scheduled executions in its own thread
    // For now, in order to create a proof of concept and demonstrate the project it will simply run in a loop and sleep

    // TODO: This function is based on seconds and runs at real time, eventually it will need to be able to run for
    // longer durations and at a faster-than-real-time rate

    pub fn run(&self, sim: &LnSimulation) {
        println!("Running LnEventManager for simulation: {} for {} seconds", sim.name, sim.duration);
        let one_sec = time::Duration::from_secs(1);
        let mut current_sec = 0;
        while current_sec <= sim.duration {
            if self.events.contains_key(&current_sec) {
                // There is at least one event at this time
                let current_events = &self.events[&current_sec];
                let current_events_iter = current_events.iter();
                for e in current_events_iter {
                    println!("Running a {} event for {}", e.get_name(), e.get_node_name());
                    let node = &sim.nodes[&e.get_node_name()];
                    e.execute(node);
                }
            }

            current_sec += 1;
            thread::sleep(one_sec);
        }
    }

    // Add LnEvent object to the list of events to execute
    pub fn add_event(&mut self, event: Box<dyn LnEvent>, time: u64) {
        if self.events.contains_key(&time) {
            let current_events = self.events.get_mut(&time).unwrap();
            current_events.push(event);
        } else {
            let mut current_events: Vec<Box<dyn LnEvent>> = Vec::new();
            current_events.push(event);
            self.events.insert(time, current_events);
        }
    }
}