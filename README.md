# ln-ms-framework
A Bitcoin Lightning Network Modeling and Simulation Framework.
Instead of using Docker containers to model Lightning Network nodes, this framework uses the Lightning Development Kit (https://lightningdevkit.org) and Sensei (https://l2.technology/sensei) in order to create lightweight nodes that allow for very large networks to be modeled. In addition to creating large scale Lightning Networks for testing the user can define automated events to take place on the simulated network.

## ln_ms_lib
A library for creating a Lightning Network simulation

## ln_ms_server
An API that uses ln_ms_lib to define and run a Lightning Network simulation

## Building/Testing the library
```
cd ln_ms_lib
cargo build
cargo test -- --show-output
```

## Building/Testing the server
```
cd ln_ms_server
cargo build
cargo run
```
- After starting the server, view the swagger API documentation here: http://localhost:8080/swagger-ui/index.html
- After creating a simulation, view the network monitor here: http://localhost:8080/network_monitor

## Dependencies
- Currently only runs on a linux OS
- Nigiri must be installed (https://vulpem.com/nigiri.html)
- Rust must be installed (https://www.rust-lang.org)
- A fork of the sensei project (https://github.com/bjohnson5/sensei/tree/simulation-dev)
- A fork of the ldk project (https://github.com/bjohnson5/rust-lightning)

## Goals
- Scale: Trim down the implementation in order to run large simulations with lots of nodes and channels
- Liquidity Modeling: Allow for configuring the overall liquidity of the network
- Configuration: Allow for more user configuration
- Simulated Network Traffic: Create simulated transactions that impact the traffic and liquidity of the network
- Front end web interface to create/save/run simulations: Build a front end to the ln_ms_server API that users can use to build sims, save them to a database and load them later
- Interoperability: Allow for different LN implementations to be added to the simulation (lnd, core lightning, etc...)
- Real Node In The Loop: Build this simulation framework with the flexibility to allow a real node (controlled by the user) to join and interact on the simulation network