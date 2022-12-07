use crate::{
    core_types::{event::PaymentEvent, time::Time},
    payment::Payment,
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation,
};

use log::{error, info, trace};

impl Simulation {
    /// Sends an MPP and fails when payment can no longer be split into smaller parts
    /// Triggers an event either way
    /// Includes pathfinding and ultimate routing
    pub(crate) fn send_mpp_payment(&mut self, payment: &mut Payment) -> bool {
        let mut succeeded = false;
        let mut failed = false;
        let graph = Box::new(self.graph.clone());
        // fail immediately if sender's total balance < amount
        let total_out_balance = graph.get_total_node_balance(&payment.source);
        if total_out_balance < payment.amount_msat {
            error!("Payment failing. {} total balance insufficient for payment. Amount {}, max balance {}", payment.source, payment.amount_msat, total_out_balance);
            failed = true;
        }
        // we don't care about reversing a single payment since it is already happened in the
        // returning function if necessary
        if !succeeded {
            (succeeded, _) = self.send_one_payment(payment);
        }

        let mut split_and_attempt = |payment: &mut Payment| -> (bool, bool) {
            let num_parts_to_try = payment.num_parts * 2;
            let parts = num_parts_to_try;
            let mut parts: Vec<Payment> = Vec::with_capacity(parts);
            let mut success = false;
            let mut failure = false;
            payment.num_parts = num_parts_to_try;
            // divide the payment amount by num_parts_to_try/2 which should be split equally
            // among parts
            let amt_to_split = payment.amount_msat / (num_parts_to_try / 2);
            // divide by 2 so that split results in num_parts_to_try shards
            for _ in 0..(num_parts_to_try / 2) {
                if let Some(shard) = Payment::split_payment(payment, amt_to_split) {
                    parts.push(shard.0);
                    parts.push(shard.1);
                } else {
                    error!("Payment splitting has failed. Ending..");
                    failure = true;
                    break;
                }
            }
            trace!("Payment split into {} parts.", parts.len());
            if !failure {
                success = self.send_mpp_shards(payment, &mut parts);
                if !success {
                    trace!("Will now try {} parts.", num_parts_to_try * 2);
                }
            }
            (success, failure)
        };
        while !succeeded && !failed {
            (succeeded, failed) = split_and_attempt(payment);
        }
        let now = self.event_queue.now() + Time::from_secs(crate::SIM_DELAY_IN_SECS);
        let event = if succeeded {
            payment.succeeded = true;
            info!(
                "Payment from {} to {} delivered in {} parts.",
                payment.source, payment.dest, payment.num_parts
            );
            PaymentEvent::UpdateSuccesful {
                payment: payment.to_owned(),
            }
        } else if failed {
            PaymentEvent::UpdateFailed {
                payment: payment.to_owned(),
            }
        } else {
            panic!("Unexpected payment status {:?}", payment);
        };
        self.event_queue.schedule(now, event);
        succeeded
    }

    /// Expects a list of shards belonging to one payment and tries to send them atomically
    fn send_mpp_shards(&mut self, root: &mut Payment, shards: &mut Vec<Payment>) -> bool {
        let mut succeeded = true;
        let mut issued_payments = Vec::new();
        for shard in shards.iter_mut() {
            let (success, maybe_reverse) = self.send_one_payment(shard);
            root.htlc_attempts += shard.htlc_attempts;
            issued_payments.push(maybe_reverse);
            succeeded &= success;
        }
        // some payment failed so all must now be reversed
        if !succeeded {
            for transfers in issued_payments {
                self.revert_payment(&transfers);
            }
        } else {
            for shard in shards {
                root.used_paths.extend(shard.used_paths.clone());
            }
        }
        succeeded
    }
}

