# Main Threads
1. Network Analyzer - This thread collects stats and data about the network while a simulation runs. After the simulation is complete it will create reports for the user to analyze.
2. Transaction Generator - This thread will create and pay invoices on the network at a random or regular interval.
3. Event Manager - This thread will keep track of events and fire them off at different times throughout the simulation.

# Public API
- import_network - Import an initial state of the network from a file. This could parse Polar network configurations or real LN network information.
- create_node - Create a lightweight node in the Sensei network.
- create_channel - Create a channel in the Sensei network.
- create_event - Create an event that will take place at a specified time in the simulation.
- run - Start the simulation.

# Uses
- ln_ms_lib - A rust library that could be integrated into other projects to allow users to build and run LN simulations.
- ln_ms_server - A rust web api that users could use to build a front end LN simulation tool.