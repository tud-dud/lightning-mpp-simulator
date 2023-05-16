use serde::Deserialize;
use serde_aux::prelude::*;
use std::hash::{Hash, Hasher};

use crate::*;

#[derive(Deserialize, Debug, Default)]
pub struct RawLnresearchGraph {
    pub(crate) nodes: Vec<RawNode>,
    #[serde(alias = "adjacency")]
    pub(crate) edges: Vec<Vec<LnresearchRawEdge>>,
}
#[derive(Deserialize, Debug, Default)]
pub struct RawLndGraph {
    pub(crate) nodes: Vec<RawNode>,
    #[serde(alias = "adjacency")]
    pub(crate) edges: Vec<LndRawEdge>,
}

#[derive(Deserialize, Debug, Clone, Default, Eq, PartialEq)]
pub struct RawNode {
    #[serde(alias = "pub_key")]
    pub(crate) id: Option<String>,
    pub(crate) alias: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct LnresearchRawEdge {
    #[serde(rename = "scid")]
    pub channel_id: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub fee_base_msat: Option<u64>,
    pub fee_proportional_millionths: Option<u64>,
    pub htlc_minimim_msat: Option<u64>,
    pub htlc_maximum_msat: Option<u64>,
    pub cltv_expiry_delta: Option<u64>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct LndRawEdge {
    pub channel_id: Option<String>,
    #[serde(alias = "node1_pub")]
    pub source: Option<String>,
    #[serde(alias = "node2_pub")]
    pub destination: Option<String>,
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    /// Denominated in sat
    pub capacity: Option<u64>,
    pub node1_policy: Option<NodePolicy>,
    pub node2_policy: Option<NodePolicy>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct NodePolicy {
    /// Denominated in msat
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub fee_base_msat: Option<u64>,
    /// Denominated in ppm msat
    #[serde(alias = "fee_rate_milli_msat")]
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub fee_proportional_millionths: Option<u64>,
    /// Denominated in msat
    #[serde(alias = "min_htlc")]
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub htlc_minimim_msat: Option<u64>,
    #[serde(alias = "max_htlc_msat")]
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub htlc_maximum_msat: Option<u64>,
    #[serde(alias = "time_lock_delta")]
    pub cltv_expiry_delta: Option<u64>,
    pub last_update: u32,
}

impl Node {
    pub(crate) fn from_raw(raw_node: RawNode) -> Node {
        Node {
            id: raw_node.id.expect("Error in node ID"),
            alias: raw_node.alias.unwrap_or_default(),
            last_update: Default::default(),
        }
    }
}

impl Edge {
    /// We remove "orphaned" edges - edges where the source node is not in the list of nodes
    pub(crate) fn from_lnresearch_raw(raw_edge: &LnresearchRawEdge) -> Option<Edge> {
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
                fee_base_msat: raw_edge
                    .fee_base_msat
                    .expect("Error in fee_base_msat field")
                    .try_into()
                    .expect("Error in fee_base_msat field"),
                fee_proportional_millionths: raw_edge
                    .fee_proportional_millionths
                    .expect("Error in fee_proportional_millionths field")
                    .try_into()
                    .expect("Error in fee_proportional_millionths field"),
                htlc_minimim_msat: raw_edge
                    .htlc_minimim_msat
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(usize::default()),
                htlc_maximum_msat: raw_edge
                    .htlc_maximum_msat
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(usize::default()),
                cltv_expiry_delta: raw_edge
                    .cltv_expiry_delta
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or(usize::default()),
                balance: 0,
                liquidity: 0,
                capacity: 0,
            })
        }
    }
    /// We remove "orphaned" edges - edges where the source node is not in the list of nodes
    pub(crate) fn from_lnd_raw(raw_edge: &LndRawEdge) -> Option<(Edge, Edge)> {
        if raw_edge.node1_policy.is_none()
            || raw_edge.node2_policy.is_none()
            || raw_edge
                .node1_policy
                .as_ref()
                .unwrap()
                .fee_base_msat
                .is_none()
            || raw_edge
                .node1_policy
                .as_ref()
                .unwrap()
                .fee_proportional_millionths
                .is_none()
            || raw_edge
                .node1_policy
                .as_ref()
                .unwrap()
                .htlc_maximum_msat
                .is_none()
            || raw_edge
                .node2_policy
                .as_ref()
                .unwrap()
                .fee_base_msat
                .is_none()
            || raw_edge
                .node2_policy
                .as_ref()
                .unwrap()
                .fee_proportional_millionths
                .is_none()
            || raw_edge
                .node2_policy
                .as_ref()
                .unwrap()
                .htlc_maximum_msat
                .is_none()
        {
            None
        } else {
            let node1_policy = raw_edge.node1_policy.clone().unwrap(); // safe because of the earlier check
            let node2_policy = raw_edge.node2_policy.clone().unwrap();
            Some((
                Edge {
                    channel_id: raw_edge.channel_id.clone().expect("scid not found"),
                    source: raw_edge.source.clone().unwrap_or_default(),
                    destination: raw_edge.destination.clone().unwrap_or_default(),
                    fee_base_msat: node1_policy
                        .fee_base_msat
                        .expect("Error in fee_base_msat field")
                        .try_into()
                        .expect("Error in fee_base_msat field"),
                    fee_proportional_millionths: node1_policy
                        .fee_proportional_millionths
                        .expect("Error in fee_proportional_millionths field")
                        .try_into()
                        .expect("Error in fee_proportional_millionths field"),
                    htlc_minimim_msat: node1_policy
                        .htlc_minimim_msat
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default()),
                    htlc_maximum_msat: node1_policy
                        .htlc_maximum_msat
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default()),
                    cltv_expiry_delta: node1_policy
                        .cltv_expiry_delta
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default()),
                    balance: 0,
                    liquidity: 0,
                    capacity: raw_edge
                        .capacity
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default())
                        * 1000,
                },
                Edge {
                    channel_id: raw_edge.channel_id.clone().expect("scid not found"),
                    destination: raw_edge.source.clone().unwrap_or_default(),
                    source: raw_edge.destination.clone().unwrap_or_default(),
                    fee_base_msat: node2_policy
                        .fee_base_msat
                        .expect("Error in fee_base_msat field")
                        .try_into()
                        .expect("Error in fee_base_msat field"),
                    fee_proportional_millionths: node2_policy
                        .fee_proportional_millionths
                        .expect("Error in fee_proportional_millionths field")
                        .try_into()
                        .expect("Error in fee_proportional_millionths field"),
                    htlc_minimim_msat: node2_policy
                        .htlc_minimim_msat
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default()),
                    htlc_maximum_msat: node2_policy
                        .htlc_maximum_msat
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default()),
                    cltv_expiry_delta: node2_policy
                        .cltv_expiry_delta
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default()),
                    balance: 0,
                    liquidity: 0,
                    capacity: raw_edge
                        .capacity
                        .unwrap_or_default()
                        .try_into()
                        .unwrap_or(usize::default())
                        * 1000,
                },
            ))
        }
    }
}

