use crate::{
    payment::Payment,
    stats::{Adversaries, Diversity},
    traversal::pathfinding::CandidatePath,
    WeightPartsCombi,
};
use serde::Serialize;

pub mod output;
pub use output::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output(Vec<Results>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub amount: usize,
    pub total_num: usize,
    pub num_succesful: usize,
    pub num_failed: usize,
    pub payments: Vec<PaymentInfo>,
    pub adversaries: Vec<Adversaries>,
    pub path_distances: Vec<usize>,
    pub path_diversity: Vec<Diversity>,
}

/// run and reports
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    pub scenario: WeightPartsCombi,
    pub run: u64,
    pub reports: Vec<Report>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PaymentInfo {
    pub(crate) id: usize,
    pub succeeded: bool,
    /// Number of parts this payment has been split into
    pub num_parts: usize,
    pub htlc_attempts: usize,
    pub used_paths: Vec<PathInfo>,
    pub failed_paths: Vec<PathInfo>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// Describes the path used by amounts - may or may not have failed
pub struct PathInfo {
    /// the amount that was transferred by this path
    pub amount: usize,
    /// The aggregated path fees describing how costly the path is
    pub total_fees: usize,
    pub total_time: usize,
    pub path_len: usize,
}

impl PathInfo {
    pub(super) fn from_payment(paths: &[CandidatePath]) -> Vec<Self> {
        paths
            .iter()
            .filter(|p| !p.path.hops.is_empty())
            .map(|path| Self {
                amount: crate::to_sat(path.path_amount()),
                total_fees: crate::to_sat(path.path_fees()),
                total_time: path.time,
                path_len: path.path.path_length(),
            })
            .collect()
    }
}

impl PaymentInfo {
    pub(super) fn from_payment(payment: &Payment) -> Self {
        let used_paths = PathInfo::from_payment(&payment.used_paths);
        let failed_paths = PathInfo::from_payment(&payment.failed_paths);
        Self {
            id: payment.payment_id,
            succeeded: payment.succeeded,
            num_parts: payment.num_parts,
            htlc_attempts: payment.htlc_attempts,
            used_paths,
            failed_paths,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traversal::pathfinding::{CandidatePath, Path};
    use std::collections::VecDeque;

    #[test]
    fn payment_info_from_payment() {
        let used_paths = vec![
            CandidatePath {
                path: Path {
                    src: "bob".to_string(),
                    dest: "alice".to_string(),
                    hops: VecDeque::from([
                        ("bob".to_string(), 7010, 5, "bob-carol".to_string()),
                        ("carol".to_string(), 1010, 5, "carol-alice".to_string()),
                        ("alice".to_string(), 6000, 0, "alice-carol".to_string()),
                    ]),
                },
                weight: 1010.0,
                amount: 2010,
                time: 5,
            },
            CandidatePath {
                path: Path {
                    src: "bob".to_string(),
                    dest: "alice".to_string(),
                    hops: VecDeque::from([
                        ("bob".to_string(), 9000, 10, "bob-eve".to_string()),
                        ("eve".to_string(), 2000, 5, "eve-carol".to_string()),
                        ("carol".to_string(), 1000, 5, "carol-alice".to_string()),
                        ("alice".to_string(), 6000, 0, "alice-carol".to_string()),
                    ]),
                },
                weight: 3000.0,
                amount: 5030,
                time: 10,
            },
        ];
        let source = "bob".to_string();
        let dest = "alice".to_string();
        let amount_msat = 2000;
        let payment = Payment {
            payment_id: 0,
            source: source.clone(),
            dest: dest.clone(),
            amount_msat,
            succeeded: false,
            min_shard_amt: 10,
            htlc_attempts: 2,
            num_parts: 1,
            used_paths,
            failed_amounts: Vec::default(),
            successful_shards: Vec::default(),
            failed_paths: vec![],
        };
        let actual = PaymentInfo::from_payment(&payment);
        let expected = PaymentInfo {
            id: 0,
            num_parts: 1,
            htlc_attempts: 2,
            succeeded: false,
            used_paths: vec![
                PathInfo {
                    total_fees: 1,
                    total_time: 5,
                    path_len: 2,
                },
                PathInfo {
                    total_fees: 3,
                    total_time: 10,
                    path_len: 3,
                },
            ],
            failed_paths: vec![],
        };
        assert_eq!(actual, expected);
    }
}
