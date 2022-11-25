use crate::{
    payment::{Payment, PaymentShard},
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation, ID,
};

use log::{debug, error, info, trace};
use std::time::Instant;

impl Simulation {
    /// Attempts to send a payment until it fails
    pub(crate) fn send_one_payment(&mut self, payment: &mut Payment) -> bool {
        let graph = Box::new(self.graph.clone());
        let mut succeeded = false;
        let mut failed = false;
        // fail immediately if sender's balance on each of their edges < amount
        // Checked for single-path payments earlier already but the check is necessary here for
        // MPP.
        let max_out_balance = graph.get_max_node_balance(&payment.source);
        if max_out_balance < payment.amount_msat {
            error!("Payment shard failing. Sender has no edge with sufficient balance. Amount {}, max balance {}", payment.amount_msat, max_out_balance);
            failed = true;
        }
        if !failed {
            let mut path_finder = PathFinder::new(
                payment.source.clone(),
                payment.dest.clone(),
                payment.amount_msat,
                graph,
                self.routing_metric,
                self.payment_parts,
            );
            let start = Instant::now();
            while !succeeded && !failed {
                if let Some(candidate_path) = path_finder.find_path() {
                    payment.paths = candidate_path.clone();
                    let duration_in_ms = start.elapsed().as_millis();
                    info!("Found paths after {} ms.", duration_in_ms);
                    let mut payment_shard = payment.to_shard(payment.amount_msat);
                    payment_shard.attempts += 1;
                    succeeded = self.attempt_payment(&mut payment_shard, &candidate_path);
                    *payment = payment_shard.to_payment(1);
                } else {
                    error!("No paths to destination found.");
                    succeeded = false;
                    failed = true;
                }
            }
        }
        succeeded
    }

    /// Tries to move the funds as is specified in the shard.
    /// This is the actual transaction
    pub(crate) fn attempt_payment(
        &mut self,
        mut payment_shard: &mut PaymentShard,
        candidate_path: &CandidatePath,
    ) -> bool {
        let hops = candidate_path.path.hops.clone();
        info!(
            "{} attempting to send {} msats to {} via {} hops.",
            payment_shard.source,
            payment_shard.amount,
            payment_shard.dest,
            hops.len()
        );
        let mut remaining_transferable_amount = 0;
        // used in case we need to revert
        let mut transferred_amounts: Vec<(ID, String, usize)> = Vec::new();
        for (idx, node) in hops.iter().enumerate() {
            let (id, fees, timelock, channel_id) = node.clone();
            // Subtract paymount amount (includes fees) from source
            if id == payment_shard.source {
                let current_balance = self.graph.get_channel_balance(&id, &channel_id);
                if current_balance > candidate_path.amount {
                    self.graph.update_channel_balance(
                        &channel_id,
                        current_balance - candidate_path.amount,
                    );
                    remaining_transferable_amount = candidate_path.amount;
                    transferred_amounts.push((id, channel_id, remaining_transferable_amount));
                } else {
                    error!(
                        "Payment {:?} failed at source due to insufficient balance",
                        payment_shard
                    );
                    payment_shard.succeeded = false;
                    return payment_shard.succeeded;
                }
            } else if id == payment_shard.dest {
                // add remaining_amount to the node balance / or capacity
                // check if we have such an invoice and received amount matches
                // if yes: success = true
                //if remaining_transferable_amount == invoice
                match self.get_invoices_for_node(&id) {
                    Some(invoices) => {
                        if let Some(invoice) = invoices.get(&payment_shard.payment_id) {
                            // FIXME this will fail for MPP
                            if invoice.amount == remaining_transferable_amount
                                && invoice.source == payment_shard.source
                            {
                                let current_balance =
                                    self.graph.get_channel_balance(&id, &channel_id);
                                self.graph.update_channel_balance(
                                    &channel_id,
                                    current_balance + remaining_transferable_amount,
                                );
                                payment_shard.used_path = candidate_path.to_owned();
                                // TODO: remove invoice
                                info!(
                                    "Successfully delivered payment of {} msats from {} to {}.",
                                    payment_shard.amount, payment_shard.source, payment_shard.dest,
                                );
                                // not necessary as we won't be reversing the payment since we got
                                // this far
                                transferred_amounts.push((
                                    id,
                                    channel_id,
                                    remaining_transferable_amount,
                                ));
                                payment_shard.succeeded = true;
                                //TODO: fail if dest has all the channel capacity already (and
                                //check for all intermediate hops)
                            } else {
                                error!("Payment failure at destination. Payment {:?}, remaining_amount {}, invoice {:?}", payment_shard, remaining_transferable_amount, invoice);
                                payment_shard.succeeded = false;
                                self.revert_payment(&transferred_amounts)
                            }
                        }
                    }
                    None => {
                        payment_shard.succeeded = false;
                        self.revert_payment(&transferred_amounts)
                    }
                };
            // a hop along the path
            } else {
                // subtract fee and add to own balance
                let current_balance = self.graph.get_channel_balance(&id, &channel_id);
                if current_balance >= (remaining_transferable_amount - fees) {
                    self.graph
                        .update_channel_balance(&channel_id, current_balance + fees);
                    remaining_transferable_amount -= fees;
                    transferred_amounts.push((id, channel_id, fees));
                } else {
                    let src = &id;
                    let dest = hops[idx + 1].0.clone();
                    trace!(
                        "Discarding channel {} between {} and {}",
                        src,
                        dest,
                        channel_id
                    );
                    self.graph.remove_edge(src, &dest);
                    payment_shard.succeeded = false;
                    self.revert_payment(&transferred_amounts);
                    return payment_shard.succeeded;
                }
            }
        }
        payment_shard.succeeded
    }

