// External Modules
use serde::{Serialize, Deserialize};

// This struct represents a request to create an invoice
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimTransaction {
    pub src_node: String,
    pub dest_node: String,
    pub amount: u64
}