use std::collections::HashSet;

use super::Diversity;
use crate::{Simulation, ID};
use itertools::Itertools;

type NodeLinkID = (ID, String);

impl Simulation {
    /// Calculates the Levenshtein distances of mpp paths and the diversity as defined by Rohrer et
    /// al.
    pub(crate) fn eval_path_similarity(&mut self) {
        let mut levenshtein_distances = vec![];
        let mut path_diversity = vec![];
        let lambdas = vec![0.2, 0.5, 0.7, 1.0];
        for (i, lambda) in lambdas.into_iter().enumerate() {
            let mut epds = vec![];
            for payment in &self.successful_payments {
                if payment.num_parts <= 1 {
                    continue;
                }
                let paths: Vec<Vec<NodeLinkID>> = payment
                    .used_paths
                    .iter()
                    .map(|p| {
                        p.path
                            .hops
                            .iter()
                            .map(|h| (h.0.clone(), h.3.clone()))
                            .collect()
                    })
                    .collect();
                epds.push(Self::calculate_effective_path_diversity(&paths, lambda));
                if i == 0 {
                    // we only need to calculate this once: hacky
                    levenshtein_distances.extend(Self::calculate_levenshtein_distance(&paths));
                }
            }
            path_diversity.push(Diversity {
                lambda,
                diversity: epds,
            });
        }
        self.path_distances.0 = levenshtein_distances;
        self.path_diversity.0 = path_diversity;
    }

    /// The EPD is an aggregation of path diversities for a selected set of paths between a given
    /// node- pair
    fn calculate_effective_path_diversity(paths: &[Vec<NodeLinkID>], lambda: f32) -> f32 {
        let mut aggregated_div = 0.0;
        for idx in 0..paths.len() {
            let mut div_min_path_i = f32::MAX;
            let mut alternate_paths = paths.to_vec();
            // the base path should be the only item returned by drain
            if let Some(base_path) = alternate_paths.drain(idx..idx + 1).last() {
                for path in alternate_paths {
                    let div = Self::calculate_path_diversity(&base_path, &path);
                    div_min_path_i = f32::min(div_min_path_i, div);
                }
            }
            aggregated_div += div_min_path_i;
        }
        1.0 - std::f32::consts::E.powf(-lambda * aggregated_div)
    }

    /// Using the metric defined by Rohrer et al. in Multipath at the transport layer
    /// Returns a diversity score for each used path compared to the shortest path
    pub fn calculate_path_diversity(
        base_path: &[(ID, String)],
        alternate_path: &[(ID, String)],
    ) -> f32 {
        let base_path = Self::get_intermediate_node_and_edges(base_path);
        let alternate_path = Self::get_intermediate_node_and_edges(alternate_path);
        1.0 - (base_path.intersection(&alternate_path).count() as f32 / base_path.len() as f32)
    }

