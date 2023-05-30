// Standard Modules
use std::process::Command;

/*
 * These functions controll the nigiri instance that is running bitcoind.
 * TODO: Is there a better way to control the underlying bitcoin blockchain? Instead of running bash commands to control nigiri?
 *       - Use the bitciond_client to control the chain
 */

/*
 * Start nigiri
 */
pub fn start() {
    Command::new("sh")
    .arg("-c")
    .arg("nigiri start")
    .output()
    .expect("failed to execute start process");
}

/*
 * Stop nigiri
 */
pub fn stop() {
    Command::new("sh")
    .arg("-c")
    .arg("nigiri stop --delete")
    .output()
    .expect("failed to execute stop process");
}

/*
 * Mine 10 blocks
 */
pub fn mine() {
    Command::new("sh")
    .arg("-c")
    .arg("nigiri rpc -generate 10")
    .output()
    .expect("failed to execute mine process");
}

/*
 * Send bitcoin to an address and mine a block
 */ 
pub fn fund_address(addr: String, amount: u64) {
    let amount_btc = amount as f32 / 100000000.0;
    let arg = String::from("nigiri faucet ") + &addr + &String::from(" ") + &amount_btc.to_string();
    Command::new("sh")
    .arg("-c")
    .arg(&arg)
    .output()
    .expect("failed to execute fund process");
}