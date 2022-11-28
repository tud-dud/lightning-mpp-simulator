use crate::{
    payment::{Payment, PaymentShard},
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation, ID,
};

use log::{debug, error, info, trace};
use std::time::Instant;

impl Simulation {
    /// Attempts to send a payment until it fails.
    /// Unsuccessful payments are reversed immediately while we return the successful ones in case
    /// they should be reversed later
    pub(crate) fn send_one_payment(
        &mut self,
        payment: &mut Payment,
    ) -> (bool, Vec<(ID, String, usize)>) {
        let graph = self.graph.clone();
        let mut succeeded = false;
        let mut failed = false;
        let mut to_revert = Vec::new();
        // fail immediately if sender's balance on each of their edges < amount
        // Checked for single-path payments earlier already but the check is necessary here for
        // MPP.
        let max_out_balance = graph.get_max_node_balance(&payment.source);
        if max_out_balance < payment.amount_msat {
            error!("Payment shard failing. Sender does not have sufficient balance. Amount {}, max balance {}", payment.amount_msat, max_out_balance);
            failed = true;
        }
        let graph_copy = self.graph.clone();
        if !failed {
            let mut path_finder = PathFinder::new(
                payment.source.clone(),
                payment.dest.clone(),
                payment.amount_msat,
                &graph_copy,
                self.routing_metric,
                self.payment_parts,
            );
            path_finder.graph.edges = path_finder.remove_inadequate_edges(payment.amount_msat);
            while !succeeded && !failed {
                let start = Instant::now();
                if let Some(candidate_path) = path_finder.find_path() {
                    payment.paths = candidate_path.clone();
                    let duration_in_ms = start.elapsed().as_millis();
                    info!("Found path after {} ms.", duration_in_ms);
                    // maybe the sender's balance is not enough after we have discovered the full
                    // path's fees
                    let hops = candidate_path.path.hops.clone();
                    let (sender, out_channel) = (&hops[0].0, &hops[0].3);
                    if self.graph.get_channel_balance(sender, out_channel) < candidate_path.amount {
                        error!("Payment shard failing. Sender does not have sufficient balance to cover fees. Amount {}, max balance {}", candidate_path.amount, max_out_balance);
                        failed = true;
                    };
                    let mut payment_shard = payment.to_shard(payment.amount_msat);
                    payment_shard.attempts += 1;
                    (succeeded, to_revert) =
                        self.attempt_payment(&mut payment_shard, &candidate_path);
                    *payment = payment_shard.to_payment(1);
                    if !succeeded {
                        self.revert_payment(&to_revert);
                    }
                } else {
                    error!("No paths to destination found.");
                    succeeded = false;
                    failed = true;
                }
            }
        }
        if succeeded {
            (succeeded, to_revert)
        } else {
            (succeeded, Vec::new()) // the payments have already been reversed if the payment was
                                    // Unsuccessful hence there is nothing to do
        }
    }

