// Copyright 2024 The Android Open Source Project (reference geometry and motion).
// Copyright 2026 FerroUI contributors (Rust control/layout implementation).
// SPDX-License-Identifier: Apache-2.0
//! Expressive extended floating action buttons retaining FerroUI Button semantics.
//!
//! Reference: AndroidX Material3 1.5.0-beta02, `FloatingActionButton.kt` at
//! `e4d933c98df7baa7ea2b83eea6e086915961e0ff`:
//! <https://github.com/androidx/androidx/blob/e4d933c98df7baa7ea2b83eea6e086915961e0ff/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/FloatingActionButton.kt>.
//! Medium/large icon-label gaps are the implementation's corrected 12/16dp values,
//! not the stale 16/20dp generated tokens. Width is measured from real content.

use crate::motion::{
    MaterialMotionRole, MaterialMotionScheme, MaterialSpringState, animations_enabled,
};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    BoxedValue, DirectProperty, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Rect, Ref, Size, StyledElement,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ButtonImpl, ContentControlImpl, Control, ControlImpl, Panel, PanelImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The three expressive extended-FAB sizes in the pinned component implementation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialExtendedFabSize {
    #[default]
    Small,
    Medium,
    Large,
}
ferroui_base::ferro_markup_enum!(
    MaterialExtendedFabSize { Small, Medium, Large },
    { namespace: "FerroUI.Themes.Material" }
);

#[derive(Clone, Copy)]
struct Metrics {
    height: f64,
    icon: f64,
    leading: f64,
    trailing: f64,
    gap: f64,
}

impl MaterialExtendedFabSize {
    fn metrics(self) -> Metrics {
        match self {
            Self::Small => Metrics {
                height: 56.0,
                icon: 24.0,
                leading: 16.0,
                trailing: 16.0,
                gap: 8.0,
            },
            Self::Medium => Metrics {
                height: 80.0,
                icon: 28.0,
                leading: 26.0,
                trailing: 26.0,
                gap: 12.0,
            },
            // The public large icon recommendation corrects the 32dp generated token to 36dp.
            Self::Large => Metrics {
                height: 96.0,
                icon: 36.0,
                leading: 28.0,
                trailing: 28.0,
                gap: 16.0,
            },
        }
    }
}

/// A normal Button with an icon, a label in `Content`, and an animated expanded state.
/// Commands, keyboard activation, routed Click and the invoke automation pattern are inherited.
#[repr(C)]
pub struct MaterialExtendedFab {
    base: Button,
}
ferro_class!(MaterialExtendedFab: Button);
ferro_class_info!(MaterialExtendedFab {
    new: MaterialExtendedFab::new,
    markup: { attributes: [PseudoClasses(":small", ":medium", ":large", ":expanded")] },
});
ferro_impl_classes!(MaterialExtendedFab: StyledElementImpl, VisualImpl, InteractiveImpl,
    InputElementImpl, LayoutableImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl, ButtonImpl);

impl FerroObjectImpl for MaterialExtendedFab {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_classes();
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::icon_property().as_property() {
            let (old, new) = change.get_old_and_new_value::<Option<BoxedValue>>();
            if let Some(old) = old.and_then(|value| Control::logical_from_boxed(&value)) {
                StyledElement::logical_children(this).remove(&old);
            }
            if let Some(new) = new.and_then(|value| Control::logical_from_boxed(&value)) {
                StyledElement::logical_children(this).add(new);
            }
        }
        if change.property() == Self::size_variant_property().as_property()
            || change.property() == Self::is_expanded_property().as_property()
        {
            this.update_classes();
        }
    }
}

ferro_properties! { impl MaterialExtendedFab {
    pub fn icon_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<MaterialExtendedFab, _>("Icon", None) }
    pub fn is_expanded_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialExtendedFab, _>("IsExpanded", true) }
    pub fn size_variant_property() -> StyledProperty<MaterialExtendedFabSize> {
        FerroProperty::register::<MaterialExtendedFab, _>("SizeVariant", MaterialExtendedFabSize::Small)
    }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> {
        FerroProperty::register::<MaterialExtendedFab, _>("MotionScheme", MaterialMotionScheme::Expressive)
    }
    pub fn reduced_motion_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialExtendedFab, _>("ReducedMotion", false) }
}}

impl MaterialExtendedFab {
    pub fn construct() -> Self {
        Self {
            base: Button::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }
    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value);
    }
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }
    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value);
    }
    pub fn size_variant(&self) -> MaterialExtendedFabSize {
        self.get_value(Self::size_variant_property())
    }
    pub fn set_size_variant(&self, value: MaterialExtendedFabSize) {
        self.set_value(Self::size_variant_property(), value);
    }
    pub fn motion_scheme(&self) -> MaterialMotionScheme {
        self.get_value(Self::motion_scheme_property())
    }
    pub fn set_motion_scheme(&self, value: MaterialMotionScheme) {
        self.set_value(Self::motion_scheme_property(), value);
    }
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }

    fn update_classes(&self) {
        self.pseudo_classes().set(
            ":small",
            self.size_variant() == MaterialExtendedFabSize::Small,
        );
        self.pseudo_classes().set(
            ":medium",
            self.size_variant() == MaterialExtendedFabSize::Medium,
        );
        self.pseudo_classes().set(
            ":large",
            self.size_variant() == MaterialExtendedFabSize::Large,
        );
        self.pseudo_classes().set(":expanded", self.is_expanded());
    }
}

