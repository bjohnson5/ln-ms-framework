// This struct will control the lightweight nodes in a docker container where the simulated network lives
// TODO: Connect to Sensei nodes and implement the different operations for this struct
pub struct LnNode {
    pub name: String
}

impl LnNode {
    pub fn start(&self) {
        println!("LnNode:{} -- starting {}", crate::get_current_time(), self.name);
    }

    pub fn stop(&self) {
        println!("LnNode:{} -- stopping {}", crate::get_current_time(), self.name);
    }
}