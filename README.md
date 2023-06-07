# lightning-simulator

![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
[![CI](https://github.com/open-anonymous-science/lightning-simulator/actions/workflows/test.yml/badge.svg)](https://github.com/open-anonymous-science/lightning-simulator/actions/workflows/test.yml)
[![codecov](https://codecov.io/gh/open-anonymous-science/lightning-simulator/branch/main/graph/badge.svg?token=QZH345MHCJ)](https://codecov.io/gh/open-anonymous-science/lightning-simulator)
[![dependency status](https://deps.rs/repo/github/open-anonymous-science/lightning-simulator/status.svg)](https://deps.rs/repo/github/open-anonymous-science/lightning-simulator)

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

A binary that uses the `network-parser` to read the JSON files and simulate
payments in the LN.

## Build

Build all members of the project:

`cargo build --release`

## Simulation results

Here are the results of a series of simulations conducted on a snapshot of the
network on 2023-05-16.

<details open>
  <summary><b>Performance</b></summary>

   ![](plots/success_rate.png)

   ![](plots/transaction_fees.png)

   ![](plots/path_length.png)

   ![](plots/htlc_attempts.png)

   ![](plots/failed_path_length.png)

   ![](plots/splits.png)

</details>

<details open>
  <summary><b>Privacy</b></summary>

   ![](plots/observation_rate.png)

   ![](plots/predecessor_guesses.png)

   ![](plots/vulnerable_successful_payments.png)

   ![](plots/path_diversity.png)

</details>
