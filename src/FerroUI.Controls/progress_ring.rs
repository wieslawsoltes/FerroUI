// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Range, state and legacy template-setting contracts adapted from ProgressRing.cpp at
// microsoft/microsoft-ui-xaml a71c42bd3e352060e5f5ecf3b5e1a98f8a745809.

use crate::automation::peers::{AutomationPeer, ProgressRingAutomationPeer};
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::primitives::{
    ProgressRingTemplateSettings,
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::shapes::Arc;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::{Control, ControlImpl, Grid};
use ferroui_base::animation::{Animatable, IClock, IGlobalClock, TimeSpan};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{IBrush, PenLineCap, SolidColorBrush};
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroObject, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Ref, StyledElementImpl, StyledProperty,
    Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Default)]
struct Parts {
    root: Option<Ref<Grid>>,
    track: Option<Ref<Arc>>,
    indicator: Option<Ref<Arc>>,
}

struct RangeUpdate<'a>(&'a Cell<bool>);
impl Drop for RangeUpdate<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// WinUI progress state and range semantics in the native control and rendering tree.
///
/// The replaceable default template uses native Arc shapes. Its motion is driven by
/// the inherited framework clock and stops when inactive, hidden, detached, or when
/// a determinate transition finishes. It does not emulate AnimatedVisualPlayer or
/// silently accept WinUI's preview custom animation-source properties.
#[repr(C)]
pub struct ProgressRing {
    base: TemplatedControl,
    settings: Ref<ProgressRingTemplateSettings>,
    range_updating: Cell<bool>,
    attached: Cell<bool>,
    parts: RefCell<Parts>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
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
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(ProgressRing: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl FerroObjectImpl for ProgressRing {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let visible = this.is_effectively_visible_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_animation();
            }
        });
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        this.subscriptions
            .borrow_mut()
            .extend([visible, resources, theme]);
        this.update_states();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if [
            Self::value_property().as_property(),
            Self::minimum_property().as_property(),
            Self::maximum_property().as_property(),
        ]
        .contains(&property)
        {
            if !this.range_updating.replace(true) {
                let _guard = RangeUpdate(&this.range_updating);
                // WinUI gives the property that was just set precedence over the other bound.
                if property == Self::minimum_property().as_property()
                    && this.maximum() < this.minimum()
                {
                    this.set_current_value(Self::maximum_property(), this.minimum());
                } else if property == Self::maximum_property().as_property()
                    && this.minimum() > this.maximum()
                {
                    this.set_current_value(Self::minimum_property(), this.maximum());
                }
                let value = this.value();
                if !value.is_nan() && !(value >= this.minimum() && value <= this.maximum()) {
                    this.set_current_value(
                        Self::value_property(),
                        if value > this.maximum() {
                            this.maximum()
                        } else {
                            this.minimum()
                        },
                    );
                }
                this.update_progress();
            }
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
            this.settings.update(this.bounds().width);
            this.update_parts();
        } else if property == TemplatedControl::foreground_property().as_property()
            || property == TemplatedControl::background_property().as_property()
        {
            this.update_parts();
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
        this.parts.replace(Parts::default());
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        *this.parts.borrow_mut() = Parts {
            root: scope.find_as::<Grid>("LayoutRoot"),
            track: scope.find_as::<Arc>("PART_Track"),
            indicator: scope.find_as::<Arc>("PART_Indicator"),
        };
        this.settings.update(this.bounds().width);
        this.update_states();
        this.update_progress();
        this.update_parts();
    }
}

ferro_properties! { impl ProgressRing {
    pub fn is_active_property() -> StyledProperty<bool> { FerroProperty::register::<ProgressRing, _>("IsActive", true) }
    pub fn is_indeterminate_property() -> StyledProperty<bool> { FerroProperty::register::<ProgressRing, _>("IsIndeterminate", true) }
    pub fn value_property() -> StyledProperty<f64> { FerroProperty::register::<ProgressRing, _>("Value", 0.0) }
    pub fn minimum_property() -> StyledProperty<f64> { FerroProperty::register::<ProgressRing, _>("Minimum", 0.0) }
    pub fn maximum_property() -> StyledProperty<f64> { FerroProperty::register::<ProgressRing, _>("Maximum", 100.0) }
    pub fn template_settings_property() -> DirectProperty<ProgressRing, Ref<ProgressRingTemplateSettings>> {
        FerroProperty::register_direct::<ProgressRing, _>("TemplateSettings", |r| r.template_settings(), None, ProgressRingTemplateSettings::new())
    }
} }

