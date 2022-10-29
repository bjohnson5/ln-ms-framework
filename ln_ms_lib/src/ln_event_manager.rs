use crate::ln_event::LnEvent;
use crate::LnSimulation;

pub struct LnEventManager {
    pub events: Vec<Box<dyn LnEvent>>
}

impl LnEventManager {
    pub fn new() -> Self {
        let em = LnEventManager {
            events: Vec::new()
        };

        em
    }

    // Run the EventManager thread using the borrowed LnSimulation object to borrow the LnNode that each event is for
    pub fn run(&self, sim: &LnSimulation) {
        println!("Running LnEventManager");
        for e in &self.events {
            println!("Running a {} event", e.get_name());
            let node = &sim.nodes[&e.get_node_name()];
            e.execute(node);
        }
    }

    // Add LnEvent object to the list of events to execute
    pub fn add_event(&mut self, event: Box<dyn LnEvent>) {
        self.events.push(event);
    }
}