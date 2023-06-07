# lightning-simulator

[![Rust](https://camo.githubusercontent.com/5782bcc58a7786e9a7d00e2cf45937db8a2598232d9524ec9dcd149c7218671b/68747470733a2f2f696d672e736869656c64732e696f2f62616467652f527573742d50726f6772616d6d696e672532304c616e67756167652d626c61636b3f7374796c653d666c6174266c6f676f3d72757374)](www.rust-lang.org)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)

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

Here are the results of a series of simulations conducted with a snapshot dated
2023-05-16.

<details open>
  <summary><b>Performance</b></summary>

  <p float="middle">
      <em>Median success rate</em>
      <img src="plots/success_rate.png" width="45%" />
      <em>Absolute and relative fees in sat</em>
      <img src="plots/transaction_fees.png" width="45%" />
  </p>
  <p float="middle">
      <em>Successful paths' lengths</em>
      <img src="plots/path_length.png" width="45%" />
      <em>Total and relative number of payment attempts</em>
      <img src="plots/htlc_attempts.png" width="45%" />
  </p>
  <p float="middle">
      <em>Failed paths' lengths</em>
      <img src="plots/failed_path_length.png" width="45%" />
      <em>Number of payment parts</em>
      <img src="plots/splits.png" width="45%" />
  </p>

</details>

<details open>
  <summary><b>Privacy</b></summary>

  <p float="middle">
      <em>Observation rate for successful payments</em>
      <img src="plots/observation_rate.png" width="45%" />
      <em>Predecessor/successor attack probability</em>
      <img src="plots/predecessor_guesses.png" width="45%" />
  </p>
  <p float="middle">
      <em>Number of payments vulnerable to confirmation attacks</em>
      <img src="plots/vulnerable_successful_payments.png" width="45%" />
      <em>Path diversity</em>
      <img src="plots/path_diversity.png" width="45%" />
  </p>

</details>
