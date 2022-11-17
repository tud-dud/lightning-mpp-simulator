use crate::traversal::pathfinding::Hop;

#[derive(Debug, Clone)]
pub(crate) struct Route {
    /// Route a payment: one hop for single payments and multiple for multipath
    pub(crate) hops: Vec<Hop>,
}

impl Route {}

#[cfg(test)]
mod tests {}
