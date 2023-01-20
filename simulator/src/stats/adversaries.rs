use crate::{
    graph::Graph,
    payment::Payment,
    traversal::pathfinding::{CandidatePath, Path, PathFinder},
    Adversaries, Simulation, ID,
};

use log::{info, trace};
use std::collections::VecDeque;

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
            let adv: Vec<ID> =
                Simulation::draw_adversaries(&self.graph.get_node_ids(), percent).collect();
            let (hits, hits_successful) = Self::adversary_hits(&all_payments, &adv);
            self.deanonymise_tx_pairs(&adv);
            self.adversaries.push(Adversaries {
                percentage: percent,
                hits,
                hits_successful,
            });
        }
    }

    fn deanonymise_tx_pairs(&self, adversaries: &[ID]) -> (Vec<usize>, Vec<usize>, usize) {
        info!("Computing anonymity sets.");
        let mut sd_anon_set_sizes = vec![];
        let mut rx_anon_set_sizes = vec![];
        let graph = self.graph.clone();
        let payments = &self.successful_payments;
        for payment in payments {
            let mut used_paths = payment.used_paths.to_owned();
            used_paths.extend(payment.failed_paths.to_owned());
            let sender = payment.source.clone();
            let recipient = payment.dest.clone();
            for p in used_paths.iter() {
                let adv_along_path = p.path.path_contains_adversary(adversaries);
                for (idx, adv) in adv_along_path.iter().enumerate() {
                    let adversary_id = adv.0.clone();
                    let pred = p.path.get_pred(&adversary_id);
                    let succ = p.path.get_succ(&adversary_id);
                    let amount_to_succ = adv.1 - p.path.hops[idx].1; // subtract adv's fee
                    let ttl_to_rx = adv.2
                        - (idx + 1..p.path.hops.len())
                            .map(|h| p.path.hops[h].2)
                            .sum::<usize>();
                    let mut g = graph.clone();
                    g.remove_node(&pred);
                    g.remove_node(&adversary_id);
                    g.edges = PathFinder::remove_inadequate_edges(&graph, amount_to_succ); //hm - which amount?
                                                                                           // prepend pred and adv to each path
                    if let Some(mut paths) =
                        Self::get_all_reachable_paths(&g, &succ, amount_to_succ, ttl_to_rx)
                    {
                        // TODO: Source can stay the same
                        paths.iter_mut().for_each(|p| {
                            p.path
                                .hops
                                .push_front((adversary_id.clone(), 0, 0, String::default()))
                        });
                        paths.iter_mut().for_each(|p| {
                            p.path
                                .hops
                                .push_front((pred.clone(), 0, 0, String::default()))
                        });
                        self.compute_reference_paths(paths, amount_to_succ); // TODO: Which graph? amt?
                    }
                }
            }
        }
        (sd_anon_set_sizes, rx_anon_set_sizes, 0)
    }

    /// Determine if the last node in the path is a potential receiver
    /// If so, we also determine potential senders
    fn compute_reference_paths(&self, found_paths: Vec<CandidatePath>, amount: usize) {
        // 1. computes paths from all nodes in the network to the last node in the path
        // - first node "N" of the computed path is an intermediary and charges a fee
        let graph = self.graph.clone();
        for path in found_paths {
            let rec = path.path.dest.clone();
            for src in graph.get_node_ids().into_iter() {
                if src == *rec {
                    continue;
                }
                // TODO: Does pathfinding alg matter? Yes because that defines how routes are
                // looked for! But parts probably does not
                let mut path_finder = PathFinder::new(
                    src.clone(),
                    rec.clone(),
                    amount,
                    &graph,
                    self.routing_metric,
                    self.payment_parts,
                );
                if let Some(shortest_path) = path_finder.shortest_path_from(&src) {
                    // determine cost for path - treat src as an intermediary
                    trace!(
                        "Got shortest path to potential receiver {}.",
                        path_finder.dest
                    );
                    let mut path = Path::new(path_finder.src.clone(), path_finder.dest.clone());
                    // the weights and timelock are set as the total path costs are calculated
                    path.hops = shortest_path
                        .0
                        .into_iter()
                        .map(|h| (h, usize::default(), usize::default(), String::default()))
                        .collect();
                    let mut candidate_path = CandidatePath::new_with_path(path);
                    path_finder.get_aggregated_path_cost(&mut candidate_path, true);
                    // path we have computed = candidate_path
                    // TODO: Continue from step 2
                }
            }
        }
    }

    /// Looks for all paths with at most DEPTH many hops that are reachable from the node
    fn get_all_reachable_paths(
        graph: &Graph,
        next: &ID,
        amount: usize,
        ttl: usize,
    ) -> Option<Vec<CandidatePath>> {
        let mut paths = vec![];
        for edge in graph.get_outedges(next) {
            let timelock_next = edge.cltv_expiry_delta;
            // timelock is equal to ttl
            if timelock_next.eq(&ttl) && edge.capacity >= amount {
                // return path next->edge.dest
                let mut path = Path::new(next.clone(), edge.destination.clone());
                path.hops = VecDeque::from([
                    (
                        next.clone(),
                        usize::default(),
                        usize::default(),
                        String::default(),
                    ),
                    (
                        edge.destination,
                        usize::default(),
                        usize::default(),
                        String::default(),
                    ),
                ]);
                paths.push(CandidatePath::new_with_path(path));
            // timelock is lower - we still have a change of succeeding
            } else if timelock_next < ttl {
                for second_hop in graph.get_outedges(&edge.destination) {
                    let total_timelock = edge.cltv_expiry_delta + second_hop.cltv_expiry_delta;
                    if total_timelock.eq(&ttl) && second_hop.capacity >= amount {
                        // return path next->edge.dest->second_hop.dest
                        let mut path = Path::new(next.clone(), second_hop.destination.clone());
                        path.hops = VecDeque::from([
                            (
                                next.clone(),
                                usize::default(),
                                usize::default(),
                                String::default(),
                            ),
                            (
                                edge.destination.clone(),
                                usize::default(),
                                usize::default(),
                                String::default(),
                            ),
                            (
                                second_hop.destination,
                                usize::default(),
                                usize::default(),
                                String::default(),
                            ),
                        ]);
                        paths.push(CandidatePath::new_with_path(path));
                        // timelock is lower - we still have a change of succeeding
                    } else if total_timelock < ttl {
                        // 3 hops away
                        for third_hop in graph.get_outedges(&second_hop.destination) {
                            let total_timelock = edge.htlc_maximum_msat
                                + second_hop.htlc_maximum_msat
                                + third_hop.htlc_maximum_msat;
                            if total_timelock.eq(&ttl) && third_hop.capacity >= amount {
                                // return path next->edge.dest->second_hop.dest->third_hop.dest
                                let mut path =
                                    Path::new(next.clone(), third_hop.destination.clone());
                                path.hops = VecDeque::from([
                                    (
                                        next.clone(),
                                        usize::default(),
                                        usize::default(),
                                        String::default(),
                                    ),
                                    (
                                        edge.destination.clone(),
                                        usize::default(),
                                        usize::default(),
                                        String::default(),
                                    ),
                                    (
                                        second_hop.destination.clone(),
                                        usize::default(),
                                        usize::default(),
                                        String::default(),
                                    ),
                                    (
                                        third_hop.destination,
                                        usize::default(),
                                        usize::default(),
                                        String::default(),
                                    ),
                                ]);
                                paths.push(CandidatePath::new_with_path(path));
                            }
                        }
                    }
                }
            }
        }
        if paths.is_empty() {
            None
        } else {
            Some(paths)
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

    use super::*;

    #[test]
    fn adversary_hits() {
        let fraction_of_adversaries = 100; // all three nodes are adversaries
                                           // alice -> bob -> chan
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(fraction_of_adversaries));
        let sim_result = simulator.run(vec![(source, dest)].into_iter());
        assert_eq!(sim_result.num_succesful, 1);
        simulator.eval_adversaries();
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

    #[test]
    fn reachable_paths_from_adv() {
        let simulator = crate::attempt::tests::init_sim(None, None);
        let graph = simulator.graph.clone();
        let amount = 100;
        let next = "alice".to_string();
        let ttl = 5;
        let expected = vec![CandidatePath {
            path: Path {
                src: "alice".to_owned(),
                dest: "bob".to_owned(),
                hops: VecDeque::from([
                    ("alice".to_owned(), 0, 0, "".to_owned()),
                    ("bob".to_owned(), 0, 0, "".to_owned()),
                ]),
            },
            weight: 0.0,
            amount: 0,
            time: 0,
        }];
        let actual = Simulation::get_all_reachable_paths(&graph, &next, amount, ttl);
        assert!(actual.is_some());
        assert_eq!(actual.unwrap(), expected);
        let next = "bob".to_string();
        let ttl = 40;
        let expected = vec![
            CandidatePath {
                path: Path {
                    src: "bob".to_owned(),
                    dest: "alice".to_owned(),
                    hops: VecDeque::from([
                        ("bob".to_owned(), 0, 0, "".to_owned()),
                        ("alice".to_owned(), 0, 0, "".to_owned()),
                    ]),
                },
                weight: 0.0,
                amount: 0,
                time: 0,
            },
            CandidatePath {
                path: Path {
                    src: "bob".to_owned(),
                    dest: "chan".to_owned(),
                    hops: VecDeque::from([
                        ("bob".to_owned(), 0, 0, "".to_owned()),
                        ("chan".to_owned(), 0, 0, "".to_owned()),
                    ]),
                },
                weight: 0.0,
                amount: 0,
                time: 0,
            },
        ];
        let actual = Simulation::get_all_reachable_paths(&graph, &next, amount, ttl);
        assert!(actual.is_some());
        for path in actual.unwrap() {
            assert!(expected.contains(&path))
        }
        let next = "bob".to_string();
        let ttl = 55;
        let expected = vec![CandidatePath {
            path: Path {
                src: "bob".to_owned(),
                dest: "dina".to_owned(),
                hops: VecDeque::from([
                    ("bob".to_owned(), 0, 0, "".to_owned()),
                    ("chan".to_owned(), 0, 0, "".to_owned()),
                    ("dina".to_owned(), 0, 0, "".to_owned()),
                ]),
            },
            weight: 0.0,
            amount: 0,
            time: 0,
        }];
        let actual = Simulation::get_all_reachable_paths(&graph, &next, amount, ttl);
        assert!(actual.is_some());
        assert_eq!(actual.unwrap(), expected);
    }
}
