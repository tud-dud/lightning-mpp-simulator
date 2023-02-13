use crate::{
    event::*, payment::Payment, stats::TargetedAttack, time::Time, Invoice, PaymentParts,
    Simulation, ID,
};

use itertools::EitherOrBoth::{Both, Left, Right};
use itertools::Itertools;
#[cfg(not(test))]
use log::{debug, info};
#[cfg(test)]
use std::{println as info, println as debug};

impl Simulation {
    pub(crate) fn run_scenario_simulator(&self, targets: &[ID]) -> TargetedAttack {
        info!(
            "Simulating targeted node attacks for {:?}, {:?} of {} sats.",
            self.routing_metric, self.payment_parts, self.amount
        );
        let mut sim = self.clone();
        sim.delete_targets(targets);
        let pp = sim.reconstruct_payment_pairs();
        sim.failed_payments = vec![];
        sim.successful_payments = vec![];
        sim.event_queue = EventQueue::new();
        sim.total_num_payments = pp.size_hint().0;
        assert_eq!(sim.payment_parts, self.payment_parts);
        assert_eq!(sim.routing_metric, self.routing_metric);
        sim.simulate(pp)
    }

    fn simulate(&mut self, payment_pairs: impl Iterator<Item = (ID, ID)>) -> TargetedAttack {
        info!(
            "# Payment pairs = {}, Pathfinding weight = {:?}, Single/MMP payments: {:?}",
            payment_pairs.size_hint().0,
            self.routing_metric,
            self.payment_parts
        );
        let mut now = Time::from_secs(0.0); // start simulation at (0)
        for (src, dest) in payment_pairs {
            let payment_id = self.next_payment_id();
            let invoice = Invoice::new(payment_id, self.amount, &src, &dest);
            self.add_invoice(invoice);
            let payment = Payment::new(payment_id, src, dest, self.amount);
            let event = PaymentEvent::Scheduled { payment };
            self.event_queue.schedule(now, event);
            now += Time::from_secs(crate::SIM_DELAY_IN_SECS);
        }
        self.total_num_payments = self.event_queue.queue_length();
        debug!(
            "Queued {} events for simulation.",
            self.event_queue.queue_length()
        );

        info!("Starting simulation.");
        // this is where the actual simulation happens
        while let Some(event) = self.event_queue.next() {
            match event {
                PaymentEvent::Scheduled { mut payment } => {
                    debug!(
                        "Dispatching scheduled payment {} at simulation time = {}.",
                        payment.payment_id,
                        self.event_queue.now()
                    );
                    let _ = match self.payment_parts {
                        PaymentParts::Single => self.send_single_payment(&mut payment),
                        PaymentParts::Split => self.send_mpp_payment(&mut payment),
                    };
                }
                PaymentEvent::UpdateFailed { payment: _ } => {
                    self.num_failed += 1;
                }
                PaymentEvent::UpdateSuccesful { payment: _ } => {
                    self.num_successful += 1;
                }
            }
        }
        info!("Completed simulation of targeted attacks.");
        TargetedAttack {
            num_successful: self.num_successful,
            num_failed: self.num_failed,
        }
    }

    fn delete_targets(&mut self, targets: &[ID]) {
        for node in targets {
            self.graph.remove_node(node);
        }
    }

    fn reconstruct_payment_pairs(&self) -> (impl Iterator<Item = (ID, ID)> + Clone) {
        let mut payment_pairs = vec![];
        for payments_iter in self
            .successful_payments
            .iter()
            .zip_longest(self.failed_payments.iter())
        {
            let mut check_and_add_payment = |payment: &Payment| {
                if self.graph.node_is_in_graph(&payment.source)
                    && self.graph.node_is_in_graph(&payment.dest)
                {
                    payment_pairs.push((payment.source.clone(), payment.dest.clone()));
                }
            };
            match payments_iter {
                Both(s, f) => {
                    check_and_add_payment(s);
                    check_and_add_payment(f);
                }
                Left(s) => check_and_add_payment(s),
                Right(f) => check_and_add_payment(f),
            }
        }
        info!(
            "Reusing {} % of payment pairs.",
            (payment_pairs.len() as f32 / self.total_num_payments as f32) * 100.0
        );
        payment_pairs.into_iter()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn delete_targets() {
        let mut simulator = crate::attempt::tests::init_sim(None, None);
        let targets = ["bob".to_string()];
        assert_eq!(simulator.graph.node_count(), 4);
        simulator.delete_targets(&targets);
        assert_eq!(simulator.graph.node_count(), 3);
    }

    #[test]
    fn payment_pairs() {
        let mut simulator = crate::attempt::tests::init_sim(None, None);
        simulator.successful_payments = vec![
            Payment {
                payment_id: 0,
                source: "alice".to_string(),
                dest: "bob".to_string(),
                ..Default::default()
            },
            Payment {
                payment_id: 0,
                source: "dina".to_string(),
                dest: "alice".to_string(),
                ..Default::default()
            },
        ];
        simulator.failed_payments = vec![Payment {
            payment_id: 0,
            source: "alice".to_string(),
            dest: "chan".to_string(),
            ..Default::default()
        }];
        let targets = ["bob".to_string()];
        simulator.delete_targets(&targets);
        let expected = [
            ("alice".to_string(), "chan".to_string()),
            ("dina".to_string(), "alice".to_string()),
        ]
        .into_iter();
        let actual = simulator.reconstruct_payment_pairs();
        assert!(expected.eq(actual));
    }

    #[test]
    fn run() {
        let mut simulator = crate::attempt::tests::init_sim(None, None);
        simulator.successful_payments = vec![
            Payment {
                payment_id: 0,
                source: "alice".to_string(),
                dest: "bob".to_string(),
                ..Default::default()
            },
            Payment {
                payment_id: 0,
                source: "dina".to_string(),
                dest: "chan".to_string(),
                ..Default::default()
            },
        ];
        simulator.failed_payments = vec![Payment {
            payment_id: 0,
            source: "chan".to_string(),
            dest: "dina".to_string(),
            ..Default::default()
        }];
        let targets = ["bob".to_string()];
        let actual = simulator.run_scenario_simulator(&targets);
        let expected = TargetedAttack {
            // dina <-> chan
            num_succesful: 2,
            num_failed: 0,
        };
        assert_eq!(actual, expected);
        let targets = ["bob".to_string(), "chan".to_string()];
        let actual = simulator.run_scenario_simulator(&targets);
        let expected = TargetedAttack {
            num_succesful: 0,
            num_failed: 0,
        };
        assert_eq!(actual, expected);
    }
}
