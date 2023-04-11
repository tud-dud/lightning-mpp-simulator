use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::path::Path;

mod helpers;
use helpers::*;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Graph {
    pub nodes: HashSet<Node>,
    #[serde(rename = "adjacency")]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edges: HashMap<ID, HashSet<Edge>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Node {
    /// Pubkey
    pub id: ID,
    pub alias: String,
    pub addresses: Vec<String>,
    pub rgb_color: String,
    pub out_degree: u32,
    pub in_degree: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Edge {
    /// Short channel id
    pub channel_id: String,
    /// The source node
    pub source: String,
    /// The destination node
    pub destination: String,
    /// BOLT #9 features bitmap for this channel
    pub features: String,
    /// Base fee changed by source to use this channel
    pub fee_base_msat: usize,
    /// Proportional fee changed by source to use this channel, in parts-per-million
    pub fee_proportional_millionths: usize,
    /// The smallest payment source will allow via this channel
    pub htlc_minimim_msat: usize,
    /// The largest payment source will allow via this channel
    pub htlc_maximum_msat: usize,
    /// CLTV delta across channel
    /// minimum difference between the expiration of an incoming and outgoing HTLC
    pub cltv_expiry_delta: usize,
    pub id: String,
    /// node's edge balance which we calculate after graph creation
    pub balance: usize,
    /// edge balance minus commited HTLCs
    pub liquidity: usize,
    /// channel capacity which is calculated after graph creation. The min of the involved nodes'
    /// max msat
    pub capacity: usize,
}

pub type ID = String;
pub type NodeRanks = Vec<ID>;

impl Graph {
    pub fn get_nodes(self) -> HashSet<Node> {
        self.nodes
    }
    pub fn get_nodes_as_vec(self) -> Vec<Node> {
        self.nodes.into_iter().collect()
    }
    pub fn get_edges(&self) -> HashMap<ID, HashSet<Edge>> {
        self.edges.clone()
    }
    pub fn get_edges_as_vec_vec(self) -> Vec<Vec<Edge>> {
        self.edges
            .into_iter()
            .map(|node_adj| node_adj.1.into_iter().collect())
            .collect()
    }
    pub fn get_edges_for_node(&self, node_id: &ID) -> HashSet<Edge> {
        match self.get_edges().get(node_id) {
            Some(adj_list) => adj_list.to_owned(),
            None => HashSet::default(),
        }
    }
    pub fn edge_count(self) -> usize {
        self.get_edges_as_vec_vec().iter().map(Vec::len).sum()
    }

    #[allow(unused)]
    pub(crate) fn get_node_ids(&self) -> Vec<String> {
        self.nodes.iter().map(|n| n.id.clone()).collect()
    }
}

pub fn read_node_rankings_from_file(
    nodes: &[ID],
    path: &Path,
) -> Result<NodeRanks, std::io::Error> {
    let file = File::open(path).unwrap_or_else(|_| panic!("Error reading {}.", path.display()));
    let reader = BufReader::new(file);
    let mut ranks: NodeRanks = vec![];
    for line in reader.lines().flatten() {
        if nodes.contains(&line) {
            ranks.push(line);
        }
    }
    Ok(ranks)
}

pub fn from_json_file(path: &Path) -> Result<Graph, serde_json::Error> {
    let json_str = fs::read_to_string(path).expect("Error reading file");
    from_json_str(&json_str)
}

pub fn from_web_str(json_str: &str) -> Result<Graph, serde_json::Error> {
    let web_graph: Result<WebGraph, serde_json::Error> = serde_json::from_str(json_str);
    match web_graph {
        Ok(web_graph) => {
            if let Ok(raw_str) = from_web_graph_to_raw(web_graph) {
                from_json_str(&raw_str)
            } else {
                panic!("Error parsing raw graph string")
            }
        }
        Err(_) => panic!("Error reading web graph."),
    }
}

pub fn from_json_str(json_str: &str) -> Result<Graph, serde_json::Error> {
    let raw_graph = from_json_to_raw(json_str).expect("Error deserialising JSON str!");
    // discard nodes without ID
    let nodes: HashSet<Node> = raw_graph
        .nodes
        .iter()
        .filter(|raw_node| raw_node.id.clone().unwrap_or_default() != ID::default())
        .map(|raw_node| Node::from_raw(raw_node.clone()))
        .collect();

    let mut edges: HashMap<ID, HashSet<Edge>> = HashMap::with_capacity(raw_graph.edges.len());
    // discard edges with unknown IDs
    let edges_vec: Vec<HashSet<Edge>> = raw_graph
        .edges
        .iter()
        .map(|adj| {
            adj.iter()
                .filter(|raw_edge| {
                    // We only need the ID to know if the node exists
                    let src_node = Node {
                        id: raw_edge.source.clone().unwrap(),
                        ..Default::default()
                    };
                    let dest_node = Node {
                        id: raw_edge.destination.clone().unwrap(),
                        ..Default::default()
                    };
                    nodes.contains(&src_node) && nodes.contains(&dest_node)
                })
                .filter(|raw_edge| Edge::from_raw(&(*raw_edge).clone()).is_some())
                .map(|raw_edge| Edge::from_raw(raw_edge).unwrap())
                .collect()
        })
        .collect();
    for node_adj in edges_vec {
        for edge in node_adj {
            match edges.get_mut(&edge.source) {
                Some(node) => node.insert(edge),
                None => {
                    edges.insert(edge.source.clone(), HashSet::from([edge]));
                    true // weird so that match arms return same type
                }
            };
        }
    }
    Ok(Graph { nodes, edges })
}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Node {}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Edge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.channel_id.hash(state);
    }
}

