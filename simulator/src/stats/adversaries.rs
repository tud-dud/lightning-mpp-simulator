use crate::{
    payment::Payment,
    stats::{Adversaries, Statistics},
    AdversarySelection, PaymentId, Simulation, ID,
};

#[cfg(not(test))]
use log::{info, warn};
use rayon::prelude::*;
use std::{
    collections::{hash_map::Entry, HashMap},
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
            (1..10).collect() // in percent
        };
        let (all_adversaries, chunks) = self.get_adversaries(&fraction_of_adversaries);
        let mut all_payments = self.failed_payments.clone();
        all_payments.extend(self.successful_payments.clone());
        let adversaries = Arc::new(Mutex::new(vec![]));
        self.adversary_selection.par_iter().for_each(|strategy| {
            let mut statistics: Vec<Statistics> = vec![];
            let mut attacked_payments: HashMap<PaymentId, usize> = HashMap::new();
            let mut attacked_successful_payments: HashMap<PaymentId, usize> = HashMap::new();
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
                let targeted_attack = self.rerun_simulation(&adv);
                Self::adversary_hits(&mut attacked_payments, &mut attacked_successful_payments, &all_payments, &adv);
                let (hits, hits_successful) =
                    (attacked_payments.iter().filter(|e| *e.1 != 0).count(), attacked_successful_payments.iter().filter(|e| *e.1 != 0).count());
                let mut num_attacks = HashMap::new();
                let mut num_attacks_successful = HashMap::new();
                // (num attacks, num of payments)
                for v in attacked_payments.values() {
                     num_attacks
                    .entry(*v)
                    .and_modify(|occ|
                        *occ += 1
                        )
                    .or_insert(1);
                }
                for v in attacked_successful_payments.values() {
                     num_attacks_successful
                    .entry(*v)
                    .and_modify(|occ|
                        *occ += 1
                        )
                    .or_insert(1);
                }
                info!("Completed counting adversary occurences in payments.");
                let anonymity_sets = if percent % 11 == 0 {
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
                statistics.push(Statistics {
                    percentage: percent,
                    hits,
                    hits_successful,
                    anonymity_sets,
                    attacked_all: num_attacks,
                    attacked_successful: num_attacks_successful,
                    targeted_attack,
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

    /// Count how many adversaries are included in a payment's path
    /// MPP payment parts are considered jointly
    fn adversary_hits(
        hits_all_payments: &mut HashMap<PaymentId, usize>,
        hits_successful_payments: &mut HashMap<PaymentId, usize>,
        payments: &[Payment],
        adv: &[ID],
    ) {
        for payment in payments {
            let mut used_paths = payment.used_paths.to_owned();
            used_paths.extend(payment.failed_paths.to_owned());
            for path in used_paths.iter() {
                let num_adv = path.path.path_contains_adversary(adv).len();
                if let Entry::Occupied(mut o) = hits_all_payments.entry(payment.payment_id) {
                    *o.get_mut() += num_adv;
                } else {
                    hits_all_payments.insert(payment.payment_id, num_adv);
                }
                if payment.succeeded {
                    if let Entry::Occupied(mut o) =
                        hits_successful_payments.entry(payment.payment_id)
                    {
                        *o.get_mut() += num_adv;
                    } else {
                        hits_successful_payments.insert(payment.payment_id, num_adv);
                    }
                }
            }
        }
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

    use super::*;
    use crate::AdversarySelection;

    #[test]
    fn adversary_hits() {
        let fraction_of_adversaries = 100; // all three nodes are adversaries
                                           // alice -> bob -> chan
        let source = "alice".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(fraction_of_adversaries));
        let sim_result = simulator.run(
            vec![
                (source.clone(), "chan".to_string()),
                (source.clone(), "dina".to_string()),
                (source, "dina".to_string()),
            ]
            .into_iter(),
        );
        assert_eq!(sim_result.num_succesful, 3);
        let statistics = &simulator.adversaries[0].statistics;
        assert_eq!(
            simulator.adversaries[0].selection_strategy,
            AdversarySelection::Random
        );
        assert_eq!(statistics[0].percentage, fraction_of_adversaries);
        assert_eq!(statistics[0].hits, 3); // we send three payments
        assert_eq!(statistics[0].hits_successful, 3);
        let num_attacks = [(1, 1), (2, 2)];
        let attacked_all = statistics[0].attacked_all.clone();
        let attacked_successful = statistics[0].attacked_successful.clone();
        assert_eq!(attacked_all.len(), num_attacks.len());
        assert_eq!(attacked_successful.len(), num_attacks.len());
        for k in attacked_all {
            assert!(num_attacks.contains(&k));
        }
        for k in attacked_successful {
            assert!(num_attacks.contains(&k));
        }
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
        assert_eq!(statistics[0].attacked_all, HashMap::from([(0, 1)])); // 1 payment attacked 0
                                                                         // times
        assert_eq!(statistics[0].attacked_successful, HashMap::from([(0, 1)]));
    }
}
