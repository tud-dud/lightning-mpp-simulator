use crate::{payment::Payment, stats::Adversaries, AdversarySelection, Simulation, ID};

#[cfg(not(test))]
use log::{info, warn};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::{println as info, println as warn};

impl Simulation {
    pub(crate) fn eval_adversaries(&mut self) {
        info!("Starting adversary evaluation scenarios..");
        let fraction_of_adversaries = if let Some(percent) = self.fraction_of_adversaries {
            vec![percent]
        } else {
            (1..10).map(|v| v * 10).collect() // in percent
        };
        let mut all_payments = self.failed_payments.clone();
        all_payments.extend(self.successful_payments.clone());
        for percent in fraction_of_adversaries {
            let nodes = self.graph.get_node_ids();
            // safely round upwards
            let num_adv =
                (nodes.len() * percent) / 100 + (nodes.len() * percent % 100 != 0) as usize;
            let adversaries_results = Arc::new(Mutex::new(vec![]));
            self.adversary_selection.par_iter().for_each(|strategy| {
                info!(
                    "Starting adversary scenario: {:?} with {} nodes.",
                    strategy, num_adv
                );
                let adv: Vec<ID> = match strategy {
                    AdversarySelection::Random => {
                        Simulation::draw_adversaries(&nodes, num_adv).collect()
                    }
                    AdversarySelection::HighBetweenness(path)
                    | AdversarySelection::HighDegree(path) => {
                        match network_parser::read_node_rankings_from_file(&nodes, path.as_path()) {
                            Ok(mut scores) => {
                                scores.truncate(num_adv);
                                scores
                            }
                            Err(e) => {
                                warn!("No scores available {}. Proceeding with 0 adversaries.", e);
                                vec![]
                            }
                        }
                    }
                };
                let (hits, hits_successful) = Self::adversary_hits(&all_payments, &adv);
                // Don't run this analysis on all percentages
                //if percent % 20 == 0 {
                if percent % 20 == 11 {
                    self.deanonymise_tx_pairs(&adv);
                }
                adversaries_results.lock().unwrap().push(Adversaries {
                    selection_strategy: strategy.to_owned(),
                    percentage: percent,
                    hits,
                    hits_successful,
                    anonymits_sets: self.anonymity_sets.to_owned(),
                });
                info!(
                    "Completed adversary scenario: {:?} with {} nodes.",
                    strategy, num_adv
                );
            });

            self.adversaries
                .append(&mut adversaries_results.lock().unwrap());
        }
    }

    fn adversary_hits(payments: &[Payment], adv: &[ID]) -> (usize, usize) {
        let mut adversary_hits = 0;
        let mut adversary_hits_successful = 0;
        let mut count_occurences = |payments: &[Payment]| {
            'main: for payment in payments {
                let mut used_paths = payment.used_paths.to_owned();
                used_paths.extend(payment.failed_paths.to_owned());
                for path in used_paths.iter() {
                    if !path.path.path_contains_adversary(adv).is_empty() {
                        adversary_hits += 1;
                        if payment.succeeded {
                            adversary_hits_successful += 1;
                        }
                        // we know that this payment contains an adversary and don't need to
                        // look at all paths
                        continue 'main;
                    }
                }
            }
        };
        count_occurences(payments);
        (adversary_hits, adversary_hits_successful)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn adversary_hits() {
        let fraction_of_adversaries = 100; // all three nodes are adversaries
                                           // alice -> bob -> chan
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(fraction_of_adversaries));
        let sim_result = simulator.run(vec![(source, dest)].into_iter());
        assert_eq!(sim_result.num_succesful, 1);
        assert_eq!(simulator.adversaries[0].percentage, fraction_of_adversaries);
        assert_eq!(simulator.adversaries[0].hits, 1); // we only send one payment
        assert_eq!(simulator.adversaries[0].hits_successful, 1);
        let fraction_of_adversaries = 0;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(fraction_of_adversaries));
        let sim_result = simulator.run(vec![(source, dest)].into_iter());
        assert_eq!(sim_result.num_succesful, 1);
        simulator.eval_adversaries();
        assert_eq!(simulator.adversaries[0].percentage, fraction_of_adversaries);
        assert_eq!(simulator.adversaries[0].hits, 0); // we only send one payment
        assert_eq!(simulator.adversaries[0].hits_successful, 0);
    }
}
