use serde::{Serialize, Deserialize};

// This struct represents a channel defined in the simuation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimChannel {
    pub node1: String,
    pub node2: String,
    pub node1_balance: i32,
    pub node2_balance: i32
}