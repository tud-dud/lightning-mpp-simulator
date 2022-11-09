use crate::*;

#[derive(Debug, Clone)]
pub struct Path {
    source_id: Node,
    dest_id: Node,
    edges: Vec<Edge>,
}

// Route {
//  hop : Path for single
//  and Vec<Path> for AMP
// }
