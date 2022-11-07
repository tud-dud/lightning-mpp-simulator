use network_parser;
use petgraph::prelude::{DiGraph, NodeIndex};

use crate::Petgraph;

#[derive(Clone)]
pub struct Graph(pub Petgraph);

impl Graph {
    /// Transform to petgraph to allow graph operations such as SCC and shortest path computations
    pub fn to_petgraph(net_graph: &network_parser::Graph) -> Petgraph {
        let mut graph: Petgraph =
            DiGraph::with_capacity(net_graph.nodes.len(), net_graph.edges.len());
        let node_idx: Vec<NodeIndex> = net_graph
            .clone()
            .nodes
            .into_iter()
            .map(|node| graph.add_node(node))
            .collect();
        for src in node_idx {
            let id = &graph[src].id;
            let edges = net_graph.clone().get_edges_for_node(&id.clone());
            for edge in edges {
                let dest = graph
                    .node_indices()
                    .find(|i| graph[*i].id == edge.destination)
                    .unwrap();
                graph.add_edge(src, dest, edge);
            }
        }
        graph
    }

    pub fn reduce_to_greatest_scc(self) -> Petgraph {
        let sccs = self.clone().get_sccs();
        let mut greatest_scc_idx: usize = 0;
        for (idx, cc) in sccs.iter().enumerate() {
            if cc.len() >= greatest_scc_idx {
                greatest_scc_idx = idx;
            }
        }
        let greatest_scc = sccs[greatest_scc_idx].clone();
        let mut scc_subgraph = self.0;
        for node_id in greatest_scc {
            scc_subgraph.remove_node(node_id);
        }
        scc_subgraph
    }

    fn get_sccs(self) -> Vec<Vec<petgraph::stable_graph::NodeIndex>> {
        petgraph::algo::tarjan_scc(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_str() -> String {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "random0",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "random1",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "random2",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                }
            ],
            "adjacency": [
                [
                  {
                    "scid": "714105x2146x0/0",
                    "source": "random0",
                    "destination": "random1",
                    "timestamp": 1656588194,
                    "features": "",
                    "fee_base_msat": 5,
                    "fee_proportional_millionths": 270,
                    "htlc_minimim_msat": 1000,
                    "htlc_maximum_msat": 5564111000,
                    "cltv_expiry_delta": 34
                  }
                ],
                [
                  {
                    "scid": "714116x477x0/0",
                    "source": "random1",
                    "destination": "random2",
                    "timestamp": 1656522407,
                    "features": "",
                    "fee_base_msat": 0,
                    "fee_proportional_millionths": 555,
                    "htlc_minimim_msat": 1,
                    "htlc_maximum_msat": 5545472000,
                    "cltv_expiry_delta": 34
                  }
                ],
                [
                  {
                    "scid": "714116xx0/0",
                    "source": "random2",
                    "destination": "random1",
                    "timestamp": 1656522407,
                    "features": "",
                    "fee_base_msat": 0,
                    "fee_proportional_millionths": 555,
                    "htlc_minimim_msat": 1,
                    "htlc_maximum_msat": 5545472000,
                    "cltv_expiry_delta": 34
                  },
                  {
                    "scid": "71116xx0/0",
                    "source": "random2",
                    "destination": "random0",
                    "timestamp": 1656522407,
                    "features": "",
                    "fee_base_msat": 0,
                    "fee_proportional_millionths": 555,
                    "htlc_minimim_msat": 1,
                    "htlc_maximum_msat": 5545472000,
                    "cltv_expiry_delta": 34
                  }
                ]
              ]
            }"##;
        json_str.to_string()
    }

    #[test]
    fn transform_works() {
        let json_str = json_str();
        let graph = network_parser::from_json_str(&json_str).unwrap();
        let petgraph = Graph::to_petgraph(&graph);
        let num_nodes = petgraph.node_count();
        assert_eq!(num_nodes, 3);
        let num_edges = petgraph.edge_count();
        assert_eq!(num_edges, 4);
    }

    #[test]
    fn scc_compuatation() {
        let json_str = json_str();
        let graph = Graph(Graph::to_petgraph(
            &network_parser::from_json_str(&json_str).unwrap(),
        ));
        let actual = graph.get_sccs();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].len(), 3);
    }

    #[test]
    fn greatest_scc_subgraph() {
        let json_str = json_str();
        let mut graph = Graph(Graph::to_petgraph(
            &network_parser::from_json_str(&json_str).unwrap(),
        ));
        let node1 = graph.0.add_node(network_parser::Node::default());
        let node2 = graph.0.add_node(network_parser::Node::default());
        graph
            .0
            .add_edge(node1, node2, network_parser::Edge::default());
        graph
            .0
            .add_edge(node2, node1, network_parser::Edge::default());
        let sccs = graph.clone().get_sccs();
        assert_eq!(sccs.len(), 2);
        let actual = graph.reduce_to_greatest_scc();
        assert_eq!(actual.node_count(), 3);
        assert_eq!(actual.edge_count(), 4);
    }
}
