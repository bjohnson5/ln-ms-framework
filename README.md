# ln-ms-framework
A Bitcoin Lightning Network Modeling and Simulation Framework

## ln_ms_lib
A library for creating a Lightning Network simulation

## ln_ms_server
An API that uses ln_ms_lib to define and run a Lightning Network simulation

## Proof of Concept User Story
1. Create Node A
2. Create Node B
3. Open channel from Node A to Node B for 500 sats
4. Create a simulation with start time 0s and end time 100s
5. Create a transaction generator that sends random amounts every 5s through the channel
6. Create an event at 30s that tells Node A to go offline
7. Create an event at 50s that tells Node A to go online
8. Run the simulation and check the results
