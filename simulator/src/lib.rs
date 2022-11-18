use lazy_static::lazy_static;
use rand::{rngs::SmallRng, SeedableRng};
use std::sync::Mutex;

pub mod core_types;
pub mod payments;
pub mod sim;
pub(crate) mod traversal;

pub use core_types::*;
pub use payments::*;
pub use sim::*;

pub type ID = String;
pub type PaymentId = usize;
pub type Node = network_parser::Node;
pub type Edge = network_parser::Edge;
pub type EdgeWeight = usize;

pub(crate) static SIM_DELAY_IN_SECS: f32 = 120.0;
/// Number of shortest paths to compute during pathfinding
pub(crate) static K: usize = 3;
/// Minimum amount of msats that can be sent in a shard
pub(crate) static MIN_SHARD_AMOUNT: usize = 100;

/// Metric to use when looking for a route
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RoutingMetric {
    /// Use Dijkstra to minimise fees along a route
    MinFee,
    /// Route based on probabilty of success
    MaxProb,
}

/// How should the payment be sent
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PaymentParts {
    /// Send the whole payment at once
    Single,
    /// Split the payment into multiple payments and route independently
    Split,
}

lazy_static! {
    static ref RNG: Mutex<SmallRng> = {
        let small_rng = SmallRng::from_entropy();
        Mutex::new(small_rng)
    };
}

impl clap::ValueEnum for RoutingMetric {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::MinFee, Self::MaxProb]
    }

    fn to_possible_value<'a>(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::MinFee => Some(clap::builder::PossibleValue::new("minfee")),
            Self::MaxProb => Some(clap::builder::PossibleValue::new("maxprob")),
        }
    }
}
