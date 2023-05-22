// External Modules
use serde::{Serialize, Deserialize};

/*
 * This struct represents a channel defined in the simuation
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimChannel {
    pub id: u64,
    pub short_id: Option<u64>, // Set when the channel is created in sensei
    pub src_node: String,
    pub dest_node: String,
    pub src_balance: u64,
    pub dest_balance: u64,
}