# lightning-simulator

[![Rust](https://camo.githubusercontent.com/5782bcc58a7786e9a7d00e2cf45937db8a2598232d9524ec9dcd149c7218671b/68747470733a2f2f696d672e736869656c64732e696f2f62616467652f527573742d50726f6772616d6d696e672532304c616e67756167652d626c61636b3f7374796c653d666c6174266c6f676f3d72757374)](www.rust-lang.org)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)

This is a collection of projects related to simulating payments in the Lightning
network.

Each subproject contains a detailed description of the project itself along with
usage instructions.

## network-parser

A library to deserialise the topology graphs provided by
[lnresearch](https://github.com/lnresearch/topology).

## simulator

A binary that uses the `network-parser` to read the JSON files and simulate
payments in the LN.

## Build

Build all members of the project:

`cargo build --release`
