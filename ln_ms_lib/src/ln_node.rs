// This struct will control the lightweight nodes in a docker container where the simulated network lives
pub struct LnNode {
    pub name: String
}

impl LnNode {
    pub fn start(&self) {
        println!("starting {}", self.name);
    }

    pub fn stop(&self) {
        println!("stopping {}", self.name);
    }
}
