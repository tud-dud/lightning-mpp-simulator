use crate::{
    graph::Graph, payment::Payment, traversal::pathfinding, Adversaries, Edge, Simulation, ID,
};

use std::collections::{HashSet, VecDeque};

impl Simulation {
    pub(crate) fn eval_adversaries(&mut self) {
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
            Self::deanonymise_tx_pairs(&self.successful_payments, &adv, &self.graph.clone());
            self.adversaries.push(Adversaries {
                percentage: percent,
                hits,
                hits_successful,
            });
        }
    }

    fn deanonymise_tx_pairs(
        payments: &[Payment],
        adversaries: &[ID],
        graph: &Graph,
    ) -> (Vec<usize>, Vec<usize>, usize) {
        let mut sd_anon_set_sizes = vec![];
        let mut rx_anon_set_sizes = vec![];
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
                    g.edges =
                        pathfinding::PathFinder::remove_inadequate_edges(graph, amount_to_succ); //hm - which amount?
                    Self::get_all_reachable_paths(&g, &succ, ttl_to_rx);
                }
            }
        }
        (sd_anon_set_sizes, rx_anon_set_sizes, 0)
    }

    /// Looks for all paths with at most DEPTH many hops that are reachable from the node
    fn get_all_reachable_paths(graph: &Graph, next: &ID, ttl: usize) {
        let mut depth = 0;

        let mut filter = |current_timelock: usize, node: &ID| {
            let out = graph
                .get_outedges(node)
                .into_iter()
                .filter(|e| e.cltv_expiry_delta + current_timelock <= ttl)
                .collect::<Vec<Edge>>();
            out
        };
        let mut stack = vec![graph.get_outedges(next)];
        while let Some(out_edges) = stack.pop() {
            let mut timelock = 0;
            for node in out_edges {
                stack.push(
                    graph
                        .get_outedges(&node.source)
                        .into_iter()
                        .filter(|e| e.cltv_expiry_delta <= ttl)
                        .collect(),
                );
            }
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
}
