# simulator

## Tools

1. `lightning-simulator`

- Simulate a set of payments with a selected pathfinding approach and payment type.

  <details>
    <summary>Usage</summary>

    ```
    lightning-simulator [OPTIONS] --amount <AMOUNT> --centrality <SCORE_FILE> --path-metric <EDGE_WEIGHT> <GRAPH_FILE>

    Arguments:
      <GRAPH_FILE>  Path to JSON ile describing topology

    Options:
      -a, --amount <AMOUNT>                 The payment anount to be simulated in sat
      -r, --run <RUN>                       Set the seed for the simulation [default: 19]
      -n, --pairs <NUM_PAIRS>               Number of src/dest pairs to use in the simulation [default: 1000]
      -m, --adversaries <NUM_ADV>...        Percentage of adversarial nodes
      -s, --split                           Split the payment and route independently. Default is not to split and send as a single payment
      -p, --path-metric <EDGE_WEIGHT>       Route finding heuristic to use [possible values: minfee, maxprob]
      -l, --log <LOG_LEVEL>                 [default: info]
      -o, --out <OUTPUT_DIR>                Path to directory in which the results will be stored
      -b, --betweenness <BETWEENNESS_FILE>  Path to file containing betweenness scores
      -d, --degree <DEGREE_FILE>            Path to file containing betweenness scores
          --random                          Select adversaries using random sampling
          --min <MIN_SHARD>                 Min shard when using MPP
      -g, --graph-source <GRAPH_TYPE>       [possible values: lnd, lnr]
          --verbose
      -h, --help                            Print help information
      -V, --version                         Print version information
  ```

  </details>

2. `batch-simulator`

- Simulate a set of payments with all possible combinations of pathfinding
  approaches and payment types.

## Simulation results

Here are some results of a batch of simulations conducted with a snapshot dated
2023-05-16.

<details>
  <summary><b>Performance</b></summary>

  <p float="middle">
      <img src="plots/success_rate.pdf" width="45%" />
      <img src="plots/transaction_fees.pdf" width="45%" />
  </p>
  <p float="middle">
      <img src="plots/path_length.pdf" width="45%" />
      <img src="plots/htlc_attempts.pdf" width="45%" />
  </p>

</details>

<details>
  <summary><b>Privacy</b></summary>

  <p float="middle">
    <img src="plots/observation_rate.pdf" width="45%" />
    <img src="plots/predecessor_guesses.pdf" width="45%" />
  </p>
  <p float="middle">
    <img src="plots/path_diversity.pdf" width="45%" />
  </p>

</details>
