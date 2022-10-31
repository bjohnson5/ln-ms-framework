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

## To run the Proof of Concept
```
cd ln_ms_server
cargo build
./target/debug/ln_ms_server
```
Then navigate to http://localhost:8080/swagger-ui/index.html to view the swagger API

You can then create nodes and events and run the simulation. The output will be logged to the terminal.
NOTE: the first thing to do is use the create_sim endpoint, otherwise adding nodes and events will not work.

Example:
1. create_sim - Request Body:
```
{
  "duration": 20,
  "name": "test"
}
```

2. create_node - Request Body:
```
{
  "name": "blake"
}
```

3. create_event - Request Body:
```
{
  "event_type": "NodeOnlineEvent",
  "node_name": "blake",
  "time": 10
}
```

4. run_sim - Request Body:
```
{
  "name": "test"
}
```

5. Watch the terminal execute the online event at 10s