    /// Returns a distance for each pair of given paths
    fn calculate_levenshtein_distance(paths: &[Vec<NodeLinkID>]) -> Vec<usize> {
        let mut distances = vec![];
        let pairs = paths.iter().cloned().cartesian_product(paths);
        let mut seen_pairs: Vec<Vec<ID>> = vec![];
        for (lhs, rhs) in pairs {
            let lhs: Vec<ID> = lhs.iter().map(|l| l.0.clone()).collect();
            let rhs: Vec<ID> = rhs.iter().map(|l| l.0.clone()).collect();
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

    // Returns the base path and the rest of paths
    #[allow(unused)]
    fn get_reference_paths(paths: &[Vec<NodeLinkID>]) -> (Vec<NodeLinkID>, Vec<Vec<NodeLinkID>>) {
        let mut alternates = paths.to_vec();
        // shortest paths
        let mut base_path = alternates[0].clone();
        let mut base_path_pos = 0;
        for (pos, path) in paths.iter().enumerate() {
            if path.len() < base_path.len() {
                base_path = path.to_vec();
                base_path_pos = pos;
            }
        }
        // this is the reference path and we need to remove it
        alternates.remove(base_path_pos);
        (base_path, alternates)
    }

    pub fn get_intermediate_node_and_edges(hops: &[NodeLinkID]) -> HashSet<String> {
        let nodes: Vec<ID> = (1..hops.len() - 1).map(|h| hops[h].0.clone()).collect();
        let links: Vec<ID> = (0..hops.len() - 1).map(|h| hops[h].1.clone()).collect();
        let mut path = HashSet::from_iter(nodes);
        path.extend(links);
        path
    }
}

#[cfg(test)]
mod tests {

    use crate::payment::Payment;
    use crate::traversal::pathfinding::{CandidatePath, Path};
    use approx::*;
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
            vec![
                ("a".to_string(), "".to_string()),
                ("b".to_string(), "".to_string()),
                ("c".to_string(), "".to_string()),
            ],
            vec![
                ("a".to_string(), "".to_string()),
                ("e".to_string(), "".to_string()),
                ("c".to_string(), "".to_string()),
            ],
            vec![("a".to_string(), "".to_string())],
        ];
        let actual = Simulation::calculate_levenshtein_distance(&used_paths);
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

    #[test]
    fn split_base_from_other_paths() {
        let paths = vec![
            vec![
                ("a".to_string(), "ab".to_string()),
                ("b".to_string(), "bc".to_string()),
                ("c".to_string(), "cb".to_string()),
            ],
            vec![
                ("a".to_string(), "ae".to_string()),
                ("e".to_string(), "ec".to_string()),
                ("c".to_string(), "ce".to_string()),
            ],
            vec![("a".to_string(), "".to_string())],
        ];
        let actual = Simulation::get_reference_paths(&paths);
        let expected = (paths[2].clone(), vec![paths[0].clone(), paths[1].clone()]);
        assert_eq!(actual, expected);
        let paths = vec![
            vec![
                ("a".to_string(), "ab".to_string()),
                ("b".to_string(), "bc".to_string()),
                ("c".to_string(), "cb".to_string()),
            ],
            vec![
                ("a".to_string(), "ae".to_string()),
                ("e".to_string(), "ec".to_string()),
                ("c".to_string(), "ce".to_string()),
            ],
        ];
        let actual = Simulation::get_reference_paths(&paths);
        let expected = (paths[0].clone(), vec![paths[1].clone()]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn intermediates_and_links() {
        let path = vec![
            ("a".to_string(), "ab".to_string()),
            ("b".to_string(), "bc".to_string()),
            ("c".to_string(), "cb".to_string()),
        ];
        let actual = Simulation::get_intermediate_node_and_edges(&path);
        let expected = HashSet::from(["b".to_string(), "ab".to_string(), "bc".to_string()]);
        assert_eq!(actual, expected);
        let path = vec![
            ("a".to_string(), "ab".to_string()),
            ("b".to_string(), "ba".to_string()),
        ];
        let actual = Simulation::get_intermediate_node_and_edges(&path);
        let expected = HashSet::from(["ab".to_string()]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn path_diversity() {
        let base_path = vec![
            ("0".to_string(), "01".to_string()),
            ("1".to_string(), "12".to_string()),
            ("2".to_string(), "21".to_string()),
        ];
        let alternate_path = vec![
            ("0".to_string(), "03".to_string()),
            ("3".to_string(), "31".to_string()),
            ("1".to_string(), "15".to_string()),
            ("5".to_string(), "52".to_string()),
            ("2".to_string(), "25".to_string()),
        ];
        let actual = Simulation::calculate_path_diversity(&base_path, &alternate_path);
        let expected = 0.66;
        assert_abs_diff_eq!(actual, expected, epsilon = 0.01f32);
        let alternate_path = vec![
            ("0".to_string(), "03".to_string()),
            ("3".to_string(), "34".to_string()),
            ("4".to_string(), "45".to_string()),
            ("5".to_string(), "52".to_string()),
            ("2".to_string(), "25".to_string()),
        ];
        let actual = Simulation::calculate_path_diversity(&base_path, &alternate_path);
        let expected = 1.0;
        assert_abs_diff_eq!(actual, expected, epsilon = 0.01f32);
    }

    #[test]
    fn effective_path_diversity() {
        let lambda = 0.5;
        let paths = vec![
            vec![
                ("0".to_string(), "01".to_string()),
                ("1".to_string(), "12".to_string()),
                ("2".to_string(), "21".to_string()),
            ],
            vec![
                ("0".to_string(), "03".to_string()),
                ("3".to_string(), "31".to_string()),
                ("1".to_string(), "15".to_string()),
                ("5".to_string(), "52".to_string()),
                ("2".to_string(), "25".to_string()),
            ],
        ];
        let mut agg_diversity = Simulation::calculate_path_diversity(&paths[0], &paths[1]);
        agg_diversity += Simulation::calculate_path_diversity(&paths[1], &paths[0]);
        let expected = 1.0 - std::f32::consts::E.powf(-lambda * agg_diversity);
        let actual = Simulation::calculate_effective_path_diversity(&paths, lambda);
        assert_eq!(actual, expected);
    }
}