impl Eq for Edge {}
impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        self.channel_id == other.channel_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn nodes_from_json_str() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
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
              ]
            }"##;
        let graph = from_json_str(json_str).unwrap();
        let nodes: Vec<Node> = graph.nodes.into_iter().collect();
        let actual = &nodes[0];
        let expected = Node {
            id: "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32".to_string(),
            alias: "MilliBit".to_string(),
            rgb_color: "550055".to_string(),
            addresses: vec!["ipv4://83.85.142.36:9735".to_string()],
            out_degree: 25,
            in_degree: 9,
        };
        assert_eq!(*actual, expected);
    }

    #[test]
    fn edges_from_json_str() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                },
                {
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                }
            ],
            "adjacency": [
                [
                  {
                    "scid": "714105x2146x0/0",
                    "source": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "destination": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c",
                    "timestamp": 1656588194,
                    "features": "",
                    "fee_base_msat": 5,
                    "fee_proportional_millionths": 270,
                    "htlc_minimim_msat": 1000,
                    "htlc_maximum_msat": 5564111000,
                    "cltv_expiry_delta": 34,
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                  },
                  {
                    "scid": "714116x477x0/0",
                    "source": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "destination": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d",
                    "timestamp": 1656522407,
                    "features": "",
                    "fee_base_msat": 0,
                    "fee_proportional_millionths": 555,
                    "htlc_minimim_msat": 1,
                    "htlc_maximum_msat": 5545472000,
                    "cltv_expiry_delta": 34,
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                  }
                ]
              ]
            }"##;
        let graph = from_json_str(json_str).unwrap();
        let actual = graph.edges;
        let expected = HashMap::from([(
            "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32".to_string(),
            HashSet::from([
                Edge {
                    channel_id: "714105x2146x0/0".to_string(),
                    source: "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32"
                        .to_string(),
                    destination:
                        "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                            .to_string(),
                    features: String::default(),
                    fee_base_msat: 5,
                    fee_proportional_millionths: 270,
                    htlc_minimim_msat: 1000,
                    htlc_maximum_msat: 5564111000,
                    cltv_expiry_delta: 34,
                    id: "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                        .to_string(),
                    balance: 0,
                    capacity: 0,
                    liquidity: 0,
                },
                Edge {
                    channel_id: "714116x477x0/0".to_string(),
                    source: "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32"
                        .to_string(),
                    destination:
                        "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                            .to_string(),
                    features: String::default(),
                    fee_base_msat: 0,
                    fee_proportional_millionths: 555,
                    htlc_minimim_msat: 1,
                    htlc_maximum_msat: 5545472000,
                    cltv_expiry_delta: 34,
                    id: "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                        .to_string(),
                    balance: 0,
                    liquidity: 0,
                    capacity: 0,
                },
            ]),
        )]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn empty_fields_in_nodes_are_ok() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32"
                }
            ],
            "adjacency": [
              ]
            }"##;
        let graph = from_json_str(json_str).unwrap();
        let nodes: Vec<Node> = graph.nodes.into_iter().collect();
        let actual = &nodes[0];
        let expected = Node {
            id: "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32".to_string(),
            alias: String::default(),
            rgb_color: String::default(),
            addresses: Vec::default(),
            out_degree: u32::default(),
            in_degree: u32::default(),
        };
        assert_eq!(*actual, expected);
    }

    #[test]
    fn graph_from_json_file() {
        let path_to_file = Path::new("../test_data/trivial.json");
        let actual = from_json_file(path_to_file);
        assert!(actual.is_ok());
        let graph = actual.unwrap();
        let edges: HashMap<ID, HashSet<Edge>> = graph.edges.into_iter().collect();
        assert_eq!(graph.nodes.len(), 7);
        assert!(edges
            .contains_key("021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32"));
    }

    #[test]
    fn edges_to_vec() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d",
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
                    "source": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "destination": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c",
                    "timestamp": 1656588194,
                    "features": "",
                    "fee_base_msat": 5,
                    "fee_proportional_millionths": 270,
                    "htlc_minimim_msat": 1000,
                    "htlc_maximum_msat": 5564111000,
                    "cltv_expiry_delta": 34,
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                  },
                  {
                    "scid": "714116x477x0/0",
                    "source": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "destination": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d",
                    "timestamp": 1656522407,
                    "features": "",
                    "fee_base_msat": 0,
                    "fee_proportional_millionths": 555,
                    "htlc_minimim_msat": 1,
                    "htlc_maximum_msat": 5545472000,
                    "cltv_expiry_delta": 34,
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                  }
                ]
              ]
            }"##;
        let graph = from_json_str(json_str).unwrap();
        let actual = graph.get_edges_as_vec_vec();
        let expected = vec![
            Edge {
                channel_id: "714105x2146x0/0".to_string(),
                source: "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32"
                    .to_string(),
                destination: "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                    .to_string(),
                features: String::default(),
                fee_base_msat: 5,
                fee_proportional_millionths: 270,
                htlc_minimim_msat: 1000,
                htlc_maximum_msat: 5564111000,
                cltv_expiry_delta: 34,
                id: "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                    .to_string(),
                balance: 0,
                liquidity: 0,
                capacity: 0,
            },
            Edge {
                channel_id: "714116x477x0/0".to_string(),
                source: "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32"
                    .to_string(),
                destination: "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                    .to_string(),
                features: String::default(),
                fee_base_msat: 0,
                fee_proportional_millionths: 555,
                htlc_minimim_msat: 1,
                htlc_maximum_msat: 5545472000,
                cltv_expiry_delta: 34,
                id: "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                    .to_string(),
                balance: 0,
                liquidity: 0,
                capacity: 0,
            },
        ];
        for edge in expected {
            assert!(actual[0].contains(&edge));
        }
    }

    #[test]
    fn get_edges_for_node_wo_edges() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
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
              ]
            }"##;
        let graph = from_json_str(&json_str).unwrap();
        let actual = graph.get_edges_for_node(
            &"021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32".to_string(),
        );
        let expected = HashSet::default();
        assert_eq!(actual, expected);
    }

    #[test]
    fn num_edges_in_graph() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d",
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
                    "source": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "destination": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c",
                    "timestamp": 1656588194,
                    "features": "",
                    "fee_base_msat": 5,
                    "fee_proportional_millionths": 270,
                    "htlc_minimim_msat": 1000,
                    "htlc_maximum_msat": 5564111000,
                    "cltv_expiry_delta": 34,
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                  },
                  {
                    "scid": "714116x477x0/0",
                    "source": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d",
                    "destination": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "timestamp": 1656522407,
                    "features": "",
                    "fee_base_msat": 0,
                    "fee_proportional_millionths": 555,
                    "htlc_minimim_msat": 1,
                    "htlc_maximum_msat": 5545472000,
                    "cltv_expiry_delta": 34,
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                  }
                ]
              ]
            }"##;
        let graph = from_json_str(&json_str).unwrap();
        let actual = graph.edge_count();
        let expected = 2;
        assert_eq!(actual, expected);
    }

    #[test]
    fn discard_edges_without_necessary_fields() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                },
                {
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                }
            ],
            "adjacency": [
                [
                  {
                    "scid": "714105x2146x0/0",
                    "source": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "destination": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c",
                    "timestamp": 1656588194,
                    "features": "",
                    "fee_base_msat": 5,
                    "fee_proportional_millionths": 270,
                    "htlc_minimim_msat": 1000,
                    "cltv_expiry_delta": 34,
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                  },
                  {
                    "scid": "714116x477x0/0",
                    "source": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "destination": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d",
                    "timestamp": 1656522407,
                    "features": "",
                    "fee_base_msat": 0,
                    "htlc_minimim_msat": 1,
                    "htlc_maximum_msat": 5545472000,
                    "cltv_expiry_delta": 34,
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                  }
                ]
              ]
            }"##;
        let graph = from_json_str(&json_str).unwrap();
        let actual = graph.edge_count();
        let expected = 0;
        assert_eq!(expected, actual);
    }

    #[test]
    fn get_nodes() {
        let path_to_file = Path::new("../test_data/trivial_connected.json");
        let graph = from_json_file(path_to_file).unwrap();
        let actual = graph.get_node_ids();
        let expected = vec!["025".to_owned(), "034".to_owned(), "036".to_owned()];
        assert_eq!(actual.len(), expected.len());
        for id in actual {
            assert!(expected.contains(&id));
        }
    }

    #[test]
    fn read_rankings() {
        let mut rankings_file = NamedTempFile::new().expect("Error opening NamedTempFile.");
        let _ = writeln!(rankings_file, "036");
        let _ = writeln!(rankings_file, "034");
        let _ = writeln!(rankings_file, "025");
        let nodes = ["036".to_string(), "034".to_string(), "025".to_string()];
        let actual = read_node_rankings_from_file(&nodes, rankings_file.path());
        assert!(actual.is_ok());
        let actual = actual.unwrap();
        assert_eq!(actual.len(), nodes.len());
        let expected = vec!["036".to_owned(), "025".to_owned(), "034".to_owned()];
        assert_eq!(actual.len(), expected.len());
        for id in actual {
            assert!(expected.contains(&id));
        }
        // node is not in the graph - should not change anything
        let _ = writeln!(rankings_file, "043");
        let actual = read_node_rankings_from_file(&nodes, rankings_file.path());
        assert!(actual.is_ok());
        let actual = actual.unwrap();
        assert_eq!(actual.len(), nodes.len());
        let expected = vec!["036".to_owned(), "025".to_owned(), "034".to_owned()];
        assert_eq!(actual.len(), expected.len());
        for id in actual {
            assert!(expected.contains(&id));
        }
    }
}
