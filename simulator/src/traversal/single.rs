use crate::{
    core_types::{event::EventType, time::Time},
    payment::Payment,
    traversal::pathfinding::{CandidatePath, Path, PathFinder},
    Simulation,
};

use log::{error, trace};

impl Simulation {
    pub(crate) fn send_single_payment(&mut self, payment: &mut Payment) -> bool {
        let graph = Box::new(self.graph.clone());
        let mut succeeded = false;
        let mut failed = false;
        // fail immediately if sender's balance on each of their edges < amount
        let max_out_balance = graph.get_max_node_balance(&payment.source);
        if max_out_balance < payment.amount_msat {
            error!("Payment failing. Sender has no edge with sufficient balance. Amount {}, max balance {}", payment.amount_msat, max_out_balance);
            failed = true;
        }
        if !failed {
            // we are not interested in reversing payments here for single path payments
            succeeded = self.send_one_payment(payment).0;
        }
        let now = self.event_queue.now() + Time::from_secs(crate::SIM_DELAY_IN_SECS);
        let event = if succeeded {
            EventType::UpdateSuccesfulPayment {
                payment: payment.to_owned(),
            }
        } else {
            EventType::UpdateFailedPayment {
                payment: payment.to_owned(),
            }
        };
        self.event_queue.schedule(now, event);
        succeeded
    }
}

impl PathFinder {
    /// Returns a route, the total amount due and lock time and none if no route is found
    /// Search for paths from dest to src
    pub(super) fn find_path_single_payment(&mut self) -> Option<CandidatePath> {
        self.remove_inadequate_edges();
        // returns distinct paths including src and dest sorted in ascending cost order
        let shortest_path = self.shortest_path_from(&self.src);
        match shortest_path {
            None => {
                trace!("No shortest path between {} and {}.", self.src, self.dest);
                None
            }
            // construct candipaths using k_shortest_path
            // - calculate total path cost
            Some(shortest_path) => {
                trace!("Got shortest path between {} and {}.", self.src, self.dest);
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
                Some(candidate_path)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{core_types::graph::Graph, Invoice, PaymentParts, RoutingMetric};
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
        let expected: CandidatePath = CandidatePath {
            path: expected_path,
            weight: 175,  // fees (b->c, c->d)
            amount: 5175, // amount + fees
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
        let expected: CandidatePath = CandidatePath {
            path: expected_path,
            weight: 1,    // prob (b->c, c->d)
            amount: 5175, // amount + fees
            time: 55,
        };
        assert_eq!(actual, expected);
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

    #[test]
    fn send_single_path_payment() {
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None);
        let amount_msat = 1000;
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: true,
            min_shard_amt: 10,
            attempts: 0,
            num_parts: 1,
            paths: CandidatePath::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        assert!(simulator.send_single_payment(payment));
    }
}
