mod adversaries;
mod deanonymisation;
pub mod diversity;
mod failures;

use crate::io::PaymentInfo;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Adversaries {
    pub selection_strategy: crate::AdversarySelection,
    pub statistics: Vec<Statistics>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    /// how many adversaries
    pub number: usize,
    /// Number of times a payment contained at least one adversary
    pub hits: usize,
    /// Number of times a successful payment contained at least one adversary
    pub hits_successful: usize,
    // independent of the number of adversaries
    pub(crate) anonymity_sets: Vec<AnonymitySet>, // one for each adversary in a payment path (MPP payments are treated like separate payments
    /// Contains the updated sim results when some nodes are removed
    pub targeted_attack: TargetedAttack,
    /// Number of payments an adversary could corelate (incl. failed + successful payments)
    pub correlated: usize,
    /// Number of successful payments an adversary could corelate
    pub correlated_successful: usize,
    /// probabilities based on equation in https://eprint.iacr.org/2020/303.pdf
    /// The probability that a path is vulnerable
    pub prone_paths_prob: f32,
    pub prone_paths_successful_prob: f32,
    /// The probability that a payment is vulnerable
    pub prone_payments_prob: f32,
    pub prone_payments_successful_prob: f32,
}

/// All the distances in the simulated payments' paths
#[derive(Debug, Default, Serialize, Clone, PartialEq, Eq)]
pub struct PathDistances(pub Vec<usize>);

/// All the diversity scorres in the simulated payments' paths
#[derive(Debug, Default, Serialize, Clone, PartialEq)]
pub struct PathDiversity(pub Vec<Diversity>);

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnonymitySet {
    /// Possible senders
    sender: usize,
    /// Possible recipients
    recipient: usize,
    /// True if the recipient is included in the recipient anonymity set
    correct_recipient: bool,
    correct_source: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TargetedAttack {
    pub total_num: usize,
    pub num_successful: usize,
    pub num_failed: usize,
    pub payments: Vec<PaymentInfo>,
    pub path_distances: PathDistances,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Diversity {
    pub lambda: f32,
    /// one value for each set of paths
    pub diversity: Vec<f32>,
}
