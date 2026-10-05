use crate::{Ref, Visual};
use std::cmp::Ordering;

/// Orders visuals by their Z index.
pub struct ZIndexComparer;

impl ZIndexComparer {
    /// Compares two visuals by Z index (the comparison is stable: use it
    /// with a stable sort to keep collection order among equal indices).
    pub fn compare(x: &Ref<Visual>, y: &Ref<Visual>) -> Ordering {
        x.z_index().cmp(&y.z_index())
    }

    /// Compares two optional visuals; a missing visual has Z index zero.
    pub fn compare_optional(x: Option<&Ref<Visual>>, y: Option<&Ref<Visual>>) -> Ordering {
        x.map_or(0, |v| v.z_index()).cmp(&y.map_or(0, |v| v.z_index()))
    }
}
