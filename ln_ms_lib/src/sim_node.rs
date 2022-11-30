use serde::{Serialize, Deserialize};

// This struct represents a node defined in the simulation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimNode {
    pub name: String,
    pub initial_balance: i32,
    pub running: bool
}
