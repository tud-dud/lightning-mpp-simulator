use crate::{
    payment::Payment,
    stats::{Adversaries, Statistics},
    AdversarySelection, Simulation, ID,
};

#[cfg(not(test))]
use log::{info, warn};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
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
        let (all_adversaries, chunks) = self.get_adversaries(&fraction_of_adversaries);
        let mut all_payments = self.failed_payments.clone();
        all_payments.extend(self.successful_payments.clone());
        let adversaries = Arc::new(Mutex::new(vec![]));
        self.adversary_selection.par_iter().for_each(|strategy| {
            let mut statistics: Vec<Statistics> = vec![];
            for i in 0..fraction_of_adversaries.len() {
                let percent = fraction_of_adversaries[i];
                let num_adv = chunks[i + 1];
                let adv = match all_adversaries.get(strategy) {
                    None => vec![],
                    Some(all_adversaries) => all_adversaries[chunks[i]..chunks[i + 1]].to_vec(),
                };
                info!(
                    "Starting adversary scenario: {} sat: {:?} with {} nodes.",
                    self.amount, strategy, num_adv,
                );
                let (hits, hits_successful) = if i == 0 {
                    Self::adversary_hits(&all_payments, &adv)
                } else {
                    let (hits, hits_successful) = Self::adversary_hits(&all_payments, &adv);
                    (
                        statistics[i - 1].hits + hits,
                        statistics[i - 1].hits_successful + hits_successful,
                    )
                };
                let anonymity_sets = if percent % 20 == 0 {
                    if i == 0 {
                        self.deanonymise_tx_pairs(&adv)
                    } else {
                        let mut anonymity_sets = self.deanonymise_tx_pairs(&adv);
                        anonymity_sets.extend(statistics[i - 1].anonymity_sets.clone());
                        info!(
                            "Completed anonymity sets for {:?}, {:?} of {} sat with {} {:?} adversaries.",
                            self.routing_metric, self.payment_parts, self.amount, num_adv, strategy,
                        );
                        anonymity_sets
                    }
                } else {
                    vec![]
                };
                let (attacks, attacked_successful_payments) = if i == 0 {
                    Self::count_adversaries_in_payments(&all_payments, &adv)
                } else {
                    let (mut attacks, mut attacked_successful_payments) =
                        Self::count_adversaries_in_payments(&all_payments, &adv);
                    for (k, v) in statistics[i - 1].attacked_all.iter() {
                        attacks.entry(*k).and_modify(|u| *u += v).or_insert(*v);
                    }
                    for (k, v) in statistics[i - 1].attacked_successful.iter() {
                        attacked_successful_payments
                            .entry(*k)
                            .and_modify(|u| *u += v)
                            .or_insert(*v);
                    }
                    (attacks, attacked_successful_payments)
                };
                statistics.push(Statistics {
                    percentage: percent,
                    hits,
                    hits_successful,
                    anonymity_sets,
                    attacked_all: attacks,
                    attacked_successful: attacked_successful_payments,
                });
                info!(
                    "Completed adversary scenario: {:?} with {} nodes and {} sat.",
                    strategy, num_adv, self.amount,
                );
            }
            adversaries.lock().unwrap().push(Adversaries {
                selection_strategy: strategy.clone(),
                statistics,
            });
        });
        if let Ok(arc) = Arc::try_unwrap(adversaries) {
            if let Ok(mutex) = arc.into_inner() {
                self.adversaries = mutex;
            }
        }
    }

    fn count_adversaries_in_payments(
        payments: &[Payment],
        adv: &[ID],
    ) -> (HashMap<usize, usize>, HashMap<usize, usize>) {
        info!("Counting adversary occurences in payments.");
        let mut attacked_payments: HashMap<usize, usize> = HashMap::new();
        let mut attacked_successful_payments = HashMap::new();
        for payment in payments {
            let mut used_paths = payment.used_paths.to_owned();
            used_paths.extend(payment.failed_paths.to_owned());
            let mut num_attacks = 0;
            for path in used_paths.iter() {
                num_attacks += path.path.path_contains_adversary(adv).len();
            }
            attacked_payments
                .entry(num_attacks)
                .and_modify(|occ| *occ += 1)
                .or_insert(1);
            if payment.succeeded {
                attacked_successful_payments
                    .entry(num_attacks)
                    .and_modify(|occ| *occ += 1)
                    .or_insert(1);
            }
        }
        info!("Completed counting adversary occurences in payments.");
        (attacked_payments, attacked_successful_payments)
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

    fn get_adversaries(
        &self,
        fraction_of_adversaries: &[usize],
    ) -> (HashMap<AdversarySelection, Vec<ID>>, Vec<usize>) {
        let nodes = self.graph.get_node_ids();
        let max_num_adversaries =
            (nodes.len() * fraction_of_adversaries[fraction_of_adversaries.len() - 1]) / 100
                + (nodes.len() * (fraction_of_adversaries.len() - 1) % 100 != 0) as usize;
        let mut chunks = vec![0];
        let mut all_adversaries: HashMap<AdversarySelection, Vec<ID>> = HashMap::new();
        for strategy in self.adversary_selection.iter() {
            let adv: Vec<ID> = match strategy {
                AdversarySelection::Random => {
                    Simulation::draw_adversaries(&nodes, max_num_adversaries).collect()
                }
                AdversarySelection::HighBetweenness(path)
                | AdversarySelection::HighDegree(path) => {
                    match network_parser::read_node_rankings_from_file(&nodes, path.as_path()) {
                        Ok(mut scores) => {
                            scores.truncate(max_num_adversaries);
                            scores
                        }
                        Err(e) => {
                            warn!("No scores available {}. Proceeding with 0 adversaries.", e);
                            vec![]
                        }
                    }
                }
            };
            all_adversaries.insert(strategy.clone(), adv);
        }
        for percent in fraction_of_adversaries {
            // safely round downwards
            chunks.push((nodes.len() * percent) / 100);
        }
        (all_adversaries, chunks)
    }
}

#[cfg(test)]
mod tests {

    use crate::AdversarySelection;

    #[test]
    fn adversary_hits() {
        let fraction_of_adversaries = 100; // all three nodes are adversaries
                                           // alice -> bob -> chan
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(fraction_of_adversaries));
        let sim_result = simulator.run(vec![(source, dest)].into_iter());
        assert_eq!(sim_result.num_succesful, 1);
        let statistics = &simulator.adversaries[0].statistics;
        assert_eq!(
            simulator.adversaries[0].selection_strategy,
            AdversarySelection::Random
        );
        assert_eq!(statistics[0].percentage, fraction_of_adversaries);
        assert_eq!(statistics[0].hits, 1); // we only send one payment
        assert_eq!(statistics[0].hits_successful, 1);
        let fraction_of_adversaries = 0;
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(fraction_of_adversaries));
        let sim_result = simulator.run(vec![(source, dest)].into_iter());
        assert_eq!(sim_result.num_succesful, 1);
        let statistics = &simulator.adversaries[0].statistics;
        assert_eq!(statistics[0].percentage, fraction_of_adversaries);
        assert_eq!(statistics[0].hits, 0); // we only send one payment
        assert_eq!(statistics[0].hits_successful, 0);
    }
}
