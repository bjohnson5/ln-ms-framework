use crate::ln_node::LnNode;

// Each Event should implement this trait
pub trait LnEvent {
    fn execute(&self, node: &LnNode);
    fn get_node_name(&self) -> String;
    fn get_name(&self) -> String;
}

// An Event to stop a node
pub struct NodeOfflineEvent {
    pub node_name: String
}

impl LnEvent for NodeOfflineEvent {
    fn execute(&self, node: &LnNode) {
        println!("Node going offline: {}", self.node_name);
        node.stop();
    }

    fn get_node_name(&self) -> String {
        let s = self.node_name.clone();
        s
    }

    fn get_name(&self) -> String {
        String::from("NodeOfflineEvent")
    }
}

// An Event to start a node
pub struct NodeOnlineEvent {
    pub node_name: String
}

impl LnEvent for NodeOnlineEvent {
    fn execute(&self, node: &LnNode) {
        println!("Node going online: {}", self.node_name);
        node.start()
    }

    fn get_node_name(&self) -> String {
        let s = self.node_name.clone();
        s
    }

    fn get_name(&self) -> String {
        String::from("NodeOnlineEvent")
    }
}