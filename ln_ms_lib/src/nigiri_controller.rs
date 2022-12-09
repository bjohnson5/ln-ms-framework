// Standard Modules
use std::process::Command;

pub fn start() {
    Command::new("sh")
    .arg("-c")
    .arg("nigiri start")
    .output()
    .expect("failed to execute start process");
}

pub fn stop() {
    Command::new("sh")
    .arg("-c")
    .arg("nigiri stop --delete")
    .output()
    .expect("failed to execute stop process");
}

pub fn fund_address(addr: String, amount: i32) {
    let amount_btc = amount as f32 / 100000000.0;
    let arg = String::from("nigiri faucet ") + &addr + &String::from(" ") + &amount_btc.to_string();
    Command::new("sh")
    .arg("-c")
    .arg(&arg)
    .output()
    .expect("failed to execute fund process");
}