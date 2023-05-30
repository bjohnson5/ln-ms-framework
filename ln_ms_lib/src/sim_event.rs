// Project Modules
use crate::sim_channel::SimChannel;
use crate::sim_transaction::SimTransaction;

// Standard Modules
use std::fmt;

/*
 * This enum represents all of the events that can be added to a simulation
 */
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    StartNodeEvent(String), // param: node name to start
    StopNodeEvent(String), // param: node name to stop
    OpenChannelEvent(SimChannel), // param: the details of the channel to open
    CloseChannelEvent(String, u64), // param: node name and simulation defined channel id of the channel to close
    TransactionEvent(SimTransaction), // param: the details of the transaction to attempt
    PaymentPathSuccessful(SimPaymentPath), // sent from ln_event_processor when the node notifies us that a payment was successful
    PaymentFailedEvent(String), // sent from ln_event_processor when the node notifies us that a payment failed. Param: payment id that failed
    PaymentSuccessEvent(String, u64), // sent from ln_event_processor when the node notifies us that a payment was successful. Param: payment Id and the fee paid
    CloseChannelSuccessEvent(String), // sent from ln_event_processor when the node notifies us that a channel closed. Param: node implementation channel id
    SimulationEndedEvent // simulation has ended
}

impl fmt::Display for SimulationEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SimulationEvent::StartNodeEvent(_) => write!(f, "StartNodeEvent"),
            SimulationEvent::StopNodeEvent(_) => write!(f, "StopNodeEvent"),
            SimulationEvent::OpenChannelEvent(_) => write!(f, "OpenChannelEvent"),
            SimulationEvent::CloseChannelEvent(_,_) => write!(f, "CloseChannelEvent"),
            SimulationEvent::TransactionEvent(_) => write!(f, "TransactionEvent"),
            SimulationEvent::PaymentPathSuccessful(_) => write!(f, "PaymentPathSuccessful"),
            SimulationEvent::PaymentFailedEvent(_) => write!(f, "PaymentFailedEvent"),
            SimulationEvent::PaymentSuccessEvent(_, _) => write!(f, "PaymentSuccessEvent"),
            SimulationEvent::CloseChannelSuccessEvent(_) => write!(f, "CloseChannelSuccessEvent"),
            SimulationEvent::SimulationEndedEvent => write!(f, "SimulationEndedEvent"),
        }
    }
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
    pub sim_time: Option<u64>, // set to none for events that occur in response to another event: PaymentPathSuccessful, PaymentFailedEvent, PaymentSuccessEvent, CloseChannelSuccessEvent
    pub success: bool,
    pub event: SimulationEvent
}