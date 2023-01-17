use crate::{Simulation, ID};

use itertools::Itertools;

impl Simulation {
    pub(crate) fn eval_path_similarity(&mut self) {
        let mut levenshtein_distances = vec![];
        for payment in &self.successful_payments {
            if payment.used_paths.len() <= 1 {
                continue;
            }
            let paths: Vec<Vec<ID>> = payment
                .used_paths
                .iter()
                .map(|p| p.path.hops.iter().map(|h| h.0.clone()).collect())
                .collect();
            levenshtein_distances.extend(Self::calc_distance(paths));
        }
        self.path_distances.0 = levenshtein_distances;
    }

    /// Returns a distance for each pair of given paths
    fn calc_distance(paths: Vec<Vec<ID>>) -> Vec<usize> {
        let mut distances = vec![];
        let pairs = paths.iter().cloned().cartesian_product(paths.clone());
        let mut seen_pairs: Vec<Vec<ID>> = vec![];
        for (lhs, rhs) in pairs {
            // skip same vecs as pair
            if lhs != rhs {
                // order of pairs does not matter so we skip half the pairs
                if seen_pairs.contains(&rhs) {
                    continue;
                }
                distances.push(Self::levenshtein(lhs.clone(), rhs));
                seen_pairs.push(lhs);
            }
        }
        distances
    }

    /// Implements the Levenshtein distance for the used paths of a payment
    fn levenshtein(lhs: Vec<ID>, rhs: Vec<ID>) -> usize {
        let mut result = 0;
        let lhs_len = lhs.len();
        let rhs_len = rhs.len();
        if lhs.is_empty() {
            result = rhs_len;
            return result;
        }
        if rhs.is_empty() {
            result = lhs_len;
            return result;
        }
        let mut cache: Vec<usize> = (1..).take(lhs_len).collect();
        let mut lhs_distance;
        let mut rhs_distance;
        for (rhs_idx, rhs_node) in rhs.iter().enumerate() {
            result = rhs_idx;
            lhs_distance = rhs_idx;
            for (lhs_idx, lhs_node) in lhs.iter().enumerate() {
                rhs_distance = if lhs_node.eq(rhs_node) {
                    lhs_distance
                } else {
                    lhs_distance + 1
                };
                lhs_distance = cache[lhs_idx];
                result = if lhs_distance > result {
                    if rhs_distance > result {
                        result + 1
                    } else {
                        rhs_distance
                    }
                } else if rhs_distance > lhs_distance {
                    lhs_distance + 1
                } else {
                    rhs_distance
                };
                cache[lhs_idx] = result;
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::payment::Payment;
    use crate::traversal::pathfinding::{CandidatePath, Path};
    use std::collections::VecDeque;

    use super::*;

    #[test]
    fn path_difference() {
        let lhs = vec![];
        let rhs = vec![];
        let lhs_len = lhs.len();
        let actual = Simulation::levenshtein(lhs.clone(), rhs.clone());
        let expected = lhs_len;
        assert_eq!(actual, expected);
        let actual = Simulation::levenshtein(lhs, rhs);
        let expected = 0;
        assert_eq!(actual, expected);
        let lhs = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let rhs = vec!["a".to_string(), "e".to_string(), "c".to_string()];
        let actual = Simulation::levenshtein(lhs.clone(), rhs);
        let expected = 1;
        assert_eq!(actual, expected);
        let rhs = vec![
            "a".to_string(),
            "e".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
        ];
        let actual = Simulation::levenshtein(lhs, rhs);
        let expected = 3;
        assert_eq!(actual, expected);
    }
    #[test]
    fn all_paths_difference() {
        let used_paths = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            vec!["a".to_string(), "e".to_string(), "c".to_string()],
            vec!["a".to_string()],
        ];
        let actual = Simulation::calc_distance(used_paths);
        let expected = vec![1, 2, 2];
        assert_eq!(actual, expected);
    }
    #[test]
    fn all_payments_distances() {
        let mut simulator = crate::attempt::tests::init_sim(None, None);
        let amount = 100;
        let source = String::from("a");
        let dest = String::from("d");
        let successful_payments = vec![Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat: amount,
            succeeded: false,
            min_shard_amt: 10,
            htlc_attempts: 0,
            num_parts: 2,
            used_paths: vec![
                CandidatePath {
                    path: Path {
                        src: String::from("a"),
                        dest: String::from("d"),
                        hops: VecDeque::from([
                            ("alice".to_string(), 5175, 55, "alice1".to_string()),
                            ("bob".to_string(), 100, 40, "bob2".to_string()),
                            ("chan".to_string(), 75, 15, "chan2".to_string()),
                            ("dina".to_string(), 5000, 0, "dina1".to_string()),
                        ]),
                    },
                    weight: 175.0, // fees (b->c, c->d)
                    amount: 5175,  // amount + fees
                    time: 55,
                },
                CandidatePath {
                    path: Path {
                        src: String::from("a"),
                        dest: String::from("d"),
                        hops: VecDeque::from([
                            ("alice".to_string(), 5175, 55, "alice1".to_string()),
                            ("dina".to_string(), 5000, 0, "dina1".to_string()),
                        ]),
                    },
                    weight: 15.0,
                    amount: 55,
                    time: 5,
                },
            ],
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
            failed_paths: vec![],
        }];
        simulator.successful_payments = successful_payments;
        simulator.eval_path_similarity();
        let actual = simulator.path_distances.0;
        let expected = vec![2];
        assert_eq!(actual, expected);
    }
}
