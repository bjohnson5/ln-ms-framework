# Demos
The two demo videos show the simulation library API being used through a web client (with swagger).
- The `memory_demo.mp4` video shows the difference in memory that is used by Docker containers compared to the simulation framework. 15 LND instances are started in Docker containers, each of them using about 40 MB of memory. Then 100 LDK Lightning Nodes are started and only use a total of about 35 MB.
- The `event_demo.mp4` video shows a Lightning Network of 100 nodes being started with several events occuring during the run. Nodes turn red in the network monitor when they go offline and new channels are shown by connecting different nodes.

Each of the simulation nodes in the network graph that is shown is backed by a Sensei node and is completely capable of accepting peer connections, opening/closing channels, and making payments.

## Memory Demo

## Event Demo
