use crate::{graph::Graph, Edge, EdgeWeight, RoutingMetric, ID};

use log::debug;
use std::collections::{BTreeMap, VecDeque};

/// Describes an edge between two nodes
#[derive(Debug, Clone)]
pub(crate) struct Hop {
    src: ID,
    dest: ID,
}

/// Describes a path between two nodes
#[derive(Debug, Clone)]
pub(crate) struct Path {
    pub(crate) src: ID,
    pub(crate) dest: ID,
    /// the edges of the path described from sender to receiver
    pub(crate) hops: VecDeque<ID>,
}

/// Pathfinding object
#[derive(Debug, Clone)]
pub(crate) struct PathFinder {
    /// Network topolgy graph
    pub(crate) graph: Box<Graph>,
    /// Node looking for a route
    src: ID,
    /// the destination node
    dest: ID,
    current_node: ID,
    /// How much is being sent from src to dest
    amount: usize,
    routing_metric: RoutingMetric,
    edge_weights: BTreeMap<(ID, ID), EdgeWeight>,
}

/// A path that we may use to route from src to dest
#[derive(Debug, Clone)]
pub(crate) struct CandidatePath {
    pub(crate) path: Path,
    /// The aggregated path weight (fees or probability)
    weight: EdgeWeight,
    /// The aggregated amount due when using this path (amount + fees)
    amount: usize,
    /// The aggregated timelock
    time: usize,
}

impl Path {
    pub(crate) fn new(src: ID, dest: ID) -> Self {
        let hops = VecDeque::new();
        Self { src, dest, hops }
    }

    /// Excluding src and dest
    fn get_involved_nodes(&self) -> Vec<ID> {
        self.hops.clone().into_iter().collect()
    }

    fn add_hop(&mut self, hop: ID) {
        // use with self.hops.pop_front()
        self.hops.push_back(hop);
    }
}

impl Hop {}

impl CandidatePath {
    pub(crate) fn new(src: ID, dest: ID, amount: usize) -> Self {
        let path = Path::new(src, dest);
        let time = 0;
        let weight = 0;
        CandidatePath {
            path,
            weight,
            amount,
            time,
        }
    }

    pub(crate) fn new_with_path(path: Path) -> Self {
        CandidatePath {
            path,
            weight: EdgeWeight::default(),
            amount: usize::default(),
            time: usize::default(),
        }
    }
}

impl PathFinder {
    /// New PathFinder for payment from src to dest transferring amount of msats
    pub(crate) fn new(
        src: ID,
        dest: ID,
        amount: usize,
        graph: Box<Graph>,
        routing_metric: RoutingMetric,
    ) -> Self {
        let current_node = dest.clone();
        Self {
            graph,
            src,
            dest,
            amount,
            current_node,
            routing_metric,
            edge_weights: BTreeMap::default(),
        }
    }

    /// Returns a route, the total amount due and lock time and none if no route is found
    /// Search for paths from dest to src
    pub(crate) fn find_path(&mut self) -> Option<Vec<CandidatePath>> {
        let mut candidate_paths = None;
        self.remove_inadequate_edges();
        debug!(
            "Looking for shortest paths between src {}, dest {} using {:?} as weight.",
            self.src, self.dest, self.routing_metric
        );
        let successors = |node: &ID| -> Vec<(ID, usize)> {
            let succs = match self.graph.get_edges_for_node(node) {
                Some(edges) => edges
                    .iter()
                    .map(|e| {
                        (
                            e.destination.clone(),
                            Self::get_edge_weight(e, self.amount, self.routing_metric),
                        )
                    })
                    .collect(),
                None => Vec::default(),
            };
            succs
        };
        // returns distinct paths including src and dest
        let k_shortest_paths =
            pathfinding::prelude::yen(&self.src, successors, |n| *n == self.dest, crate::K);
        // construct candipaths using k_shortest_path
        // - calculate total path cost
        for shortest_path in k_shortest_paths {
            println!("path {:?}", shortest_path);
            let mut path = Path::new(self.src.clone(), self.dest.clone());
            path.hops = shortest_path.0.into_iter().collect();
            let mut candidate_path = CandidatePath::new_with_path(path);
            Self::get_aggregated_path_cost(self, &mut candidate_path);
            // sort
        }
        candidate_paths
    }

    fn get_edge_weight(edge: &Edge, amount: usize, metric: RoutingMetric) -> EdgeWeight {
        match metric {
            RoutingMetric::MinFee => Self::get_edge_fee(edge, amount),
            RoutingMetric::MaxProb => Self::get_edge_success_probabilty(edge, amount),
        }
    }

    fn get_aggregated_path_cost(&mut self, candidate_path: &mut CandidatePath) {
        Self::get_aggregated_path_costs(self, candidate_path)
    }

    /// Computes the weight of an edge as done in [LND](https://github.com/lightningnetwork/lnd/blob/290b78e700021e238f7e6bdce6acc80de8d0a64f/routing/pathfind.go#L263)
    /// Used when searching for the shortest path between two nodes.
    fn get_edge_fee(edge: &Edge, amount: usize) -> EdgeWeight {
        let risk_factor = 15;
        let millionths = 1000000;
        let billionths = 1000000000;
        let base_fee = edge.fee_base_msat;
        let prop_fee = amount * edge.fee_proportional_millionths / millionths;
        let time_lock_penalty = amount * edge.cltv_expiry_delta * risk_factor / billionths;
        base_fee + prop_fee + time_lock_penalty
    }

