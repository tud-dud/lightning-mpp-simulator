use crate::{ID, RNG};
use network_parser::{Edge, Node};

use log::{debug, info};
use pathfinding::directed::strongly_connected_components::strongly_connected_components;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use std::{cmp, collections::HashMap};

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
        let greatest_scc = graph.reduce_to_greatest_scc();
        let mut greatest_scc = greatest_scc.remove_unidrectional_edges();
        greatest_scc.set_channel_balances();
        greatest_scc
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

    pub(crate) fn get_edges_for_node(&self, node_id: &ID) -> Option<Vec<Edge>> {
        self.get_edges()
            .get(node_id)
            .map(|adj_list| adj_list.to_owned())
    }

    /// Will try to remove the edge in both directions
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

    /// We calculate balances based on the edges' max_sat values using a random uniform
    /// distribution
    fn set_channel_balances(&mut self) {
        info!("Calculating channel balances.");
        // hm
        let graph_copy = self.clone();
        let mut rng = RNG.lock().unwrap();
        for (src, edges) in self.edges.iter_mut() {
            for out_edge in edges.iter_mut() {
                // means we haven't visited the edge before; might break if htlc_maximum_msat == 0
                if out_edge.balance == usize::default() {
                    // Channel capacity is assumed to be the lower htlc_maximum_msat value
                    if let Some(mut reverse_edge) = graph_copy.get_edge(&out_edge.destination, src)
                    {
                        let src_capacity_dist: f32 = rng.gen();
                        let max_src_htlc = &out_edge.htlc_maximum_msat;
                        let max_dest_htlc = reverse_edge.htlc_maximum_msat;
                        let capacity = *cmp::min(max_src_htlc, &max_dest_htlc) as f32;
                        let src_balance = (src_capacity_dist * capacity).round();
                        let dest_balance = capacity - src_balance;
                        reverse_edge.balance = dest_balance as usize;
                        out_edge.balance = src_balance as usize;
                    }
                }
            }
        }
    }

    fn remove_unidrectional_edges(&self) -> Self {
        info!("Deleting unidirectional edges from graph.");
        let mut graph_copy = self.clone();
        let mut num_removed = 0;
        for (src, edges) in self.edges.iter() {
            let from = src;
            for out in edges.iter() {
                let to = &out.destination;
                // check if to->from exists
                let edges_from_to: Vec<ID> = if let Some(edges) = self.get_edges_for_node(to) {
                    edges.iter().map(|edge| edge.destination.clone()).collect()
                } else {
                    Vec::default()
                };
                if !edges_from_to.contains(from) {
                    graph_copy.remove_edge(from, to);
                    num_removed += 1;
                }
            }
        }
        debug!("Removed {} unidirectional edges.", num_removed);
        info!(
            "Proceeding with {} nodes and {} edges.",
            graph_copy.node_count(),
            graph_copy.edge_count()
        );
        graph_copy
    }

    /// Use get_all_src_dest_edges to get all such edges
    fn get_edge(&self, from: &ID, to: &ID) -> Option<Edge> {
        let out_edges = self.get_outedges(from);
        // Assumes there is at most one edge from dest to src
        out_edges
            .iter()
            .find(|out| out.destination == to.clone())
            .cloned()
    }

    /// Returns all edges between two nodes. Empty if there are none
    pub(crate) fn get_all_src_dest_edges(&self, from: &ID, to: &ID) -> Vec<Edge> {
        self.get_outedges(from)
            .into_iter()
            .filter(|edge| edge.destination == to.clone())
            .collect()
    }

    pub(crate) fn get_random_pair_of_nodes(&self) -> (ID, ID) {
        let node_ids = self.get_node_ids();
        assert!(
            !node_ids.is_empty(),
            "Empty node list cannot be sampled for pairs."
        );
        assert!(node_ids.len() >= 2, "Set of nodes is too small to sample.");
        let mut rng = RNG.lock().unwrap();
        let pair: Vec<ID> = node_ids.choose_multiple(&mut *rng, 2).cloned().collect();
        (pair[0].to_owned(), pair[1].to_owned())
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
    use std::path::Path;

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
        assert_eq!(num_edges, 2);
    }

    #[test]
    fn scc_compuatation() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let actual = graph.get_sccs();
        assert_eq!(actual.len(), 2);
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
        assert_eq!(sccs.len(), 5); //empty string is an SCC. somehow..
        let actual = graph.reduce_to_greatest_scc();
        assert_eq!(actual.node_count(), 2);
        assert_eq!(actual.edge_count(), 2);
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
        assert!(graph.get_node_ids().contains(&actual_src));
        assert!(graph.get_node_ids().contains(&actual_dest));
    }

    #[test]
    fn get_edge_from_src_to_dest() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let from = "random0".to_string();
        let to = "random1".to_string();
        let actual = graph.get_edge(&from, &to);
        assert!(actual.is_none());
        let from = "random2".to_string();
        let actual = graph.get_edge(&from, &to);
        let expected = Some(Edge {
            channel_id: String::from("714116xx0/0"),
            source: from,
            destination: to,
            features: String::default(),
            fee_base_msat: 5,
            fee_proportional_millionths: 270,
            htlc_minimim_msat: 1000,
            htlc_maximum_msat: 5564111000,
            cltv_expiry_delta: 34,
            id: String::default(),
            balance: actual.clone().unwrap().balance, // hacky because it depends on the RNG
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn get_nodes_outedges() {
        let json_str = json_str();
        let graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        let node = String::from("random1");
        let actual = graph.get_outedges(&node);
        let expected = vec![Edge {
            channel_id: String::from("714116x477x0/0"),
            source: node,
            destination: String::from("random1"),
            features: String::default(),
            fee_base_msat: 0,
            fee_proportional_millionths: 555,
            htlc_minimim_msat: 1,
            htlc_maximum_msat: 5545472000,
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

    #[test]
    fn add_edge_balances() {
        let json_str = json_str();
        let mut graph = Graph::to_sim_graph(&network_parser::from_json_str(&json_str).unwrap());
        graph.set_channel_balances();
        println!("graph {:?}", graph);
        for edges in graph.edges.into_values() {
            for e in edges {
                assert!(e.balance != usize::default());
                assert!(e.balance <= e.htlc_maximum_msat);
            }
        }
    }

    #[test]
    fn all_edges_between_two_nodes() {
        let graph = Graph::to_sim_graph(
            &network_parser::from_json_file(&Path::new("../test_data/trivial_connected.json"))
                .unwrap(),
        );
        let nodes = graph.get_node_ids();
        for (idx, node) in nodes.iter().enumerate() {
            let from = node;
            let to = nodes[idx + 1 % nodes.len() - 1].clone();
            if *from != to {
                let actual = graph.get_all_src_dest_edges(&from, &to);
                assert_eq!(actual.len(), 1);
            }
        }
    }
}
