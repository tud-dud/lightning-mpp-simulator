use crate::{
    core_types::{event::EventType, time::Time},
    payment::Payment,
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation,
};

use log::error;

impl Simulation {
    pub(crate) fn send_mpp_payment(&mut self, payment: &mut Payment) -> bool {
        let graph = Box::new(self.graph.clone());
        let mut succeeded = false;
        let mut failed = false;
        // fail immediately if sender's total balance < amount
        let total_out_balance = graph.get_total_node_balance(&payment.source);
        if total_out_balance < payment.amount_msat {
            error!("Payment failing. {} total balance insufficient for payment. Amount {}, max balance {}", payment.source, payment.amount_msat, total_out_balance);
            println!("Payment failing. {} total balance insufficient for payment. Amount {}, max balance {}", payment.source, payment.amount_msat, total_out_balance);
            failed = true;
        }
        if !failed {
            succeeded = self.send_one_payment(payment);
        } else {
            // split payment and try again
            error!(
                "Payment from {} to {} of amount {} failed. Will try splitting.",
                payment.source, payment.dest, payment.amount_msat
            );
        }
        while !succeeded && !failed {
            payment.failed_amounts.push(payment.amount_msat);
            if let Some(shard) = Payment::split_payment(payment) {
                let mut shard1 = shard.0;
                let mut shard2 = shard.1;
                payment.num_parts += 2;
                let shard1_succeeded = self.send_mpp_payment(&mut shard1);
                let shard2_succeeded = self.send_mpp_payment(&mut shard2);
                println!(
                    "shard1_succeeded {}, shard2_succeeded {}",
                    shard1_succeeded, shard2_succeeded
                );
                succeeded = shard1_succeeded && shard2_succeeded;
                println!(
                    "succeeded {}, amount1 {}, amount2 {}",
                    succeeded, shard1.amount_msat, shard2.amount_msat
                );
            } else {
                // final failure
                error!("Not able to split further.");
                succeeded = false;
                failed = true;
            }
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
    #[ignore]
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

        simulator.payment_parts = PaymentParts::Single;
        assert!(simulator.send_mpp_payment(payment));
    }
}
