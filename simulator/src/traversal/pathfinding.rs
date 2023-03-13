use crate::{graph::Graph, Edge, EdgeWeight, PaymentParts, RoutingMetric, ID};

use log::{debug, trace};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

/// Describes a path between two nodes
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Path {
    pub(crate) src: ID,
    pub(crate) dest: ID,
    /// the edges of the path described from sender to receiver including fees and timelock over
    /// the edge ID
    /// The dest's hop describes the channel whose balance will increase and used for reverting.
    /// Format: (hop, fees, timelock, channel_id)
    pub hops: VecDeque<(ID, usize, usize, String)>,
}

/// Pathfinding object
#[derive(Debug, Clone)]
pub struct PathFinder {
    /// Network topolgy graph
    pub(crate) graph: Box<Graph>,
    /// Node looking for a route
    pub(crate) src: ID,
    /// the destination node
    pub(crate) dest: ID,
    /// How much is being sent from src to dest
    pub(super) amount: usize,
    pub(super) routing_metric: RoutingMetric,
    pub(super) payment_parts: PaymentParts,
}

/// A path that we may use to route from src to dest
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct CandidatePath {
    pub path: Path,
    /// The aggregated path weight (fees or probability) describing how costly the path is
    pub(crate) weight: f32,
    /// The aggregated amount due when using this path (amount + fees)
    pub(crate) amount: usize,
    /// The aggregated timelock
    pub(crate) time: usize,
}

impl Path {
    pub fn new(src: ID, dest: ID) -> Self {
        let hops = VecDeque::new();
        Self { src, dest, hops }
    }

    /// Including src and dest
    pub(crate) fn get_involved_nodes(&self) -> Vec<ID> {
        self.hops.iter().map(|h| h.0.clone()).collect()
    }

    // will fail if node is the first hop
    pub(crate) fn get_pred(&self, node: &ID) -> ID {
        let node_pos = self.get_involved_nodes().iter().position(|n| n.eq(node));
        match node_pos {
            Some(idx) => {
                if idx == 0 {
                    String::default()
                } else {
                    self.hops[idx - 1].0.clone()
                }
            }
            None => String::default(),
        }
    }

    // will fail if node is the final hop
    pub(crate) fn get_succ(&self, node: &ID) -> ID {
        let node_pos = self.get_involved_nodes().iter().position(|n| n.eq(node));
        match node_pos {
            Some(idx) => {
                if idx == self.hops.len() - 1 {
                    String::default()
                } else {
                    self.hops[idx + 1].0.clone()
                }
            }
            None => String::default(),
        }
    }

    ///Ignores the first and final hops and returns the contained adversaries "ID, received_amt,
    ///received_timelock" if any
    /// TODO: We don't use the tx data?
    pub(crate) fn path_contains_adversary(&self, adv: &[ID]) -> Vec<(ID, usize, usize)> {
        let mut adversaries = vec![];
        let involved_nodes = self.get_involved_nodes()[1..self.hops.len()].to_owned(); //excludes src
        for i in 0..involved_nodes.len() - 1 {
            // exclude dest
            let node = involved_nodes[i].to_string();
            if adv.contains(&node) {
                let offset = self.hops.len() - involved_nodes.len();
                let hops = self.hops.clone();
                let amt_rcvd: usize = (i..involved_nodes.len()).map(|h| hops[h + offset].1).sum();
                let ttl_left: usize = (i..involved_nodes.len()).map(|h| hops[h + offset].2).sum();
                let tx_data = (node.to_owned(), amt_rcvd, ttl_left);
                adversaries.push(tx_data);
            }
        }
        adversaries
    }

    /// Returns the subpath from (including) the given node
    pub(crate) fn subpath_from(&self, node: &ID) -> VecDeque<(ID, usize, usize, String)> {
        match self.get_involved_nodes().iter().position(|n| n.eq(node)) {
            Some(node_pos) => self
                .hops
                .range(node_pos..)
                .cloned()
                .collect::<VecDeque<_>>(),
            None => VecDeque::default(),
        }
    }

