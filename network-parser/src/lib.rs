use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

mod helpers;
use helpers::*;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Graph {
    pub nodes: HashSet<Node>,
    #[serde(rename = "adjacency")]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub edges: HashMap<ID, HashSet<Edge>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Node {
    pub id: ID,
    pub alias: String,
    pub addresses: Addresses,
    pub rgb_color: String,
    pub out_degree: u32,
    pub in_degree: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Edge {
    pub channel_id: String,
    pub source: String,
    pub destination: String,
    pub features: String,
    pub fee_base_msat: u32,
    pub fee_proportional_millionths: usize,
    pub htlc_minimim_msat: usize,
    pub htlc_maximum_msat: usize,
    pub cltv_expiry_delta: u32,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Addresses(pub Vec<String>);

pub type ID = String;

impl Graph {
    pub fn get_nodes(self) -> HashSet<Node> {
        self.nodes
    }
    pub fn get_nodes_as_vec(self) -> Vec<Node> {
        self.nodes.into_iter().collect()
    }
    pub fn get_edges(self) -> HashMap<ID, HashSet<Edge>> {
        self.edges
    }
    pub fn get_edges_as_vec_vec(self) -> Vec<Vec<Edge>> {
        self.edges
            .into_iter()
            .map(|node_adj| node_adj.1.into_iter().collect())
            .collect()
    }
    pub fn get_edges_for_node(self, node_id: &ID) -> HashSet<Edge> {
        match self.get_edges().get(node_id) {
            Some(adj_list) => adj_list.to_owned(),
            None => HashSet::default(),
        }
    }
}

pub fn from_json_file(path: &Path) -> Result<Graph, serde_json::Error> {
    let json_str = fs::read_to_string(path).expect("Error reading file");
    from_json_str(&json_str)
}

pub fn from_json_str(json_str: &str) -> Result<Graph, serde_json::Error> {
    let raw_graph = from_json_to_raw(json_str).expect("Error deserialising JSON str!");
    let nodes: HashSet<Node> = raw_graph
        .nodes
        .iter()
        .map(|raw_node| Node::from_raw(raw_node.clone()))
        .collect();

    let mut edges: HashMap<ID, HashSet<Edge>> = HashMap::with_capacity(raw_graph.edges.len());
    let edges_vec: Vec<HashSet<Edge>> = raw_graph
        .edges
        .iter()
        .map(|adj| {
            adj.iter()
                .map(|raw_edge| Edge::from_raw(raw_edge.clone()))
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
            addresses: Addresses(vec!["ipv4://83.85.142.36:9735".to_string()]),
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
            addresses: Addresses::default(),
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
        assert_eq!(graph.nodes.len(), 4);
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
}
