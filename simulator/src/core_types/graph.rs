use std::collections::HashMap;

use log::{debug, info};
use network_parser::{Edge, Node};
use pathfinding::directed::strongly_connected_components::strongly_connected_components;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::{ID, RNG};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Graph {
    pub(crate) nodes: Vec<Node>,
    #[serde(rename = "adjacency")]
    pub(crate) edges: HashMap<ID, Vec<Edge>>,
}

impl Graph {
    /// Transform to another type of graph to allow graph operations such as SCC and shortest path computations
    pub fn to_sim_graph(net_graph: &network_parser::Graph) -> Graph {
        let nodes: Vec<Node> = net_graph.nodes.clone().into_iter().collect();
        let edges: HashMap<ID, Vec<Edge>> = net_graph
            .clone()
            .edges
            .into_iter()
            .map(|(id, edge)| (id, Vec::from_iter(edge)))
            .collect();
        let graph = Graph { nodes, edges };
        let mut greatest_scc = graph.reduce_to_greatest_scc();
        greatest_scc.set_channel_balances();
        greatest_scc
    }

    /// We calculate balances based on the edges' max_sat values using a random uniform
    /// distribution
    fn set_channel_balances(&mut self) {
        info!("Calculating channel balances..");
        let mut rng = RNG.lock().unwrap();
        for edges in self.edges.iter() {
            let src = edges.0;
            // edges from src
            for e in edges.1 {
                let to = e.destination.clone();
                let edges_from_to = self.get_outedges(&to);
                // get edge to->src
                let max_msat_from_src = e.htlc_maximum_msat;
            }
        }
    }

    fn reduce_to_greatest_scc(&self) -> Graph {
        info!(
            "Reducing graph with {} nodes and {} edges to greatest SCC.",
            self.node_count(),
            self.edge_count()
        );
        let mut sccs = self.get_sccs();
        sccs.retain(|scc| !scc.is_empty());
        let mut greatest_scc_idx: usize = 0;
        let mut greatest_scc_len: usize = 0;
        for (idx, cc) in sccs.iter().enumerate() {
            if cc.len() >= greatest_scc_len {
                greatest_scc_len = cc.len();
                greatest_scc_idx = idx;
            }
        }
        let greatest_scc_nodes: Vec<Node> = self
            .nodes
            .clone()
            .into_iter()
            .filter(|n| sccs[greatest_scc_idx].contains(&n.id))
            .into_iter()
            .clone()
            .collect();
        let greatest_scc_edges: HashMap<ID, Vec<Edge>> = greatest_scc_nodes
            .iter()
            .map(|n| (n.id.clone(), self.edges.get(&n.id).unwrap().clone()))
            .collect();

        let g = Graph {
            nodes: greatest_scc_nodes,
            edges: greatest_scc_edges,
        };
        info!(
            "Reduced to graph with {} nodes and {} edges.",
            g.node_count(),
            g.edge_count()
        );
        g
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.clone().into_iter().map(|(_, v)| v.len()).sum()
    }

    pub fn get_node_ids(&self) -> Vec<ID> {
        self.nodes.iter().map(|n| n.id.clone()).collect()
    }

    pub(crate) fn get_edges(&self) -> &HashMap<String, Vec<Edge>> {
        &self.edges
    }

    /// Remove the edge in both directions
    pub(crate) fn remove_edge(&mut self, src: &ID, dest: &ID) {
        // The edge (src, dest) exists
        if let Some(src_edges) = self.edges.get_mut(src) {
            src_edges.retain(|edges| edges.destination != dest.clone());
        }
        // The edge (dest, src) exists
        if let Some(dest_edges) = self.edges.get_mut(dest) {
            dest_edges.retain(|edges| edges.destination != src.clone());
        }
    }

    pub(crate) fn get_outedges(&self, node_id: &ID) -> Vec<Edge> {
        if let Some(out_edges) = self.edges.get(node_id) {
            out_edges.clone()
        } else {
            Vec::default()
        }
    }

    // FIXME: Parallel edges between nodes
    fn get_edge(&self, from: &ID, to: &ID) -> Option<Edge> {
        let out_edges = self.get_outedges(from);
        // Assumes there is at most one edge from dest to src
        out_edges
            .iter()
            .find(|out| out.destination == to.clone())
            .cloned()
    }

    /// TODO: We can use choose_multiple here
    pub(crate) fn get_random_pair_of_nodes(&self) -> (ID, ID) {
        let node_ids = self.get_node_ids();
        assert!(
            !node_ids.is_empty(),
            "Empty node list cannot be sampled for pairs."
        );
        assert!(node_ids.len() >= 2, "Set of nodes is too small to sample.");
        let mut rng = RNG.lock().unwrap();
        let src = node_ids.choose(&mut *rng).unwrap();
        let mut dest = node_ids.choose(&mut *rng).unwrap();
        while dest == src {
            dest = node_ids.choose(&mut *rng).unwrap()
        }
        (src.clone(), dest.clone())
    }