impl ProgressRing {
    fn static_constructor() {
        InputElement::is_hit_test_visible_property().override_default_value::<ProgressRing>(false);
        InputElement::focusable_property().override_default_value::<ProgressRing>(false);
        Layoutable::width_property().override_default_value::<ProgressRing>(32.0);
        Layoutable::height_property().override_default_value::<ProgressRing>(32.0);
        Layoutable::min_width_property().override_default_value::<ProgressRing>(16.0);
        Layoutable::min_height_property().override_default_value::<ProgressRing>(16.0);
        Layoutable::horizontal_alignment_property()
            .override_default_value::<ProgressRing>(HorizontalAlignment::Center);
        Layoutable::vertical_alignment_property()
            .override_default_value::<ProgressRing>(VerticalAlignment::Center);
        let template = FuncControlTemplate::for_type::<ProgressRing>(|_, scope| {
            let root = Grid::new();
            root.set_name(Some("LayoutRoot".into()));
            for name in ["PART_Track", "PART_Indicator"] {
                let arc = Arc::new();
                arc.set_name(Some(name.into()));
                arc.set_stroke_line_cap(PenLineCap::Round);
                arc.set_start_angle(-90.0);
                arc.set_sweep_angle(if name == "PART_Track" { 360.0 } else { 40.0 });
                AutomationProperties::set_accessibility_view(&arc, AccessibilityView::Raw);
                root.children()
                    .add(arc.register_in_name_scope(&**scope).upcast::<Control>());
            }
            root.register_in_name_scope(&**scope).upcast()
        });
        TemplatedControl::template_property()
            .override_default_value::<ProgressRing>(Some(template));
    }

    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            settings: ProgressRingTemplateSettings::new(),
            range_updating: Cell::new(false),
            attached: Cell::new(false),
            parts: RefCell::new(Parts::default()),
            subscriptions: RefCell::new(Vec::new()),
            animation_subscription: RefCell::new(None),
            animation_epoch: Cell::new(0),
            animation_start: Cell::new(None),
            animation_from: Cell::new(0.0),
            animation_target: Cell::new(0.0),
            displayed_progress: Cell::new(0.0),
            indeterminate_phase: Cell::new(0.0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_active(&self) -> bool {
        self.get_value(Self::is_active_property())
    }
    pub fn set_is_active(&self, value: bool) {
        self.set_value(Self::is_active_property(), value);
    }
    pub fn is_indeterminate(&self) -> bool {
        self.get_value(Self::is_indeterminate_property())
    }
    pub fn set_is_indeterminate(&self, value: bool) {
        self.set_value(Self::is_indeterminate_property(), value);
    }
    pub fn value(&self) -> f64 {
        self.get_value(Self::value_property())
    }
    pub fn set_range_value(&self, value: f64) {
        self.set_value(Self::value_property(), value);
    }
    pub fn minimum(&self) -> f64 {
        self.get_value(Self::minimum_property())
    }
    pub fn set_minimum(&self, value: f64) {
        FerroObject::set_value(self, Self::minimum_property(), value);
    }
    pub fn maximum(&self) -> f64 {
        self.get_value(Self::maximum_property())
    }
    pub fn set_maximum(&self, value: f64) {
        FerroObject::set_value(self, Self::maximum_property(), value);
    }
    pub fn template_settings(&self) -> Ref<ProgressRingTemplateSettings> {
        self.settings.clone()
    }

    /// The finite fraction sent to native geometry. NaN, infinite and collapsed ranges
    /// have no drawable progress; their public property values remain unchanged.
    pub fn normalized_value(&self) -> f64 {
        let range = self.maximum() - self.minimum();
        let fraction = (self.value() - self.minimum()) / range;
        if range > 0.0 && range.is_finite() && fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn update_states(&self) {
        let active = self.is_active();
        self.pseudo_classes().set(":active", active);
        self.pseudo_classes()
            .set(":indeterminate", self.is_indeterminate());
        self.pseudo_classes()
            .set(":determinate", !self.is_indeterminate());
        AutomationProperties::set_accessibility_view(
            self,
            if active {
                AccessibilityView::Content
            } else {
                AccessibilityView::Raw
            },
        );
        let root = self.parts.borrow().root.clone();
        if let Some(root) = root {
            root.set_opacity(if active { 1.0 } else { 0.0 });
        }
    }

    fn update_progress(&self) {
        let target = self.normalized_value();
        self.animation_from.set(self.displayed_progress.get());
        self.animation_target.set(target);
        // Numeric changes do not restart the independent indeterminate source.
        if !self.is_indeterminate() {
            self.animation_start.set(None);
        }
        if !self.is_indeterminate()
            && (target <= self.displayed_progress.get()
                || !self.attached.get()
                || !self.is_active()
                || !self.is_effectively_visible())
        {
            self.displayed_progress.set(target);
        }
        self.update_geometry();
        self.update_animation();
    }

    fn wants_animation(&self) -> bool {
        self.attached.get()
            && self.is_active()
            && self.is_effectively_visible()
            && self.parts.borrow().indicator.is_some()
            && (self.is_indeterminate()
                || self.displayed_progress.get() < self.animation_target.get())
    }

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
            FerroLocator::current()
                .get_service::<dyn IGlobalClock>()
                .map(|clock| clock as Rc<dyn IClock>)
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
        // A custom clock may synchronously emit while subscribing. Never retain a
        // subscription made obsolete by that first emission or a reentrant handler.
        if self.animation_epoch.get() == epoch && self.wants_animation() {
            *self.animation_subscription.borrow_mut() = Some(subscription);
        } else {
            subscription.dispose();
        }
    }

    fn stop_animation(&self) {
        self.animation_epoch
            .set(self.animation_epoch.get().wrapping_add(1));
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
            self.indeterminate_phase.set((elapsed / 2.0).fract());
        } else {
            // ProgressRingDeterminate has a two-second timeline. PlayAsync(from,
            // to, false) traverses only the corresponding interval at normal speed.
            let progress = self.animation_from.get() + elapsed / 2.0;
            self.displayed_progress
                .set(progress.min(self.animation_target.get()));
            if progress >= self.animation_target.get() {
                self.displayed_progress.set(self.animation_target.get());
                self.stop_animation();
            }
        }
        self.update_geometry();
    }

