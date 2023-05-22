use crate::{
    payment::Payment,
    stats::{Adversaries, Statistics, TargetedAttack},
    AdversarySelection, Simulation, ID,
};

#[cfg(not(test))]
use log::{error, info, warn};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
#[cfg(test)]
use std::{println as info, println as warn, println as error};

impl Simulation {
    pub(crate) fn eval_adversaries(&mut self, run_all: bool) {
        info!("Starting adversary evaluation scenarios..");
        let number_of_adversaries =
            if let Some(mut number_of_adversaries) = self.number_of_adversaries.clone() {
                number_of_adversaries.sort();
                number_of_adversaries
            } else {
                vec![1, 2, 3, 4, 5, 10, 12, 15, 20]
            };
        if !number_of_adversaries.is_empty() && self.adversary_selection.is_empty() {
            error!("Aborting adversary evaluation as no strategy was passed.");
            return;
        }
        let selected_adversaries =
            self.get_adversaries(number_of_adversaries[number_of_adversaries.len() - 1]);
        let mut all_payments = self.failed_payments.clone();
        all_payments.extend(self.successful_payments.clone());
        let adversaries = Arc::new(Mutex::new(vec![]));
        self.adversary_selection.par_iter().for_each(|strategy| {
            let mut statistics: Vec<Statistics> = vec![];
            for (idx, num_adv) in number_of_adversaries.iter().enumerate() {
                let adv = match selected_adversaries.get(strategy) {
                    None => vec![],
                    Some(selected_adversaries) => selected_adversaries[0..*num_adv].to_vec(),
                };
                info!(
                    "Starting adversary scenario: {} sat: {:?} with {} nodes.",
                    self.amount, strategy, num_adv,
                );
                let (hits, parts_hits, payment_attacks) = Self::adversary_hits(&all_payments, &adv);
                let (adv_count, adv_count_successful) = if idx == 0 {
                    payment_attacks
                } else {
                    let (mut attacks, mut attacks_successful) = payment_attacks.clone();
                    for (k, v) in statistics[idx - 1].adv_count.iter() {
                        attacks.entry(*k).and_modify(|u| *u += v).or_insert(*v);
                    }
                    for (k, v) in statistics[idx - 1].adv_count_successful.iter() {
                        attacks_successful
                            .entry(*k)
                            .and_modify(|u| *u += v)
                            .or_insert(*v);
                    }
                    (attacks, attacks_successful)
                };
                info!("Completed counting adversary occurences in payments.");
                /*let anonymity_sets = if let Some(adversary) = adv.last() {
                        let set = self.deanonymise_tx_pairs(adversary);
                        info!(
                            "Completed anonymity sets for {:?}, {:?} of {} sat with {} {:?} adversaries.",
                            self.routing_metric, self.payment_parts, self.amount, num_adv, strategy,
                        );
                        set
                } else {
                    vec![]
                };*/
                let anonymity_sets = vec![];
                let targeted_attack = if run_all {
                    self.rerun_simulation(&adv)
                } else {
                    TargetedAttack::default()
                };
                statistics.push(Statistics {
                    number: *num_adv,
                    hits: hits.0,
                    hits_successful: hits.1,
                    anonymity_sets,
                    targeted_attack,
                    part_hits: parts_hits.0,
                    part_hits_successful: parts_hits.1,
                    adv_count,
                    adv_count_successful,
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
    #[allow(clippy::type_complexity)]
    fn adversary_hits(
        payments: &[Payment],
        adv: &[ID],
    ) -> (
        (usize, usize),
        (usize, usize),
        (HashMap<usize, usize>, HashMap<usize, usize>),
    ) {
        let mut hits = 0;
        let mut hits_successful = 0;
        let mut part_hits = 0;
        let mut part_hits_successful = 0;
        let mut adv_count: HashMap<usize, usize> = HashMap::default();
        let mut adv_count_successful: HashMap<usize, usize> = HashMap::default();
        let mut contains_an_adversary = |payment: &Payment| {
            let mut all_paths = payment.used_paths.to_owned();
            all_paths.extend(payment.failed_paths.to_owned());
            for path in all_paths.iter() {
                if !path.path.path_contains_adversary(adv).is_empty() {
                    hits += 1;
                    if payment.succeeded {
                        hits_successful += 1;
                    }
                    continue;
                }
            }
        };
        for payment in payments {
            contains_an_adversary(payment);
            let mut used_paths = payment.used_paths.to_owned();
            used_paths.extend(payment.failed_paths.to_owned());
            let mut num_attacks = 0;
            for path in used_paths.iter() {
                let num_adv = path.path.path_contains_adversary(adv);
                if !num_adv.is_empty() {
                    part_hits += 1;
                    if payment.succeeded {
                        part_hits_successful += 1;
                    }
                }
                num_attacks += num_adv.len();
                adv_count
                    .entry(num_attacks)
                    .and_modify(|occ| *occ += 1)
                    .or_insert(1);
                if payment.succeeded {
                    adv_count_successful
                        .entry(num_attacks)
                        .and_modify(|occ| *occ += 1)
                        .or_insert(1);
                }
            }
        }
        (
            (hits, hits_successful),
            (part_hits, part_hits_successful),
            (adv_count, adv_count_successful),
        )
    }

    fn get_adversaries(
        &self,
        number_of_adversaries: usize,
    ) -> HashMap<AdversarySelection, Vec<ID>> {
        let nodes = self.graph.get_node_ids();
        let mut all_adversaries: HashMap<AdversarySelection, Vec<ID>> = HashMap::new();
        for strategy in self.adversary_selection.iter() {
            let adv: Vec<ID> = match strategy {
                AdversarySelection::Random => {
                    Simulation::draw_adversaries(&nodes, number_of_adversaries).collect()
                }
                AdversarySelection::HighBetweenness(path)
                | AdversarySelection::HighDegree(path) => {
                    match network_parser::read_node_rankings_from_file(&nodes, path.as_path()) {
                        Ok(scores) => scores[0..number_of_adversaries].to_owned(),
                        Err(e) => {
                            warn!("No scores available {}. Proceeding with 0 adversaries.", e);
                            vec![]
                        }
                    }
                }
                AdversarySelection::HighBetweennessWeb(ranking) => {
                    ranking[0..number_of_adversaries].to_owned()
                }
            };
            all_adversaries.insert(strategy.clone(), adv);
        }
        all_adversaries
    }
}

#[cfg(test)]
mod tests {

    use crate::AdversarySelection;
    use std::collections::HashMap;

    #[test]
    fn adversary_hits() {
        let number_of_adversaries = 4; // all four nodes are adversaries
                                       // alice -> bob -> chan
        let source = "alice".to_string();
        let mut simulator =
            crate::attempt::tests::init_sim(None, Some(vec![number_of_adversaries]));
        let sim_result = simulator.run(
            vec![
                (source.clone(), "chan".to_string()),
                (source.clone(), "dina".to_string()),
            ]
            .into_iter(),
            None,
            true,
        );
        assert_eq!(sim_result.num_succesful, 2);
        let statistics = &simulator.adversaries[0].statistics;
        assert_eq!(
            simulator.adversaries[0].selection_strategy,
            AdversarySelection::Random
        );
        assert_eq!(statistics[0].number, number_of_adversaries);
        assert_eq!(statistics[0].hits, 2); // we send two payments
        assert_eq!(statistics[0].hits_successful, 2);
        assert_eq!(statistics[0].part_hits, 2); // we send two payments as single payments
        assert_eq!(statistics[0].part_hits_successful, 2);
        assert_eq!(statistics[0].targeted_attack.num_successful, 0);
        assert_eq!(statistics[0].targeted_attack.num_failed, 0);
        let expected_adv_count = HashMap::from([(1, 1), (2, 1)]);
        assert_eq!(expected_adv_count, statistics[0].adv_count);
        let expected_adv_count_successful = HashMap::from([(1, 1), (2, 1)]);
        assert_eq!(
            expected_adv_count_successful,
            statistics[0].adv_count_successful
        );
        let number_of_adversaries = 0;
        let mut simulator =
            crate::attempt::tests::init_sim(None, Some(vec![number_of_adversaries]));
        let sim_result = simulator.run(
            vec![
                (source.clone(), "chan".to_string()),
                (source, "dina".to_string()),
            ]
            .into_iter(),
            None,
            true,
        );
        assert_eq!(sim_result.num_succesful, 2);
        let statistics = &simulator.adversaries[0].statistics;
        assert_eq!(statistics[0].number, number_of_adversaries);
        assert_eq!(statistics[0].hits, 0); // we have no adversaries
        assert_eq!(statistics[0].hits_successful, 0);
        assert_eq!(statistics[0].part_hits, 0);
        assert_eq!(statistics[0].part_hits_successful, 0);
        assert_eq!(statistics[0].targeted_attack.num_successful, 2);
        assert_eq!(statistics[0].targeted_attack.num_failed, 0);
        let expected_adv_count = HashMap::from([(0, 2)]);
        assert_eq!(expected_adv_count, statistics[0].adv_count);
        let expected_adv_count_successful = HashMap::from([(0, 2)]);
        assert_eq!(
            expected_adv_count_successful,
            statistics[0].adv_count_successful
        );
    }

    #[test]
    fn choose_adversaries() {
        let number_of_adversaries = 4;
        let simulator = crate::attempt::tests::init_sim(None, Some(vec![number_of_adversaries]));
        let adversaries = simulator.get_adversaries(number_of_adversaries);
        assert!(adversaries.get(&AdversarySelection::Random).is_some());
        let actual = adversaries.get(&AdversarySelection::Random).unwrap();
        assert_eq!(actual.len(), number_of_adversaries);
        for node in simulator.graph.get_node_ids() {
            assert!(actual.contains(&node));
        }
    }
}
