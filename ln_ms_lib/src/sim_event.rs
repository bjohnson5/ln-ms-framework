// Project Modules
use crate::sim_channel::SimChannel;
use crate::sim_transaction::SimTransaction;
use crate::sim_node_status::SimNodeStatus;

// This enum represents all of the events that can be added to a simulation
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    StartNodeEvent(String),
    StopNodeEvent(String),
    OpenChannelEvent(SimChannel),
    CloseChannelEvent(SimChannel),
    TransactionEvent(SimTransaction),
    SimulationEndedEvent
}

#[derive(Clone, Debug)]
pub struct SimEvent {
    pub sim_time: u64,
    pub event: SimulationEvent
}

#[derive(Clone, Debug)]
pub struct SimResultsEvent {
    pub sim_time: u64,
    pub success: bool,
    pub event: SimulationEvent,
    pub status: Option<SimNodeStatus>
}