    /// Checks if the other path is a subpath of self
    pub(crate) fn is_subpath(&self, other: &Path) -> bool {
        // find first matching element in
        let mut is_subpath = false;
        if self.hops.len() > other.hops.len() {
            is_subpath = false;
        } else {
            let first_elem_in_both = self
                .get_involved_nodes()
                .iter()
                .position(|n| other.get_involved_nodes().contains(n));
            match first_elem_in_both {
                Some(idx) => {
                    match other
                        .get_involved_nodes()
                        .iter()
                        .position(|n| *n == self.hops[idx].0)
                    {
                        Some(i) => {
                            // start at the next element since we know i == idx
                            for (s, o) in (idx + 1..self.hops.len()).zip(i + 1..other.hops.len()) {
                                if self.hops[s].0 == other.hops[o].0 {
                                    is_subpath = true;
                                    break;
                                }
                            }
                        }
                        None => is_subpath = false,
                    }
                }
                None => is_subpath = false,
            };
        }
        is_subpath
    }

    pub(crate) fn is_equal(
        lhs: &VecDeque<(ID, usize, usize, String)>,
        rhs: &VecDeque<(ID, usize, usize, String)>,
    ) -> bool {
        if lhs.len() != rhs.len() {
            return false;
        } else {
            for (l, r) in (0..lhs.len()).zip(0..rhs.len()) {
                if lhs[l].0 != rhs[r].0 {
                    return false;
                }
            }
        }
        true
    }

    fn update_hop(&mut self, hop_id: ID, fees: usize, timelock: usize, edge_id: &String) {
        for hop in self.hops.iter_mut() {
            if hop.0 == hop_id {
                *hop = (hop.0.clone(), fees, timelock, edge_id.to_owned())
            }
        }
    }

    pub(crate) fn path_length(&self) -> usize {
        self.hops.len() - 1 // hops includes src and dest
    }
}

impl CandidatePath {
    pub fn new_with_path(path: Path) -> Self {
        CandidatePath {
            path,
            weight: f32::default(),
            amount: usize::default(),
            time: usize::default(),
        }
    }

    /// Returns the fees paid. We consider the separate parts' amounts for MPP payments.
    pub(crate) fn path_fees(&self) -> usize {
        // payments that fail immediately because of insufficient sender balance can be empty
        if !self.path.hops.is_empty() {
            self.path.hops[0].1 - self.path.hops[self.path.hops.len() - 1].1
        } else {
            0
        }
    }
}

impl PathFinder {
    /// New PathFinder for payment from src to dest transferring amount of msats
    pub fn new(
        src: ID,
        dest: ID,
        amount: usize,
        graph: &Graph,
        routing_metric: RoutingMetric,
        payment_parts: PaymentParts,
    ) -> Self {
        Self {
            graph: Box::new(graph.clone()),
            src,
            dest,
            amount,
            routing_metric,
            payment_parts,
        }
    }

    pub(crate) fn find_path(&mut self) -> Option<CandidatePath> {
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
        ordered_float::OrderedFloat((base_fee + prop_fee + time_lock_penalty) as f32)
    }

    /// Returns the edge failure probabilty (amt/ cap) of given amount so that the shortest path
    /// weights it accordingly
    /// The higher the returned value, the lower the chances of success
    /// https://github.com/lnbook/lnbook/blob/develop/12_path_finding.asciidoc#liquidity-uncertainty-and-probability
    fn get_edge_failure_probabilty(edge: &Edge, amount: usize) -> EdgeWeight {
        let success_prob: f32 = (edge.htlc_maximum_msat as f32 + 1.0 - amount as f32)
            / (edge.htlc_maximum_msat as f32 + 1.0);
        ordered_float::OrderedFloat(1.0 - success_prob)
    }

