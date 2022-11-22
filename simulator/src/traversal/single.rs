use crate::{
    traversal::pathfinding::{CandidatePath, Path, PathFinder},
    RoutingMetric, ID,
};
use log::trace;

impl PathFinder {
    /// Returns a route, the total amount due and lock time and none if no route is found
    /// Search for paths from dest to src
    pub(super) fn find_path_single_payment(&mut self) -> Option<Vec<CandidatePath>> {
        let mut candidate_paths = Vec::default();
        self.remove_inadequate_edges();
        trace!(
            "Looking for shortest paths between src {}, dest {} using {:?} as weight.",
            self.src,
            self.dest,
            self.routing_metric
        );
        let successors = |node: &ID| -> Vec<(ID, usize)> {
            let succs = match self.graph.get_edges_for_node(node) {
                Some(edges) => edges
                    .iter()
                    .map(|e| {
                        (
                            e.destination.clone(),
                            if e.source != self.src {
                                Self::get_edge_weight(e, self.amount, self.routing_metric)
                            } else {
                                if self.routing_metric == RoutingMetric::MinFee {
                                    0
                                } else {
                                    1
                                }
                            },
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
            // the weights and timelock are set  as the total path costs are calculated
            path.hops = shortest_path
                .0
                .into_iter()
                .map(|h| (h, usize::default(), usize::default(), String::default()))
                .collect();
            let mut candidate_path = CandidatePath::new_with_path(path);
            self.get_aggregated_path_cost(&mut candidate_path);
            candidate_paths.push(candidate_path);
        }
        // sort? already sorted by cost
        Some(candidate_paths)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{core_types::graph::Graph, PaymentParts, RoutingMetric};
    use std::collections::VecDeque;

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
        let payment_parts = PaymentParts::Single;
        let mut path_finder =
            PathFinder::new(src, dest, amount, graph, routing_metric, payment_parts);
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
        let expected: Vec<CandidatePath> = vec![CandidatePath {
            path: expected_path,
            weight: 175,  // fees (b->c, c->d)
            amount: 5175, // amount + fees
            time: 55,
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
        let mut path_finder = PathFinder::new(
            src,
            dest,
            amount,
            graph,
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
        let expected: Vec<CandidatePath> = vec![CandidatePath {
            path: expected_path,
            weight: 1,    // prob (b->c, c->d)
            amount: 5175, // amount + fees
            time: 55,
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
        PathFinder::get_aggregated_path_cost(&mut path_finder, &mut candidate_path);
        let (actual_weight, actual_amount, actual_time) = (
            candidate_path.weight,
            candidate_path.amount,
            candidate_path.time,
        );
        let expected_weight = 100;
        let expected_amount = 10100;
        let expected_time = 20;

        assert_eq!(actual_weight, expected_weight);
        assert_eq!(actual_amount, expected_amount);
        assert_eq!(actual_time, expected_time);

        path_finder.routing_metric = RoutingMetric::MaxProb;
    }
}