/// Two-child layout used by the extended-FAB template: icon first, label second.
/// It measures the expanded label at full available width while reporting an interpolated
/// width. The container's rounded trailing edge therefore follows the animation naturally.
#[repr(C)]
pub struct MaterialExtendedFabLayout {
    base: Panel,
    width_motion: RefCell<MaterialSpringState>,
    alpha_motion: RefCell<MaterialSpringState>,
    expansion_progress: Cell<f64>,
    label_opacity: Cell<f64>,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    last_frame: Cell<Option<f64>>,
}
ferro_class!(MaterialExtendedFabLayout: Panel);
ferro_class_info!(MaterialExtendedFabLayout {
    new: MaterialExtendedFabLayout::new
});
ferro_impl_classes!(MaterialExtendedFabLayout: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);

impl FerroObjectImpl for MaterialExtendedFabLayout {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::is_expanded_property().as_property() {
            let target = if this.is_expanded() { 1.0 } else { 0.0 };
            this.width_motion.borrow_mut().retarget(target);
            this.alpha_motion.borrow_mut().retarget(target);
            // Initial markup state is already settled when it enters the tree.
            if !this.is_attached_to_visual_tree() {
                this.width_motion.borrow_mut().finish();
                this.alpha_motion.borrow_mut().finish();
            }
            this.publish_frame();
            this.update_clock();
        } else if property == Self::motion_scheme_property().as_property()
            || property == Self::reduced_motion_property().as_property()
        {
            let scheme = this.motion_scheme();
            this.width_motion
                .borrow_mut()
                .set_spec(scheme.spring(MaterialMotionRole::FastSpatial));
            this.alpha_motion
                .borrow_mut()
                .set_spec(scheme.spring(MaterialMotionRole::FastEffects));
            this.advance_motion(0.0);
            this.update_clock();
        }
    }
}

impl VisualImpl for MaterialExtendedFabLayout {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let weak = this.to_ref().downgrade();
        let subscription = this.is_effectively_visible_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_clock();
            }
        });
        if let Some(old) = this.visibility_subscription.replace(Some(subscription)) {
            old.dispose();
        }
        this.publish_frame();
        this.update_clock();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.stop_clock();
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl LayoutableImpl for MaterialExtendedFabLayout {
    fn measure_override(this: &Self, available: Size) -> Size {
        let metrics = this.size_variant().metrics();
        let children = this.children().snapshot();
        let icon = children.first();
        let label = children.get(1);
        if let Some(icon) = icon {
            icon.measure(Size::new(metrics.icon, metrics.icon));
        }
        let has_icon = icon.is_some_and(|icon| icon.desired_size().width > 0.0);
        let icon_width = if has_icon { metrics.icon } else { 0.0 };
        let gap = if has_icon { metrics.gap } else { 0.0 };
        let horizontal = metrics.leading + icon_width + gap + metrics.trailing;
        if let Some(label) = label {
            label.measure(Size::new(
                (available.width - horizontal).max(0.0),
                available.height,
            ));
        }
        let label_size = label.map_or(Size::default(), |label| label.desired_size());
        let expanded_width = (horizontal + label_size.width).max(metrics.height);
        let width = metrics.height + (expanded_width - metrics.height) * this.expansion_progress();
        Size::new(
            width.max(48.0),
            metrics.height.max(label_size.height).max(icon_width),
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let metrics = this.size_variant().metrics();
        let children = this.children().snapshot();
        let icon = children.first();
        let label = children.get(1);
        let has_icon = icon.is_some_and(|icon| icon.desired_size().width > 0.0);
        let icon_width = if has_icon { metrics.icon } else { 0.0 };
        if let Some(icon) = icon {
            icon.arrange(Rect::new(
                metrics.leading,
                (final_size.height - metrics.icon) / 2.0,
                icon_width,
                metrics.icon,
            ));
        }
        if let Some(label) = label {
            let desired = label.desired_size();
            label.arrange(Rect::new(
                metrics.leading + icon_width + if has_icon { metrics.gap } else { 0.0 },
                (final_size.height - desired.height) / 2.0,
                desired.width,
                desired.height,
            ));
        }
        final_size
    }
}

ferro_properties! { impl MaterialExtendedFabLayout {
    pub fn is_expanded_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialExtendedFabLayout, _>("IsExpanded", true) }
    pub fn size_variant_property() -> StyledProperty<MaterialExtendedFabSize> {
        FerroProperty::register::<MaterialExtendedFabLayout, _>("SizeVariant", MaterialExtendedFabSize::Small)
    }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> {
        FerroProperty::register::<MaterialExtendedFabLayout, _>("MotionScheme", MaterialMotionScheme::Expressive)
    }
    pub fn reduced_motion_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialExtendedFabLayout, _>("ReducedMotion", false) }
    pub fn expansion_progress_property() -> DirectProperty<MaterialExtendedFabLayout, f64> {
        FerroProperty::register_direct::<MaterialExtendedFabLayout, _>("ExpansionProgress", |o| o.expansion_progress(), None, 1.0)
    }
    pub fn label_opacity_property() -> DirectProperty<MaterialExtendedFabLayout, f64> {
        FerroProperty::register_direct::<MaterialExtendedFabLayout, _>("LabelOpacity", |o| o.label_opacity(), None, 1.0)
    }
}}

