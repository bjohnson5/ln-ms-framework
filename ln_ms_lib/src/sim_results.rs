// Project Modules
use crate::sim_transaction::SimTransaction;
use crate::sim_channel::SimChannel;
use crate::sim_event::SimResultsEvent;

// External Modules
use build_html::{Container, ContainerType, HtmlContainer, Html};

// Standard Modules
use std::collections::HashMap;
use std::fs;

/*
 * The results of the simulation are stored in this struct and returned at the end of a run
 */
#[derive(Clone)]
pub struct SimResults {
    pub balance: BalanceResults,
    pub transactions: TxResults,
    pub channels: ChannelResults,
    pub status: StatusResults,
    pub failed_events: Vec<SimResultsEvent>,
    pub event_times: Vec<u64>
}

impl SimResults {
    pub fn new() -> Self {
        let r = SimResults {
            balance: BalanceResults { on_chain: HashMap::new(), off_chain: HashMap::new() },
            transactions: TxResults { txs: Vec::new() },
            channels: ChannelResults { open_channels: HashMap::new(), closed_channels: HashMap::new() },
            status: StatusResults { nodes: HashMap::new() },
            failed_events: Vec::new(),
            event_times: Vec::new()
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
     * Get the results of this simulation formatted in an HTML webpage
     */
    pub fn get_results_page(&self) -> String {
        // A list of all the nodes in the simulation
        let mut node_list = Container::new(ContainerType::UnorderedList);
        
        // A details section about each node
        let mut node_details = Container::new(ContainerType::Div);
        
        // A timeline showing the times in the simulation where an event occurred
        let mut timeline = Container::new(ContainerType::Div).with_attributes([("id", "tl"), ("class", "timeline")]);

        // A details section about each time in the simulation
        let mut time_details = Container::new(ContainerType::Div);
        
        // Create the node list and node details sections
        let mut i = 0;
        for node_name in &self.get_nodes() {
            i = i + 1;
            let id = format!("node{i}");
            let select_fn = format!("selectNode({i})");
            node_list.add_container(
                Container::new(ContainerType::Div)
                .with_attributes([("id", id.as_str()), ("class", "event"), ("onclick", select_fn.as_str())])
                .with_paragraph(node_name));

            let mut l = 0;
            for time in &self.event_times {
                l = l + 1;
                node_details.add_container(self.get_node_html(i, l, time, node_name));
            }
        }

        // Create the timeline and time details sections
        let mut j = 0;
        for time in &self.event_times {
            j = j + 1;
            let container_side;
            if j % 2 != 0 {
                container_side = "container left";
            } else {
                container_side = "container right";
            }
            let id = format!("item-{j}");
            let select_fn = format!("selectItem({j})");
            timeline.add_container(
                Container::new(ContainerType::Div)
                .with_attributes([("class", container_side)])
                .with_container(Container::new(ContainerType::Div)
                .with_attributes([("id", id.as_str()), ("class", "timeline-item"), ("onclick", select_fn.as_str())])
                .with_paragraph(time)));

            time_details.add_container(self.get_time_html(j, time));
        }

        // Read in the template HTML page and replace the relevant sections with the sections created above
        let template = fs::read_to_string("/home/blake/Projects/ln-ms-framework/ln_ms_lib/src/results_html/index.html");
        match template {
            Ok(html_page) => {
                let page1 = html_page.as_str().replace("<!--NODE_LIST-->", node_list.to_html_string().as_str());
                let page2 = page1.as_str().replace("<!--NODE_DETAILS-->", node_details.to_html_string().as_str());
                let page3 = page2.as_str().replace("<!--TIMELINE-->", timeline.to_html_string().as_str());
                let html = page3.as_str().replace("<!--TIME_DETAILS-->", time_details.to_html_string().as_str());
                html
            },
            Err(e) => {
                println!("Error reading the results template file: {:?}", e);
                String::from("")
            }
        }

    }

    /*
     * Get an html element containing a formatted string with the details of a given node at a given time
     */
    fn get_node_html(&self, node_num: i32, time_num: i32, time: &u64, node: &String) -> Container {
        let title_id = format!("node-title{node_num}{time_num}");
        let desc_id = format!("node-description{node_num}{time_num}");

        // Get the status and balances of this node at this time in the simulation
        let status = if self.get_node_status(time.clone(), node) {"ONLINE"} else {"OFFLINE"};
        let onchain = match self.get_on_chain_bal(time.clone(), node) {
            Some(b) => {b},
            None => 0
        };
        let offchain = match self.get_off_chain_bal(time.clone(), node) {
            Some(b) => {b},
            None => 0
        };

        // Get the open channels for this node at this time in the simulation
        let mut channels_string = String::from("");
        match self.get_open_channels(time.clone()) {
            Some(channels) => {
                for c in channels {
                    if &c.src_node == node || &c.dest_node == node {
                        let srcbal = c.src_balance_sats.clone();
                        let destbal = c.dest_balance_sats.clone();
                        channels_string = channels_string + &c.src_node + " " + &String::from("&#8594") + " " + &c.dest_node + " (" + &format!("{srcbal}") + " " + &String::from("&#8594") + " " + &format!("{destbal}") + ")\n\t";
                    }
                }
            },
            None => {}
        };
        
        // Create the container
        let desc = format!("TIME: {time}\n\nSTATUS: {status}\n\nONCHAIN BALANCE: {onchain}\n\nLN BALANCE: {offchain}\n\nCHANNELS: \n\t{channels_string}");
        let node_details = Container::new(ContainerType::Div)
                                        .with_attributes([("class", "details")])
                                        .with_paragraph_attr(format!("NODE: {node}"), [("id", title_id.as_str())])
                                        .with_paragraph_attr(desc.as_str(), [("id", desc_id.as_str())]);
        node_details
    }

    /*
     * Get an html element containing a formatted string with the details of a given time in the simulation
     */
    fn get_time_html(&self, time_num: i32, time: &u64) -> Container {
        let title_id = format!("title{time_num}");
        let desc_id = format!("description{time_num}");

        // Get the open channels at this time in the simulation
        let mut openchannels = String::from("");
        match self.get_open_channels(time.clone()) {
            Some(channels) => {
                for c in channels {
                    let srcbal = c.src_balance_sats.clone();
                    let destbal = c.dest_balance_sats.clone();
                    openchannels = openchannels + &c.src_node + " " + &String::from("&#8594") + " " + &c.dest_node + " (" + &format!("{srcbal}") + " " + &String::from("&#8594") + " " + &format!("{destbal}") + ")\n\t";
                }
            },
            None => {}
        };

        // Get the transactions that occurred at this time in the simulation
        let mut transactions = String::from("");
        for t in self.get_all_transactions().unwrap() {
            if t.time == time.clone() {
                let amount = t.transaction.amount_sats.clone();
                transactions = transactions + &t.transaction.src_node + " " + &String::from("&#8594") + " " + &t.transaction.dest_node + " (" + &format!("{amount}") + ")\n\t";
            }
        }

        // Get the closed channels at this time in the simulation
        let mut closedchannels = String::from("");
        match self.get_closed_channels(time.clone()) {
            Some(channels) => {
                for c in channels {
                    let srcbal = c.src_balance_sats.clone();
                    let destbal = c.dest_balance_sats.clone();
                    closedchannels = closedchannels + &c.src_node + " " + &String::from("&#8594") + " " + &c.dest_node + " (" + &format!("{srcbal}") + " " + &String::from("&#8594") + " " + &format!("{destbal}") + ")\n\t";
                }
            },
            None => {}
        }

        // Get the failed events at this time in the simulation
        let mut failed = String::from("");
        for f in &self.failed_events {
            if f.sim_time.is_some() && f.sim_time.unwrap() == time.clone() {
                failed = failed + &f.event.to_string() + " \n";
            }
        }

        // Create the container
        let desc = format!("TRANSACTIONS:\n\t{transactions}\n\nOPEN CHANNELS:\n\t{openchannels}\n\nCLOSED CHANNELS:\n\t{closedchannels}\n\nFAILED EVENTS:\n\t{failed}");
        let time_details = Container::new(ContainerType::Div)
                                        .with_attributes([("class", "details")])
                                        .with_paragraph_attr(format!("SIM TIME: {time}"), [("id", title_id.as_str())])
                                        .with_paragraph_attr(desc.as_str(), [("id", desc_id.as_str())]);
        time_details
    }

    /*
     * Get all the node names in the simulation
     */
    fn get_nodes(&self) -> Vec<String> {
        let mut nodes: Vec<String> = Vec::new();
        for name in self.balance.on_chain.keys() {
            nodes.push(name.clone());
        }

        nodes
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