    /// Credits all edges in the path (Source gains whereas the rest lose)
    fn revert_payment(&mut self, amounts: &[(ID, String, usize)]) {
        debug!("Reverting failed payment");
        for (idx, (node, channel_id, amt)) in amounts.iter().enumerate() {
            // source
            if idx == 0 {
                let current_balance = self.graph.get_channel_balance(node, channel_id);
                self.graph
                    .update_channel_balance(channel_id, current_balance + amt);
            } else {
                let current_balance = self.graph.get_channel_balance(node, channel_id);
                self.graph
                    .update_channel_balance(channel_id, current_balance - amt);
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use super::*;
    use crate::{core_types::graph::Graph, Invoice, PaymentParts, RoutingMetric};

    pub fn init_sim() -> Simulation {
        let seed = 1;
        let amount = 1000;
        let pairs = 2;
        let json_file = std::path::Path::new("../test_data/lnbook_example.json");
        let mut graph = Graph::to_sim_graph(&network_parser::from_json_file(&json_file).unwrap());
        let routing_metric = RoutingMetric::MinFee;
        let payment_parts = PaymentParts::Single;
        // set balances because of rng
        let balance = 4711;
        for edges in graph.edges.values_mut() {
            for e in edges {
                e.balance = balance;
            }
        }
        Simulation::new(
            seed,
            graph.clone(),
            amount,
            pairs,
            routing_metric,
            payment_parts,
        )
    }

    #[test]
    fn reverse_payment() {
        let balance = 4711;
        let mut simulator = init_sim();
        // failed payment from alice to chan
        let amounts_to_reverse = Vec::from([
            ("alice".to_string(), "alice1".to_string(), 130),
            ("bob".to_string(), "bob2".to_string(), 30),
            ("chan".to_string(), "chan1".to_string(), 100),
        ]);
        simulator.revert_payment(&amounts_to_reverse);
        // we can use get_edge since there are no parallel edges
        let expected = balance + 130;
        let actual = simulator
            .graph
            .get_channel_balance(&"alice".to_string(), &"alice1".to_string());
        assert_eq!(expected, actual);
        let expected = balance - 30;
        let actual = simulator
            .graph
            .get_channel_balance(&"bob".to_string(), &"bob2".to_string());
        assert_eq!(expected, actual);
        let expected = balance - 100;
        let actual = simulator
            .graph
            .get_channel_balance(&"chan".to_string(), &"chan1".to_string());
        assert_eq!(expected, actual);
    }

    #[test]
    fn payment_transfer_success() {
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = init_sim();
        let amount = 1000;
        let balance = 4711;
        simulator.add_invoice(Invoice::new(0, amount, &source, &dest));
        let graph = Box::new(simulator.graph.clone());
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            graph,
            RoutingMetric::MinFee,
            PaymentParts::Single,
        );
        let candidate_paths = path_finder.find_path().unwrap();
        let payment_shard = &mut PaymentShard {
            payment_id: 0,
            source,
            dest,
            amount,
            succeeded: true,
            used_path: candidate_paths.clone(),
            min_shard_amt: 10,
            attempts: 0,
        };
        assert!(simulator.attempt_payment(payment_shard, &candidate_paths));
        let expected = balance - 1100;
        let actual = simulator
            .graph
            .get_channel_balance(&"alice".to_string(), &"alice1".to_string());
        assert_eq!(expected, actual);
        let expected = balance + 100;
        let actual = simulator
            .graph
            .get_channel_balance(&"bob".to_string(), &"bob2".to_string());
        assert_eq!(expected, actual);
        let expected = balance + 1000;
        let actual = simulator
            .graph
            .get_channel_balance(&"chan".to_string(), &"chan1".to_string());
        assert_eq!(expected, actual);
    }

    #[test]
    // checking that balances are unaltered. Failure at the last node due to no invoice
    fn payment_failure_no_invoice() {
        let amount = 1000;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = init_sim();
        let graph = Box::new(simulator.graph.clone());
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            graph,
            RoutingMetric::MinFee,
            PaymentParts::Single,
        );
        let candidate_paths = path_finder.find_path().unwrap();
        let payment_shard = &mut PaymentShard {
            payment_id: 0,
            source,
            dest,
            amount,
            succeeded: true,
            used_path: candidate_paths.clone(),
            min_shard_amt: 10,
            attempts: 0,
        };
        assert!(!simulator.attempt_payment(payment_shard, &candidate_paths));
        for edges in simulator.graph.edges.values() {
            for e in edges {
                assert_eq!(e.balance, 4711);
            }
        }
    }

    #[test]
    // checking that balances are unaltered. Failure at the last node due to insufficient funds at
    // bob
    fn payment_failure_insufficient_funds() {
        let amount = 1000;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let channel_id = "bob2".to_string(); // channel from bob to chan
        let balance = 100;
        let mut simulator = init_sim();
        let graph = Box::new(simulator.graph.clone());
        simulator.graph.update_channel_balance(&channel_id, balance);
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            graph,
            RoutingMetric::MinFee,
            PaymentParts::Single,
        );
        let candidate_paths = path_finder.find_path().unwrap();
        let payment_shard = &mut PaymentShard {
            payment_id: 0,
            source,
            dest,
            amount,
            succeeded: false,
            used_path: candidate_paths.clone(),
            min_shard_amt: 10,
            attempts: 0,
        };
        assert!(!simulator.attempt_payment(payment_shard, &candidate_paths));
        assert_eq!(
            simulator
                .graph
                .get_channel_balance(&"alice".to_string(), &"alice1".to_string()),
            4711
        );
    }

