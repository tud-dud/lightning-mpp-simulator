# Decompress then restore snapshot

- Topology dataset provided by
  [lnresearch](https://github.com/lnresearch/topology)
- Epoch time ```1656633600``` (2022-07-01:0000 UTC)
- Epoch time ```1630965600``` (2021-09-06:0000 UTC)

- restored using:
    ```
        cd topology
        python -m lntopo timemachine restore --fmt=json
        ../ln-sim/data/gossip-20220823.gsp.bz2 1656633600 >
        ../ln-sim/data/gossip-20220823.json
    ```

## Datasets in this directory

1. `./gossip-20210906-1000UTC.{gml, json}` - snapshot of the LN on 21st
   September 2021, 10am UTC
2. `./betweenness-centrality-20210906.txt` - node IDs of the above snapshot
   sorted in descending order of betweenness centrality
3. `./degree-centrality-centrality-20210906.txt` - node IDs of the above snapshot
   sorted in descending order of degree centrality
