# lightning-simulator

![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
[![CI](https://github.com/cndolo/lightning-simulator/actions/workflows/test.yml/badge.svg)](https://github.com/cndolo/lightning-simulator/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/cndolo/lightning-simulator/branch/main/graph/badge.svg?token=QZH345MHCJ)](https://codecov.io/gh/cndolo/lightning-simulator)
[![dependency status](https://deps.rs/repo/github/cndolo/lightning-simulator/status.svg)](https://deps.rs/repo/github/cndolo/lightning-simulator)

This is a collection of projects related to simulating payments in the Lightning
network.

Each subproject contains a detailed description of the project itself along with
usage instructions.

## graph-diversity

A binary that does computations on the diversity in the LN channel graph.

## network-parser

A library to deserialise the channel graphs provided by either
[LND](https://lightning.engineering/api-docs/api/lnd/lightning/describe-graph/index.html)
or [lnresearch](https://github.com/lnresearch/topology).

## simulator

A library and binary that use the `network-parser` to read the JSON files and simulate
payments in the LN.
The library implements routing and payment splitting logic and can be used as
follows.

```
lightning-simulator = {git = "https://github.com/cndolo/lightning-simulator"}
use simlib::*;
```

## Build

Build all members of the project:

`cargo build --release`

## Simulation results

We maintain interactive charts with simulation results at
[https://anonymous.4open.science/w/lightning-simulator-13A7/](https://anonymous.4open.science/w/lightning-simulator-13A7/).
