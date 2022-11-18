use crate::{
    traversal::pathfinding::{CandidatePath, Path, PathFinder},
    ID,
};
use log::{debug, trace};

impl PathFinder {
    /// includes finding number of parts to split payment
    pub(super) fn find_path_mpp_payment(&mut self) -> Option<Vec<CandidatePath>> {
        None
    }
}
