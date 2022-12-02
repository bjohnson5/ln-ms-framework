# ln-ms-framework
A Bitcoin Lightning Network Modeling and Simulation Framework.
Instead of using Docker containers to model Lightning Network nodes, this framework uses the Lightning Development Kit (https://lightningdevkit.org) and Sensei (https://l2.technology/sensei) in order to create lightweight nodes that allow for very large networks to be modeled. In addition to creating large scale Lightning Networks for testing the user can define automated events to take place on the simulated network.

## ln_ms_lib
A library for creating a Lightning Network simulation

## ln_ms_server
An API that uses ln_ms_lib to define and run a Lightning Network simulation

### Building/Testing the library
```
cd ln_ms_lib
cargo build
cargo test -- --show-output
```

### Building/Testing the server
```
cd ln_ms_server
cargo build
cargo run
```
- View the swagger API documentation here: http://localhost:8080/swagger-ui/index.html
- After creating a simulation, view the network monitor here: http://localhost:8080/network_monitor

### Dependencies
- Currently only runs on a linux OS
- Nigiri must be installed (https://vulpem.com/nigiri.html)
- Rust must be installed (https://www.rust-lang.org)