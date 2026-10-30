//! WinUI's state triggers: `StateTriggerBase`, `StateTrigger` and `AdaptiveTrigger`
//! (`Microsoft.UI.Xaml`; `dxaml/xcp/core/core/elements/StateTriggerBase.cpp`, `StateTrigger.cpp`,
//! `AdaptiveTrigger.cpp`, the qualifiers of `dxaml/xcp/components/qualifiers/` and
//! `dxaml/xcp/dxaml/lib/StateTriggerBase_Partial.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::VisualStateGroup;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size, StyledProperty, WeakRef,
};
use std::cell::{Cell, RefCell};

/// The base class of the triggers of a [`VisualState`](super::VisualState): while a trigger of a
/// state is active, the state is a candidate to be the current state of its group
/// (docs/porting/winui-calculator-gaps.md, section 5.7).
///
/// **The extension point of custom triggers.** A trigger of an application derives from this
/// class and calls [`set_active`](Self::set_active) when its condition changes (WinUI's protected
/// `SetActive`; it is public in Rust, which has no protected members, and is meant for the
/// derived trigger only):
///
/// ```ignore
/// #[repr(C)]
/// pub struct ControlSizeTrigger { base: StateTriggerBase, /* ... */ }
/// ferro_class!(ControlSizeTrigger: StateTriggerBase);
/// // ... on a size change of the source:
/// trigger.set_active(size.width >= trigger.min_width());
/// ```
///
/// Among the states of a group whose triggers are active, a [`StateTrigger`] or a custom trigger
/// wins over an [`AdaptiveTrigger`]; among adaptive triggers the larger `MinWindowWidth` wins,
/// then the larger `MinWindowHeight`; then the first in the order of the source.
#[repr(C)]
pub struct StateTriggerBase {
    base: FerroObject,
    active: Cell<bool>,
    // The group whose states this trigger selects, while the group's element is in a tree.
    owner: RefCell<Option<WeakRef<VisualStateGroup>>>,
}

ferro_class!(StateTriggerBase: FerroObject);
ferro_class_info!(StateTriggerBase { markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(StateTriggerBase: FerroObjectImpl);

impl StateTriggerBase {
    /// Creates the class data of a derived trigger (WinUI's protected constructor); the trigger
    /// is not active.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), active: Cell::new(false), owner: RefCell::new(None) }
    }

    /// Sets whether the trigger is active (WinUI's protected `SetActive`). The group of the
    /// trigger selects its state again when the value changes.
    pub fn set_active(&self, value: bool) {
        if self.active.replace(value) != value {
            self.owner_changed();
        }
    }

    /// Whether the trigger is active (what the last [`set_active`](Self::set_active) set).
    pub(crate) fn is_active(&self) -> bool {
        self.active.get()
    }

    pub(crate) fn set_owner(&self, group: Option<&Ref<VisualStateGroup>>) {
        *self.owner.borrow_mut() = group.map(|group| group.downgrade());
    }

    /// The condition of the trigger changed: its group selects its state again.
    pub(crate) fn owner_changed(&self) {
        let owner = self.owner.borrow().as_ref().and_then(WeakRef::upgrade);
        if let Some(group) = owner {
            super::state_triggers::evaluate_group(&group);
        }
    }

    /// How the trigger qualifies its state for a window of `window` (WinUI's qualifiers): `None`
    /// when it does not.
    pub(crate) fn qualification(&self, window: Option<Size>) -> Option<TriggerRank> {
        if let Some(adaptive) = self.downcast_ref::<AdaptiveTrigger>() {
            return adaptive.adaptive_qualification(window);
        }
        self.is_active().then_some(TriggerRank { extensible: true, min_width: 0.0, min_height: 0.0 })
    }
}

/// The rank of a qualified trigger (WinUI's `VariantMap` order): extensible triggers (state
/// triggers and custom triggers) before adaptive ones, then the larger minimum width, then the
/// larger minimum height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TriggerRank {
    extensible: bool,
    min_width: f64,
    min_height: f64,
}

