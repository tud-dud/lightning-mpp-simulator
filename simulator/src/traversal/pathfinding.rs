use crate::{graph::Graph, Edge, EdgeWeight, PaymentParts, RoutingMetric, ID};

use log::{debug, trace};
use std::collections::VecDeque;

/// Describes a path between two nodes
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Path {
    pub(crate) src: ID,
    pub(crate) dest: ID,
    /// the edges of the path described from sender to receiver including fees and timelock over
    /// the edge with the ID
    pub(crate) hops: VecDeque<(ID, usize, usize, String)>,
}

/// Pathfinding object
#[derive(Debug, Clone)]
pub(crate) struct PathFinder {
    /// Network topolgy graph
    pub(crate) graph: Box<Graph>,
    /// Node looking for a route
    pub(super) src: ID,
    /// the destination node
    pub(super) dest: ID,
    /// How much is being sent from src to dest
    pub(super) amount: usize,
    pub(super) routing_metric: RoutingMetric,
    pub(super) payment_parts: PaymentParts,
}

/// A path that we may use to route from src to dest
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CandidatePath {
    pub(crate) path: Path,
    /// The aggregated path weight (fees or probability) describing how costly the path is
    pub(crate) weight: EdgeWeight,
    /// The aggregated amount due when using this path (amount + fees)
    pub(crate) amount: usize,
    /// The aggregated timelock
    pub(crate) time: usize,
}

impl Path {
    pub(crate) fn new(src: ID, dest: ID) -> Self {
        let hops = VecDeque::new();
        Self { src, dest, hops }
    }

    /// Including src and dest
    fn get_involved_nodes(&self) -> Vec<ID> {
        self.hops.iter().map(|h| h.0.clone()).collect()
    }

    fn add_hop(&mut self, hop: ID, fees: usize, timelock: usize, edge_id: String) {
        // use with self.hops.pop_front()
        self.hops.push_back((hop, fees, timelock, edge_id));
    }

    fn update_hop(&mut self, hop_id: ID, fees: usize, timelock: usize, edge_id: &String) {
        for hop in self.hops.iter_mut() {
            if hop.0 == hop_id {
                *hop = (hop.0.clone(), fees, timelock, edge_id.to_owned())
            }
        }
    }
}

impl CandidatePath {
    fn new(src: ID, dest: ID, amount: usize) -> Self {
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
        payment_parts: PaymentParts,
    ) -> Self {
        Self {
            graph,
            src,
            dest,
            amount,
            routing_metric,
            payment_parts,
        }
    }

    pub(crate) fn find_path(&mut self) -> Option<Vec<CandidatePath>> {
        match self.payment_parts {
            PaymentParts::Single => self.find_path_single_payment(),
            PaymentParts::Split => self.find_path_mpp_payment(),
        }
    }