    fn resource_brush(&self, key: &str) -> Option<Rc<dyn IBrush>> {
        let value = self
            .try_find_resource(
                &ResourceKey::from(key),
                self.actual_theme_variant().as_ref(),
            )
            .flatten()?;
        ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IBrush>>())?
            .downcast_ref::<Rc<dyn IBrush>>()
            .cloned()
    }

    fn update_parts(&self) {
        let parts = self.parts.borrow().clone();
        let dark = self.actual_theme_variant().as_ref() == Some(&ThemeVariant::dark());
        let foreground = self
            .is_set(TemplatedControl::foreground_property())
            .then(|| self.foreground())
            .flatten()
            .or_else(|| self.resource_brush("ProgressRingForegroundThemeBrush"))
            .or_else(|| self.resource_brush("SystemControlHighlightAccentBrush"))
            .unwrap_or_else(|| {
                SolidColorBrush::from_uint32(if dark { 0xFF60CDFF } else { 0xFF0067C0 }).into()
            });
        let background = self
            .background()
            .or_else(|| self.resource_brush("ProgressRingBackgroundThemeBrush"))
            .unwrap_or_else(|| {
                SolidColorBrush::from_uint32(if dark { 0x15FFFFFF } else { 0x0F000000 }).into()
            });
        let side = self.bounds().width.min(self.bounds().height).max(16.0);
        let stroke = (side * 0.1).max(1.0);
        if let Some(track) = parts.track {
            track.set_stroke(Some(background));
            track.set_stroke_thickness(stroke);
        }
        if let Some(indicator) = parts.indicator {
            indicator.set_stroke(Some(foreground));
            indicator.set_stroke_thickness(stroke);
        }
        self.update_geometry();
    }

    // The animation hot path changes only the native geometry properties. Brush
    // resolution, theme lookups and template traversal stay outside frame ticks.
    fn update_geometry(&self) {
        let indicator = self.parts.borrow().indicator.clone();
        if let Some(indicator) = indicator {
            if self.is_indeterminate() {
                let phase = self.indeterminate_phase.get();
                // Scalar keyframes of ProgressRingIndeterminate.cpp: a two-second
                // 900-degree rotation; TrimEnd grows during the first half, then
                // TrimStart grows during the second. The source's Bezier points
                // have x == y, so interpolation is linear.
                let (start, end) = if phase < 0.5 {
                    (0.0, 0.0001 + phase * 2.0 * (0.5 - 0.0001))
                } else {
                    (phase - 0.5, 0.5)
                };
                indicator.set_start_angle(-90.0 + phase * 900.0 + start * 360.0);
                indicator.set_sweep_angle((end - start) * 360.0);
            } else {
                indicator.set_start_angle(-90.0);
                indicator.set_sweep_angle(self.displayed_progress.get() * 360.0);
            }
        }
    }
}

impl Drop for ProgressRing {
    fn drop(&mut self) {
        if let Some(subscription) = self.animation_subscription.get_mut().take() {
            subscription.dispose();
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
