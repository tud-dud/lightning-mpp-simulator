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
    use crate::Invoice;

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
