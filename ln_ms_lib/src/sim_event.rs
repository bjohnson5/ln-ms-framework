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

/*
 * The path that a successful payment took
 */
#[derive(Clone, Debug)]
pub struct SimPaymentPath {
    pub payment_id: String,
    pub path: Vec<PathHop>
}

/*
 * A node along a successful payment path
 */
#[derive(Clone, Debug)]
pub struct PathHop {
    pub short_channel_id: u64,
    pub amount: u64,
    pub node_pub_key: String
}

/*
 * An event that should take place at a given time
 */
#[derive(Clone, Debug)]
pub struct SimEvent {
    pub sim_time: u64,
    pub event: SimulationEvent
}

/*
 * An event that reports the results of a SimEvent taking place
 */
#[derive(Clone, Debug)]
pub struct SimResultsEvent {
    pub sim_time: Option<u64>, // Set to None for events that occur in response to another event: PaymentPathSuccessful, PaymentFailedEvent, PaymentSuccessEvent
    pub success: bool,
    pub event: SimulationEvent
}