impl Hash for LnresearchRawEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.channel_id.hash(state);
    }
}
impl Eq for LnresearchRawEdge {}
impl PartialEq for LnresearchRawEdge {
    fn eq(&self, other: &Self) -> bool {
        self.channel_id == other.channel_id
    }
}
impl Hash for LndRawEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.channel_id.hash(state);
    }
}
impl Eq for LndRawEdge {}
impl PartialEq for LndRawEdge {
    fn eq(&self, other: &Self) -> bool {
        self.channel_id == other.channel_id && self.source == other.source
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
        let graph = Graph::from_lnresearch_json_str(json_str).unwrap();
        let actual = graph.nodes.len();
        let expected = 1;
        assert_eq!(actual, expected);
        let graph = Graph::from_lnd_json_str(json_str).unwrap();
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
        let graph = Graph::from_lnresearch_json_str(json_str).unwrap();
        let expected = HashSet::from([Edge {
            channel_id: "714505x2146x0/0".to_string(),
            source: "validnode".to_string(),
            destination: "othervalidnode".to_string(),
            fee_base_msat: 0,
            fee_proportional_millionths: 555,
            htlc_minimim_msat: 1,
            htlc_maximum_msat: 5545472000,
            cltv_expiry_delta: 34,
            balance: 0,
            liquidity: 0,
            capacity: 0,
        }]);
        let actual = graph.edges.get("validnode").unwrap().clone();
        assert_eq!(expected, actual);
    }

