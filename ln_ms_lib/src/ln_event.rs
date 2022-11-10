#[derive(Clone, Debug)]
pub enum SimulationEvent {
    NodeOnlineEvent(String),
    NodeOfflineEvent(String),
    SimulationEnded
}