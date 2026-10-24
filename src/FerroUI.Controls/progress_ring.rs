//! WinUI's `ProgressRing` (`controls/dev/ProgressRing/ProgressRing.cpp`, `ProgressRing.h` and
//! `ProgressRing.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md). The motion follows the animated visuals
//! `AnimatedVisuals/ProgressRingIndeterminate.cpp` and `ProgressRingDeterminate.cpp` of the same
//! commit.

use crate::automation::peers::{AutomationPeer, ProgressRingAutomationPeer};
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::primitives::{
    ProgressRingTemplateSettings, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::shapes::Arc;
use crate::ControlImpl;
use ferroui_base::animation::{Animatable, IClock, IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, DirectProperty,
    FerroLocator, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    LocatorExtensions, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PC_ACTIVE: &str = ":active";
const PC_DETERMINATE_ACTIVE: &str = ":determinate-active";
const PC_INACTIVE: &str = ":inactive";

/// The length of the timelines of the animated visuals of WinUI (`c_durationTicks`, two
/// seconds).
const TIMELINE_SECONDS: f64 = 2.0;

/// Resets a flag when it goes out of scope.
struct ResetOnDrop<'a>(&'a Cell<bool>);

impl Drop for ResetOnDrop<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// A ring that shows that something is in progress (WinUI's `ProgressRing`): a spinning arc while
/// [`is_indeterminate`](Self::is_indeterminate) (the default), the part of a ring of the
/// [`value`](Self::value) between [`minimum`](Self::minimum) and [`maximum`](Self::maximum)
/// otherwise, and nothing while not [`is_active`](Self::is_active).
///
/// As in WinUI, `Value`, `Minimum` and `Maximum` are properties of the ring (not of `RangeBase`),
/// coerced as `ProgressRing.cpp` coerces them: the bound set last wins, and a value outside the
/// range is moved into it. The control sets the states of WinUI as pseudo-classes, which the
/// control theme of the WinUI theme (`src/FerroUI.Themes.WinUI/Controls/ProgressRing.xaml`)
/// styles: `:active` (Active), `:determinate-active` (DeterminateActive) and `:inactive`
/// (Inactive).
///
/// WinUI draws the ring with an `AnimatedVisualPlayer` and the Lottie compositions of its animated
/// visuals, which the code plays. FerroUI has no player: the template draws the ring with `Arc`
/// shapes, and the control moves the arc of the template part `PART_Indicator` as the code of
/// WinUI plays the compositions: the indeterminate composition loops over two seconds (the arc
/// grows to half the ring and shrinks while it turns 900 degrees), and a determinate change plays
/// the determinate composition from the old to the new value (two seconds for the whole ring), a
/// decrease at once. The clock is the ring's `Clock`, or the global clock; the motion stops while
/// the ring is inactive, hidden or out of the visual tree. `DeterminateSource` and
/// `IndeterminateSource` (preview members of WinUI) are not ported. The Fluent and Simple themes
/// have no template for it (docs/porting/winui-controls.md, section 5.1).
#[repr(C)]
pub struct ProgressRing {
    base: TemplatedControl,
    settings: Ref<ProgressRingTemplateSettings>,
    range_base_property_updating: Cell<bool>,
    attached: Cell<bool>,
    indicator: RefCell<Option<Ref<Arc>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    animation_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    animation_epoch: Cell<u64>,
    animation_start: Cell<Option<TimeSpan>>,
    animation_from: Cell<f64>,
    animation_target: Cell<f64>,
    displayed_progress: Cell<f64>,
    indeterminate_phase: Cell<f64>,
}

ferro_class!(ProgressRing: TemplatedControl);
ferro_class_info!(ProgressRing {
    new: ProgressRing::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [
            TemplatePart("PART_Indicator", type(Ref<Arc>)),
            PseudoClasses(":active", ":determinate-active", ":inactive"),
        ],
    },
});
ferro_impl_classes!(ProgressRing: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl FerroObjectImpl for ProgressRing {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let subscription = this.is_effectively_visible_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_animation();
            }
        });
        *this.visibility_subscription.borrow_mut() = Some(subscription);
        this.update_states();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::value_property().as_property() {
            this.on_range_base_property_changed(|ring| ring.coerce_value());
        } else if property == Self::maximum_property().as_property() {
            this.on_range_base_property_changed(|ring| {
                ring.coerce_minimum();
                ring.coerce_value();
            });
        } else if property == Self::minimum_property().as_property() {
            this.on_range_base_property_changed(|ring| {
                ring.coerce_maximum();
                ring.coerce_value();
            });
        } else if property == Self::is_active_property().as_property()
            || property == Self::is_indeterminate_property().as_property()
        {
            this.stop_animation();
            this.indeterminate_phase.set(0.0);
            this.update_states();
            this.update_progress();
        } else if property == Animatable::clock_property().as_property() {
            this.stop_animation();
            this.update_animation();
        } else if property == Visual::bounds_property().as_property() {
            // ProgressRing::OnSizeChanged.
            this.settings.update(this.bounds().width);
        }
    }
}

