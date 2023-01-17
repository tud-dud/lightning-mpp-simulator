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
            count_occurences(&self.successful_payments);
            count_occurences(&self.failed_payments);
            self.adversaries.push(Adversaries {
                percentage: percent,
                hits: adversary_hits,
                hits_successful: adversary_hits_successful,
            });
        }
    }
}