    fn get_sccs(&self) -> Vec<Vec<ID>> {
        let successors = |node: &ID| -> Vec<ID> {
            if let Some(succs) = self.edges.get(&node.to_owned()) {
                let nbrs: Vec<ID> = succs.iter().map(|e| e.destination.clone()).collect();
                nbrs
            } else {
                Vec::default()
            }
        };
        let nodes: Vec<ID> = self.nodes.iter().map(|n| n.id.clone()).collect();
        let sccs = strongly_connected_components(&nodes, successors);
        debug!("Got {} SCCs", sccs.len());
        sccs
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
        let digraph = Graph::to_sim_graph(&graph);
        let num_nodes = digraph.node_count();
        assert_eq!(num_nodes, 3);
        let num_edges = digraph.edge_count();
        assert_eq!(num_edges, 4);
    }

    #[test]
    fn scc_compuatation() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let actual = graph.get_sccs();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0].len(), 3);
    }

    #[test]
    fn greatest_scc_subgraph() {
        let json_str = json_str();
        let mut graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        graph.nodes.push(network_parser::Node {
            id: "scc1".to_string(),
            ..Default::default()
        });
        graph.nodes.push(network_parser::Node {
            id: "scc2".to_string(),
            ..Default::default()
        });
        graph
            .edges
            .insert("scc1".to_string(), vec![network_parser::Edge::default()]);
        graph
            .edges
            .insert("scc2".to_string(), vec![network_parser::Edge::default()]);
        let sccs = graph.clone().get_sccs();
        assert_eq!(sccs.len(), 4); //empty string is an SCC. somehow..
        let actual = graph.reduce_to_greatest_scc();
        assert_eq!(actual.node_count(), 3);
        assert_eq!(actual.edge_count(), 4);
    }

    #[test]
    fn fetch_node_ids() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let actual = graph.get_node_ids();
        assert_eq!(actual.len(), graph.nodes.len());
        for node in graph.nodes {
            assert!(actual.contains(&node.id));
        }
    }

    #[test]
    fn random_pair_of_nodes() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let (actual_src, actual_dest) = graph.get_random_pair_of_nodes();
        assert_ne!(actual_src, actual_dest);
        assert!(graph.get_node_ids().contains(&actual_src));
        assert!(graph.get_node_ids().contains(&actual_dest));
    }

    #[test]
    fn get_edge_src_to_dest() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let from = "random0".to_string();
        let to = "random1".to_string();
        let actual = graph.get_edge(&from, &to);
        let expected = Some(Edge {
            channel_id: String::from("714105x2146x0/0"),
            source: from,
            destination: to,
            features: String::default(),
            fee_base_msat: 5,
            fee_proportional_millionths: 270,
            htlc_minimim_msat: 1000,
            htlc_maximum_msat: 5564111000,
            cltv_expiry_delta: 34,
            id: String::default(),
            balance: 0,
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn get_nodes_outedges() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let node = String::from("random0");
        let actual = graph.get_outedges(&node);
        let expected = vec![Edge {
            channel_id: String::from("714105x2146x0/0"),
            source: node,
            destination: String::from("random1"),
            features: String::default(),
            fee_base_msat: 5,
            fee_proportional_millionths: 270,
            htlc_minimim_msat: 1000,
            htlc_maximum_msat: 5564111000,
            cltv_expiry_delta: 34,
            id: String::default(),
            balance: 0,
        }];
        assert_eq!(actual, expected);
    }

    #[test]
    fn delete_edge() {
        let json_str = json_str();
        let mut graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let node1 = String::from("random1");
        let node2 = String::from("random2");
        let node1_edge_len = graph.edges[&node1].len();
        let node2_edge_len = graph.edges[&node2].len();
        assert!(graph.get_edge(&node1, &node2).is_some());
        assert!(graph.get_edge(&node2, &node1).is_some());
        graph.remove_edge(&node1, &node2);
        let node1_edge_new_len = graph.edges[&node1].len();
        let node2_edge_new_len = graph.edges[&node2].len();
        assert_eq!(node1_edge_len - 1, node1_edge_new_len);
        assert_eq!(node2_edge_len - 1, node2_edge_new_len);
        assert!(graph.get_edge(&node1, &node2).is_none());
        assert!(graph.get_edge(&node2, &node1).is_none());
    }
}
