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
            for (_idx, num_adv) in number_of_adversaries.iter().enumerate() {
                let adv = match selected_adversaries.get(strategy) {
                    None => vec![],
                    Some(selected_adversaries) => selected_adversaries[0..*num_adv].to_vec(),
                };
                info!(
                    "Starting adversary scenario: {} sat: {:?} with {} nodes.",
                    self.amount, strategy, num_adv,
                );
                let (hits, _parts_hits, _payment_attacks) =
                    Self::adversary_hits(&all_payments, &adv);
                let (correlated, correlated_successful) =
                    Self::colluding_adversaries(&all_payments, &adv);
                let (prone_paths, prone_payments) =
                    Self::prone_paths_and_payments(&all_payments, &adv);
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
                    correlated,
                    correlated_successful,
                    prone_paths_prob: prone_paths.0,
                    prone_paths_successful_prob: prone_paths.1,
                    prone_payments_prob: prone_payments.0,
                    prone_payments_successful_prob: prone_payments.1,
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

    /// Returns the number of paths and payments that were observed the first and last hops
    fn prone_paths_and_payments(payments: &[Payment], adv: &[ID]) -> ((f32, f32), (f32, f32)) {
        let mut prone_paths = 0;
        let mut prone_paths_successful = 0;
        let mut safe_paths = 0;
        let mut safe_paths_successful = 0;
        let mut prone_payments = 0;
        let mut prone_payments_successful = 0;
        let mut safe_payments = 0;
        let mut safe_payments_successful = 0;
        for payment in payments {
            let mut payment_is_prone = false;
            for path in payment.used_paths.iter() {
                // no need to exclude the src and dest and the called function takes that into account
                if !path.path.path_contains_adversary(adv).is_empty() {
                    let mut first_hop = false;
                    let mut final_hop = false;
                    for a in adv {
                        if path.path.is_first_hop(a) {
                            first_hop = true;
                        }
                        if path.path.is_last_hop(a) {
                            final_hop = true;
                        }
                    }
                    if first_hop && final_hop {
                        payment_is_prone = true;
                        prone_paths += 1;
                        if payment.succeeded {
                            prone_paths_successful += 1;
                        }
                    } else {
                        safe_paths += 1;
                        if payment.succeeded {
                            safe_paths_successful += 1;
                        }
                    }
                }
            }
            if payment_is_prone {
                prone_payments += 1;
                if payment.succeeded {
                    prone_payments_successful += 1;
                }
            } else {
                safe_payments += 1;
                if payment.succeeded {
                    safe_payments_successful += 1;
                }
            }
        }
        let prob_paths = prone_paths as f32 / (prone_paths + safe_paths) as f32;
        let prob_paths_successful =
            prone_paths_successful as f32 / (prone_paths_successful + safe_paths_successful) as f32;
        let prob_payments = prone_payments as f32 / (prone_payments + safe_payments) as f32;
        let prob_payments_successful = prone_payments_successful as f32
            / (prone_payments_successful + safe_payments_successful) as f32;
        (
            (prob_paths, prob_paths_successful),
            (prob_payments, prob_payments_successful),
        )
    }

    /// Counts the number of paths per payment that could be correlated by colluding adversaries.
    /// Includes all payment attempts
    /// Returns the number of payments that were observed on multiple occasions
    fn colluding_adversaries(payments: &[Payment], adv: &[ID]) -> (usize, usize) {
        info!("Counting colluding adversaries.");
        let mut correlated = 0;
        let mut correlated_successful = 0;
        for payment in payments {
            let mut all_paths = payment.used_paths.to_owned();
            all_paths.extend(payment.failed_paths.to_owned());
            let mut paths_containing_adversaries = 0;
            for path in all_paths.iter() {
                // no need to exclude the src and dest and the called function takes that into account
                if !path.path.path_contains_adversary(adv).is_empty() {
                    paths_containing_adversaries += 1;
                }
            }
            // because the same payment was seen more than once
            if paths_containing_adversaries >= 2 {
                correlated += 1;
                if payment.succeeded {
                    correlated_successful += 1;
                }
            }
        }
        (correlated, correlated_successful)
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

    use super::*;
    use crate::{
        payment::Payment,
        traversal::pathfinding::{CandidatePath, Path},
        AdversarySelection,
    };
    use approx::*;
    use std::collections::VecDeque;

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
        assert_eq!(statistics[0].targeted_attack.num_successful, 0);
        assert_eq!(statistics[0].targeted_attack.num_failed, 0);
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
        assert_eq!(statistics[0].targeted_attack.num_successful, 2);
        assert_eq!(statistics[0].targeted_attack.num_failed, 0);
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

    #[test]
    fn count_correlations() {
        let number_of_adversaries = 4; // all four nodes are adversaries
        let source = "alice".to_string();
        let mut simulator =
            crate::attempt::tests::init_sim(None, Some(vec![number_of_adversaries]));
        let sim_result = simulator.run(
            vec![
                (source.clone(), "chan".to_string()), // alice -> bob -> chan
                (source.clone(), "dina".to_string()), // alice-> bob -> chan-> dina
            ]
            .into_iter(),
            None,
            true,
        );
        assert_eq!(sim_result.num_succesful, 2);
        let statistics = &simulator.adversaries[0].statistics;
        assert_eq!(statistics[0].correlated, 0); // only one path available
        assert_eq!(statistics[0].correlated_successful, 0);
        // we add a fake payment for testing
    }

    #[test]
    fn correlate_payments() {
        let number_of_adversaries = 4;
        let simulator = crate::attempt::tests::init_sim(None, Some(vec![number_of_adversaries]));
        let adversaries = simulator.get_adversaries(number_of_adversaries);
        let adversaries = adversaries.get(&AdversarySelection::Random).unwrap();
        let source = "alice".to_string();
        let payments = vec![
            Payment {
                payment_id: 2,
                source: source.clone(),
                dest: "eric".to_string(),
                amount_msat: 1000,
                succeeded: true,
                min_shard_amt: crate::MIN_SHARD_AMOUNT,
                num_parts: 1,
                htlc_attempts: 2,
                used_paths: vec![CandidatePath {
                    path: Path {
                        src: source.to_string(),
                        dest: "eric".to_string(),
                        // the fees and all don't matter here
                        hops: VecDeque::from([
                            ("alice".to_string(), 1100, 40, "alice1".to_string()),
                            ("bob".to_string(), 100, 40, "bob2".to_string()),
                            ("chan".to_string(), 1000, 0, "chan1".to_string()),
                            ("eric".to_string(), 1000, 0, "eric1".to_string()),
                        ]),
                    },
                    weight: 100.0,
                    amount: 1100,
                    time: 40,
                }],
                failed_amounts: Vec::default(),
                successful_shards: Vec::default(),
                failed_paths: vec![CandidatePath {
                    path: Path {
                        src: "alice".to_string(),
                        dest: "eric".to_string(),
                        hops: VecDeque::from([
                            ("alice".to_string(), 1100, 40, "alice1".to_string()),
                            ("bob".to_string(), 100, 40, "bob2".to_string()),
                            ("eric".to_string(), 1000, 0, "eric1".to_string()),
                        ]),
                    },
                    weight: 100.0,
                    amount: 1100,
                    time: 40,
                }],
            },
            Payment {
                payment_id: 2,
                source: source.clone(),
                dest: "eric".to_string(),
                amount_msat: 1000,
                succeeded: false,
                min_shard_amt: crate::MIN_SHARD_AMOUNT,
                num_parts: 1,
                htlc_attempts: 2,
                used_paths: vec![CandidatePath {
                    path: Path {
                        src: source.to_string(),
                        dest: "eric".to_string(),
                        // the fees and all don't matter here
                        hops: VecDeque::from([
                            ("alice".to_string(), 1100, 40, "alice1".to_string()),
                            ("bob".to_string(), 100, 40, "bob2".to_string()),
                            ("chan".to_string(), 1000, 0, "chan1".to_string()),
                            ("eric".to_string(), 1000, 0, "eric1".to_string()),
                        ]),
                    },
                    weight: 100.0,
                    amount: 1100,
                    time: 40,
                }],
                failed_amounts: Vec::default(),
                successful_shards: Vec::default(),
                failed_paths: vec![CandidatePath {
                    path: Path {
                        src: "alice".to_string(),
                        dest: "eric".to_string(),
                        hops: VecDeque::from([
                            ("alice".to_string(), 1100, 40, "alice1".to_string()),
                            ("bob".to_string(), 100, 40, "bob2".to_string()),
                            ("eric".to_string(), 1000, 0, "eric1".to_string()),
                        ]),
                    },
                    weight: 100.0,
                    amount: 1100,
                    time: 40,
                }],
            },
        ];
        let (correlation_count, correlation_count_successful) =
            Simulation::colluding_adversaries(&payments, &adversaries);
        assert_eq!(correlation_count, 2); // bob sees the payment twice
        assert_eq!(correlation_count_successful, 1);
        let (prone_paths, prone_payments) =
            Simulation::prone_paths_and_payments(&payments, &adversaries);
        // all paths are susceptible
        assert_abs_diff_eq!(prone_paths.0, 1.0, epsilon = 0.001f32);
        // all successful payemnts' paths are susceptible
        assert_abs_diff_eq!(prone_paths.1, 1.0, epsilon = 0.001f32);
        // both payments
        assert_abs_diff_eq!(prone_payments.1, 1.0, epsilon = 0.001f32);
        // the only successful payment is prone
        assert_abs_diff_eq!(prone_payments.1, 1.0, epsilon = 0.001f32);
    }
}