impl MaterialExtendedFabLayout {
    fn static_constructor() {
        Layoutable::affects_measure::<MaterialExtendedFabLayout>(&[
            Self::size_variant_property().as_property()
        ]);
    }
    pub fn construct() -> Self {
        let scheme = MaterialMotionScheme::Expressive;
        Self {
            base: Panel::construct(),
            width_motion: RefCell::new(MaterialSpringState::new(
                scheme.spring(MaterialMotionRole::FastSpatial),
                1.0,
            )),
            alpha_motion: RefCell::new(MaterialSpringState::new(
                scheme.spring(MaterialMotionRole::FastEffects),
                1.0,
            )),
            expansion_progress: Cell::new(1.0),
            label_opacity: Cell::new(1.0),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            last_frame: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }
    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value);
    }
    pub fn size_variant(&self) -> MaterialExtendedFabSize {
        self.get_value(Self::size_variant_property())
    }
    pub fn set_size_variant(&self, value: MaterialExtendedFabSize) {
        self.set_value(Self::size_variant_property(), value);
    }
    pub fn motion_scheme(&self) -> MaterialMotionScheme {
        self.get_value(Self::motion_scheme_property())
    }
    pub fn set_motion_scheme(&self, value: MaterialMotionScheme) {
        self.set_value(Self::motion_scheme_property(), value);
    }
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }
    pub fn expansion_progress(&self) -> f64 {
        self.expansion_progress.get()
    }
    pub fn label_opacity(&self) -> f64 {
        self.label_opacity.get()
    }

    fn motion_pending(&self) -> bool {
        !self.width_motion.borrow().is_at_rest(0.01, 0.625)
            || !self.alpha_motion.borrow().is_at_rest(0.01, 0.625)
    }
    fn publish_frame(&self) {
        let progress = self.width_motion.borrow().current().value;
        let alpha = self.alpha_motion.borrow().current().value.clamp(0.0, 1.0);
        let label_visible = self.is_expanded() || self.motion_pending();
        self.set_and_raise_cell(
            Self::expansion_progress_property(),
            &self.expansion_progress,
            progress,
        );
        self.set_and_raise_cell(Self::label_opacity_property(), &self.label_opacity, alpha);
        let label = self.children().snapshot().get(1).cloned();
        if let Some(label) = label {
            label.set_is_visible(label_visible);
            label.set_opacity(alpha);
        }
        self.invalidate_measure();
    }
    fn advance_motion(&self, seconds: f64) {
        let reduced = !animations_enabled(self.reduced_motion());
        for state in [&self.width_motion, &self.alpha_motion] {
            let mut state = state.borrow_mut();
            state.set_reduced_motion(reduced);
            state.advance(seconds);
            if state.is_at_rest(0.01, 0.625) {
                state.finish();
            }
        }
        self.publish_frame();
    }
    fn update_clock(&self) {
        if !animations_enabled(self.reduced_motion()) {
            self.advance_motion(0.0);
        }
        if !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
            || !self.motion_pending()
        {
            self.stop_clock();
            return;
        }
        if self.clock_subscription.borrow().is_some() {
            return;
        }
        let Some(clock) = FerroLocator::current().get_service::<dyn IGlobalClock>() else {
            self.advance_motion(f64::INFINITY);
            return;
        };
        self.last_frame.set(None);
        let weak = self.to_ref().downgrade();
        let subscription =
            clock.subscribe(Rc::new(AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    // A clock may have snapshotted this observer just before a detach.
                    if !this.is_attached_to_visual_tree() || !this.is_effectively_visible() {
                        this.stop_clock();
                        return;
                    }
                    let time = time.total_seconds();
                    if !time.is_finite() {
                        return;
                    }
                    let delta = this
                        .last_frame
                        .replace(Some(time))
                        .map_or(0.0, |previous| (time - previous).max(0.0));
                    this.advance_motion(delta);
                    if !this.motion_pending() {
                        this.stop_clock();
                    }
                }
            })));
        *self.clock_subscription.borrow_mut() = Some(subscription);
    }
    fn stop_clock(&self) {
        if let Some(subscription) = self.clock_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.last_frame.set(None);
    }
}

impl Drop for MaterialExtendedFabLayout {
    fn drop(&mut self) {
        if let Some(subscription) = self.clock_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
