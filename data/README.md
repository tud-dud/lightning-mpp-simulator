# Decompress then restore snapshot

- Topology dataset provided by
  [lnresearch](https://github.com/lnresearch/topology)
- Epoch time ```1656633600``` (2022-07-01:0000 UTC)

- restored using:
    ```
        cd topology
        python -m lntopo timemachine restore --fmt=json
        ../ln-sim/data/gossip-20220823.gsp.bz2 1656633600 >
        ../ln-sim/data/gossip-20220823.json
    ```
