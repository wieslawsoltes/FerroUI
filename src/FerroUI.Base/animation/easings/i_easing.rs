/// Defines the interface for easing classes.
pub trait IEasing: 'static {
    /// Returns the transformed value for a linear progress.
    fn ease(&self, progress: f64) -> f64;

    /// Whether the object is an easing class: upstream, a class deriving from
    /// the `Easing` base class, which code tests for with a cast. True for
    /// every easing of this module and by default, because implementing this
    /// trait is how an easing class is written here; an object that stands
    /// for a bare implementation of the contract (not an easing class)
    /// answers false.
    fn derives_from_easing(&self) -> bool {
        true
    }
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IEasing {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
