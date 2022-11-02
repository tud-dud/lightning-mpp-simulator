use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    #[serde(rename = "adjacency")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<Vec<Edge>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Node {
    pub id: String,
    pub alias: String,
    pub addresses: Addresses,
    pub rgb_color: String,
    pub out_degree: u32,
    pub in_degree: u32,
}

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Edge {
    pub scid: String,
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
pub struct Addresses(Vec<String>);

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
struct RawGraph {
    nodes: Vec<RawNode>,
    #[serde(rename = "adjacency")]
    edges: Vec<Vec<RawEdge>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
struct RawNode {
    id: Option<String>,
    alias: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "addresses_deserialize")]
    addresses: RawAddresses,
    rgb_color: Option<String>,
    out_degree: Option<u32>,
    in_degree: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
struct RawEdge {
    scid: Option<String>,
    source: Option<String>,
    destination: Option<String>,
    pub features: Option<String>,
    fee_base_msat: Option<u32>,
    fee_proportional_millionths: Option<usize>,
    htlc_minimim_msat: Option<usize>,
    htlc_maximum_msat: Option<usize>,
    cltv_expiry_delta: Option<u32>,
    id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
struct RawAddresses(Option<Vec<String>>);

impl Node {
    // return default values if some are not pleasant
    // TODO: maybe discard if certain fields like ID are missing?
    fn from_raw(raw_node: RawNode) -> Node {
        Node {
            id: raw_node.id.unwrap_or_default(),
            alias: raw_node.alias.unwrap_or_default(),
            addresses: Addresses(raw_node.addresses.0.unwrap_or_default()),
            rgb_color: raw_node.rgb_color.unwrap_or_default(),
            out_degree: raw_node.out_degree.unwrap_or_default(),
            in_degree: raw_node.in_degree.unwrap_or_default(),
        }
    }
}

impl Edge {
    fn from_raw(raw_edge: RawEdge) -> Edge {
        Edge {
            scid: raw_edge.scid.unwrap_or_default(),
            source: raw_edge.source.unwrap_or_default(),
            destination: raw_edge.destination.unwrap_or_default(),
            features: raw_edge.features.unwrap_or_default(),
            fee_base_msat: raw_edge.fee_base_msat.unwrap_or_default(),
            fee_proportional_millionths: raw_edge.fee_proportional_millionths.unwrap_or_default(),
            htlc_minimim_msat: raw_edge.htlc_minimim_msat.unwrap_or_default(),
            htlc_maximum_msat: raw_edge.htlc_maximum_msat.unwrap_or_default(),
            cltv_expiry_delta: raw_edge.cltv_expiry_delta.unwrap_or_default(),
            id: raw_edge.id.unwrap_or_default(),
        }
    }
}

fn from_json_to_raw(json_str: &str) -> Result<RawGraph, serde_json::Error> {
    serde_json::from_str(json_str)
}

pub fn from_json_str(json_str: &str) -> Result<Graph, serde_json::Error> {
    let raw_graph = from_json_to_raw(json_str).expect("Error deserialising JSON str!");
    let nodes: Vec<Node> = raw_graph
        .nodes
        .iter()
        .map(|raw_node| Node::from_raw(raw_node.clone()))
        .collect();
    let edges: Vec<Vec<Edge>> = raw_graph
        .edges
        .iter()
        .map(|adj| {
            adj.iter()
                .map(|raw_edge| Edge::from_raw(raw_edge.clone()))
                .collect()
        })
        .collect();

    Ok(Graph { nodes, edges })
}

pub fn from_json_file(path: &Path) -> Result<Graph, serde_json::Error> {
    let json_str = fs::read_to_string(path).expect("Error reading file");
    from_json_str(&json_str)
}

fn addresses_deserialize<'de, D>(deserializer: D) -> Result<RawAddresses, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    let addresses: Vec<String> = str_sequence
        .split(',')
        .map(|item| item.to_owned())
        .collect();
    Ok(RawAddresses(Some(addresses)))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let actual = &graph.nodes[0];
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
        let expected = vec![vec![
            Edge {
                scid: "714105x2146x0/0".to_string(),
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
                scid: "714116x477x0/0".to_string(),
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
        ]];
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
        let actual = &graph.nodes[0];
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
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(
            graph.edges[0][0],
            Edge {
                scid: "714105x2146x0/0".to_string(),
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
            }
        );
    }
}
