// External Modules
use serde::{Serialize, Deserialize};

/*
 * This struct represents a channel defined in the simuation
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimChannel {
    pub id: u64, // simulation defined id
    pub short_id: Option<u64>, // set when the channel is created in the node implementation
    pub run_time_id: Option<String>, // node implementation id, set when the channel is created in node implementation
    pub funding_tx: Option<String>, // id of the transaction that opens the channel, set when the channel is created in node implementation
    pub src_node: String, // the node that initiated the channel
    pub dest_node: String, // the node that accepted the incoming channel
    pub src_balance_sats: u64, // the outbound liquidity from the source node, inbound liquidity of the dest node
    pub dest_balance_sats: u64, // the outbound liquidity from the dest node, inbound liquidity of the src node
    pub penalty_reserve_sats: Option<u64> // the reserve amount that the src node must hold back, set by the node implementation when opening the channel
}

impl SimChannel {
    pub fn get_total_balance(&self) -> u64 {
        match self.penalty_reserve_sats {
            Some(p) => {
                self.src_balance_sats + self.dest_balance_sats + p
            },
            None => {
                self.src_balance_sats + self.dest_balance_sats
            }
        }
    }

    pub fn get_src_balance(&self) -> u64 {
        match self.penalty_reserve_sats {
            Some(p) => {
                self.src_balance_sats + p
            },
            None => {
                self.src_balance_sats
            }
        }
    }

    pub fn get_dest_balance(&self) -> u64 {
        self.dest_balance_sats
    }
}