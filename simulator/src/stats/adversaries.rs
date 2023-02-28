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
        let number_of_adversaries = if let Some(number) = self.number_of_adversaries {
            vec![number]
        } else {
            (1..21).collect()
        };
        let selected_adversaries =
            self.get_adversaries(number_of_adversaries[number_of_adversaries.len() - 1]);
        let mut all_payments = self.failed_payments.clone();
        all_payments.extend(self.successful_payments.clone());
        let adversaries = Arc::new(Mutex::new(vec![]));
        self.adversary_selection.par_iter().for_each(|strategy| {
            let mut statistics: Vec<Statistics> = vec![];
            for num_adv in number_of_adversaries.iter() {
                let adv = match selected_adversaries.get(strategy) {
                    None => vec![],
                    Some(selected_adversaries) => selected_adversaries[0..*num_adv].to_vec(),
                };
                info!(
                    "Starting adversary scenario: {} sat: {:?} with {} nodes.",
                    self.amount, strategy, num_adv,
                );
                let (hits, hits_successful) = Self::adversary_hits(&all_payments, &adv);
                info!("Completed counting adversary occurences in payments.");
                let anonymity_sets = if let Some(adversary) = adv.last() {
                        let set = self.deanonymise_tx_pairs(adversary);
                        info!(
                            "Completed anonymity sets for {:?}, {:?} of {} sat with {} {:?} adversaries.",
                            self.routing_metric, self.payment_parts, self.amount, num_adv, strategy,
                        );
                        set
                } else {
                    vec![]
                };
                let targeted_attack = self.rerun_simulation(&adv);
                statistics.push(Statistics {
                    number: *num_adv,
                    hits,
                    hits_successful,
                    anonymity_sets,
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
    fn adversary_hits(payments: &[Payment], adv: &[ID]) -> (usize, usize) {
        let mut hits = 0;
        let mut hits_successful = 0;
        for payment in payments {
            let mut used_paths = payment.used_paths.to_owned();
            used_paths.extend(payment.failed_paths.to_owned());
            for path in used_paths.iter() {
                if !path.path.path_contains_adversary(adv).is_empty() {
                    hits += 1;
                    if payment.succeeded {
                        hits_successful += 1;
                    }
                    continue;
                }
            }
        }
        (hits, hits_successful)
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
            };
            all_adversaries.insert(strategy.clone(), adv);
        }
        all_adversaries
    }
}

#[cfg(test)]
mod tests {

    use crate::AdversarySelection;

    #[test]
    fn adversary_hits() {
        let number_of_adversaries = 4; // all four nodes are adversaries
                                       // alice -> bob -> chan
        let source = "alice".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(number_of_adversaries));
        let sim_result = simulator.run(
            vec![
                (source.clone(), "chan".to_string()),
                (source.clone(), "dina".to_string()),
            ]
            .into_iter(),
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
        assert_eq!(statistics[0].targeted_attack.num_successful, 0);
        assert_eq!(statistics[0].targeted_attack.num_failed, 0);
        let number_of_adversaries = 0;
        let mut simulator = crate::attempt::tests::init_sim(None, Some(number_of_adversaries));
        let sim_result = simulator.run(
            vec![
                (source.clone(), "chan".to_string()),
                (source, "dina".to_string()),
            ]
            .into_iter(),
        );
        assert_eq!(sim_result.num_succesful, 2);
        let statistics = &simulator.adversaries[0].statistics;
        assert_eq!(statistics[0].number, number_of_adversaries);
        assert_eq!(statistics[0].hits, 0); // we only send one payment
        assert_eq!(statistics[0].hits_successful, 0); // times
        assert_eq!(statistics[0].targeted_attack.num_successful, 2);
        assert_eq!(statistics[0].targeted_attack.num_failed, 0);
    }

    #[test]
    fn choose_adversaries() {
        let number_of_adversaries = 4;
        let simulator = crate::attempt::tests::init_sim(None, Some(number_of_adversaries));
        let adversaries = simulator.get_adversaries(number_of_adversaries);
        assert!(adversaries.get(&AdversarySelection::Random).is_some());
        let actual = adversaries.get(&AdversarySelection::Random).unwrap();
        assert_eq!(actual.len(), number_of_adversaries);
        for node in simulator.graph.get_node_ids() {
            assert!(actual.contains(&node));
        }
    }
}
