use crate::{
    core_types::{event::PaymentEvent, time::Time},
    payment::Payment,
    traversal::pathfinding::{CandidatePath, Path, PathFinder},
    Simulation,
};

use log::{error, trace};

impl Simulation {
    /// Sends a single path payment and fails when payment cannot be delivered
    /// Triggers an event either way
    /// Includes pathfinding and ultimate routing
    pub(crate) fn send_single_payment(&mut self, payment: &mut Payment) -> bool {
        let mut succeeded = false;
        let mut failed = false;
        // fail immediately if sender's balance on each of their edges < amount
        let max_out_balance = self.graph.get_max_node_balance(&payment.source);
        if max_out_balance < payment.amount_msat {
            error!("Payment failing. Sender has no edge with sufficient balance. Amount {}, max balance {}", payment.amount_msat, max_out_balance);
            failed = true;
        }
        // we are not interested in reversing payments here for single path payments
        if !failed {
            succeeded = self.send_one_payment(payment).0;
        }
        let now = self.event_queue.now() + Time::from_secs(crate::SIM_DELAY_IN_SECS);
        let event = if succeeded {
            PaymentEvent::UpdateSuccesful {
                payment: payment.to_owned(),
            }
        } else {
            // used paths is empty for failed payments. failed paths maybe
            assert!(payment.used_paths.is_empty());
            PaymentEvent::UpdateFailed {
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
        // shortest path from src to dest including src and dest sorted in ascending cost order
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
                self.get_aggregated_path_cost(&mut candidate_path, false);
                Some(candidate_path)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::VecDeque;

    use super::*;
    use crate::Invoice;

    #[test]
    fn send_single_path_payment() {
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, None);
        let amount_msat = 1000;
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: true,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
            failed_paths: vec![],
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        assert!(simulator.send_single_payment(payment));
    }

    #[test]
    fn successful_payment_contains_correct_info() {
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, None);
        let amount_msat = 1000;
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: true,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
            failed_paths: vec![],
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        assert!(simulator.send_single_payment(payment));
        let expected_used_path = CandidatePath {
            path: Path {
                src: "alice".to_string(),
                dest: "chan".to_string(),
                hops: VecDeque::from([
                    ("alice".to_string(), 1100, 40, "alice1".to_string()),
                    ("bob".to_string(), 100, 40, "bob2".to_string()),
                    ("chan".to_string(), 1000, 0, "chan1".to_string()),
                ]),
            },
            weight: 100.0,
            amount: 1100,
            time: 40,
        };
        assert_eq!(payment.htlc_attempts, 2);
        assert!(payment.succeeded);
        assert_eq!(payment.used_paths.len(), 1);
        assert_eq!(payment.num_parts, 1);
        assert_eq!(expected_used_path, payment.used_paths[0]);
        assert!(payment.failed_paths.is_empty()); // since the single payment succeeds immediately
    }

    // checking that payment contains failed path. Failure at the last node due to no invoice
    #[test]
    fn failed_paths_in_failed_single_payment() {
        let amount = 1000;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, None);
        let mut payment = Payment {
            payment_id: 0,
            source,
            dest,
            amount_msat: amount,
            succeeded: false,
            used_paths: vec![],
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            failed_paths: vec![],
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
        };
        assert!(!simulator.send_single_payment(&mut payment));
        assert!(!payment.failed_paths.is_empty());
        assert!(payment.used_paths.is_empty());
    }
}
