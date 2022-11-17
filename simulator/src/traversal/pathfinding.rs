use crate::{graph::Graph, Edge, EdgeWeight, RoutingMetric, ID};

use log::{debug, trace};
use std::collections::{BTreeMap, VecDeque};

/// Describes an edge between two nodes
#[derive(Debug, Clone)]
pub(crate) struct Hop {
    src: ID,
    dest: ID,
}

/// Describes a path between two nodes
#[derive(Debug, Clone, PartialEq)]
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
    /// How much is being sent from src to dest
    amount: usize,
    routing_metric: RoutingMetric,
    edge_weights: BTreeMap<(ID, ID), EdgeWeight>,
}

/// A path that we may use to route from src to dest
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CandidatePath {
    pub(crate) path: Path,
    /// The aggregated path weight (fees or probability) describing how costly the path is
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
    ) -> Self {
        Self {
            graph,
            src,
            dest,
            amount,
            routing_metric,
            edge_weights: BTreeMap::default(),
        }
    }

    /// Returns a route, the total amount due and lock time and none if no route is found
    /// Search for paths from dest to src
    pub(crate) fn find_path(&mut self) -> Option<Vec<CandidatePath>> {
        let mut candidate_paths = Vec::default();
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
        // returns distinct paths including src and dest sorted in ascending cost order
        let k_shortest_paths =
            pathfinding::prelude::yen(&self.src, successors, |n| *n == self.dest, crate::K);
        trace!(
            "Got {} shortest paths between {} and {}.",
            k_shortest_paths.len(),
            self.src,
            self.dest
        );
        if k_shortest_paths.is_empty() {
            return None;
        }
        // construct candipaths using k_shortest_path
        // - calculate total path cost
        for shortest_path in k_shortest_paths {
            trace!(
                "Creating candidate path from {:?} shortest path.",
                shortest_path
            );
            let mut path = Path::new(self.src.clone(), self.dest.clone());
            path.hops = shortest_path.0.into_iter().collect();
            let mut candidate_path = CandidatePath::new_with_path(path);
            Self::get_aggregated_path_cost(self, &mut candidate_path);
            candidate_paths.push(candidate_path);
        }
        // sort? already sorted by cost
        Some(candidate_paths)
    }

    fn get_edge_weight(edge: &Edge, amount: usize, metric: RoutingMetric) -> EdgeWeight {
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
    fn get_aggregated_path_cost(&mut self, mut candidate_path: &mut CandidatePath) {
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
        let candidate_path_hops: VecDeque<ID> =
            candidate_path.path.hops.iter().cloned().rev().collect();
        for (idx, node_id) in candidate_path_hops.iter().enumerate() {
            // TODO: Do we need to do anything when node == src?
            if node_id.clone() == self.src || node_id.clone() == self.dest {
                continue;
            } else {
                let (dest, src) = (node_id, candidate_path_hops[idx + 1].clone());
                // we are interested in the weight from src to dest since that is the direction the
                // payment will flow in
                let cheapest_edge = match self.get_cheapest_edge(&src, dest) {
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
                accumulated_amount += Self::get_edge_fee(&cheapest_edge, accumulated_amount);
                accumulated_time += cheapest_edge.cltv_expiry_delta;
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

    #[test]
    fn find_min_fee_paths() {
        let json_file = std::path::Path::new("../test_data/lnbook_example.json");
        let mut graph = Graph::to_sim_graph(&network_parser::from_json_file(&json_file).unwrap());
        let balance = 70000; // ensure balances are not the reason for failure
        for (_, edges) in graph.edges.iter_mut() {
            for e in edges {
                e.balance = balance;
            }
        }
        let graph = Box::new(graph);
        let src = String::from("alice");
        let dest = String::from("dina");
        let amount = 5000;
        let routing_metric = RoutingMetric::MinFee;
        let mut path_finder = PathFinder::new(src, dest, amount, graph, routing_metric);
        let actual = path_finder.find_path();
        assert!(actual.is_some());
        let actual = actual.unwrap();
        let expected_path = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                "alice".to_owned(),
                "bob".to_owned(),
                "chan".to_owned(),
                "dina".to_owned(),
            ]),
        };
        let expected: Vec<CandidatePath> = vec![CandidatePath {
            path: expected_path,
            weight: 120,  // fees (a->b, b->c)
            amount: 5120, // amount + fees
            time: 45,
        }];
        assert_eq!(actual.len(), expected.len());
        for (idx, e) in expected.iter().enumerate() {
            assert_eq!(*e, actual[idx]);
        }
    }

    #[test]
    fn find_max_prob_paths() {
        let json_file = std::path::Path::new("../test_data/lnbook_example.json");
        let mut graph = Graph::to_sim_graph(&network_parser::from_json_file(&json_file).unwrap());
        let balance = 70000; // ensure balances are not the reason for failure
        for (_, edges) in graph.edges.iter_mut() {
            for e in edges {
                e.balance = balance;
            }
        }
        let graph = Box::new(graph);
        let src = String::from("alice");
        let dest = String::from("dina");
        let amount = 5000;
        let routing_metric = RoutingMetric::MaxProb;
        let mut path_finder = PathFinder::new(src, dest, amount, graph, routing_metric);
        let actual = path_finder.find_path();
        assert!(actual.is_some());
        let actual = actual.unwrap();
        let expected_path = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                "alice".to_owned(),
                "bob".to_owned(),
                "chan".to_owned(),
                "dina".to_owned(),
            ]),
        };
        let expected: Vec<CandidatePath> = vec![CandidatePath {
            path: expected_path,
            weight: 1,    // probabilty
            amount: 5120, // amount + fees
            time: 45,
        }];
        assert_eq!(actual.len(), expected.len());
        for (idx, e) in expected.iter().enumerate() {
            assert_eq!(*e, actual[idx]);
        }
    }

    #[test]
    fn aggregated_path_cost() {
        let json_file = std::path::Path::new("../test_data/lnbook_example.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(&json_file).unwrap());
        let mut path_finder = PathFinder {
            graph: Box::new(graph),
            src: "dina".to_string(),
            dest: "bob".to_string(),
            amount: 10000,
            routing_metric: RoutingMetric::MinFee,
            edge_weights: BTreeMap::default(),
        };
        let path = Path {
            src: path_finder.src.clone(),
            dest: path_finder.dest.clone(),
            hops: VecDeque::from(["dina".to_owned(), "chan".to_owned(), "bob".to_owned()]),
        };
        let mut candidate_path = &mut CandidatePath::new_with_path(path);
        PathFinder::get_aggregated_path_cost(&mut path_finder, &mut candidate_path);
        let (actual_weight, actual_amount, actual_time) = (
            candidate_path.weight,
            candidate_path.amount,
            candidate_path.time,
        );
        let expected_weight = 1000;
        let expected_amount = 11000;
        let expected_time = 40;

        assert_eq!(actual_weight, expected_weight);
        assert_eq!(actual_amount, expected_amount);
        assert_eq!(actual_time, expected_time);

        path_finder.routing_metric = RoutingMetric::MaxProb;
    }
}