    /// Returns the edge success probabilty (amt/ cap) of given amount
    /// The higher the returned value, the lower the chances of failure
    /// https://github.com/lnbook/lnbook/blob/develop/12_path_finding.asciidoc#liquidity-uncertainty-and-probability
    fn get_edge_success_probabilty(edge: &Edge, amount: usize) -> EdgeWeight {
        let success_prob: f32 = ((edge.htlc_maximum_msat as f32 + 1.0 - amount as f32)
            / (edge.htlc_maximum_msat as f32 + 1.0))
            .ceil();
        success_prob as usize
    }

    /// Calculates the total probabilty along a given path starting from dest to src
    /// TODO: Look over calculations
    fn get_aggregated_path_costs(&mut self, mut candidate_path: &mut CandidatePath) {
        // 1. for all (src, dest) pairs in the path:
        // 2. calculate weight and fee
        // 3. output: total weight, total fees and total amount due
        let mut accumulated_amount = self.amount; //amount + due fees
        let mut accumulated_weight = 1; // TODO: initialisation. fees or probabilty
        let mut accumulated_time = 0; // full timelock delta
        for (idx, node_id) in candidate_path.path.hops.iter().rev().enumerate() {
            if idx == 0 {
                // this is the dest node
            } else if idx == candidate_path.path.hops.len() - 1 {
                // this is the src node
                // TODO: Do we need to do anything when node == src?
            } else {
                let (dest, src) = (node_id, candidate_path.path.hops[idx + 1].clone());
                // we are interested in the weight from src to dest since we are iterating in
                // reverse order
                let cheapest_edge = match self.get_cheapest_edge(&src, dest) {
                    None => panic!("Edge in path does not exist!"),
                    Some(e) => e,
                };
                match self.routing_metric {
                    RoutingMetric::MaxProb => {
                        accumulated_weight *=
                            Self::get_edge_success_probabilty(&cheapest_edge, accumulated_amount)
                    }
                    RoutingMetric::MinFee => {
                        accumulated_weight += Self::get_edge_fee(&cheapest_edge, accumulated_amount)
                    }
                };
                accumulated_amount += Self::get_edge_fee(&cheapest_edge, accumulated_amount);
                accumulated_time += cheapest_edge.cltv_expiry_delta;
            }
        }
        candidate_path.weight = accumulated_weight;
        candidate_path.amount = accumulated_amount;
        candidate_path.time = accumulated_time;
        println!("candidate_path {:?}", candidate_path);
    }

    /// Returns the "cheapest" edge between src and dist bearing the routing me in mind
    /// Used after finding the shortest paths and are therefore interested in routing along the
    /// edge
    /// Necessary as we account for possible parallel edges
    fn get_cheapest_edge(&mut self, from: &ID, to: &ID) -> Option<Edge> {
        let from_to_outedges = self.graph.get_all_src_dest_edges(from, to);
        // assert fee is the same in both directions
        let mut cheapest_edge = None;
        let mut min_weight = usize::MAX;
        for edge in from_to_outedges.into_iter() {
            let edge_weight = Self::get_edge_weight(&edge, self.amount, self.routing_metric);
            if edge_weight < min_weight {
                min_weight = edge_weight;
                self.edge_weights
                    .insert((edge.source.clone(), edge.destination.clone()), edge_weight);
                cheapest_edge = Some(edge);
            }
        }
        cheapest_edge
    }

    /// Remove edges that do not meet the minimum criteria (cap < amount) from the graph
    fn remove_inadequate_edges(&mut self) {
        debug!("Removing edges with insufficient funds.");
        let mut ctr = 0;
        for edge in self.graph.edges.clone() {
            // iter each node's edges
            for e in edge.1 {
                if e.balance < self.amount {
                    ctr += 1;
                    self.graph.remove_edge(&e.source, &e.destination);
                }
            }
        }
        debug!("Removed {} edges with insufficient funds.", ctr);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn get_nodes_involved_in_path() {
        let mut path = Path::new(String::from("a"), String::from("e"));
        path.hops = VecDeque::from([
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
        ]);
        let actual = path.get_involved_nodes();
        let expected = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn edge_success_probabilty() {
        let edge = Edge {
            fee_base_msat: 100,
            fee_proportional_millionths: 1,
            htlc_maximum_msat: 500,
            cltv_expiry_delta: 40,
            ..Default::default()
        };
        let amount = 1;
        let actual = PathFinder::get_edge_success_probabilty(&edge, amount);
        let expected = 1;
        assert_eq!(actual, expected);
        let amount = 600;
        let actual = PathFinder::get_edge_success_probabilty(&edge, amount);
        let expected = 0;
        assert_eq!(actual, expected);
    }

    #[test]
    fn edge_fee() {
        let edge = Edge {
            fee_base_msat: 100,
            fee_proportional_millionths: 1,
            htlc_maximum_msat: 500,
            cltv_expiry_delta: 40,
            ..Default::default()
        };
        let amount = 1;
        let actual = PathFinder::get_edge_fee(&edge, amount);
        let expected = 100;
        assert_eq!(actual, expected);
        let amount = 600;
        let actual = PathFinder::get_edge_fee(&edge, amount);
        let expected = 100;
        assert_eq!(actual, expected);
    }
}
