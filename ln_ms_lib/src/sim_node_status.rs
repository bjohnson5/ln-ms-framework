#[derive(Clone, Debug)]
pub struct SimNodeStatus {
    pub balance: SimNodeBalance,
    pub channels: Vec<SimNodeChannel>
}

impl SimNodeStatus {
    pub fn new() -> Self {
        let status = SimNodeStatus {
            balance: SimNodeBalance::new(),
            channels: Vec::new()
        };

        status
    }
}

#[derive(Clone, Debug)]
pub struct SimNodeBalance {
    pub total: u64,
    pub onchain: u64,
    pub offchain: u64
}

impl SimNodeBalance {
    pub fn new() -> Self {
        let balance = SimNodeBalance {
            total: 0,
            onchain: 0,
            offchain: 0
        };

        balance
    }
}

#[derive(Clone, Debug)]
pub struct SimNodeChannel {
    pub confirmations_required: u32,
    pub is_usable: bool,
    pub is_public: bool,
    pub is_outbound: bool,
    pub balance: u64,
    pub outbound_capacity: u64,
    pub inbound_capacity: u64,
    pub is_channel_ready: bool
}

impl SimNodeChannel {
    pub fn new(conf_req: u32, usable: bool, public: bool, outbound: bool, bal: u64, out_bal: u64, in_bal: u64, ready: bool) -> Self {
        let channel = SimNodeChannel {
            confirmations_required: conf_req,
            is_usable: usable,
            is_public: public,
            is_outbound: outbound,
            balance: bal,
            outbound_capacity: out_bal,
            inbound_capacity: in_bal,
            is_channel_ready: ready
        };

        channel
    }
}