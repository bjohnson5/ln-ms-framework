// Project Modules
use crate::sim_channel::SimChannel;

// TODO: implement all other events
// This enum represents all of the events that can be added to a simulation
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    NodeOnlineEvent(String),
    NodeOfflineEvent(String),
    OpenChannelEvent(SimChannel),
    CloseChannelEvent(SimChannel),
    SimulationEnded
}