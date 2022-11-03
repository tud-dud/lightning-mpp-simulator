use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::*;

pub(crate) fn from_json_to_raw(json_str: &str) -> Result<RawGraph, serde_json::Error> {
    serde_json::from_str(json_str)
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct RawGraph {
    pub(crate) nodes: Vec<RawNode>,
    #[serde(rename = "adjacency")]
    pub(crate) edges: Vec<HashSet<RawEdge>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
pub(crate) struct RawNode {
    pub(crate) id: Option<String>,
    pub(crate) alias: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "addresses_deserialize")]
    pub(crate) addresses: RawAddresses,
    pub(crate) rgb_color: Option<String>,
    pub(crate) out_degree: Option<u32>,
    pub(crate) in_degree: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub(crate) struct RawEdge {
    #[serde(rename = "scid")]
    pub(crate) channel_id: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) destination: Option<String>,
    pub(crate) features: Option<String>,
    pub(crate) fee_base_msat: Option<u32>,
    pub(crate) fee_proportional_millionths: Option<usize>,
    pub(crate) htlc_minimim_msat: Option<usize>,
    pub(crate) htlc_maximum_msat: Option<usize>,
    pub(crate) cltv_expiry_delta: Option<u32>,
    pub(crate) id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RawAddresses(pub(crate) Option<Vec<String>>);

impl Node {
    // return default values if some are not present
    // TODO: maybe discard if certain fields like ID are missing?
    pub(crate) fn from_raw(raw_node: RawNode) -> Node {
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
    pub(crate) fn from_raw(raw_edge: RawEdge) -> Edge {
        Edge {
            channel_id: raw_edge.channel_id.unwrap_or_default(),
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
