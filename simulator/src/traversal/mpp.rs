use crate::{
    core_types::{event::PaymentEvent, time::Time},
    payment::Payment,
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation,
};

use log::{error, info, trace, warn};

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
        let max_receive_balance = graph.get_max_receive_amount(&payment.dest);
        if max_receive_balance < payment.amount_msat {
            error!("Payment failing due to insufficient receive capacity. Payment amount {}, max receive {}", payment.amount_msat, max_receive_balance);
            failed = true;
        }

        if !succeeded && !failed {
            payment.used_paths = Vec::new();
            payment.num_parts = 0;
            succeeded = self.send_mpp_shards(payment);
        }
        let now = self.event_queue.now() + Time::from_secs(crate::SIM_DELAY_IN_SECS);
        let event = if succeeded {
            // hacky because the recursive function messes this up
            payment.succeeded = true;
            assert!(payment.succeeded);
            assert!(payment.num_parts == payment.used_paths.len());
            // no longer needed - used to revert payments
            payment.successful_shards = vec![];
            info!(
                "Payment from {} to {} delivered in {} parts.",
                payment.source, payment.dest, payment.num_parts
            );
            PaymentEvent::UpdateSuccesful {
                payment: payment.to_owned(),
            }
        } else {
            assert!(!payment.succeeded);
            PaymentEvent::UpdateFailed {
                payment: payment.to_owned(),
            }
        };
        self.event_queue.schedule(now, event);
        succeeded
    }

    /// Splits a payment into a list of shards belonging to one payment and tries to send them atomically
    fn send_mpp_shards(&mut self, root: &mut Payment) -> bool {
        trace!(
            "Attempting MPP payment {} worth {} msat.",
            root.payment_id,
            root.amount_msat
        );
        let (success, mut to_reverse) = self.send_one_payment(root);
        let succeeded = if success {
            root.succeeded = true;
            root.successful_shards.append(&mut to_reverse);
            true
        } else {
            root.failed_amounts.push(root.amount_msat);
            // Hacky way of making sure we don't exceed max parts
            if self.amount / root.amount_msat >= crate::MAX_PARTS {
                error!(
                    "Aborting splitting as max parts of {} has been reached.",
                    crate::MAX_PARTS
                );
                return false;
            }
            trace!(
                "Splitting payment {} worth {} msat into {} parts.",
                root.payment_id,
                root.amount_msat,
                2
            );
            if let Some(shards) = Payment::split_payment(root) {
                let (mut shard1, mut shard2) = (shards.0, shards.1);
                let shard1_succeeded = self.send_mpp_shards(&mut shard1);
                root.htlc_attempts += shard1.htlc_attempts;
                root.num_parts += shard1.num_parts;
                // because some empty paths show up in MPP payments
                if shard1_succeeded {
                    let mut i = 0;
                    while i < shard1.used_paths.len() {
                        if shard1.used_paths[i].path.hops.is_empty() {
                            root.num_parts -= 1;
                            shard1.used_paths.remove(i);
                        }
                        i += 1;
                    }
                    root.used_paths.append(&mut shard1.used_paths);
                }
                let shard2_succeeded = self.send_mpp_shards(&mut shard2);
                root.htlc_attempts += shard2.htlc_attempts;
                root.num_parts += shard2.num_parts;
                if shard2_succeeded {
                    let mut i = 0;
                    while i < shard2.used_paths.len() {
                        if shard2.used_paths[i].path.hops.is_empty() {
                            root.num_parts -= 1;
                            shard2.used_paths.remove(i);
                        }
                        i += 1;
                    }
                    root.used_paths.append(&mut shard2.used_paths);
                }
                shard1_succeeded && shard2_succeeded
            } else {
                warn!(
                    "Splitting payment {} worth {} msat into {} parts failed.",
                    root.payment_id, root.amount_msat, 2
                );
                false
            }
        };
        // total failure so revert succesful payments
        // some payment failed so all must now be reversed
        if !succeeded {
            self.revert_payment(&root.successful_shards);
        }
        //root.succeeded = succeeded; //?
        succeeded
    }
}

impl PathFinder {
    pub(super) fn find_path_mpp_payment(&mut self) -> Option<CandidatePath> {
        self.find_path_single_payment()
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
            successful_shards: Vec::default(),
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
            successful_shards: Vec::default(),
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
            successful_shards: Vec::default(),
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
            successful_shards: Vec::default(),
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
                weight: 10.0,
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
                weight: 30.0,
                amount: 6030,
                time: 10,
            },
        ];
        assert_eq!(payment.htlc_attempts, 2);
        assert!(payment.succeeded);
        assert_eq!(payment.num_parts, 2);
        assert_eq!(payment.used_paths.len(), 2);
        assert_eq!(expected_used_path, payment.used_paths);
    }
}