    #[test]
    fn failing_edge_is_discarded() {
        let amount = 1000;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let channel_id = "bob2".to_string(); // channel from bob to chan
        let balance = 100;
        let mut simulator = init_sim();
        let graph = Box::new(simulator.graph.clone());
        simulator.graph.update_channel_balance(&channel_id, balance);
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            graph,
            RoutingMetric::MinFee,
            PaymentParts::Single,
        );
        let candidate_paths = path_finder.find_path().unwrap();
        let payment_shard = &mut PaymentShard {
            payment_id: 0,
            source,
            dest,
            amount,
            succeeded: false,
            used_path: candidate_paths.clone(),
            min_shard_amt: 10,
            attempts: 0,
        };
        assert!(!simulator.attempt_payment(payment_shard, &candidate_paths));
        assert!(!simulator
            .graph
            .get_edge(&String::from("bob"), &String::from("chan"))
            .is_some());
        // 0 because edges have been removed and get_balance returns 0 if edge is not found
        assert_eq!(
            simulator
                .graph
                .get_channel_balance(&"bob".to_string(), &"bob2".to_string()),
            0
        );
        assert_eq!(
            simulator
                .graph
                .get_channel_balance(&"chan".to_string(), &"chan1".to_string()),
            0
        );
    }
    #[ignore]
    fn payment_failure_max_channel_capacity() {
        let source = "alice".to_string();
        let hop = "bob".to_string();
        let dest = "chan".to_string();
        let mut simulator = init_sim();
        let graph = Box::new(simulator.graph.clone());
        let channel_id = "bob2".to_string(); // channel from bob to chan
        let bob_balance = graph.get_channel_balance(&hop, &channel_id);
        let capacity = graph.get_edge(&hop, &dest).unwrap().htlc_maximum_msat;
        let amount = capacity - bob_balance;
        simulator.add_invoice(Invoice::new(0, amount, &source, &dest));
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            graph,
            RoutingMetric::MinFee,
            PaymentParts::Single,
        );
        let candidate_paths = path_finder.find_path().unwrap();
        let payment_shard = &mut PaymentShard {
            payment_id: 0,
            source,
            dest,
            amount,
            succeeded: false,
            used_path: candidate_paths,
            min_shard_amt: 10,
            attempts: 0,
        };
    }
}
