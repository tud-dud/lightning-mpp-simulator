use itertools::Itertools;
use std::collections::VecDeque;

use crate::{graph::Graph, Edge, EdgeWeight, Node, ID};

/// Describes an edge between two nodes
#[derive(Debug, Clone)]
pub(crate) struct Hop {
    src: ID,
    dest: ID,
}

/// Describes a path between two nodes
#[derive(Debug, Clone)]
pub(crate) struct Path {
    src: ID,
    dest: ID,
    /// the edges of the path describe from sender to receiver
    hops: VecDeque<Hop>,
}

/// Pathfinding object
#[derive(Debug, Clone)]
pub(crate) struct PathFinder {
    /// Network topolgy graph
    graph: Box<Graph>,
    /// Node looking for a route
    src: ID,
    /// the destination node
    dest: ID,
    current_node: ID,
}

/// A path that we may use to route from src to dest
#[derive(Debug, Clone)]
pub(crate) struct CandidatePath {
    path: Path,
    /// The aggregated path weight
    weight: EdgeWeight,
    /// The aggregated path amount
    amount: usize,
    /// The aggregated time
    time: u16,
}

impl Path {
    pub(crate) fn new(src: ID, dest: ID) -> Self {
        let hops = VecDeque::new();
        Self { src, dest, hops }
    }

    fn get_involved_nodes(&self) -> Vec<ID> {
        // add the source to the path (might not be necessary)
        let mut node_ids = Vec::from([self.src.to_string()]);
        node_ids.extend(self.hops.iter().map(|h| h.src.clone()));
        node_ids.extend(self.hops.iter().map(|h| h.dest.clone()));
        node_ids.into_iter().unique().collect()
    }
}

impl Hop {}

impl CandidatePath {
    pub(crate) fn new(src: ID, dest: ID, amount: usize) -> Self {
        let path = Path::new(src, dest);
        let time = 0;
        let weight = 0;
        CandidatePath {
            path,
            weight,
            amount,
            time,
        }
    }
}

impl PathFinder {
    pub(crate) fn new(src: ID, dest: ID, amount: usize, graph: Box<Graph>) -> Self {
        let current_node = dest.clone();
        Self {
            graph,
            src,
            dest,
            current_node,
        }
    }

    /// Returns a route, the total amount due and lock time and none if no route is found
    pub(crate) fn find_path(&mut self) {
        /*if let Some(next_to_visit) = self.visit_next() {

        }*/
    }

    /*fn visit_next(&self) -> Option<ID> {

    }*/
}

#[cfg(test)]
mod tests {
    use std::collections::vec_deque;

    use super::*;

    #[test]
    fn get_nodes_involved_in_path() {
        let mut path = Path::new(String::from("a"), String::from("e"));
        let hops = VecDeque::from([
            Hop {
                src: "a".to_string(),
                dest: "b".to_string(),
            },
            Hop {
                src: "b".to_string(),
                dest: "c".to_string(),
            },
            Hop {
                src: "c".to_string(),
                dest: "d".to_string(),
            },
            Hop {
                src: "d".to_string(),
                dest: "e".to_string(),
            },
        ]);
        path.hops = hops;
        let actual = path.get_involved_nodes();
        let expected = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
        ];
        assert_eq!(actual, expected);
    }
}
