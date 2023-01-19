use crate::{payment::Payment, Adversaries, Simulation, ID};

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
            self.adversaries.push(Adversaries {
                percentage: percent,
                hits,
                hits_successful,
            });
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
                    for i in 1..path.path.hops.len() - 1 {
                        if adv.contains(&path.path.hops[i].0) {
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
