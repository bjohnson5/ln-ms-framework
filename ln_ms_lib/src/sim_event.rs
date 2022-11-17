// This enum represents all of the events that can be added to a simulation
#[derive(Clone, Debug)]
pub enum SimulationEvent {
    NodeOnlineEvent(String),
    NodeOfflineEvent(String),
    SimulationEnded
}