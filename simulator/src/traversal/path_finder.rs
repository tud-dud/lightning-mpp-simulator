use crate::{graph::Graph, Edge, EdgeWeight, RoutingMetric, ID};

use log::debug;
use pathfinding::prelude::yen;
use std::collections::VecDeque;

/// Describes an edge between two nodes
#[derive(Debug, Clone)]
pub(crate) struct Hop {
    src: ID,
    dest: ID,
}

/// Describes a path between two nodes
#[derive(Debug, Clone)]
pub(crate) struct Path {
    src: ID,
    dest: ID,
    /// the edges of the path describe from sender to receiver
    hops: VecDeque<ID>,
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
}

/// A path that we may use to route from src to dest
#[derive(Debug, Clone)]
pub(crate) struct CandidatePath {
    path: Path,
    /// The aggregated path weight
    weight: EdgeWeight,
    /// The aggregated path amount
    amount: usize,
    /// The aggregated time
    time: u16,
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
        }
    }

    /// Returns a route, the total amount due and lock time and none if no route is found
    /// Search for paths from dest to src
    pub(crate) fn find_path(&mut self) -> Option<(Path, usize, u16)> {
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
        let k_shortest_paths = yen(&self.src, successors, |n| *n == self.dest, crate::K);
        // construct candipaths using k_shortest_path
        // - calculate total path timelock, fees
        for shortest_path in k_shortest_paths {
            let mut path = Path::new(self.src.clone(), self.dest.clone());
            path.src = self.src.clone();
            path.dest = self.dest.clone();
            // calculate fees from dest to src
        }
        unimplemented!()
    }

    fn get_edge_weight(edge: &Edge, amount: usize, metric: RoutingMetric) -> EdgeWeight {
        match metric {
            RoutingMetric::MinFee => Self::get_edge_fee(edge, amount),
            RoutingMetric::MaxProb => Self::get_edge_probabilty(edge, amount),
        }
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

    /// Returns the success (more the failure) probabilty (amt/ cap) of given amount
    /// The higher the returned value, the lower the chances of success
    fn get_edge_probabilty(edge: &Edge, amount: usize) -> EdgeWeight {
        amount / edge.htlc_maximum_msat
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
}
