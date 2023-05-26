mod adversaries;
mod deanonymisation;
pub mod diversity;
mod failures;

use crate::io::PaymentInfo;
use serde::Serialize;
use std::collections::HashMap;

pub use diversity::*;

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

    /// Number of times a payment part contained at least one adversary
    pub part_hits: usize,
    pub part_hits_successful: usize,
    /// The number of adversaries (key) in a payment and how often (value). parts as individual
    /// payments
    pub adv_count: HashMap<usize, usize>,
    /// The number of adversaries (key) in a successful payment and how often (value). sum of parts
    pub adv_count_successful: HashMap<usize, usize>,
    // independent of the number of adversaries
    pub(crate) anonymity_sets: Vec<AnonymitySet>, // one for each adversary in a payment path (MPP payments are treated like separate payments
    /// Contains the updated sim results when some nodes are removed
    pub targeted_attack: TargetedAttack,
    /// Number of payments an adversary could corelate (incl. failed + successful payments)
    pub correlated: usize,
    /// Number of successful payments an adversary could corelate
    pub correlated_successful: usize,
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