    #[test]
    fn id_alias_works() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "alias": "MilliBit"
                },
                {
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                },
                {
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                }
            ],
            "edges": []
            }"##;
        let graph = Graph::from_lnd_json_str(&json_str).unwrap();
        assert_eq!(graph.nodes.len(), 3);
    }

    #[test]
    fn discard_lnd_edges_without_necessary_fields() {
        let json_str = r##"{
            "nodes": [
                {
                    "id": "021f0f2a5b46871b23f690a5be893f5b3ec37cf5a0fd8b89872234e984df35ea32",
                    "alias": "MilliBit"
                },
                {
                    "id": "03271338633d2d37b285dae4df40b413d8c6c791fbee7797bc5dc70812196d7d5c"
                },
                {
                    "id": "03e5ea100e6b1ef3959f79627cb575606b19071235c48b3e7f9808ebcd6d12e87d"
                }
            ],
            "edges": [
                {
                    "channel_id": "659379322247708673",
                    "chan_point": "ae07c9fe78e6a1057902441f599246d735bac33be7b159667006757609fb5a86:1",
                    "last_update": 1571278793,
                    "node1_pub": "02899d09a65c5ca768c42b12e57d0497bfdf8ac1c46b0dcc0d4faefcdbc01304c1",
                    "node2_pub": "0298f6074a454a1f5345cb2a7c6f9fce206cd0bf675d177cdbf0ca7508dd28852f",
                    "capacity": "1000000",
                    "node1_policy": {
                        "time_lock_delta": 14,
                        "min_htlc": "1000",
                        "fee_base_msat": "1000",
                        "fee_rate_milli_msat": "",
                        "disabled": false,
                        "max_htlc_msat": "990000000",
                        "last_update": 1571278793
                    },
                    "node2_policy": {
                        "time_lock_delta": 14,
                        "min_htlc": "1000",
                        "fee_base_msat": "1000",
                        "fee_rate_milli_msat": "1",
                        "disabled": false,
                        "max_htlc_msat": "990000000",
                        "last_update": 1571278793
                    }
                }
              ]
            }"##;
        let graph = Graph::from_lnd_json_str(&json_str).unwrap();
        let actual = graph.edge_count();
        let expected = 0;
        assert_eq!(expected, actual);
    }

    #[test]
    fn capacity_is_converted_to_msat() {
        let json_str = r##"{
            "nodes": [
                {
                    "last_update": 1567764428,
                    "pub_key": "0298f6074a454a1f5345cb2a7c6f9fce206cd0bf675d177cdbf0ca7508dd28852f",
                    "alias": "node1"
                },
                {
                    "last_update": 1567764428,
                    "pub_key": "02899d09a65c5ca768c42b12e57d0497bfdf8ac1c46b0dcc0d4faefcdbc01304c1",
                    "alias": "node2"
                }
            ],
            "edges": [
                {
                    "channel_id": "659379322247708673",
                    "chan_point": "ae07c9fe78e6a1057902441f599246d735bac33be7b159667006757609fb5a86:1",
                    "last_update": 1571278793,
                    "node1_pub": "02899d09a65c5ca768c42b12e57d0497bfdf8ac1c46b0dcc0d4faefcdbc01304c1",
                    "node2_pub": "0298f6074a454a1f5345cb2a7c6f9fce206cd0bf675d177cdbf0ca7508dd28852f",
                    "capacity": "1000",
                    "node1_policy": {
                        "time_lock_delta": 14,
                        "min_htlc": "1000",
                        "fee_base_msat": "1000",
                        "fee_rate_milli_msat": "1",
                        "disabled": false,
                        "max_htlc_msat": "990000000",
                        "last_update": 1571278793
                    },
                    "node2_policy": {
                        "time_lock_delta": 4,
                        "min_htlc": "100",
                        "fee_base_msat": "10000",
                        "fee_rate_milli_msat": "1",
                        "disabled": false,
                        "max_htlc_msat": "990000000",
                        "last_update": 1571278793
                    }
                }
            ]
            }"##;
        let expected = 1000 * 1000;
        let graph = Graph::from_lnd_json_str(&json_str).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 2);
        for e in graph.get_edges_as_vec_vec().into_iter().flatten() {
            assert_eq!(e.capacity, expected);
        }
    }
}
