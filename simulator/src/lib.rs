mod core_types;

pub use core_types::*;

pub type ID = String;
pub(crate) type Petgraph = petgraph::graph::DiGraph<network_parser::Node, network_parser::Edge>;