    /// Tries to move the funds as is specified in the shard.
    /// This is the actual transaction
    pub(crate) fn attempt_payment(
        &mut self,
        mut payment_shard: &mut PaymentShard,
        candidate_path: &CandidatePath,
    ) -> (bool, Vec<(ID, String, usize)>) {
        let hops = candidate_path.path.hops.clone();
        info!(
            "{} attempting to send {} msats to {} via {} hops.",
            payment_shard.source,
            payment_shard.amount,
            payment_shard.dest,
            hops.len()
        );
        let mut remaining_transferable_amount = 0;
        // used in case we need to revert (node, channel_id, amount)
        let mut transferred_amounts: Vec<(ID, String, usize)> = Vec::new();
        for (idx, node) in hops.iter().enumerate() {
            let (id, fees, _timelock, channel_id) = node.clone();
            // Subtract payment amount (includes fees) from source
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
                        "Payment {} failed at source {} due to insufficient balance. available balamce {}, total amount {}",
                        payment_shard.payment_id, payment_shard.source, current_balance, candidate_path.amount,
                    );
                    payment_shard.succeeded = false;
                    return (payment_shard.succeeded, transferred_amounts);
                }
            } else if id == payment_shard.dest {
                // add remaining_amount to the node balance / or capacity
                // check if we have such an invoice and received amount matches
                // if yes: success = true
                //if remaining_transferable_amount == invoice
                match self.get_invoices_for_node(&id) {
                    Some(invoices) => {
                        if let Some(invoice) = invoices.get(&payment_shard.payment_id) {
                            if invoice.source == payment_shard.source {
                                //&&invoice.amount == remaining_transferable_amount
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
                                // necessary as we may reverse the payment if its part of an MPP
                                // payment
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
                                // revert here
                            }
                        }
                    }
                    None => {
                        error!(
                            "No invoice for payment {}. Failing at destination.",
                            payment_shard.payment_id
                        );
                        payment_shard.succeeded = false;
                    }
                };
            // a hop along the path
            } else {
                // subtract fee and add to own balance
                let current_balance = self.graph.get_channel_balance(&id, &channel_id);
                if current_balance > (remaining_transferable_amount - fees) {
                    self.graph
                        .update_channel_balance(&channel_id, current_balance + fees);
                    remaining_transferable_amount -= fees;
                    transferred_amounts.push((id, channel_id, fees));
                } else {
                    let src = &id;
                    let dest = hops[idx + 1].0.clone();
                    error!(
                        "Payment {} failing along the way to due to insufficient funds at {}.",
                        payment_shard.payment_id, id
                    );
                    trace!(
                        "Discarding channel {} between {} and {}",
                        channel_id,
                        src,
                        dest,
                    );
                    self.graph.remove_edge(src, &hops[idx - 1].0);
                    payment_shard.succeeded = false;
                    return (payment_shard.succeeded, transferred_amounts);
                }
            }
        }
        (payment_shard.succeeded, transferred_amounts)
    }

    /// Credits all edges in the path (Source gains whereas the rest lose)
    pub(crate) fn revert_payment(&mut self, amounts: &[(ID, String, usize)]) {
        let total: usize = amounts.iter().map(|t| t.2).sum::<usize>();
        debug!("Reverting msats {}.", total);
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

    pub fn init_sim(path: Option<String>) -> Simulation {
        let seed = 1;
        let amount = 1000;
        let pairs = 2;
        let mut graph = if let Some(file_path) = path {
            let file_path = std::path::Path::new(&file_path);
            Graph::to_sim_graph(&network_parser::from_json_file(&file_path).unwrap())
        } else {
            let path = std::path::Path::new("../test_data/lnbook_example.json");
            Graph::to_sim_graph(&network_parser::from_json_file(&path).unwrap())
        };
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
        let mut simulator = init_sim(None);
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
        let mut simulator = init_sim(None);
        let amount = 1000;
        let balance = 4711;
        simulator.add_invoice(Invoice::new(0, amount, &source, &dest));
        let graph = Box::new(simulator.graph.clone());
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            &graph,
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
        assert!(simulator.attempt_payment(payment_shard, &candidate_paths).0);
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
        let mut simulator = init_sim(None);
        let graph = Box::new(simulator.graph.clone());
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            &graph,
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
        let (success, transferred) = simulator.attempt_payment(payment_shard, &candidate_paths);
        simulator.revert_payment(&transferred);
        assert!(!success);
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
        let mut simulator = init_sim(None);
        let graph = simulator.graph.clone();
        simulator.graph.update_channel_balance(&channel_id, balance);
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            &graph,
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
        let (success, transferred) = simulator.attempt_payment(payment_shard, &candidate_paths);
        simulator.revert_payment(&transferred);
        assert!(!success);
        assert_eq!(
            simulator
                .graph
                .get_channel_balance(&"alice".to_string(), &"alice1".to_string()),
            0
        );
    }

    #[test]
    fn failing_edge_is_discarded() {
        let amount = 1000;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let channel_id = "bob2".to_string(); // channel from bob to chan
        let balance = 100;
        let mut simulator = init_sim(None);
        let graph = simulator.graph.clone();
        simulator.graph.update_channel_balance(&channel_id, balance);
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            &graph,
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
        assert!(!simulator.attempt_payment(payment_shard, &candidate_paths).0);
        assert!(!simulator
            .graph
            .get_edge(&String::from("alice"), &String::from("bob"))
            .is_some());
        // 0 because edges have been removed and get_balance returns 0 if edge is not found
        assert_eq!(
            simulator
                .graph
                .get_channel_balance(&"alice".to_string(), &"alice1".to_string()),
            0
        );
        assert_eq!(
            simulator
                .graph
                .get_channel_balance(&"bob".to_string(), &"bob1".to_string()),
            0
        );
    }

    #[test]
    #[ignore] // takes too long
    fn failing_channel_is_removed() {
        let seed = 2;
        let amount = 500000;
        let pairs = 1;
        let path = std::path::Path::new("../data/gossip-20210906_1000UTC.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(&path).unwrap());
        let routing_metric = RoutingMetric::MaxProb;
        let payment_parts = PaymentParts::Single;
        let mut simulator = Simulation::new(
            seed,
            graph.clone(),
            amount,
            pairs,
            routing_metric,
            payment_parts,
        );
        let source =
            "03c45cf25622ec07c56d13b7043e59c8c27ca822be58140b213edaea6849380349".to_string();
        let dest = "0329ae9a574b7120456d2ebf6626506e6a75255edd91ac4ea03ea008b9bad67bd2".to_string();
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat: amount,
            succeeded: false,
            min_shard_amt: 10,
            attempts: 0,
            num_parts: 1,
            paths: CandidatePath::default(),
            failed_amounts: Vec::default(),
        };
        simulator.add_invoice(Invoice::new(0, amount, &source, &dest));
        assert!(simulator.send_single_payment(payment));
    }

    #[ignore]
    fn payment_failure_max_channel_capacity() {
        let source = "alice".to_string();
        let hop = "bob".to_string();
        let dest = "chan".to_string();
        let mut simulator = init_sim(None);
        let graph = simulator.graph.clone();
        let channel_id = "bob2".to_string(); // channel from bob to chan
        let bob_balance = graph.get_channel_balance(&hop, &channel_id);
        let capacity = graph.get_edge(&hop, &dest).unwrap().htlc_maximum_msat;
        let amount = capacity - bob_balance;
        simulator.add_invoice(Invoice::new(0, amount, &source, &dest));
        let mut path_finder = PathFinder::new(
            source.clone(),
            dest.clone(),
            amount,
            &graph,
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
