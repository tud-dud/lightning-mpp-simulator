use crate::ID;

#[derive(Debug, Clone)]
pub(crate) struct Route {
    /// Route a payment: one hop for single payments and multiple for multipath
    pub(crate) hops: Vec<ID>,
}

impl Route {}

#[cfg(test)]
mod tests {}
