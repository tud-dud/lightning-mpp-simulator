use lazy_static::lazy_static;
use rand::{rngs::SmallRng, SeedableRng};
use std::sync::Mutex;

pub mod core_types;
mod payments;
pub mod sim;
mod traversal;

pub use core_types::*;
pub use sim::*;

pub type ID = String;
pub type PaymentId = usize;
pub type Node = network_parser::Node;
pub type Edge = network_parser::Edge;

lazy_static! {
    static ref RNG: Mutex<SmallRng> = {
        let small_rng = SmallRng::from_entropy();
        Mutex::new(small_rng)
    };
}
