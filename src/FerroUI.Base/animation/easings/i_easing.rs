/// Defines the interface for easing classes.
///
/// The `Any` supertrait lets optional transition adapters recognize a physical easing
/// and retain its parameters across interruptions, while ordinary easings keep the
/// normalized-progress contract. Every existing `'static` easing also implements `Any`.
pub trait IEasing: std::any::Any {
    /// Returns the transformed value for a linear progress.
    fn ease(&self, progress: f64) -> f64;

    /// The easing as the render thread evaluates it: a key frame animation of
    /// the compositor runs there. An easing without parameters returns
    /// itself; one with parameters returns a copy with the parameters it has
    /// now.
    fn to_shared(&self) -> std::sync::Arc<SharedEasing>;

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

/// An easing that can be evaluated on any thread; see [`IEasing::to_shared`].
pub type SharedEasing = dyn IEasing + Send + Sync;
