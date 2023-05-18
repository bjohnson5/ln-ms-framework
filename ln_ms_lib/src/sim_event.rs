// Project Modules
use crate::sim_channel::SimChannel;
use crate::sim_transaction::SimTransaction;

// This enum represents all of the events that can be added to a simulation
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    StartNodeEvent(String),
    StopNodeEvent(String),
    OpenChannelEvent(SimChannel),
    CloseChannelEvent(SimChannel),
    TransactionEvent(SimTransaction),
    PaymentPathSuccessful(SimPaymentPath),
    PaymentFailedEvent(String),
    PaymentSuccessEvent(String, u64),
    SimulationEndedEvent
}

#[derive(Clone, Debug)]
pub struct SimPaymentPath {
    pub payment_id: String,
    pub path: Vec<PathHop>
}

#[derive(Clone, Debug)]
pub struct PathHop {
    pub short_channel_id: u64,
    pub amount: u64,
    pub node_pub_key: String
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
    pub event: SimulationEvent
}