impl VisualImpl for ProgressRing {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.attached.set(true);
        this.update_animation();
    }

    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.attached.set(false);
        this.stop_animation();
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}

impl ControlImpl for ProgressRing {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ProgressRingAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for ProgressRing {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        this.stop_animation();
        Self::parent_on_apply_template(this, args);
        // The arc that stands for the animated visual player of WinUI (LottiePlayer).
        *this.indicator.borrow_mut() = args.name_scope().find_as::<Arc>("PART_Indicator");
        this.settings.update(this.bounds().width);
        this.update_states();
        this.update_progress();
    }
}

ferro_properties! { impl ProgressRing {
    ferro_property!(
        /// Defines the `IsActive` property: whether the ring is shown and moves. `true` by
        /// default.
        pub fn is_active_property() -> StyledProperty<bool> {
            FerroProperty::register::<ProgressRing, _>("IsActive", true)
        }
    );

    ferro_property!(
        /// Defines the `IsIndeterminate` property: whether the ring spins (`true`, the default) or
        /// shows its value.
        pub fn is_indeterminate_property() -> StyledProperty<bool> {
            FerroProperty::register::<ProgressRing, _>("IsIndeterminate", true)
        }
    );

    ferro_property!(
        /// Defines the `Value` property: the progress between `Minimum` and `Maximum`. 0 by
        /// default.
        pub fn value_property() -> StyledProperty<f64> {
            FerroProperty::register::<ProgressRing, _>("Value", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Minimum` property. 0 by default.
        pub fn minimum_property() -> StyledProperty<f64> {
            FerroProperty::register::<ProgressRing, _>("Minimum", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Maximum` property. 100 by default.
        pub fn maximum_property() -> StyledProperty<f64> {
            FerroProperty::register::<ProgressRing, _>("Maximum", 100.0)
        }
    );

    ferro_property!(
        /// Defines the `TemplateSettings` property: the values the template binds to (read-only).
        pub fn template_settings_property() -> DirectProperty<ProgressRing, Ref<ProgressRingTemplateSettings>> {
            FerroProperty::register_direct::<ProgressRing, _>(
                "TemplateSettings",
                |owner| owner.template_settings(),
                None,
                ProgressRingTemplateSettings::new(),
            )
        }
    );
} }

impl ProgressRing {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            settings: ProgressRingTemplateSettings::new(),
            range_base_property_updating: Cell::new(false),
            attached: Cell::new(false),
            indicator: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            animation_subscription: RefCell::new(None),
            animation_epoch: Cell::new(0),
            animation_start: Cell::new(None),
            animation_from: Cell::new(0.0),
            animation_target: Cell::new(0.0),
            displayed_progress: Cell::new(0.0),
            indeterminate_phase: Cell::new(0.0),
        }
    }

    /// Creates a progress ring.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets whether the ring is shown and moves.
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }

    /// Sets whether the ring is shown and moves.
    pub fn set_is_active(&self, value: bool) {
        FerroObject::set_value(self, Self::is_active_property(), value);
    }

    /// Gets whether the ring spins.
    pub fn is_indeterminate(&self) -> bool {
        self.get_value(Self::is_indeterminate_property())
    }

    /// Sets whether the ring spins.
    pub fn set_is_indeterminate(&self, value: bool) {
        FerroObject::set_value(self, Self::is_indeterminate_property(), value);
    }

    /// Gets the value.
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }

    /// Sets the value (`set_range_value`, as `RangeBase` names it: `set_value` is the setter of
    /// properties of the object).
    pub fn set_range_value(&self, value: f64) {
        FerroObject::set_value(self, Self::value_property(), value);
    }

    /// Gets the minimum.
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }

    /// Sets the minimum.
    pub fn set_minimum(&self, value: f64) {
        FerroObject::set_value(self, Self::minimum_property(), value);
    }

    /// Gets the maximum.
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }

    /// Sets the maximum.
    pub fn set_maximum(&self, value: f64) {
        FerroObject::set_value(self, Self::maximum_property(), value);
    }

    /// Gets the values the template binds to; each ring has its own.
    pub fn template_settings(&self) -> Ref<ProgressRingTemplateSettings> {
        self.settings.clone()
    }

    /// The progress the determinate ring shows, between 0 and 1: `(value - minimum) / (maximum -
    /// minimum)` (ProgressRing::UpdateLottieProgress). A value that is not a number and an empty
    /// or infinite range show no progress; the properties keep their values.
    pub fn normalized_value(&self) -> f64 {
        let range = self.maximum() - self.minimum();
        let fraction = (self.value() - self.minimum()) / range;
        if range > 0.0 && range.is_finite() && fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// OnValuePropertyChanged, OnMaximumPropertyChanged, OnMinimumPropertyChanged: the coercion
    /// once (the properties it sets do not coerce again), then the progress.
    fn on_range_base_property_changed(&self, coerce: impl FnOnce(&Self)) {
        if self.range_base_property_updating.replace(true) {
            return;
        }
        let _reset = ResetOnDrop(&self.range_base_property_updating);
        coerce(self);
        if !self.is_indeterminate() {
            self.update_progress();
        }
    }

    /// ProgressRing::CoerceMinimum: the minimum is at most the maximum. The current value is set,
    /// so that a binding stays.
    fn coerce_minimum(&self) {
        let max = self.maximum();
        if self.minimum() > max {
            self.set_current_value(Self::minimum_property(), max);
        }
    }

    /// ProgressRing::CoerceMaximum: the maximum is at least the minimum.
    fn coerce_maximum(&self) {
        let min = self.minimum();
        if self.maximum() < min {
            self.set_current_value(Self::maximum_property(), min);
        }
    }

    /// ProgressRing::CoerceValue: a value outside the range is moved to the nearer bound; a value
    /// that is not a number stays.
    fn coerce_value(&self) {
        let value = self.value();
        if !value.is_nan() && !(value >= self.minimum() && value <= self.maximum()) {
            let max = self.maximum();
            self.set_current_value(Self::value_property(), if value > max { max } else { self.minimum() });
        }
    }

    /// ProgressRing::UpdateStates: Active, DeterminateActive or Inactive, and the accessibility
    /// view of the ring.
    fn update_states(&self) {
        let active = self.is_active();
        let indeterminate = self.is_indeterminate();
        let classes = self.pseudo_classes();
        classes.set(PC_ACTIVE, active && indeterminate);
        classes.set(PC_DETERMINATE_ACTIVE, active && !indeterminate);
        classes.set(PC_INACTIVE, !active);
        AutomationProperties::set_accessibility_view(
            self,
            if active { AccessibilityView::Content } else { AccessibilityView::Raw },
        );
    }

    /// ProgressRing::UpdateLottieProgress: an increase plays from the old to the new progress, a
    /// decrease shows the new progress at once. The indeterminate motion is not restarted.
    fn update_progress(&self) {
        let target = self.normalized_value();
        self.animation_from.set(self.displayed_progress.get());
        self.animation_target.set(target);
        if !self.is_indeterminate() {
            self.animation_start.set(None);
            if target <= self.displayed_progress.get()
                || !self.attached.get()
                || !self.is_active()
                || !self.is_effectively_visible()
            {
                self.displayed_progress.set(target);
            }
        }
        self.update_geometry();
        self.update_animation();
    }

    fn wants_animation(&self) -> bool {
        self.attached.get()
            && self.is_active()
            && self.is_effectively_visible()
            && self.indicator.borrow().is_some()
            && (self.is_indeterminate() || self.displayed_progress.get() < self.animation_target.get())
    }

    /// Subscribes to the clock while the ring moves, and stops otherwise.
    fn update_animation(&self) {
        if !self.wants_animation() {
            self.stop_animation();
            if !self.is_indeterminate() {
                self.displayed_progress.set(self.animation_target.get());
                self.update_geometry();
            }
            return;
        }
        if self.animation_subscription.borrow().is_some() {
            return;
        }
        let clock: Option<Rc<dyn IClock>> = self.clock().or_else(|| {
            FerroLocator::current().get_service::<dyn IGlobalClock>().map(|clock| clock as Rc<dyn IClock>)
        });
        let Some(clock) = clock else {
            return;
        };
        let epoch = self.animation_epoch.get();
        let weak = self.to_ref().downgrade();
        let subscription = clock.subscribe(Rc::new(AnonymousObserver::new(move |time| {
            if let Some(owner) = weak.upgrade() {
                if owner.animation_epoch.get() == epoch {
                    owner.animation_tick(time);
                }
            }
        })));
        // A clock may emit while subscribing: a subscription that its first emission (or a
        // handler it ran) made obsolete is not kept.
        if self.animation_epoch.get() == epoch && self.wants_animation() {
            *self.animation_subscription.borrow_mut() = Some(subscription);
        } else {
            subscription.dispose();
        }
    }

    fn stop_animation(&self) {
        self.animation_epoch.set(self.animation_epoch.get().wrapping_add(1));
        self.animation_start.set(None);
        let subscription = self.animation_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn animation_tick(&self, time: TimeSpan) {
        if !self.wants_animation() {
            self.stop_animation();
            return;
        }
        let start = self.animation_start.get().unwrap_or_else(|| {
            self.animation_start.set(Some(time));
            time
        });
        let elapsed = (time - start).total_seconds().max(0.0);
        if self.is_indeterminate() {
            // PlayAsync(0, 1, true): the composition loops.
            self.indeterminate_phase.set((elapsed / TIMELINE_SECONDS).fract());
        } else {
            // PlayAsync(from, to, false): the part of the timeline between the two progresses, at
            // the speed of the composition.
            let progress = self.animation_from.get() + elapsed / TIMELINE_SECONDS;
            if progress >= self.animation_target.get() {
                self.displayed_progress.set(self.animation_target.get());
                self.stop_animation();
            } else {
                self.displayed_progress.set(progress);
            }
        }
        self.update_geometry();
    }

    /// The angles of the arc of the template, from the state of the compositions. Only the
    /// angles change while the ring moves.
    fn update_geometry(&self) {
        let indicator = self.indicator.borrow().clone();
        let Some(indicator) = indicator else { return };
        if self.is_indeterminate() {
            // ProgressRingIndeterminate.cpp: over the two seconds the shape turns 900 degrees,
            // TrimEnd grows from 0 to 0.5 in the first half and TrimStart from 0 to 0.5 in the
            // second. The key frames are linear (their cubic Bezier points have x == y).
            let phase = self.indeterminate_phase.get();
            let (start, end) = if phase < 0.5 { (0.0, 0.0001 + phase * 2.0 * (0.5 - 0.0001)) } else { (phase - 0.5, 0.5) };
            indicator.set_start_angle(-90.0 + phase * 900.0 + start * 360.0);
            indicator.set_sweep_angle((end - start) * 360.0);
        } else {
            indicator.set_start_angle(-90.0);
            indicator.set_sweep_angle(self.displayed_progress.get() * 360.0);
        }
    }
}

impl Drop for ProgressRing {
    fn drop(&mut self) {
        if let Some(subscription) = self.animation_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
