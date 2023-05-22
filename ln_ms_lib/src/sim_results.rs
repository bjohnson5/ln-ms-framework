// Project modules
use crate::sim_transaction::SimTransaction;
use crate::sim_channel::SimChannel;
use crate::sim_event::SimResultsEvent;

// Standard Modules
use std::collections::HashMap;

/*
 * The results of the simulation are stored in this struct and returned at the end of a run
 */
#[derive(Clone)]
pub struct SimResults {
    pub balance: BalanceResults,
    pub transactions: TxResults,
    pub channels: ChannelResults,
    pub status: StatusResults,
    pub failed_events: Vec<SimResultsEvent>
}

impl SimResults {
    pub fn new() -> Self {
        let r = SimResults {
            balance: BalanceResults { on_chain: HashMap::new(), off_chain: HashMap::new() },
            transactions: TxResults { txs: Vec::new() },
            channels: ChannelResults { open_channels: HashMap::new(), closed_channels: HashMap::new() },
            status: StatusResults { nodes: HashMap::new() },
            failed_events: Vec::new()
        };

        r
    }

    /*
     * Get the on chain balance of a node at a given time in the simulation
     */
    pub fn get_on_chain_bal(&self, time: u64, node: &String) -> Option<u64> {
        match self.balance.on_chain.get(node) {
            Some(n) => {
                // The node was found so get the closest results entry to the given time
                let keys: Vec<&u64> = n.keys().collect();
                match SimResults::find_closest_less(keys, &time) {
                    Some(k) => {
                        // The time was found so get the balance
                        match n.get(&k) {
                            Some(r) => {
                                // Return the balance
                                return Some(r.clone());
                            },
                            None => {
                                // The balance was not found
                                return None;
                            }
                        }
                    },
                    None => {
                        // The time was not found
                        return None;
                    }
                }
            },
            None => {
                // The node was not found
                return None;
            }
        }
    }

    /*
     * Get the off chain balance of a node at a given time in the simulation
     */
    pub fn get_off_chain_bal(&self, time: u64, node: &String) -> Option<u64> {
        match self.balance.off_chain.get(node) {
            Some(n) => {
                // The node was found so get the closest results entry to the given time
                let keys: Vec<&u64> = n.keys().collect();
                match SimResults::find_closest_less(keys, &time) {
                    Some(k) => {
                        // The time was found so get the balance
                        match n.get(&k) {
                            Some(r) => {
                                // Return the balance
                                return Some(r.clone());
                            },
                            None => {
                                // The balance was not found
                                return None;
                            }
                        }
                    },
                    None => {
                        // The time was not found
                        return None;
                    }
                }
            },
            None => {
                // The node was not found
                return None;
            }
        }
    }

    /*
     * Get all the transactions for a given node
     */
    pub fn get_node_transactions(&self, node: &String) -> Option<Vec<Tx>> {
        let filtered_txs: Vec<Tx> = self.transactions.txs
        .iter()
        .filter(|&t| &t.transaction.src_node == node || &t.transaction.dest_node == node).cloned()
        .collect();

        Some(filtered_txs)
    }

    /*
     * Get all the transactions from the simulation
     */
    pub fn get_all_transactions(&self) -> Option<Vec<Tx>> {
        Some(self.transactions.txs.clone())
    }

    /*
     * Get the open channels at a given time in the simulation
     */
    pub fn get_open_channels(&self, time: u64) -> Option<Vec<SimChannel>> {
        let keys: Vec<&u64> = self.channels.open_channels.keys().collect();
        match SimResults::find_closest_less(keys, &time) {
            Some(k) => {
                return Some(self.channels.open_channels.get(&k).unwrap().clone());
            },
            None => {
                // The time was not found
                return None;
            }
        }
    }

    /*
     * Get the channels that have been created and closed at a given time in the simulation
     */
    pub fn get_closed_channels(&self, time: u64) -> Option<Vec<SimChannel>> {
        let keys: Vec<&u64> = self.channels.closed_channels.keys().collect();
        match SimResults::find_closest_less(keys, &time) {
            Some(k) => {
                return Some(self.channels.closed_channels.get(&k).unwrap().clone());
            },
            None => {
                return None;
            }
        }
    }

    /*
     * Get the on/off status of a node at a given time in the simulation
     */
    pub fn get_node_status(&self, time: u64, node: &String) -> bool {
        match self.status.nodes.get(node) {
            Some(n) => {
                let keys: Vec<&u64> = n.keys().collect();
                match SimResults::find_closest_less(keys, &time) {
                    Some(k) => {
                        // The time was found so get the status
                        match n.get(&k) {
                            Some(r) => {
                                // Return the status
                                return r.clone();
                            },
                            None => {
                                // The status was not found
                                return false;
                            }
                        }
                    },
                    None => {
                        // The time was not found
                        return false;
                    }
                }
            },
            None => {
                // The node was not found
                return false;
            }
        }
    }

    /*
     * Find the key in the map that is closest to the target and less than the target
     */
    fn find_closest_less(keys: Vec<&u64>, target: &u64) -> Option<u64> {
        let mut closest_key: Option<u64> = None;
    
        for key in keys {
            if key == target {
                return Some(key.clone());
            }
            if key < target {
                closest_key = match closest_key {
                    None => Some(key.clone()),
                    Some(prev_key) if key > &prev_key => Some(key.clone()),
                    _ => closest_key,
                };
            }
        }
    
        closest_key
    }
}

/*
 * The on_chain and off_chain balances for a node at a given sim time
 * key=node name, value=map of time to balance
 */
#[derive(Clone)]
pub struct BalanceResults {
    pub on_chain: HashMap<String, HashMap<u64, u64>>,
    pub off_chain: HashMap<String, HashMap<u64, u64>>,
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
 * key=node name, value=map of time to status (true=online, false=offline)
 */
#[derive(Clone)]
pub struct StatusResults {
    pub nodes: HashMap<String, HashMap<u64, bool>>
}