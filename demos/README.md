# Demos
The two demo videos show the simulation library API being used through a web client (with swagger).
- The `memory_demo.mp4` video shows the difference in memory that is used by Docker containers compared to the simulation framework. 15 LND instances are started in Docker containers, each of them using about 40 MB of memory. Then 100 LDK Lightning Nodes are started and only use a total of about 35 MB.
- The `event_demo.mp4` video shows a Lightning Network of 100 nodes being started with several events occuring during the run. Nodes turn red in the network monitor when they go offline and new channels are shown by connecting different nodes.
- The `results_demo_full.mp4` shows a simulation being created, channels being opened and a payment being routed. The results page is shown after the simulation has completed and it gives a timeline of how balances, channels and nodes changed throughout the simulation.

Each of the simulation nodes in the network graph that is shown is backed by a Sensei node and is completely capable of accepting peer connections, opening/closing channels, and making payments.

## Memory Demo
https://user-images.githubusercontent.com/23157382/206827940-3ede7c7a-ce69-4ef2-868d-3c4958a62cb1.mp4

## Event Demo
https://user-images.githubusercontent.com/23157382/206827946-36233728-81aa-49ef-a9fe-a428698c60e5.mp4

## Results Demo
https://github.com/bjohnson5/ln-ms-framework/assets/23157382/811e717c-7255-4232-8815-24d7f39cd15e

