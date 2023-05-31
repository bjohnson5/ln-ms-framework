# ln-ms-framework
A Bitcoin Lightning Network Modeling and Simulation Framework.

Instead of using Docker containers to model Lightning Network nodes, this framework uses the Lightning Development Kit (https://lightningdevkit.org) and Sensei (https://l2.technology/sensei) in order to create lightweight nodes that allow for large networks to be modeled. In addition, the user can define automated events to take place on the network and run simulations.

The goal is to provide researchers, developers, and service providers with an enhanced understanding of the Lightning Network's operations and enable effective testing of new Lightning Network products. Serving as a software library and a standalone tool, this project will facilitate the creation of accurate tests and simulations that replicate real-world Lightning Network behavior. As a complex and evolving decentralized network, predicting the behavior of the Lightning Network and products built on top of it is challenging. The development of a robust M&S software library for Bitcoin and Lightning is crucial to advancing these groundbreaking technologies.

There are currently ways to create small test Lightning Networks with a few nodes and channels, but this project will attempt to extend this capability to large scale networks that accurately represent the current Lightning Network. Each Lightning Network node is resource-intensive, making it difficult to simulate large networks on a single machine. Given the decentralized nature of the Lightning Network, modeling such a network realistically poses significant challenges. Accurate assumptions about the behaviors of individual actors within the network are vital to ensure the overall model's accuracy. This project's goal is to research and develop an efficient representation of nodes and channels, enabling the modeling of large networks while providing flexibility to customize assumptions that must be made. This project plans to accomplish this by modifying existing Lightning Network node implementations, optimizing them for simulation environments to enable large-scale simulations without compromising accuracy.

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
- After the simulation is finished, view the results here: http://localhost:8080/results

## Dependencies
- Currently only runs on a linux OS
- Nigiri must be installed (https://vulpem.com/nigiri.html)
- Rust must be installed (https://www.rust-lang.org)
- A fork of the sensei project (https://github.com/bjohnson5/sensei/tree/simulation-dev)
- A fork of the ldk project (https://github.com/bjohnson5/rust-lightning)

# Project Goals
This project’s main goal is to provide a tool for users to study the Lightning Network and learn more about it. It provides a way to test and research the Lightning Network in an isolated environment where metrics can be collected and analyzed. This framework allows for LN operations to be executed, studied and then repeated. Giving people the ability to create large simulations that can be loaded and re-run and then giving people insight into the data about that simulation will help to advance the knowledge of this distributed payment channel network.

## Large simulations
- Trim down the network implementation in order to run large simulations with lots of nodes and channels
- Quickly and efficiently start up a lot of nodes and open a lot of channels

## Use Real World LN Implementations
- `Real Node In The Loop:` Build this simulation framework with the flexibility to allow a real node (controlled by the user) to join and interact on the simulation network. This means that the simulated nodes need to be able to communicate with live nodes over the LN protocol... responding to messages, sending transactions, etc...
- `Interoperability:` Allow for different LN implementations to be added to the simulation (lnd, core lightning, etc...). These LN implementations are constantly evolving so the test framework needs to be able to plug in new node implementations to the simulation and all the implementations need to operate on the same simulated network.
- By using real LN node implementations users will get a realistic model of routing algorithms, fees, transactions, etc... and be able to connect their own nodes to the simulation and observe what the strengths and weaknesses are for their node and the whole network. In addition, this will let the simulation network provide a more accurate view of the real network that includes several different LN implementations.

## Highly Configurable
- `Liquidity configuration:` Allow for configuring the overall liquidity of the network and liquidity of the nodes. Allow a user to setup a network with nodes that each have unique liquidity.
- `Node/Channel configuration:` Give the user control on how the network and nodes are setup.
    - import network definitions, export network definitions, import transaction lists, etc...
    - routing fees, min tx amounts, punishment values, max inbound htlc percent of channel, etc...

## Accurate
- `Simulate network traffic:` Create simulated transactions that impact the traffic and liquidity of the network. Model the the transactions that are occuring on the network that we do not have control over... this will impact liquidity and routing.
- `Simulate mining:` Create a process that generates new blocks at a given interval (this could be run as real time... 10 min for a new block to allow for real time testing... or as event driven faster than real time)

## Flexible
- Build something that can meet several different needs.
    - `Front-end web interface:` Build a front end to the ln_ms_server API that users can use to build sims, save them to a database and load them later.
    - `Testing library:` A library that can be integrated into CI/CD pipelines and used to create automated functional tests.

## Automated
As time goes on nodes might go offline, close channels or run out of liquidity. All these events will change how the network operates. Current tools require the user to manually interact with the nodes on the network in order to generate these kinds of events. What if a user wanted to set up a network with a thousand nodes and have those nodes create invoices, connect to peers, open channels and make payments? This would not be possible with the current tools because it would be difficult to run a thousand Docker containers and manually create and pay invoices repeatedly.

## Data Driven
Users should be able to collect data on the network and review that data as the simulation runs and after it is finished, in order to observe how the nodes and network reacted to different conditions.

# Use Cases and Benefits
## Economic Growth
This project will benefit the LN and Bitcoin community by fostering innovation and advancements in the Lightning Network. By enabling researchers, developers, and routing node operators to optimize their products and services, this project can help create new business opportunities, attract investments, and stimulate the growth of the Lightning Network ecosystem.

## Collaboration and Knowledge Sharing
By providing a standardized and accessible M&S framework, this project can facilitate the exchange of ideas, best practices, and research findings among researchers, developers, and practitioners. This collaborative environment can accelerate the overall progress and adoption of Lightning Network technology

## Education and Training
Enhance education and training initiatives related to Bitcoin and LN technologies. Academic institutions, training programs, and educational platforms can incorporate this project into their curricula and materials, enabling students and professionals to gain practical experience and skills in working with the Lightning Network.

## Energy Efficiency
The Lightning Network has the potential to reduce energy consumption associated with Bitcoin transactions by enabling off-chain transactions. By providing a reliable modeling tool, researchers and developers can explore ways to optimize the energy efficiency of the Lightning Network further. This can contribute to sustainability efforts and reduce the environmental impact of blockchain-based transactions.

## Financial Inclusion
By improving the understanding, scalability, and efficiency of the Lightning Network, this project can indirectly contribute to expanding financial access and empowering individuals who are currently excluded from traditional financial systems.

## Accelerate LN Development
Current adoption and usage of the LN for payments is low and this is primarily due to a lack of tools that make it easy for non-technical people to safely use it. Companies are ramping up development of tools like this and in order to effectively build these tools, companies need a way to test their products. When dealing with financial technology users will gravitate towards products that are well tested and trusted, this simulation framework will help with that. A simulation product will lead to better and faster development of lightning network products, which in turn will lead to more LN adoption and get the world closer to fully utilizing this better payment system. LN development teams need CI/CD tools to help them build integration pipelines and improve their workflows. All software development teams use testing frameworks and libraries for creating high quality software. This type of library is needed for lightning specific software.

## Establish Trust in LN
Researchers and companies can use a simulation tool to create better products that can establish confidence with users. Essentially not many people are working on stress testing the LN and working through issues that might arise that will hurt adoption. If people don’t trust the LN they won’t use it. Our approach is to create something that will help build confidence in the network. Security researchers who are looking for exploits and vulnerabilities that need to be mitigated need a tool to test their approaches.

## Maximize Profitable Routing Services
Lightning node operators need a sandbox to test node configurations and use the simulation data to make decisions that will lead to a more profitable routing node.
