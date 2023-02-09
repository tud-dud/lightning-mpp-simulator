mod adversaries;
mod deanonymisation;
mod distance;
mod resillience;

use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Adversaries {
    pub(crate) selection_strategy: crate::AdversarySelection,
    pub(crate) statistics: Vec<Statistics>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Statistics {
    pub(crate) percentage: usize,
    /// Number of times an adversary was included a payment path
    pub hits: usize,
    /// Number of times an adversary was included a successful payment path
    pub hits_successful: usize,
    pub anonymity_sets: Vec<AnonymitySet>, // one for each adversary in a payment path (MPP payments are treated like separate payments
    /// Number of attacks , number of payments
    pub attacked_all: HashMap<usize, usize>,
    /// Number of attacks , number of successful payments
    pub attacked_successful: HashMap<usize, usize>,
    /// Contains the updated sim results when some nodes are removed
    pub targeted_attack: TargetedAttack,
}

/// All the distances in the simulated payments' paths
#[derive(Debug, Serialize, Clone)]
pub struct PathDistances(pub Vec<usize>);

#[derive(Debug, Serialize, Clone)]
pub struct AnonymitySet {
    /// Possible senders
    sender: usize,
    /// Possible recipients
    recipient: usize,
    /// True if the recipient is included in the recipient anonymity set
    correct_recipient: bool,
    correct_source: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct TargetedAttack {
    pub num_succesful: usize,
    pub num_failed: usize,
}