    pub(super) fn get_edge_weight(edge: &Edge, amount: usize, metric: RoutingMetric) -> EdgeWeight {
        match metric {
            RoutingMetric::MinFee => Self::get_edge_fee(edge, amount),
            RoutingMetric::MaxProb => Self::get_edge_failure_probabilty(edge, amount),
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

    /// Returns the edge failure probabilty (amt/ cap) of given amount so that the shortest path
    /// weights it accordingly
    /// The higher the returned value, the lower the chances of success
    /// https://github.com/lnbook/lnbook/blob/develop/12_path_finding.asciidoc#liquidity-uncertainty-and-probability
    fn get_edge_failure_probabilty(edge: &Edge, amount: usize) -> EdgeWeight {
        let success_prob: f32 = ((edge.htlc_maximum_msat as f32 + 1.0 - amount as f32)
            / (edge.htlc_maximum_msat as f32 + 1.0))
            .ceil();
        1 - (success_prob as usize)
    }

    /// Calculates the total probabilty along a given path starting from dest to src
    pub(super) fn get_aggregated_path_cost(&mut self, mut candidate_path: &mut CandidatePath) {
        // 1. for all (src, dest) pairs in the path:
        // 2. calculate weight and fee
        // 3. output: total weight, total fees and total amount due
        trace!(
            "Calculating total cost for CandidatePath = {:?}.",
            candidate_path
        );
        let mut accumulated_amount = self.amount; //amount + due fees
        let mut accumulated_weight = if self.routing_metric == RoutingMetric::MinFee {
            0
        } else {
            1
        };
        let mut accumulated_time = 0; // full timelock delta
        let candidate_path_hops: VecDeque<ID> = candidate_path
            .path
            .get_involved_nodes()
            .into_iter()
            .rev()
            .collect();
        for (idx, node_id) in candidate_path_hops.iter().enumerate() {
            // TODO: Do we need to do anything when node == src?
            if node_id.clone() == self.src || node_id.clone() == self.dest {
                continue;
            } else {
                let (src, dest) = (node_id, candidate_path_hops[idx - 1].clone());
                // we are interested in the weight from src to dest (the previous node in the list) since that is the direction the
                // payment will flow in
                let cheapest_edge = match self.get_cheapest_edge(src, &dest) {
                    None => panic!("Edge in path does not exist! {} -> {}", src, dest),
                    Some(e) => e,
                };
                match self.routing_metric {
                    RoutingMetric::MaxProb => {
                        accumulated_weight *= 1 - Self::get_edge_failure_probabilty(
                            &cheapest_edge,
                            accumulated_amount,
                        )
                    }
                    RoutingMetric::MinFee => {
                        accumulated_weight += Self::get_edge_fee(&cheapest_edge, accumulated_amount)
                    }
                };
                let edge_fee = Self::get_edge_fee(&cheapest_edge, accumulated_amount);
                accumulated_amount += edge_fee;
                let edge_timelock = cheapest_edge.cltv_expiry_delta;
                accumulated_time += edge_timelock;
                candidate_path.path.update_hop(
                    cheapest_edge.source,
                    edge_fee,
                    edge_timelock,
                    &cheapest_edge.channel_id,
                );
            }
        }
        candidate_path.weight = accumulated_weight;
        candidate_path.amount = accumulated_amount;
        candidate_path.time = accumulated_time;
    }

    /// Returns the "cheapest" edge between src and dist bearing the routing me in mind
    /// Used after finding the shortest paths and are therefore interested in routing along the
    /// edge
    /// Necessary as we account for possible parallel edges
    fn get_cheapest_edge(&mut self, from: &ID, to: &ID) -> Option<Edge> {
        trace!("Looking for cheapest edge between {} and {}.", from, to);
        let from_to_outedges = self.graph.get_all_src_dest_edges(from, to);
        let mut cheapest_edge = None;
        let mut min_weight = usize::MAX;
        for edge in from_to_outedges.into_iter() {
            let edge_weight = Self::get_edge_weight(&edge, self.amount, self.routing_metric);
            if edge_weight < min_weight {
                min_weight = edge_weight;
                cheapest_edge = Some(edge);
            }
        }
        cheapest_edge
    }

    /// Remove edges that do not meet the minimum criteria (cap < amount) from the graph
    pub(super) fn remove_inadequate_edges(&mut self) {
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
        trace!("Removed {} edges with insufficient funds.", ctr);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn get_nodes_involved_in_path() {
        let mut path = Path::new(String::from("a"), String::from("e"));
        path.hops = VecDeque::from([
            ("a".to_string(), 0, 0, "".to_string()),
            ("b".to_string(), 0, 0, "".to_string()),
            ("c".to_string(), 0, 0, "".to_string()),
            ("d".to_string(), 0, 0, "".to_string()),
            ("e".to_string(), 0, 0, "".to_string()),
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
    fn edge_failure_probabilty() {
        let edge = Edge {
            fee_base_msat: 100,
            fee_proportional_millionths: 1,
            htlc_maximum_msat: 500,
            cltv_expiry_delta: 40,
            ..Default::default()
        };
        let amount = 1;
        let actual = PathFinder::get_edge_failure_probabilty(&edge, amount);
        let expected = 0;
        assert_eq!(actual, expected);
        let amount = 600;
        let actual = PathFinder::get_edge_failure_probabilty(&edge, amount);
        let expected = 1;
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
