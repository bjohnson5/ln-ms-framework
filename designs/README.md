# Main Threads
1. Network Analyzer - This thread collects stats and data about the network while a simulation runs. After the simulation is complete it will create reports for the user to analyze.
2. Transaction Generator - This thread will create and pay invoices on the network at a random or regular interval.
3. Event Manager - This thread will keep track of events and fire them off at different times throughout the simulation.
4. Sensei Controller - This thread will process simulation events and make calls to the Sensei API to control the nodes.

# Uses
- `ln_ms_lib` - A rust library that could be integrated into other projects to allow users to build and run LN simulations.
- `ln_ms_server` - A rust web api that users could use to build a front end LN simulation tool.