    /// Calculates the total probabilty along a given path starting from dest to src
    /// The first node can optionally be treated as an intermediary and demand fees. Used by
    /// adversary calculations
    pub fn get_aggregated_path_cost(
        &mut self,
        mut candidate_path: &mut CandidatePath,
        include_src: bool,
    ) {
        // 1. for all (src, dest) pairs in the path:
        // 2. calculate weight and fee
        // 3. output: total weight, total fees and total amount due
        trace!(
            "Calculating total cost for CandidatePath = {:?}.",
            candidate_path
        );
        let mut accumulated_amount = self.amount; //amount + due fees
        let mut accumulated_weight = if self.routing_metric == RoutingMetric::MinFee {
            0.0
        } else {
            1.0
        };
        let mut accumulated_time = 0; // full timelock delta
        let candidate_path_hops: VecDeque<ID> = candidate_path
            .path
            .get_involved_nodes()
            .into_iter()
            .rev()
            .collect();
        for (idx, node_id) in candidate_path_hops.iter().enumerate() {
            if node_id.clone() == self.src {
                // Edge from src to first hop
                // safe because src is always last in the list
                let (src, dest) = (node_id, candidate_path_hops[idx - 1].clone());
                let cheapest_edge = match self.get_cheapest_edge(src, &dest) {
                    None => panic!("Edge in path does not exist! {src} -> {dest}"),
                    Some(e) => e,
                };
                if include_src {
                    // src charges a fee
                    match self.routing_metric {
                        RoutingMetric::MaxProb => {
                            accumulated_weight *= 1.0
                                - Self::get_edge_failure_probabilty(
                                    &cheapest_edge,
                                    accumulated_amount,
                                )
                                .into_inner()
                        }
                        RoutingMetric::MinFee => {
                            accumulated_weight +=
                                Self::get_edge_fee(&cheapest_edge, accumulated_amount).into_inner()
                        }
                    };
                    let edge_fee = Self::get_edge_fee(&cheapest_edge, accumulated_amount)
                        .into_inner() as usize;
                    accumulated_amount += edge_fee;
                    let edge_timelock = cheapest_edge.cltv_expiry_delta;
                    accumulated_time += edge_timelock;
                }
                candidate_path.path.update_hop(
                    cheapest_edge.source,
                    accumulated_amount,
                    accumulated_time,
                    &cheapest_edge.channel_id,
                );
            } else if node_id.clone() == self.dest {
                let (dest, src) = (node_id, candidate_path_hops[idx + 1].clone());
                let cheapest_edge = match self.get_cheapest_edge(dest, &src) {
                    None => panic!("Edge in path does not exist! {src} -> {dest}"),
                    Some(e) => e,
                };
                candidate_path.path.update_hop(
                    cheapest_edge.source,
                    accumulated_amount,
                    accumulated_time,
                    &cheapest_edge.channel_id,
                );
            } else {
                let (src, dest) = (node_id, candidate_path_hops[idx - 1].clone());
                // we are interested in the weight from src to dest (the previous node in the list) since that is the direction the
                // payment will flow in
                let cheapest_edge = match self.get_cheapest_edge(src, &dest) {
                    None => panic!("Edge in path does not exist! {src} -> {dest}"),
                    Some(e) => e,
                };
                match self.routing_metric {
                    RoutingMetric::MaxProb => {
                        accumulated_weight *= 1.0
                            - Self::get_edge_failure_probabilty(&cheapest_edge, accumulated_amount)
                                .into_inner()
                    }
                    RoutingMetric::MinFee => {
                        accumulated_weight +=
                            Self::get_edge_fee(&cheapest_edge, accumulated_amount).into_inner()
                    }
                };
                let edge_fee =
                    Self::get_edge_fee(&cheapest_edge, accumulated_amount).into_inner() as usize;
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

    /// Computes the shortest path beween source and dest using Dijkstra's algorithm
    pub fn shortest_path_from(&self, node: &ID) -> Option<(Vec<ID>, EdgeWeight)> {
        trace!(
            "Looking for shortest paths between src {}, dest {} using {:?} as weight.",
            self.src,
            self.dest,
            self.routing_metric
        );
        let successors = |node: &ID| -> Vec<(ID, EdgeWeight)> { self.get_successors(node) };
        pathfinding::prelude::dijkstra(node, successors, |n| *n == self.dest)
    }

    /// Computes the k shortest path beween source and dest using Dijkstra's algorithm
    pub fn k_shortest_paths_from(&self, node: &ID, k: usize) -> Vec<(Vec<ID>, EdgeWeight)> {
        trace!(
            "Looking for {} shortest paths between src {}, dest {} using {:?} as weight.",
            k,
            self.src,
            self.dest,
            self.routing_metric
        );
        let successors = |node: &ID| -> Vec<(ID, EdgeWeight)> { self.get_successors(node) };
        pathfinding::prelude::yen(node, successors, |n| *n == self.dest, k)
    }

    fn get_successors(&self, node: &ID) -> Vec<(ID, EdgeWeight)> {
        let succs = match self.graph.get_edges_for_node(node) {
            Some(edges) => edges
                .iter()
                .map(|e| {
                    (
                        e.destination.clone(),
                        if e.source != self.src {
                            Self::get_edge_weight(e, self.amount, self.routing_metric)
                        } else if self.routing_metric == RoutingMetric::MinFee {
                            ordered_float::OrderedFloat(0.0)
                        } else {
                            ordered_float::OrderedFloat(1.0)
                        },
                    )
                })
                .collect(),
            None => Vec::default(),
        };
        succs
    }

    /// Returns the "cheapest" edge between src and dist bearing the routing me in mind
    /// Used after finding the shortest paths and are therefore interested in routing along the
    /// edge
    /// Necessary as we account for possible parallel edges
    pub(crate) fn get_cheapest_edge(&mut self, from: &ID, to: &ID) -> Option<Edge> {
        let from_to_outedges = self.graph.get_all_src_dest_edges(from, to);
        let mut cheapest_edge = None;
        let mut min_weight = ordered_float::OrderedFloat(f32::MAX);
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
    pub fn remove_inadequate_edges(
        graph: &Graph,
        amount: usize,
    ) -> HashMap<String, Vec<Edge>> {
        debug!("Removing edges with insufficient funds.");
        let mut copy = graph.clone();
        let mut ctr = 0;
        for edge in graph.edges.iter() {
            // iter each node's edges
            for e in edge.1 {
                if e.balance < amount {
                    ctr += 1;
                    copy.remove_edge(&e.source, &e.destination);
                }
            }
        }
        trace!("Removed {} edges with insufficient funds.", ctr);
        copy.edges
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{core_types::graph::Graph, PaymentParts, RoutingMetric};
    use approx::*;
    use std::collections::VecDeque;

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
        let expected = 0.0;
        assert_abs_diff_eq!(actual.into_inner(), expected, epsilon = 0.2f32);
        let amount = 600;
        let actual = PathFinder::get_edge_failure_probabilty(&edge, amount);
        let expected = 1.0;
        assert_abs_diff_eq!(actual.into_inner(), expected, epsilon = 0.2f32);
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
        let expected = 100.0;
        assert_eq!(actual, expected);
        let amount = 600;
        let actual = PathFinder::get_edge_fee(&edge, amount);
        let expected = 100.0;
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
        let src = String::from("alice");
        let dest = String::from("dina");
        let amount = 5000;
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        let mut path_finder =
            PathFinder::new(src, dest, amount, &graph, routing_metric, payment_parts);
        let actual = path_finder.find_path();
        assert!(actual.is_some());
        let actual = actual.unwrap();
        let expected_path = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("alice".to_string(), 5175, 55, "alice1".to_string()),
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let expected: CandidatePath = CandidatePath {
            path: expected_path,
            weight: 175.0, // fees (b->c, c->d)
            amount: 5175,  // amount + fees
            time: 55,
        };
        assert_eq!(actual, expected);
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
        let src = String::from("alice");
        let dest = String::from("dina");
        let amount = 5000;
        let routing_metric = RoutingMetric::MaxProb;
        let mut path_finder = PathFinder::new(
            src,
            dest,
            amount,
            &graph,
            routing_metric,
            PaymentParts::Single,
        );
        let actual = path_finder.find_path();
        assert!(actual.is_some());
        let actual = actual.unwrap();
        let expected_path = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("alice".to_string(), 5175, 55, "alice1".to_string()),
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let expected: CandidatePath = CandidatePath {
            path: expected_path,
            weight: 1.0,  // prob (b->c, c->d)
            amount: 5175, // amount + fees
            time: 55,
        };
        // a and b equal if |a - b| <= epsilon
        assert_abs_diff_eq!(expected.weight, actual.weight, epsilon = 0.1f32);
        assert_eq!(actual.amount, expected.amount);
        assert_eq!(actual.time, expected.time);
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
            payment_parts: PaymentParts::Single,
        };
        let path = Path {
            src: path_finder.src.clone(),
            dest: path_finder.dest.clone(),
            hops: VecDeque::from([
                ("dina".to_string(), 0, 0, "".to_string()),
                ("chan".to_string(), 0, 0, "c".to_string()),
                ("bob".to_string(), 0, 0, "".to_string()),
            ]),
        };
        let mut candidate_path = &mut CandidatePath::new_with_path(path);
        PathFinder::get_aggregated_path_cost(&mut path_finder, &mut candidate_path, false);
        let (actual_weight, actual_amount, actual_time) = (
            candidate_path.weight,
            candidate_path.amount,
            candidate_path.time,
        );
        let expected_weight = 100.0;
        let expected_amount = 10100;
        let expected_time = 20;

        assert_eq!(actual_weight, expected_weight);
        assert_eq!(actual_amount, expected_amount);
        assert_eq!(actual_time, expected_time);

        path_finder.routing_metric = RoutingMetric::MaxProb;
    }

    // see above tests for calculations
    #[test]
    fn get_fees() {
        let json_file = std::path::Path::new("../test_data/lnbook_example.json");
        let mut graph = Graph::to_sim_graph(&network_parser::from_json_file(&json_file).unwrap());
        let balance = 70000; // ensure balances are not the reason for failure
        for (_, edges) in graph.edges.iter_mut() {
            for e in edges {
                e.balance = balance;
            }
        }
        let src = String::from("alice");
        let dest = String::from("dina");
        let amount = 5000;
        let routing_metric = RoutingMetric::MaxProb;
        let mut path_finder = PathFinder::new(
            src,
            dest,
            amount,
            &graph,
            routing_metric,
            PaymentParts::Single,
        );
        if let Some(candidate_path) = path_finder.find_path() {
            let actual = candidate_path.path_fees();
            let expected = 175;
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn get_predecessor_and_successor() {
        let path = CandidatePath {
            path: Path {
                src: String::from("alice"),
                dest: String::from("dina"),
                hops: VecDeque::from([
                    ("alice".to_string(), 5175, 55, "alice1".to_string()),
                    ("bob".to_string(), 100, 40, "bob2".to_string()),
                    ("chan".to_string(), 75, 15, "chan2".to_string()),
                    ("dina".to_string(), 5000, 0, "dina1".to_string()),
                ]),
            },
            weight: 175.0, // fees (b->c, c->d)
            amount: 5175,  // amount + fees
            time: 55,
        };
        let node = "bob".to_string();
        let pred = path.path.get_pred(&node);
        let succ = path.path.get_succ(&node);
        assert_eq!(pred, "alice".to_string());
        assert_eq!(succ, "chan".to_string());
    }

    #[test]
    fn adversary_in_path() {
        let path = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("alice".to_string(), 5175, 55, "alice1".to_string()),
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let adv = vec!["bob".to_string(), "chan".to_string(), "dina".to_string()];
        let expected = vec![("bob".to_owned(), 5175, 55), ("chan".to_string(), 5075, 15)];
        let actual = path.path_contains_adversary(&adv);
        assert_eq!(actual, expected);
        let adv = vec!["ed".to_string()];
        assert!(path.path_contains_adversary(&adv).is_empty());
    }

    #[test]
    fn aggregated_path_cost_including_src() {
        let json_file = std::path::Path::new("../test_data/lnbook_example.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(&json_file).unwrap());
        let mut path_finder = PathFinder {
            graph: Box::new(graph),
            src: "dina".to_string(),
            dest: "bob".to_string(),
            amount: 10000,
            routing_metric: RoutingMetric::MinFee,
            payment_parts: PaymentParts::Single,
        };
        let path = Path {
            src: path_finder.src.clone(),
            dest: path_finder.dest.clone(),
            hops: VecDeque::from([
                ("dina".to_string(), 0, 0, "".to_string()),
                ("chan".to_string(), 0, 0, "c".to_string()),
                ("bob".to_string(), 0, 0, "".to_string()),
            ]),
        };
        let mut candidate_path = &mut CandidatePath::new_with_path(path);
        path_finder.get_aggregated_path_cost(&mut candidate_path, true);
        let (actual_weight, actual_amount, actual_time) = (
            candidate_path.weight,
            candidate_path.amount,
            candidate_path.time,
        );
        let expected_weight = 1100.0;
        let expected_amount = 11100;
        let expected_time = 60;

        assert_eq!(actual_weight, expected_weight);
        assert_eq!(actual_amount, expected_amount);
        assert_eq!(actual_time, expected_time);
    }

    #[test]
    fn get_subpath() {
        let path = Path {
            src: "alice".to_string(),
            dest: "dina".to_string(),
            hops: VecDeque::from([
                ("a".to_string(), 0, 0, "".to_string()),
                ("alice".to_string(), 0, 0, "".to_string()),
                ("bob".to_string(), 0, 0, "".to_string()),
                ("chan".to_string(), 0, 0, "".to_string()),
                ("dina".to_string(), 0, 0, "".to_string()),
            ]),
        };
        let from = "bob".to_string();
        let actual = path.subpath_from(&from);
        let expected = VecDeque::from([
            ("bob".to_string(), 0, 0, "".to_string()),
            ("chan".to_string(), 0, 0, "".to_string()),
            ("dina".to_string(), 0, 0, "".to_string()),
        ]);
        assert_eq!(actual, expected);
        let from = "dina".to_string();
        let actual = path.subpath_from(&from);
        let expected = VecDeque::from([("dina".to_string(), 0, 0, "".to_string())]);
        assert_eq!(actual, expected);
        let from = "emily".to_string();
        let actual = path.subpath_from(&from);
        let expected = VecDeque::default();
        assert_eq!(actual, expected);
    }

    #[test]
    fn subpath() {
        let self_path = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("alice".to_string(), 5175, 55, "alice1".to_string()),
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let other_path = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("alice".to_string(), 5175, 55, "alice1".to_string()),
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let actual = self_path.is_subpath(&other_path);
        let expected = true;
        assert_eq!(actual, expected);
        let self_path = Path {
            src: String::from("bob"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let actual = self_path.is_subpath(&other_path);
        let expected = true;
        assert_eq!(actual, expected);
        let self_path = Path {
            src: String::from("bob"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let actual = self_path.is_subpath(&other_path);
        let expected = false;
        assert_eq!(actual, expected);
    }
}
