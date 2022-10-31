use crate::ln_node::LnNode;

// TODO: Implement all supported Events

// Each Event should implement this trait
// The execute function borrows a reference to the node that this event is for
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
        println!("NodeOfflineEvent:{} -- node going offline: {}", crate::get_current_time(), self.node_name);
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
        println!("NodeOfflineEvent:{} -- node going online: {}", crate::get_current_time(), self.node_name);
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