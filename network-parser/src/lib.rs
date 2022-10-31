use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub adjaceny: Vec<Edge>,
}

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Node {
    pub id: String,
    pub alias: String,
    #[serde(deserialize_with = "addresses_deserialize")]
    pub addresses: Addresses,
    pub rgb_color: String,
    pub out_degree: u32,
    pub in_degree: u32,
}

#[derive(Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Edge {
    scid: String,
    source: String,
    destination: String,
    fee_base_msat: u32,
    fee_proportional_millionths: u32,
    htlc_minimum_msat: u32,
    cltv_expiry_delta: u32,
    id: String,
}
pub type Addresses = Vec<String>;

pub fn from_json_str(json_str: &str) -> Result<Graph, serde_json::Error> {
    serde_json::from_str(json_str)
}

pub fn from_json_file(path: &Path) -> Result<Graph, serde_json::Error> {
    let json_str =
        fs::read_to_string(path).unwrap_or_else(|_| panic!("Error reading file {:?}", path));
    from_json_str(&json_str)
}

fn addresses_deserialize<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let str_sequence = String::deserialize(deserializer)?;
    Ok(str_sequence
        .split(',')
        .map(|item| item.to_owned())
        .collect())
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
            ]
            }"##;
        let graph = from_json_str(json_str).unwrap();
        let actual = &graph.nodes[0];
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
    fn graph_from_json_file() {}
}
