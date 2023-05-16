// Project modules
use crate::sim_transaction::SimTransaction;
use crate::sim_channel::SimChannel;

// Standard Modules
use std::collections::HashMap;

#[derive(Clone)]
pub struct SimResults {
    pub balance: BalanceResults,
    pub transactions: TxResults,
    pub channels: ChannelResults,
    pub status: StatusResults
}

impl SimResults {
    // Create a new SimResults
    pub fn new() -> Self {
        let r = SimResults {
            balance: BalanceResults { on_chain: HashMap::new(), off_chain: HashMap::new() },
            transactions: TxResults { txs: Vec::new() },
            channels: ChannelResults { open_channels: HashMap::new(), closed_channels: HashMap::new() },
            status: StatusResults { nodes: HashMap::new() }
        };

        r
    }
}

/*
 * The on_chain and off_chain balances for a node at a given sim time
 * key=sim time, value=map of node name to balance
 */
#[derive(Clone)]
pub struct BalanceResults {
    pub on_chain: HashMap<u64, HashMap<String, u64>>,
    pub off_chain: HashMap<u64, HashMap<String, u64>>,
}

/*
 * A list of all transactions that occurred in the sim
 */
#[derive(Clone)]
pub struct TxResults {
    pub txs: Vec<Tx>
}

/*
 * Details about each transaction
 */
#[derive(Clone)]
pub struct Tx {
    pub time: u64,
    pub success: bool,
    pub transaction: SimTransaction
}

/*
 * The open and closed channels in the simulation at a given sim time
 * key=sim time, value=list of channels at that time
 */
#[derive(Clone)]
pub struct ChannelResults {
    pub open_channels: HashMap<u64, Vec<SimChannel>>,
    pub closed_channels: HashMap<u64, Vec<SimChannel>>
}

/*
 * Node status at a given sim time
 * key=sim time, value=map of node name to status (true=online, false=offline)
 */
#[derive(Clone)]
pub struct StatusResults {
    pub nodes: HashMap<u64, HashMap<String, bool>>
}