impl PathFinder {
    pub(super) fn find_path_mpp_payment(&mut self) -> Option<CandidatePath> {
        // copy the graph so that deleted edges remain in the next attempt
        let graph_copy = self.graph.clone();
        let candidate_path = self.find_path_single_payment();
        self.graph = graph_copy;
        candidate_path
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::{traversal::pathfinding::Path, Invoice, PaymentParts};

    #[test]
    fn send_multipath_payment() {
        let source = "alice".to_string();
        let dest = "bob".to_string();
        let json_file = "../test_data/trivial_multipath.json";
        let mut simulator = crate::attempt::tests::init_sim(Some(json_file.to_string()));
        let amount_msat = 300000;
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: false,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        assert!(!simulator.send_single_payment(payment));
        simulator
            .graph
            .update_channel_balance(&String::from("alice-carol"), 100000);
        simulator
            .graph
            .update_channel_balance(&String::from("alice-dave"), 250000);

        simulator.payment_parts = PaymentParts::Split;
        simulator.send_mpp_payment(payment);
        assert!(payment.num_parts > 1);
    }

    #[test]
    // all edges have 10k balance. Bob has a total of 30k spread across 3 channels and
    // want to send alice 12k.
    // We confirm that a single payment will fail then expect it to succeed when using MPP.
    fn mpp_success_min_three_paths() {
        let json_file = "../test_data/trivial_multipath.json";
        let source = "bob".to_string();
        let dest = "alice".to_string();
        let mut simulator = crate::attempt::tests::init_sim(Some(json_file.to_string()));
        let balance = 10000;
        for edges in simulator.graph.edges.values_mut() {
            for e in edges {
                e.balance = balance;
            }
        }
        let amount_msat = 12000;
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: false,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        simulator.payment_parts = PaymentParts::Single;
        assert!(!simulator.send_single_payment(payment));
        simulator.payment_parts = PaymentParts::Split;
        assert!(simulator.send_mpp_payment(payment));
        assert!(payment.succeeded);
        assert!(payment.num_parts > 1);
    }

    #[test]
    // all edges except bob have 1k balance. Bob has a total of 15k spread across 3 channels and
    // wants to send alice 12k.
    fn mpp_failure_hops_no_funds() {
        let json_file = "../test_data/trivial_multipath.json";
        let source = "bob".to_string();
        let dest = "alice".to_string();
        let mut simulator = crate::attempt::tests::init_sim(Some(json_file.to_string()));
        let balance = 1000;
        for edges in simulator.graph.edges.values_mut() {
            for e in edges {
                e.balance = balance;
            }
        }
        let bob_eve_channel = String::from("bob-eve");
        let bob_carol_channel = String::from("bob-carol");
        let bob_dave_channel = String::from("bob-dave");
        let bob_total_balance = 15000;
        simulator
            .graph
            .update_channel_balance(&bob_eve_channel, bob_total_balance / 3);
        simulator
            .graph
            .update_channel_balance(&bob_carol_channel, bob_total_balance / 3);
        simulator
            .graph
            .update_channel_balance(&bob_dave_channel, bob_total_balance / 3);
        let amount_msat = 12000;
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: false,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        simulator.payment_parts = PaymentParts::Single;
        assert!(!simulator.send_single_payment(payment));
        simulator.payment_parts = PaymentParts::Split;
        assert!(!simulator.send_mpp_payment(payment));
    }

    #[test]
    fn successful_mpp_payment_contains_correct_info() {
        let json_file = "../test_data/trivial_multipath.json";
        let source = "bob".to_string();
        let dest = "alice".to_string();
        let mut simulator = crate::attempt::tests::init_sim(Some(json_file.to_string()));
        let balance = 10000;
        for edges in simulator.graph.edges.values_mut() {
            for e in edges {
                e.balance = balance;
            }
        }
        let amount_msat = 12000;
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: false,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        assert!(!simulator.send_single_payment(payment));
        simulator.payment_parts = PaymentParts::Split;
        assert!(simulator.send_mpp_payment(payment));
        let expected_used_path = vec![
            CandidatePath {
                path: Path {
                    src: "bob".to_string(),
                    dest: "alice".to_string(),
                    hops: VecDeque::from([
                        ("bob".to_string(), 6010, 5, "bob-carol".to_string()),
                        ("carol".to_string(), 10, 5, "carol-alice".to_string()),
                        ("alice".to_string(), 6000, 0, "alice-carol".to_string()),
                    ]),
                },
                weight: 10,
                amount: 6010,
                time: 5,
            },
            CandidatePath {
                path: Path {
                    src: "bob".to_string(),
                    dest: "alice".to_string(),
                    hops: VecDeque::from([
                        ("bob".to_string(), 6030, 10, "bob-eve".to_string()),
                        ("eve".to_string(), 20, 5, "eve-carol".to_string()),
                        ("carol".to_string(), 10, 5, "carol-alice".to_string()),
                        ("alice".to_string(), 6000, 0, "alice-carol".to_string()),
                    ]),
                },
                weight: 30,
                amount: 6030,
                time: 10,
            },
        ];
        println!("payment {:?}", payment.used_paths);
        assert_eq!(payment.htlc_attempts, 5);
        assert!(payment.succeeded);
        assert_eq!(payment.num_parts, 2);
        assert_eq!(payment.used_paths.len(), 5);
        assert_eq!(expected_used_path, payment.used_paths);
    }
}
