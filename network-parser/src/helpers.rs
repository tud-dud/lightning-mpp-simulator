use serde::{Deserialize, Serialize};
use serde_with::{formats::CommaSeparator, serde_as};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::*;

pub(crate) fn from_json_to_raw(json_str: &str) -> Result<RawGraph, serde_json::Error> {
    serde_json::from_str(json_str)
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RawGraph {
    pub(crate) nodes: Vec<RawNode>,
    #[serde(rename = "adjacency")]
    pub(crate) edges: Vec<HashSet<RawEdge>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct WebGraph {
    pub(crate) nodes: Vec<WebNode>,
    #[serde(rename = "adjacency")]
    pub(crate) edges: Vec<HashSet<WebEdge>>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
pub struct RawNode {
    pub(crate) id: Option<String>,
    pub(crate) alias: Option<String>,
    #[serde(default)]
    #[serde_as(as = "serde_with::StringWithSeparator::<CommaSeparator, String>")]
    pub(crate) addresses: Vec<String>,
    pub(crate) rgb_color: Option<String>,
    pub(crate) out_degree: Option<u32>,
    pub(crate) in_degree: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RawEdge {
    #[serde(rename = "scid")]
    pub(crate) channel_id: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) destination: Option<String>,
    pub(crate) features: Option<String>,
    pub(crate) fee_base_msat: Option<usize>,
    pub(crate) fee_proportional_millionths: Option<usize>,
    pub(crate) htlc_minimim_msat: Option<usize>,
    pub(crate) htlc_maximum_msat: Option<usize>,
    pub(crate) cltv_expiry_delta: Option<usize>,
    pub(crate) id: Option<String>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
pub struct WebNode {
    pub id: Option<String>,
    pub alias: Option<String>,
    #[serde(default)]
    #[serde_as(as = "serde_with::StringWithSeparator::<CommaSeparator, String>")]
    pub addresses: Vec<String>,
    pub rgb_color: Option<String>,
    pub out_degree: Option<u64>,
    pub in_degree: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WebEdge {
    #[serde(rename = "scid")]
    pub channel_id: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub features: Option<String>,
    pub fee_base_msat: Option<usize>,
    pub fee_proportional_millionths: Option<usize>,
    pub htlc_minimim_msat: Option<u64>,
    pub htlc_maximum_msat: Option<u64>,
    pub cltv_expiry_delta: Option<u64>,
    pub id: Option<String>,
}

impl Node {
    pub(crate) fn from_raw(raw_node: RawNode) -> Node {
        Node {
            id: raw_node.id.expect("Error in node ID"),
            alias: raw_node.alias.unwrap_or_default(),
            addresses: raw_node.addresses,
            rgb_color: raw_node.rgb_color.unwrap_or_default(),
            out_degree: raw_node.out_degree.unwrap_or_default(),
            in_degree: raw_node.in_degree.unwrap_or_default(),
        }
    }
    pub fn from_web(web_node: WebNode) -> Node {
        let out_deg = web_node.out_degree.unwrap_or_default();
        let in_deg = web_node.out_degree.unwrap_or_default();
        Node {
            id: web_node.id.expect("Error in node ID"),
            alias: web_node.alias.unwrap_or_default(),
            addresses: web_node.addresses,
            rgb_color: web_node.rgb_color.unwrap_or_default(),
            out_degree: out_deg.try_into().unwrap_or(u32::default()),
            in_degree: in_deg.try_into().unwrap_or(u32::default()),
        }
    }
}

impl Edge {
    /// We remove "orphaned" edges - edges where the source node is not in the list of nodes
    pub(crate) fn from_raw(raw_edge: &RawEdge) -> Option<Edge> {
        if raw_edge.fee_base_msat.is_none()
            || raw_edge.fee_proportional_millionths.is_none()
            || raw_edge.htlc_maximum_msat.is_none()
        {
            None
        } else {
            Some(Edge {
                channel_id: raw_edge.channel_id.clone().expect("scid not found"),
                source: raw_edge.source.clone().unwrap_or_default(),
                destination: raw_edge.destination.clone().unwrap_or_default(),
                features: raw_edge.features.clone().unwrap_or_default(),
                fee_base_msat: raw_edge
                    .fee_base_msat
                    .expect("Error in fee_base_msat field"),
                fee_proportional_millionths: raw_edge
                    .fee_proportional_millionths
                    .expect("Error in fee_proportional_millionths field"),
                htlc_minimim_msat: raw_edge.htlc_minimim_msat.unwrap_or_default(),
                htlc_maximum_msat: raw_edge.htlc_maximum_msat.unwrap_or_default(),
                cltv_expiry_delta: raw_edge.cltv_expiry_delta.unwrap_or_default(),
                id: raw_edge.id.clone().unwrap_or_default(),
                balance: 0,
                liquidity: 0,
                capacity: 0,
            })
        }
    }

    pub fn from_web(web_edge: &WebEdge) -> Option<Edge> {
        if web_edge.fee_base_msat.is_none()
            || web_edge.fee_proportional_millionths.is_none()
            || web_edge.htlc_maximum_msat.is_none()
        {
            None
        } else {
            Some(Edge {
                channel_id: web_edge.channel_id.clone().expect("scid not found"),
                source: web_edge.source.clone().unwrap_or_default(),
                destination: web_edge.destination.clone().unwrap_or_default(),
                features: web_edge.features.clone().unwrap_or_default(),
                fee_base_msat: web_edge
                    .fee_base_msat
                    .expect("Error in fee_base_msat field"),
                fee_proportional_millionths: web_edge
                    .fee_proportional_millionths
                    .expect("Error in fee_proportional_millionths field"),
                htlc_minimim_msat: web_edge
                    .htlc_minimim_msat
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(usize::default()),
                htlc_maximum_msat: web_edge
                    .htlc_maximum_msat
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(usize::default()),
                cltv_expiry_delta: web_edge
                    .cltv_expiry_delta
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(usize::default()),
                id: web_edge.id.clone().unwrap_or_default(),
                balance: 0,
                liquidity: 0,
                capacity: 0,
            })
        }
    }
}

#[allow(unused)]
pub(crate) fn edge_has_all_mandatory_fields(raw_edge: &RawEdge) -> bool {
    let mut valid = false;
    if let Some(base_fee) = raw_edge.fee_base_msat {
        if base_fee != usize::default() {
            if let Some(prop_fee) = raw_edge.fee_proportional_millionths {
                if prop_fee != usize::default() {
                    valid = true
                }
            }
        }
    }
    valid
}

impl Hash for RawEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.channel_id.hash(state);
    }
}
impl Eq for RawEdge {}
impl PartialEq for RawEdge {
    fn eq(&self, other: &Self) -> bool {
        self.channel_id == other.channel_id
    }
}

impl Hash for WebEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.channel_id.hash(state);
    }
}
impl Eq for WebEdge {}
impl PartialEq for WebEdge {
    fn eq(&self, other: &Self) -> bool {
        self.channel_id == other.channel_id
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_wo_id_is_ignored() {
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
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "incomplete",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                }
            ],
            "adjacency": [
              ]
            }"##;
        let graph = from_json_str(json_str).unwrap();
        let actual = graph.nodes.len();
        let expected = 1;
        assert_eq!(actual, expected);
    }

    #[test]
    fn ignore_unknown_edges_in_edgelist() {
        let json_str = r##"{
                "nodes": [
                {
                    "id": "validnode",
                    "timestamp": 1657607504,
                    "features": "888000080a69a2",
                    "rgb_color": "550055",
                    "alias": "MilliBit",
                    "addresses": "ipv4://83.85.142.36:9735",
                    "out_degree": 25,
                    "in_degree": 9
                },
                {
                    "id": "othervalidnode",
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
                    "source": "unknownsrc",
                    "destination": "othervalidnode",
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
                    "scid": "714505x2146x0/0",
                    "source": "validnode",
                    "destination": "othervalidnode",
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
                    "source": "validnode",
                    "destination": "unknowndest",
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
        let expected = HashSet::from([Edge {
            channel_id: "714505x2146x0/0".to_string(),
            source: "validnode".to_string(),
            destination: "othervalidnode".to_string(),
            features: String::default(),
            fee_base_msat: 0,
            fee_proportional_millionths: 555,
            htlc_minimim_msat: 1,
            htlc_maximum_msat: 5545472000,
            cltv_expiry_delta: 34,
            id: "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d".to_string(),
            balance: 0,
            liquidity: 0,
            capacity: 0,
        }]);
        let actual = graph.edges.get("validnode").unwrap().clone();
        assert_eq!(expected, actual);
    }
}
