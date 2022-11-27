use serde::{Serialize, Deserialize};

// This struct represents a node defined in the simulation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimNode {
    pub name: String,
    pub running: bool
}
