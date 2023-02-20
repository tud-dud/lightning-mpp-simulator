use crate::{
    payment::{Payment, PaymentShard},
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation, ID,
};

#[cfg(not(test))]
use log::{debug, error, info, trace};
use std::time::Instant;
#[cfg(test)]
use std::{println as info, println as debug, println as error, println as trace};

impl Simulation {
    /// attempts to send a payment until it fails.
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
            error!("Payment shard failing. Sender {} does not have sufficient balance. Amount {}, max balance {}",  payment.source, payment.amount_msat, max_out_balance);
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
            path_finder.graph.edges =
                PathFinder::remove_inadequate_edges(&graph_copy, payment.amount_msat);
            while !succeeded && !failed {
                let start = Instant::now();
                if let Some(candidate_path) = path_finder.find_path() {
                    let duration_in_ms = start.elapsed().as_millis();
                    info!("Found path after {} ms.", duration_in_ms);
                    let hops = candidate_path.path.hops.clone();
                    for hop in hops.iter().take(hops.len() - 1).skip(1) {
                        // not source and dest
                        let id = hop.0.clone();
                        self.node_hits
                            .entry(id)
                            .and_modify(|occurences| *occurences += 1)
                            .or_insert(1);
                    }
                    // maybe the sender's balance is not enough after we have discovered the full
                    // path's fees
                    let (sender, out_channel) = (&hops[0].0, &hops[0].3);
                    let channel_balance = self.graph.get_channel_balance(sender, out_channel);
                    if channel_balance < candidate_path.amount {
                        error!("Payment shard failing. Sender does not have sufficient balance to cover fees. Amount {}, channel balance {}", candidate_path.amount, channel_balance);
                        succeeded = false;
                        failed = true;
                    }
                    // edge's receive capacity not sufficient?
                    let receive_channel = &hops[hops.len() - 1].3;
                    if !self
                        .graph
                        .channel_can_receive_amount(receive_channel, payment.amount_msat)
                    {
                        error!(
                            "Payment {} of {} msat failing at destination due to max capacity. Not trying to deliver..",
                            payment.payment_id, payment.amount_msat
                        );
                        succeeded = false;
                        failed = true;
                    }
                    if !failed {
                        let mut payment_shard = payment.to_shard(payment.amount_msat);
                        (succeeded, to_revert) = self.attempt_payment(
                            &mut payment_shard,
                            &candidate_path,
                            &mut path_finder,
                        );
                        *payment = payment_shard.to_payment(1);
                        if !succeeded {
                            self.revert_payment(&to_revert);
                        }
                    }
                    // note paths that were attempted but failed for some reason
                    if failed || !succeeded {
                        payment.failed_paths.push(candidate_path);
                        payment.used_paths.clear();
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
        path_finder: &mut PathFinder,
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
                    payment_shard.htlc_attempts += 1;
                } else {
                    error!(
                        "Payment {} failed at source {} due to insufficient balance. available balamce {}, total amount {}",
                        payment_shard.payment_id, payment_shard.source, current_balance, candidate_path.amount,
                    );
                    payment_shard.htlc_attempts += 1;
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

                                // receiver would exceed channel capacity - should never get this
                                // far as we check before attempting
                                if !self.graph.channel_can_receive_amount(
                                    &channel_id,
                                    remaining_transferable_amount,
                                ) {
                                    error!(
                                        "Payment {} failing at destination due to max capacity.",
                                        payment_shard.payment_id
                                    );
                                    payment_shard.succeeded = false;
                                    let src = &id;
                                    let dest = hops[idx - 1].0.clone();
                                    // this is the failing edge
                                    trace!("Discarding channel {} due to max capacity", channel_id,);
                                    path_finder.graph.remove_channel(&channel_id);
                                    path_finder.graph.remove_edge(src, &dest);
                                } else {
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
                                        payment_shard.amount,
                                        payment_shard.source,
                                        payment_shard.dest,
                                    );
                                    // necessary as we may reverse the payment if its part of an MPP
                                    // payment
                                    transferred_amounts.push((
                                        id,
                                        channel_id,
                                        remaining_transferable_amount,
                                    ));
                                    payment_shard.succeeded = true;
                                }
                            } else {
                                error!("Payment failure at destination (no invoice). Payment {:?}, remaining_amount {}, invoice {:?}", payment_shard, remaining_transferable_amount, invoice);
                                payment_shard.succeeded = false;
                            }
                        }
                    }
                    None => {
                        error!(
                            "No invoice for payment {}. Failing at destination.",
                            payment_shard.payment_id
                        );
                        // we remove the edge because we otherwise risk running into an endless
                        // loop
                        let src = &id;
                        path_finder.graph.remove_channel(&channel_id);
                        path_finder.graph.remove_edge(src, &hops[idx - 1].0);
                        payment_shard.succeeded = false;
                    }
                };
            // a hop along the path
            } else {
                payment_shard.htlc_attempts += 1;
                // subtract fee and add to own balance
                let current_balance = self.graph.get_channel_balance(&id, &channel_id);
                if current_balance > (remaining_transferable_amount - fees)
                    && self
                        .graph
                        .channel_can_receive_amount(&channel_id, remaining_transferable_amount)
                {
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
                    // this is the failing edge
                    path_finder.graph.remove_channel(&channel_id);
                    path_finder.graph.remove_edge(src, &hops[idx - 1].0);
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
        debug!("Reverting {} msat.", total);
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
    use crate::{
        core_types::graph::Graph, AdversarySelection, Invoice, PaymentParts, RoutingMetric,
    };

    pub fn init_sim(path: Option<String>, number_of_adversaries: Option<usize>) -> Simulation {
        let seed = 0;
        let amount = 1000;
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
        let adversary_selection = vec![AdversarySelection::Random];
        Simulation::new(
            seed,
            graph.clone(),
            amount,
            routing_metric,
            payment_parts,
            number_of_adversaries,
            &adversary_selection,
        )
    }

    #[test]
    fn reverse_payment() {
        let balance = 4711;
        let mut simulator = init_sim(None, None);
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
        let mut simulator = init_sim(None, None);
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
            htlc_attempts: 0,
            failed_paths: vec![],
        };
        assert!(
            simulator
                .attempt_payment(payment_shard, &candidate_paths, &mut path_finder)
                .0
        );
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
        let mut simulator = init_sim(None, None);
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
            htlc_attempts: 0,
            failed_paths: vec![],
        };
        let (success, transferred) =
            simulator.attempt_payment(payment_shard, &candidate_paths, &mut path_finder);
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
        let mut simulator = init_sim(None, None);
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
            htlc_attempts: 0,
            failed_paths: vec![],
        };
        let (success, transferred) =
            simulator.attempt_payment(payment_shard, &candidate_paths, &mut path_finder);
        simulator.revert_payment(&transferred);
        assert!(!success);
        assert_eq!(
            path_finder
                .graph
                .get_channel_balance(&"alice".to_string(), &"alice1".to_string()),
            0
        );
    }

    #[test]
    fn failing_edge_is_not_discarded_from_sim() {
        let amount = 1000;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let channel_id = "bob2".to_string(); // channel from bob to chan
        let balance = 100;
        let mut simulator = init_sim(None, None);
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
            htlc_attempts: 0,
            failed_paths: vec![],
        };
        assert!(
            !simulator
                .attempt_payment(payment_shard, &candidate_paths, &mut path_finder)
                .0
        );
        // edge is still there for future payments
        assert!(simulator
            .graph
            .get_edge(&String::from("alice"), &String::from("bob"))
            .is_some());
        assert!(!path_finder
            .graph
            .get_edge(&String::from("alice"), &String::from("bob"))
            .is_some());
        // 0 because edges have been removed and get_balance returns 0 if edge is not found
        assert_eq!(
            path_finder
                .graph
                .get_channel_balance(&"alice".to_string(), &"alice1".to_string()),
            0
        );
        assert_eq!(
            path_finder
                .graph
                .get_channel_balance(&"bob".to_string(), &"bob1".to_string()),
            0
        );
    }

    #[test]
    #[ignore] // takes too long
    fn failing_channel_is_removed() {
        let seed = 0;
        let amount = 500000;
        let path = std::path::Path::new("../data/gossip-20210906_1000UTC.json");
        let graph = Graph::to_sim_graph(&network_parser::from_json_file(&path).unwrap());
        let routing_metric = RoutingMetric::MaxProb;
        let payment_parts = PaymentParts::Single;
        let adversary_selection = vec![AdversarySelection::Random];
        let mut simulator = Simulation::new(
            seed,
            graph.clone(),
            amount,
            routing_metric,
            payment_parts,
            Some(0),
            &adversary_selection,
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
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
            failed_paths: vec![],
        };
        simulator.add_invoice(Invoice::new(0, amount, &source, &dest));
        assert!(simulator.send_single_payment(payment));
    }

    #[test]
    fn payment_failure_max_channel_capacity() {
        let source = "alice".to_string();
        let hop = "bob".to_string();
        let dest = "chan".to_string();
        let mut simulator = init_sim(None, None);
        let graph = simulator.graph.clone();
        let capacity = graph.get_edge(&hop, &dest).unwrap().capacity;
        let amount = capacity * 2;
        simulator.add_invoice(Invoice::new(0, amount, &source, &dest));
        let payment = &mut Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat: amount,
            succeeded: false,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 1,
            used_paths: Vec::default(),
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
            failed_paths: vec![],
        };
        assert!(!simulator.send_single_payment(payment));
    }
}
