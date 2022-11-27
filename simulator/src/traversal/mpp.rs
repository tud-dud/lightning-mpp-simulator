use crate::{
    core_types::{event::EventType, time::Time},
    payment::Payment,
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation,
};

use log::{error, info, trace};

impl Simulation {
    // 1. try full payment
    // 2. if fails: split payment into parts * 2
    // 3. try all parts and revert each immediately if failure
    // 4. if fails: go to 2
    // break if no paths are returned or cannot split further
    // send event after ultimate failure or success
    /// Fails when payment cannot be split into smaller parts
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
        (succeeded, _) = self.send_one_payment(payment);

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
                    trace!("Payment split into {} parts.", parts.len());
                    success = self.send_mpp_shards(&mut parts);
                    if success {
                        info!(
                            "Payment from {} to {} delivered in {} parts.",
                            payment.source, payment.dest, payment.num_parts
                        );
                        break;
                    } else {
                        trace!("Will now try {} parts.", num_parts_to_try * 2);
                    }
                } else {
                    error!("Payment splitting has failed. Ending..");
                    failure = true;
                    break;
                }
            }
            (success, failure)
        };
        while !succeeded && !failed {
            (succeeded, failed) = split_and_attempt(payment);
        }
        let now = self.event_queue.now() + Time::from_secs(crate::SIM_DELAY_IN_SECS);
        let event = if succeeded {
            EventType::UpdateSuccesfulPayment {
                payment: payment.to_owned(),
            }
        } else if failed {
            EventType::UpdateFailedPayment {
                payment: payment.to_owned(),
            }
        } else {
            panic!("Unexpected payment status {:?}", payment);
        };
        self.event_queue.schedule(now, event);
        succeeded
    }

    // 3. try all parts and revert each immediately if failure
    // change payment attempt and revert manually so that we can revert here
    fn send_mpp_shards(&mut self, shards: &mut Vec<Payment>) -> bool {
        let mut succeeded = true;
        let mut issued_payments = Vec::new();
        for shard in shards {
            let (success, maybe_reverse) = self.send_one_payment(shard);
            issued_payments.push(maybe_reverse);
            succeeded &= success;
        }
        // some payment failed so all must now be reversed
        if !succeeded {
            for transfers in issued_payments {
                self.revert_payment(&transfers);
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
    use super::*;
    use crate::{Invoice, PaymentParts};

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
            attempts: 0,
            num_parts: 1,
            paths: CandidatePath::default(),
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
        assert!(simulator.send_mpp_payment(payment));
        assert!(payment.num_parts > 1);
    }

    #[test]
    // all edges except bob have 10k balance. Bob has a total of 15k spread across 3 channels and
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
            attempts: 0,
            num_parts: 1,
            paths: CandidatePath::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        simulator.payment_parts = PaymentParts::Single;
        assert!(!simulator.send_single_payment(payment));
        simulator.payment_parts = PaymentParts::Split;
        assert!(simulator.send_mpp_payment(payment));
        assert!(payment.num_parts >= 3);
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
            attempts: 0,
            num_parts: 1,
            paths: CandidatePath::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount_msat, &source, &dest));
        simulator.payment_parts = PaymentParts::Single;
        assert!(!simulator.send_single_payment(payment));
        simulator.payment_parts = PaymentParts::Split;
        assert!(!simulator.send_mpp_payment(payment));
    }
}
