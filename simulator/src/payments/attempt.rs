use crate::{
    payment::{Payment, PaymentShard},
    traversal::pathfinding::{CandidatePath, PathFinder},
    Simulation,
};

use log::{debug, error, info};
use std::time::Instant;

impl Simulation {
    // 2. Send payment (Try each path in order until payment succeeds (the trial-and-error loop))
    // 2.0. create payment
    // 2.1. try candidate paths sequentially (trial-and-error loop)
    // 2.2. record success or failure (where?)
    // 2.3. update states (node balances, ???)
    pub(crate) fn send_single_payment(&mut self, mut payment: &mut Payment) {
        let graph = Box::new(self.graph.clone());
        let mut succeeded = false;
        // fail immediately if sender's balance < amount
        if graph.get_max_edge_balance(&payment.source, &payment.dest) < payment.amount_msat {
            // TODO: immediate failure
            // abort attempt
        }
        let mut path_finder = PathFinder::new(
            payment.source.clone(),
            payment.dest.clone(),
            payment.amount_msat,
            graph,
            self.routing_metric,
            self.payment_parts,
        );
        let start = Instant::now();
        if let Some(candidate_paths) = path_finder.find_path() {
            payment.paths = candidate_paths.clone();
            let duration_in_ms = start.elapsed().as_millis();
            info!(
                "Found {} paths after {} ms.",
                candidate_paths.len(),
                duration_in_ms
            );
            let mut payment_shard = payment.to_shard(payment.amount_msat);
            for candidate_path in candidate_paths.iter() {
                if !succeeded {
                    succeeded = self.attempt_payment(&mut payment_shard, candidate_path);
                    if succeeded {
                        self.num_successesful += 1;
                        self.successful_payments.push(payment.to_owned());
                    } else {
                        // Payment failed
                    }
                }
            }
        } else {
            error!("No paths found.");
        }
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
        for node in hops {
            let (id, fees, timelock, channel_id) = node;
            // Subtract paymount amount (includes fees) from source
            if id == payment_shard.source {
                let current_balance = self.graph.get_channel_balance(&id, &channel_id);
                self.graph.update_channel_balance(
                    &id,
                    &channel_id,
                    current_balance - candidate_path.amount,
                );
                remaining_transferable_amount = candidate_path.amount;
            } else if id == payment_shard.dest {
                // add remaining_amount to the node balance / or capacity
                // check if we have such an invoice and received amount matches
                // if yes: success = true
                //if remaining_transferable_amount == invoice
                match self.get_invoices_for_node(&id) {
                    Some(invoices) => {
                        if let Some(invoice) = invoices.get(&payment_shard.payment_id) {
                            if invoice.amount == remaining_transferable_amount
                                && invoice.source == payment_shard.source
                            {
                                let current_balance =
                                    self.graph.get_channel_balance(&id, &channel_id);
                                self.graph.update_channel_balance(
                                    &id,
                                    &channel_id,
                                    current_balance + remaining_transferable_amount,
                                );
                                debug!(
                                    "Successfully delivered payment of {} msats from {} to {}",
                                    payment_shard.amount, payment_shard.source, payment_shard.dest
                                );
                                payment_shard.succeeded = true;
                            } else {
                                error!("Payment failure. Payment {:?}, remaining_amount {}, invoice {:?}", payment_shard, remaining_transferable_amount, invoice);
                                self.revert_payment()
                            }
                        }
                    }
                    None => self.revert_payment(),
                };
            } else {
                // subtract fee and add to own balance
                let current_balance = self.graph.get_channel_balance(&id, &channel_id);
                self.graph
                    .update_channel_balance(&id, &channel_id, current_balance + fees);
                remaining_transferable_amount -= fees;
            }
        }
        payment_shard.succeeded
    }

    pub(crate) fn revert_payment(&self) {
        todo!()
    }
}