impl TriggerRank {
    /// Whether this rank wins over `other` (the first of equal ranks wins).
    pub(crate) fn beats(&self, other: &TriggerRank) -> bool {
        (self.extensible, self.min_width, self.min_height) > (other.extensible, other.min_width, other.min_height)
    }
}

/// A trigger that is active while its [`is_active`](Self::is_active) is `true` (WinUI's
/// `StateTrigger`): a state bound to a condition of the application
/// (`<StateTrigger IsActive="{Binding IsWide}"/>`).
#[repr(C)]
pub struct StateTrigger {
    base: StateTriggerBase,
}

ferro_class!(StateTrigger: StateTriggerBase);
ferro_class_info!(StateTrigger { new: StateTrigger::new, markup: { namespace: "FerroUI.Controls" } });

impl FerroObjectImpl for StateTrigger {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::is_active_property().as_property() {
            this.set_active(this.is_active());
        }
    }
}

ferro_properties! { impl StateTrigger {
    /// Defines the `IsActive` property: whether the trigger is active. Default `false`.
    pub fn is_active_property() -> StyledProperty<bool> {
        FerroProperty::register::<StateTrigger, _>("IsActive", false)
    }
} }

impl StateTrigger {
    pub fn construct() -> Self {
        Self { base: StateTriggerBase::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the trigger is active.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }

    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
}

/// A trigger that is active while the window is at least `MinWindowWidth` wide and
/// `MinWindowHeight` high (WinUI's `AdaptiveTrigger`).
///
/// The size is the client size of the window (the presentation source) of the element whose
/// states the trigger selects, not the size of that element. A value of -1 (the default) leaves
/// that dimension out; with both at -1 the trigger never applies.
#[repr(C)]
pub struct AdaptiveTrigger {
    base: StateTriggerBase,
}

ferro_class!(AdaptiveTrigger: StateTriggerBase);
ferro_class_info!(AdaptiveTrigger { new: AdaptiveTrigger::new, markup: { namespace: "FerroUI.Controls" } });

impl FerroObjectImpl for AdaptiveTrigger {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::min_window_width_property().as_property()
            || change.property() == Self::min_window_height_property().as_property()
        {
            this.owner_changed();
        }
    }
}

ferro_properties! { impl AdaptiveTrigger {
    /// Defines the `MinWindowWidth` property: the smallest window width at which the trigger
    /// applies. Default -1 (no condition on the width).
    pub fn min_window_width_property() -> StyledProperty<f64> {
        FerroProperty::register::<AdaptiveTrigger, _>("MinWindowWidth", -1.0)
    }

    /// Defines the `MinWindowHeight` property: the smallest window height at which the trigger
    /// applies. Default -1 (no condition on the height).
    pub fn min_window_height_property() -> StyledProperty<f64> {
        FerroProperty::register::<AdaptiveTrigger, _>("MinWindowHeight", -1.0)
    }
} }

impl AdaptiveTrigger {
    pub fn construct() -> Self {
        Self { base: StateTriggerBase::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn min_window_width(&self) -> f64 {
        self.get_value(Self::min_window_width_property())
    }

    pub fn set_min_window_width(&self, value: f64) {
        self.set_value(Self::min_window_width_property(), value);
    }

    pub fn min_window_height(&self) -> f64 {
        self.get_value(Self::min_window_height_property())
    }

    pub fn set_min_window_height(&self, value: f64) {
        self.set_value(Self::min_window_height_property(), value);
    }

    /// WinUI's `MultiQualifier` of a `MinWidthQualifier` and a `MinHeightQualifier`
    /// (`QualifierFactory.cpp`): a negative value is no condition; no condition at all never
    /// qualifies.
    fn adaptive_qualification(&self, window: Option<Size>) -> Option<TriggerRank> {
        let (min_width, min_height) = (self.min_window_width(), self.min_window_height());
        if min_width < 0.0 && min_height < 0.0 {
            return None;
        }
        let window = window?;
        let fits = (min_width < 0.0 || min_width <= window.width) && (min_height < 0.0 || min_height <= window.height);
        fits.then_some(TriggerRank { extensible: false, min_width, min_height })
    }
}
