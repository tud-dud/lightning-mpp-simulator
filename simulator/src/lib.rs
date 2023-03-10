use lazy_static::lazy_static;
use rand::{rngs::StdRng, SeedableRng};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};

pub mod core_types;
pub mod io;
pub mod payments;
pub mod sim;
pub(crate) mod stats;
pub mod traversal;

pub use core_types::*;
pub use payments::*;
pub use sim::*;
pub use stats::diversity::*;
pub use traversal::pathfinding::*;

pub type ID = String;
pub type PaymentId = usize;
pub type Node = network_parser::Node;
pub type Edge = network_parser::Edge;
pub type EdgeWeight = ordered_float::OrderedFloat<f32>;

pub(crate) static SIM_DELAY_IN_SECS: f32 = 120.0;
/// Max number of hops in a path from an adversary's adjacent node
pub(crate) static _DEPTH: usize = 3;
/// Minimum amount of msats that can be sent in a shard
/// https://github.com/lightningnetwork/lnd/blob/master/routing/payment_session.go#L72
#[cfg(not(test))]
pub(crate) static MIN_SHARD_AMOUNT: usize = 10000 * SAT_SCALE;
#[cfg(test)]
pub(crate) static MIN_SHARD_AMOUNT: usize = 1000;
/// the default number of splits in LND
/// (https://github.com/lightningnetwork/lnd/blob/master/lnrpc/routerrpc/router_backend.go#L29)
pub(crate) static MAX_PARTS: usize = 16;
/// Scaling factor used to convert msats and sats
pub static SAT_SCALE: usize = 1000;

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
/// Enum combining RoutingMetric and PaymentParts enums- used to eval different scnerios
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum WeightPartsCombi {
    MinFeeSingle,
    MinFeeMulti,
    MaxProbSingle,
    MaxProbMulti,
}

/// How should the adversaries be selected
/// The caller must remember to reverse the scores for Low*
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Hash)]
pub enum AdversarySelection {
    Random,
    HighBetweenness(#[serde(skip)] PathBuf),
    HighDegree(#[serde(skip)] PathBuf),
}

lazy_static! {
    static ref RNG: Mutex<StdRng> = {
        let std_rng = StdRng::from_entropy();
        Mutex::new(std_rng)
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

/// Convert Satoshi to millisatoshi
pub fn to_millisatoshi(sat: usize) -> usize {
    sat * SAT_SCALE
}

/// Convert millisatoshi to Satoshi
pub fn to_sat(msat: usize) -> usize {
    msat / SAT_SCALE
}
