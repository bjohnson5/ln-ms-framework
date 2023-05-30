// External Modules
use serde::{Serialize, Deserialize};

/*
 * This struct represents a node defined in the simulation
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimNode {
    pub name: String,
    pub initial_balance: u64, // the on-chain balance of the node to start the simulation with
    pub running: bool // is the node running at the start of the simulation
}
