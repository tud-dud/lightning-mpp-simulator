use crate::{
    graph::Graph,
    payment::Payment,
    traversal::pathfinding::{CandidatePath, Path, PathFinder},
    Adversaries, Simulation, ID,
};

#[cfg(not(test))]
use log::{debug, info, trace};
#[cfg(test)]
use std::{println as info, println as debug, println as trace};
use std::collections::{HashMap, HashSet, VecDeque};

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

    /// Returns a set of potential recipients as well as a set of all potential recipients
    fn deanonymise_tx_pairs(&self, adversaries: &[ID]) -> (HashSet<ID>, HashSet<ID>) {
        info!("Computing anonymity sets.");
        let mut sd_anon_set = HashSet::new();
        let mut rx_anon_set = HashSet::new();
        let graph = self.graph.clone();
        let payments = &self.successful_payments;
        for payment in payments {
            let mut used_paths = payment.used_paths.to_owned();
            used_paths.extend(payment.failed_paths.to_owned());
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
                                                                                           // Phase 1 paths = P_i in the paper, i.e. all paths with appropriate timelock
                                                                                           // and capacity
                    let phase1_paths = if let Some(paths) =
                        Self::get_all_reachable_paths(&g, &succ, amount_to_succ, ttl_to_rx)
                    {
                        paths
                    } else {
                        vec![]
                    };
                    // for all Pi for a list of potential recipients as R and potential senders for each such recipient
                    // The union of the potential senders for all potential recipients is the sender anonymity set
                    for mut p_i in phase1_paths {
                        rx_anon_set.insert(p_i.path.dest.clone());
                        // TODO: Source can stay the same
                        p_i.path
                            .hops
                            .push_front((adversary_id.clone(), 0, 0, String::default()));
                        p_i.path
                            .hops
                            .push_front((pred.clone(), 0, 0, String::default()));
                        // phase 2, step 1 paths = P[N]
                        let shorest_paths_to_p_i_rcpt =
                            self.compute_shortest_paths(&p_i, amount_to_succ); // TODO: Which graph? amt?
                        let senders_for_r = self.find_potential_sources(
                            &p_i,
                            shorest_paths_to_p_i_rcpt,
                            &adversary_id,
                            &pred,
                        );
                        sd_anon_set = sd_anon_set.union(&senders_for_r).cloned().collect();
                    }
                }
            }
        }
        (rx_anon_set, sd_anon_set)
    }

    /// Determine if the last node in the path is a potential receiver
    /// If so, we also determine potential senders
    /// Phase 2, Step 2-4
    fn find_potential_sources(
        &self,
        p_i: &CandidatePath,
        shortest_paths: impl Iterator<Item = (ID, CandidatePath)>,
        adversary: &ID,
        pre: &ID,
    ) -> HashSet<ID> {
        let mut possible_sources = HashSet::new();
        let all_shortest_paths = shortest_paths.collect::<HashMap<ID, CandidatePath>>();
        println!("shortest_paths {:?}", all_shortest_paths);
        println!("p_i {:?}", p_i);
        for hops_in_pi in p_i.path.hops.range(0..p_i.path.hops.len()) {
            // step 2: check if subpath
            let pj = hops_in_pi.0.clone(); //pj
            let path_from_pj = all_shortest_paths.get(&pj);
            if let Some(path_from_pj) = path_from_pj {
                let pi_pj_subpath = p_i.path.subpath_from(&pj);
                println!("pi_pj_subpath {:?}", pi_pj_subpath);
                // Exit if any subpath is not in the path
                if pi_pj_subpath.is_empty() {
                    debug!("Exiting due to empty subpath.");
                    possible_sources = HashSet::new();
                    break;
                }
                // if subpath from curr != path by curr: exit
                if !Path::is_equal(&pi_pj_subpath, &path_from_pj.path.hops) {
                    debug!("Subpath from {:?} unequal to path computed by self.", pj);
                    debug!("self subpath {:?}", pi_pj_subpath);
                    debug!("{} subpath {:?}", pj, path_from_pj);
                    possible_sources = HashSet::new();
                    break;
                } else {
                    // we know that the paths from the adversary are the same
                    if pj.eq(adversary) && path_from_pj.path.get_involved_nodes().contains(pre) {
                        debug!("Looking at adversary's path.");
                        possible_sources.extend(self.all_potential_senders(
                            p_i,
                            all_shortest_paths.values().cloned().collect(),
                        ));
                        break;
                    }
                }
                if pj.eq(pre) {
                    debug!("Looking at predecessor's path.");
                    // If pre is the source, the path from pre should not match the path found
                    // since, the cost from the source to the second node is computed differently.
                    match all_shortest_paths.get(pre) {
                        // Step 3 - pre cannot be an intermediary but must be the only sender of the
                        // transaction since it preceded A
                        Some(path_from_pre) => {
                            if Path::is_equal(&path_from_pre.path.hops, &p_i.path.hops) {
                                possible_sources.insert(pre.clone());
                                break;
                            } else {
                                // paths match so pre is just one of the possible senders
                                possible_sources.insert(pre.clone());
                                if path_from_pj.path.get_involved_nodes().contains(pre) {
                                    possible_sources.extend(self.all_potential_senders(
                                        p_i,
                                        all_shortest_paths.values().cloned().collect(),
                                    ));
                                    break;
                                }
                            }
                        }
                        None => continue,
                    }
                }
                } else {
                    println!("No path {}", hops_in_pi.0);
                    println!("No path {:?}", all_shortest_paths[&"chan".to_string()]);
                    println!("No path {:?}", all_shortest_paths[&"chan".to_string()]);
            }
        }
        possible_sources
    }

    /// Given a node, return all neighbours of the node than are not in the path
    fn all_potential_senders(
        &self,
        p_i: &CandidatePath,
        shortest_paths: Vec<CandidatePath>,
    ) -> HashSet<ID> {
        debug!("Adding all remaining nodes to sources");
        let mut possible_sources = HashSet::new();
        // step 4
        for p_n in shortest_paths {
            // add all neighbours of N that are not in P[N] as a potential senders
            // TODO: Includes sender and reciver
            if p_i.path.is_subpath(&p_n.path) {
                possible_sources.extend(
                    self.graph
                        .get_outedges(&p_n.path.src) // outedges = inedges since the graph is symetric
                        .iter()
                        .filter(|e| !p_n.path.get_involved_nodes().contains(&e.destination))
                        .map(|e| e.destination.clone())
                        .collect::<HashSet<ID>>(),
                );
            }
        }
        possible_sources
    }

    /// Compute all paths to a potential receiver from all nodes in the graph
    /// Returns a map of <source, CandidatePath>
    fn compute_shortest_paths(
        &self,
        found_path: &CandidatePath,
        amount: usize,
    ) -> impl Iterator<Item = (ID, CandidatePath)> {
        // 1. computes paths from all nodes in the network to the last node in the path
        // - first node "N" of the computed path is an intermediary and charges a fee
        let graph = self.graph.clone();
        let mut all_paths: HashMap<ID, CandidatePath> = HashMap::new();
        let rec = found_path.path.dest.clone();
        for src in graph.get_node_ids().iter() {
            if src.clone() == rec.clone() {
                continue;
            }
            if all_paths.contains_key(&src.clone()) {
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
            if let Some(shortest_path) = path_finder.shortest_path_from(src) {
                // determine cost for path - treat src as an intermediary
                trace!(
                    "Got shortest path from potential sender {}.",
                    path_finder.src
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
                // store computed paths in hashmap <src, path>
                all_paths.insert(path_finder.src, candidate_path);
            }
        }
        all_paths.into_iter()
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
            //let mut edges = vec![];
            let timelock_next = edge.cltv_expiry_delta;
            // timelock is equal to ttl
            if timelock_next.eq(&ttl) && edge.capacity >= amount  {
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
                //edges.push(edge.destination);
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

    #[test]
    fn reference_paths() {
        let simulator = crate::attempt::tests::init_sim(None, None);
        let graph = simulator.graph.clone();
        let amount = 100;
        let next = "alice".to_string();
        let ttl = 5;
        let reachable_paths =
            Simulation::get_all_reachable_paths(&graph, &next, amount, ttl).unwrap();
        assert_eq!(reachable_paths.len(), 1); // only bob as destination
        for found_path in reachable_paths {
            let reference_paths: HashMap<ID, CandidatePath> = simulator
                .compute_shortest_paths(&found_path, amount)
                .collect();
            assert_eq!(reference_paths.len(), 3); // a->b, c->b, d->b
            let expected_keys = vec![
                ("alice".to_string()),
                ("chan".to_string()),
                ("dina".to_string()),
            ];
            for r in reference_paths {
                assert!(expected_keys.contains(&r.0));
            }
        }
        let next = "bob".to_string();
        let ttl = 40;
        let reachable_paths =
            Simulation::get_all_reachable_paths(&graph, &next, amount, ttl).unwrap();
        assert_eq!(reachable_paths.len(), 2); // alice, chan as destinations
        for found_path in reachable_paths {
            let reference_paths: HashMap<ID, CandidatePath> = simulator
                .compute_shortest_paths(&found_path, amount)
                .collect();
            assert_eq!(reference_paths.len(), 3); // b->a c->a, d->a, a->c b->c d->c
        }
    }

    #[test]
    fn add_all_senders() {
        let simulator = crate::attempt::tests::init_sim(None, None);
        let graph = simulator.graph.clone();
        let amount = 100;
        let next = "alice".to_string();
        let ttl = 5;
        let reachable_path =
            Simulation::get_all_reachable_paths(&graph, &next, amount, ttl).unwrap();
        assert_eq!(reachable_path.len(), 1); // only bob as destination
        let reference_paths: Vec<CandidatePath> = simulator
            .compute_shortest_paths(&reachable_path[0], amount)
            .map(|r| r.1)
            .collect();
        assert_eq!(reference_paths.len(), 3); // a->b, c->b, d->b
        let p_i = reachable_path[0].to_owned();
        assert!(simulator
            .all_potential_senders(&p_i, reference_paths)
            .is_empty());
        let p_i = CandidatePath {
            path: Path {
                src: "chan".to_owned(),
                dest: "dina".to_owned(),
                hops: VecDeque::from([
                    ("chan".to_owned(), 0, 0, "".to_owned()),
                    ("dina".to_owned(), 0, 0, "".to_owned()),
                ]),
            },
            weight: 0.0,
            amount: 0,
            time: 0,
        };
        let reference_paths = vec![CandidatePath {
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
        let actual = simulator.all_potential_senders(&p_i, reference_paths.clone());
        let expected = HashSet::from(["alice".to_owned()]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn potential_senders() {
        // does not make too much sense since bob has two roles
        let simulator = crate::attempt::tests::init_sim(None, None);
        let graph = simulator.graph.clone();
        let amount = 100;
        let next_reachable = "bob".to_string();
        let ttl = 40; // alice and chan as destinations
        let reachable_path =
            Simulation::get_all_reachable_paths(&graph, &next_reachable, amount, ttl).unwrap();
        assert_eq!(reachable_path.len(), 2); // only bob as destination
        let shortest_paths = simulator
            .compute_shortest_paths(&reachable_path[0], amount);
        let adversary = "chan".to_string();
        let pre = "bob".to_string();
        let p_i = reachable_path[0].clone();
        let actual = simulator.find_potential_sources(&p_i, shortest_paths, &adversary, &pre);
        let expected = HashSet::from(["bob".to_string()]);
        assert_eq!(actual, expected);
        // a->b->c->d
        let adversary = "bob".to_string();
        let pre = "alice".to_string();
        let next_reachable = "chan".to_string();
        let ttl = 15; // bob-chan+dina ttl
        let reachable_path =
            Simulation::get_all_reachable_paths(&graph, &next_reachable, amount, ttl).unwrap();
        println!("reachable_path {:?}", reachable_path);
        assert_eq!(reachable_path.len(), 1); // only dina as destination
        let shortest_paths = simulator
            .compute_shortest_paths(&reachable_path[0], amount);
        let p_i = reachable_path[0].clone();
        let actual = simulator.find_potential_sources(&p_i, shortest_paths, &adversary, &pre);
        let expected = HashSet::from(["dina".to_string()]);
        assert_eq!(actual, expected);
    }
}
