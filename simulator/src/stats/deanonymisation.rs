use crate::{
    graph::Graph,
    stats::AnonymitySet,
    traversal::pathfinding::{CandidatePath, Path, PathFinder},
    Simulation, ID,
};

#[cfg(not(test))]
use log::{debug, info, trace, warn};
use rand::seq::IteratorRandom;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::{println as info, println as debug, println as trace, println as warn};

impl Simulation {
    /// Returns a set of potential recipients as well as a set of all potential recipients
    #[allow(dead_code)]
    pub(crate) fn deanonymise_tx_pairs(&self, adversary: &ID) -> Vec<AnonymitySet> {
        info!(
            "Computing anonymity sets for {:?}, {:?} of {} sat.",
            self.routing_metric, self.payment_parts, self.amount,
        );
        let all_anonymits_sets = Arc::new(Mutex::new(vec![]));
        let graph = self.graph.clone();
        let mut rng = crate::RNG.lock().unwrap();
        // randomly pick 20% of the payments
        let payments = self
            .successful_payments
            .iter()
            .cloned()
            .choose_multiple(&mut *rng, self.successful_payments.len() * 20 / 100);
        info!(
            "Evaluating {} successful payments for anonymity sets.",
            payments.len()
        );
        payments.par_iter().for_each(|payment| {
            // only using the successful paths - Kumble et al only attempt once
            payment.used_paths.par_iter().for_each(|p| {
                // multiple sets per payment as each adversary has their own set
                let mut sd_anon_set = HashSet::new();
                let mut rx_anon_set = HashSet::new();
                // will only be one at most
                if let Some(adv) = p.path.path_contains_adversary(&[adversary.clone()]).first() {
                    let adversary_id = adv.0.clone();
                    let (pred, succ, amount_to_succ, ttl_to_rx) =
                        Self::extract_tx_info(p, &adversary_id);
                    let mut g = graph.clone();
                    g.remove_node(&pred);
                    g.remove_node(&adversary_id);
                    g.set_edges(PathFinder::remove_inadequate_edges(&graph, amount_to_succ)); //hm - which amount?
                                                                                           // prepend pred and adv to each path
                                                                                           // Phase 1 paths = P_i in the paper, i.e. all paths with appropriate timelock
                                                                                           // stores (src, dest): path
                    let mut shortest_paths: HashMap<(ID, ID), CandidatePath> = HashMap::new();
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
                    info!(
                        "Got {} possible paths from adversary {}. {:?}, {:?}, {}",
                        phase1_paths.len(),
                        adversary_id,
                        self.routing_metric,
                        self.payment_parts,
                        self.amount
                    );
                    let mut compute_all_paths = false;
                    for (idx, p_i) in phase1_paths.iter().enumerate() {
                        trace!("Looking at path {}", idx);
                        let mut sd_potential = HashSet::new();
                        if let Some(path_from_adv) = self.compute_shortest_paths_from(
                            &p_i.path.dest,
                            amount_to_succ,
                            &adversary_id.clone(),
                            &mut shortest_paths,
                        ) {
                            // TODO: Source can stay the same
                            let mut p_i_prime = p_i.clone();
                            if !p_i_prime.path.get_involved_nodes().contains(&adversary_id)
                                && !p_i_prime.path.get_involved_nodes().contains(&pred)
                            {
                                p_i_prime.path.hops.push_front((
                                    adversary_id.clone(),
                                    0,
                                    0,
                                    String::default(),
                                ));
                                p_i_prime.path.hops.push_front((
                                    pred.clone(),
                                    0,
                                    0,
                                    String::default(),
                                ));
                            }
                            if Self::is_potential_destination(
                                p_i, // or p_i_prime
                                &path_from_adv,
                                &adversary_id,
                                ttl_to_rx,
                            ) {
                                // phase 2, step 2 - check if the node is a potential recipient
                                rx_anon_set.insert(p_i.path.dest.clone());
                                info!("Destination found.");
                                // phase 2 - step 3
                                if self.is_pred_definitive_sender(
                                    &p_i_prime,
                                    &pred,
                                    amount_to_succ,
                                    &mut shortest_paths,
                                ) {
                                    info!("Found definitive sender.");
                                    sd_potential.insert(pred.clone()); // only possible sender for
                                    continue;
                                    // rec_i
                                } else {
                                    trace!("Definitive sender not found. Looking at all paths..");
                                    sd_potential.insert(pred.clone());
                                    compute_all_paths = true;
                                }
                            } else {
                                debug!("Destination not possible. Moving to next reachable path.");
                                continue;
                            }
                        }
                        if compute_all_paths {
                            let pot_senders_for_r = self.find_all_potential_senders(
                                p_i,
                                &mut shortest_paths,
                                amount_to_succ,
                            );
                            sd_potential.extend(pot_senders_for_r);
                        }
                        sd_anon_set = sd_anon_set.union(&sd_potential).cloned().collect();
                    }
                    let correct_recipient = rx_anon_set.contains(&payment.dest);
                    let correct_source = sd_anon_set.contains(&payment.source);
                    all_anonymits_sets.lock().unwrap().push(AnonymitySet {
                        sender: sd_anon_set.len(),
                        recipient: rx_anon_set.len(),
                        correct_recipient,
                        correct_source,
                    });
                }
            });
        });
        if let Ok(arc) = Arc::try_unwrap(all_anonymits_sets) {
            if let Ok(mutex) = arc.into_inner() {
                mutex
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    fn is_potential_destination(
        p_i: &CandidatePath,
        path_from_adv: &CandidatePath,
        adversary: &ID,
        ttl: usize,
    ) -> bool {
        info!("Checking if {} is a potential destination.", p_i.path.dest);
        if ttl == 0 {
            true
        } else {
            Path::is_equal(&p_i.path.subpath_from(adversary), &path_from_adv.path.hops)
        }
    }

    /// Compare shortest path from the pred to the path known by the adversary
    fn is_pred_definitive_sender(
        &self,
        p_i_prime: &CandidatePath,
        pred: &ID,
        amount: usize,
        all_shortest_paths: &mut HashMap<(ID, ID), CandidatePath>,
    ) -> bool {
        debug!("Looking at predecessor's {} path.", pred);
        if let Some(path_from_pre) =
            self.compute_shortest_paths_from(&p_i_prime.path.dest, amount, pred, all_shortest_paths)
        {
            // Step 3 - pre cannot be an intermediary but must be the only sender of the
            // transaction since it preceded A
            !Path::is_equal(&path_from_pre.path.hops, &p_i_prime.path.hops)
        } else {
            false
        }
    }

    /// Given a node, return all neighbours of the node than are not in the path since we haven't
    /// been able to determine the sender with certainty so we now look for all possible sources
    /// Phase 2, Step 4
    fn find_all_potential_senders(
        &self,
        p_i: &CandidatePath,
        all_shortest_paths: &mut HashMap<(ID, ID), CandidatePath>,
        amount: usize,
    ) -> HashSet<ID> {
        info!("Adding all remaining nodes to sources");
        // step 4
        let mut possible_sources = HashSet::new();
        let get_neighbours = |p_n: &CandidatePath| -> HashSet<ID> {
            // add all neighbours of N that are not in P[N] as a potential senders
            // TODO: Includes sender and reciver
            let mut sources = HashSet::new();
            if p_i.path.is_subpath(&p_n.path) {
                sources.extend(
                    self.graph
                        .get_outedges(&p_n.path.src) // outedges = inedges since the graph is symmetric
                        .iter()
                        .filter(|e| !p_n.path.get_involved_nodes().contains(&e.destination))
                        .map(|e| e.destination.clone())
                        .collect::<HashSet<ID>>(),
                );
            }
            sources
        };
        let rec = p_i.path.dest.to_owned();
        for n in self.graph.get_node_ids() {
            if n == rec {
                continue;
            }
            if let Some(path_from_n) =
                self.compute_shortest_paths_from(&rec, amount, &n, all_shortest_paths)
            {
                possible_sources.extend(get_neighbours(&path_from_n));
            }
        }
        possible_sources
    }

    /// Compute a path to a potential receiver from all nodes in the graph and eturns a map of <source, CandidatePath>
    fn compute_shortest_paths_from(
        &self,
        rec: &ID,
        amount: usize,
        src: &ID,
        all_shortest_paths: &mut HashMap<(ID, ID), CandidatePath>,
    ) -> Option<CandidatePath> {
        // 1. computes paths from src to the last node in the path
        // - first node "N" of the computed path is an intermediary and charges a fee
        if rec == src {
            warn!("Not looking for shortest path between src==rec");
            return None;
        }
        let path =
            if let Some(path_from_src) = all_shortest_paths.get(&(src.to_owned(), rec.clone())) {
                Some(path_from_src.clone())
            } else {
                let graph = self.graph.clone();
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
                    // store computed paths in hashmap <src, path>
                    all_shortest_paths.insert((src.clone(), rec.clone()), candidate_path.clone());
                    Some(candidate_path)
                } else {
                    None
                }
            };
        path
    }

    /// Looks for all paths with at most DEPTH many hops that are reachable from the node
    fn get_all_reachable_paths(
        graph: &Graph,
        next: &ID,
        amount: usize,
        ttl: usize,
    ) -> Option<Vec<CandidatePath>> {
        info!("Looking for all paths from {} reachable in {}.", next, ttl);
        let mut paths = vec![];
        for edge in graph.get_outedges(next) {
            let timelock_next = edge.cltv_expiry_delta;
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
            } else if timelock_next < ttl && edge.capacity >= amount {
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
                    } else if total_timelock < ttl && second_hop.capacity >= amount {
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

    fn extract_tx_info(p: &CandidatePath, adversary: &ID) -> (ID, ID, usize, usize) {
        let pred = p.path.get_pred(adversary);
        let succ = p.path.get_succ(adversary);
        let adv_pos = p.path.hops.iter().position(|n| n.0.eq(adversary));
        let (amount_to_succ, ttl_to_rx) = match adv_pos {
            Some(idx) => {
                let mut amount_to_succ = 0;
                let mut ttl_to_rx = 0;
                for hop in (idx + 1..p.path.hops.len()).rev() {
                    amount_to_succ += p.path.hops[hop].1;
                    ttl_to_rx += p.path.hops[hop].2;
                }
                (amount_to_succ, ttl_to_rx)
            }
            None => (usize::default(), usize::default()),
        };
        (pred, succ, amount_to_succ, ttl_to_rx)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

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
        let mut graph = simulator.graph.clone();
        let amount = 100;
        let adversary = "chan".to_string();
        let pred = "dina".to_string();
        graph.remove_node(&adversary);
        graph.remove_node(&pred);
        // dina -> chan -> bob -> alice
        let next_reachable = "bob".to_string();
        let ttl = 40; // bob as destination
        let reachable_path =
            Simulation::get_all_reachable_paths(&graph, &next_reachable, amount, ttl).unwrap();
        assert_eq!(reachable_path.len(), 1); // only alice as destination
    }

    #[test]
    fn add_all_senders() {
        let simulator = crate::attempt::tests::init_sim(None, None);
        let amount = 100;
        let dest = "dina".to_string();
        let p_i = CandidatePath {
            path: Path {
                src: "chan".to_owned(),
                dest: dest.to_owned(),
                hops: VecDeque::from([
                    ("chan".to_owned(), 0, 0, "".to_owned()),
                    (dest.to_owned(), 0, 0, "".to_owned()),
                ]),
            },
            weight: 0.0,
            amount: 0,
            time: 0,
        };
        // alice's neighbours
        let mut shortest_paths = HashMap::from([
            (
                ("alice".to_string(), dest.clone()),
                CandidatePath {
                    path: p_i.path.clone(),
                    weight: 0.0,
                    amount: 0,
                    time: 0,
                },
            ),
            (
                ("bob".to_string(), dest.clone()),
                CandidatePath {
                    path: Path {
                        src: "bob".to_owned(),
                        dest: dest.clone(),
                        hops: VecDeque::from([
                            ("chan".to_owned(), 0, 0, "".to_owned()),
                            ("bob".to_owned(), 0, 0, "".to_owned()),
                            (dest.to_owned(), 0, 0, "".to_owned()),
                        ]),
                    },
                    weight: 0.0,
                    amount: 0,
                    time: 0,
                },
            ),
        ]);
        let actual = simulator.find_all_potential_senders(&p_i, &mut shortest_paths, amount);
        let expected = HashSet::from(["bob".to_owned()]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn pred_is_distinct_sender() {
        // this doesn't make much sense but serves as a sanity test for path comparisons
        let simulator = crate::attempt::tests::init_sim(None, None);
        let amount = 100;
        let pre = "bob".to_string();
        let next = "dina".to_string();
        // path from pre to next must not be equal to the found path
        let p_i_prime = CandidatePath {
            path: Path {
                src: "bob".to_owned(),
                dest: "alice".to_owned(),
                hops: VecDeque::from([
                    ("pre".to_owned(), 0, 0, "".to_owned()),
                    ("adv".to_owned(), 0, 0, "".to_owned()),
                    ("bob".to_owned(), 0, 0, "".to_owned()),
                    ("alice".to_owned(), 0, 0, "".to_owned()),
                ]),
            },
            weight: 0.0,
            amount: 0,
            time: 0,
        };
        let path_from_pre = CandidatePath {
            path: Path {
                src: "bob".to_owned(),
                dest: "alice".to_owned(),
                hops: VecDeque::from([
                    ("pre".to_owned(), 0, 0, "".to_owned()),
                    ("adv".to_owned(), 0, 0, "".to_owned()),
                    ("bob".to_owned(), 0, 0, "".to_owned()),
                    ("dina".to_owned(), 0, 0, "".to_owned()),
                ]),
            },
            weight: 0.0,
            amount: 0,
            time: 0,
        };
        let mut shortest_paths = HashMap::from([((pre.to_owned(), next), path_from_pre)]);
        assert!(simulator.is_pred_definitive_sender(&p_i_prime, &pre, amount, &mut shortest_paths));
    }

    #[test]
    fn potential_destination() {
        let adversary = "chan".to_string();
        let ttl = 15; // bob-chan+dina ttl
                      // path chan -> dina
        let p_i_prime = CandidatePath {
            path: Path {
                src: String::default(),
                dest: String::default(),
                hops: VecDeque::from([
                    ("alice".to_string(), 5175, 55, "alice1".to_string()),
                    ("bob".to_string(), 100, 40, "bob2".to_string()),
                    ("chan".to_string(), 75, 15, "chan2".to_string()),
                    ("dina".to_string(), 5000, 0, "dina1".to_string()),
                ]),
            },
            weight: 5175.0,
            amount: 5175,
            time: 90,
        };
        let path_from_adv = CandidatePath {
            path: Path {
                src: String::default(),
                dest: String::default(),
                hops: VecDeque::from([
                    ("chan".to_string(), 75, 15, "chan2".to_string()),
                    ("dina".to_string(), 5000, 0, "dina1".to_string()),
                ]),
            },
            weight: 5175.0,
            amount: 5175,
            time: 90,
        };
        assert!(Simulation::is_potential_destination(
            &p_i_prime,
            &path_from_adv,
            &adversary,
            ttl
        ));
        let path_from_adv = CandidatePath {
            path: Path {
                src: String::default(),
                dest: String::default(),
                hops: VecDeque::from([
                    ("alice".to_string(), 5175, 55, "alice1".to_string()),
                    ("bob".to_string(), 100, 40, "bob2".to_string()),
                    ("chan".to_string(), 75, 15, "chan2".to_string()),
                    ("dina".to_string(), 5000, 0, "dina1".to_string()),
                ]),
            },
            weight: 5175.0,
            amount: 5175,
            time: 90,
        };
        assert!(!Simulation::is_potential_destination(
            &p_i_prime,
            &path_from_adv,
            &adversary,
            ttl
        ));
    }

    #[test]
    fn get_tx_info() {
        let p = Path {
            src: String::from("alice"),
            dest: String::from("dina"),
            hops: VecDeque::from([
                ("alice".to_string(), 5175, 55, "alice1".to_string()),
                ("bob".to_string(), 100, 40, "bob2".to_string()),
                ("chan".to_string(), 75, 15, "chan2".to_string()),
                ("dina".to_string(), 5000, 0, "dina1".to_string()),
            ]),
        };
        let path = CandidatePath::new_with_path(p);
        let adversary = "bob".to_string();
        let (actual_pred, actual_succ, actual_amt, actual_ttl) =
            Simulation::extract_tx_info(&path, &adversary);
        let (expected_pred, expected_succ, expected_amt, expected_ttl) =
            (String::from("alice"), String::from("chan"), 5075, 15);
        assert_eq!(actual_pred, expected_pred);
        assert_eq!(actual_succ, expected_succ);
        assert_eq!(actual_amt, expected_amt);
        assert_eq!(actual_ttl, expected_ttl);
        let adversary = "alice".to_string();
        let (actual_pred, actual_succ, actual_amt, actual_ttl) =
            Simulation::extract_tx_info(&path, &adversary);
        let (expected_pred, expected_succ, expected_amt, expected_ttl) =
            (String::from(""), String::from("bob"), 5175, 55);
        assert_eq!(actual_pred, expected_pred);
        assert_eq!(actual_succ, expected_succ);
        assert_eq!(actual_amt, expected_amt);
        assert_eq!(actual_ttl, expected_ttl);
    }

    #[test]
    fn attempt_deanonymisation() {
        let number_of_adversaries = 4; // all 4 nodes
        let source = "alice".to_string();
        let dest = "chan".to_string();
        let mut simulator = crate::attempt::tests::init_sim(None, Some(number_of_adversaries));
        let sim_result = simulator.run(vec![(source, dest)].into_iter());
        assert_eq!(sim_result.num_succesful, 1);
        assert_eq!(
            simulator.adversaries[0].selection_strategy,
            crate::AdversarySelection::Random
        );
    }
}
