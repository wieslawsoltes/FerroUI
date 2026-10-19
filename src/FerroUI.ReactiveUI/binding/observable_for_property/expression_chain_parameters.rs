//! `ExpressionChainParameters<TSender>`.

use ferroui_system::component_model::PropertyPath;
use std::rc::{Rc, Weak};

/// Compares two values of a property, for the gate that drops repeats.
pub type ValueComparer<V> = Rc<dyn Fn(&V, &V) -> bool>;

/// How one property path is to be observed (`ExpressionChainParameters<TSender>`).
// Deviation (DEVIATIONS.md, ReactiveUI): the source is held weakly (there is no garbage collector
// to collect the cycle between an object and the observation of its own properties), and the
// comparison of the values is a closure, `PartialEq` by default, where upstream uses
// `EqualityComparer<T>.Default`.
pub struct ExpressionChainParameters<T, V> {
    /// The root object of the path (`Source`).
    pub source: Weak<T>,
    /// The path: its text is the expression of the changes, its steps the links (`Expression`,
    /// `Links`).
    pub path: PropertyPath<Rc<T>, V>,
    /// Whether the values are observed before they change (`BeforeChange`).
    pub before_change: bool,
    /// Whether the first value is dropped (`SkipInitial`).
    pub skip_initial: bool,
    /// Whether a value equal to the last one is dropped (`IsDistinct`).
    pub is_distinct: bool,
    /// Whether the warning of a property that cannot notify is suppressed (`SuppressWarnings`).
    pub suppress_warnings: bool,
    /// How two values are compared; without it, no two values are equal.
    pub comparer: Option<ValueComparer<V>>,
    /// Whether a notification that repeats the value read after an attachment is kept. Upstream's
    /// run-time chain drops it once; its generated `WhenChanging` of one property, which observes
    /// `PropertyChanging` directly, keeps it.
    pub keeps_repeat_after_attachment: bool,
}

impl<T, V> Clone for ExpressionChainParameters<T, V> {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            path: self.path.clone(),
            before_change: self.before_change,
            skip_initial: self.skip_initial,
            is_distinct: self.is_distinct,
            suppress_warnings: self.suppress_warnings,
            comparer: self.comparer.clone(),
            keeps_repeat_after_attachment: self.keeps_repeat_after_attachment,
        }
    }
}

impl<T: 'static, V: PartialEq + 'static> ExpressionChainParameters<T, V> {
    /// The parameters of the observation of `path` on `source`, comparing the values with
    /// `PartialEq`.
    pub fn new(source: &Rc<T>, path: PropertyPath<Rc<T>, V>, before_change: bool, skip_initial: bool, is_distinct: bool, suppress_warnings: bool) -> Self {
        Self {
            source: Rc::downgrade(source),
            path,
            before_change,
            skip_initial,
            is_distinct,
            suppress_warnings,
            comparer: Some(Rc::new(|first: &V, second: &V| first == second)),
            keeps_repeat_after_attachment: false,
        }
    }
}

impl<T: 'static, V: 'static> ExpressionChainParameters<T, V> {
    /// The parameters of the observation of `path` on `source`, comparing the values with
    /// `comparer` (`None`: no two values are equal).
    #[allow(clippy::too_many_arguments)]
    pub fn with_comparer(
        source: &Rc<T>,
        path: PropertyPath<Rc<T>, V>,
        before_change: bool,
        skip_initial: bool,
        is_distinct: bool,
        suppress_warnings: bool,
        comparer: Option<ValueComparer<V>>,
    ) -> Self {
        Self { source: Rc::downgrade(source), path, before_change, skip_initial, is_distinct, suppress_warnings, comparer, keeps_repeat_after_attachment: false }
    }
}
