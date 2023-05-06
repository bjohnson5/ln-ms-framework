// Project Modules
use crate::sim_channel::SimChannel;
use crate::sim_transaction::SimTransaction;

// This enum represents all of the events that can be added to a simulation
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    StartNodeEvent(String),
    StopNodeEvent(String),
    OpenChannelEvent(SimChannel),
    CloseChannelEvent(String, u64),
    TransactionEvent(SimTransaction),
    NodeStatusEvent(String),
    SimulationEndedEvent
}