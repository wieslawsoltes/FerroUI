// Copyright 2023 The Android Open Source Project (reference layout and motion).
// Copyright 2026 FerroUI contributors (Rust adaptation).
// SPDX-License-Identifier: Apache-2.0
//! Stable icon reservation and the independent native segmented-label/check springs.

use crate::motion::{
    MaterialMotionRole, MaterialMotionScheme, MaterialSpringState, animations_enabled,
};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    DirectProperty, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Two-child layout: an 18 dp check, then the label. Its width always reserves the
/// check and 8 dp gap. An unchecked label is centered by a 13 dp logical offset.
/// Check removal is immediate; check entry has its own scale and opacity springs.
#[repr(C)]
pub struct MaterialSegmentedContent {
    base: Panel,
    states: RefCell<[MaterialSpringState; 3]>,
    label_progress: Cell<f64>,
    icon_scale: Cell<f64>,
    icon_opacity: Cell<f64>,
    clock: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility: RefCell<Option<Rc<dyn IDisposable>>>,
    last_frame: Cell<Option<f64>>,
}
ferro_class!(MaterialSegmentedContent: Panel);
ferro_class_info!(MaterialSegmentedContent {
    new: MaterialSegmentedContent::new
});
ferro_impl_classes!(MaterialSegmentedContent: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);
impl FerroObjectImpl for MaterialSegmentedContent {
    fn on_property_changed(this: &Self, event: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, event);
        if event.property() == Self::is_selected_property().as_property() {
            let selected = this.is_selected();
            let target = if selected { 1.0 } else { 0.0 };
            {
                let mut states = this.states.borrow_mut();
                for state in states.iter_mut() {
                    state.retarget(target);
                }
                if !selected {
                    // Android's ExitTransition.None removes the check immediately.
                    states[1].finish();
                    states[2].finish();
                }
                if !this.is_attached_to_visual_tree() {
                    for state in states.iter_mut() {
                        state.finish();
                    }
                }
            }
            this.publish_frame();
            this.update_clock();
        } else if event.property() == Self::motion_scheme_property().as_property()
            || event.property() == Self::reduced_motion_property().as_property()
        {
            let scheme = this.motion_scheme();
            {
                let mut states = this.states.borrow_mut();
                states[0].set_spec(scheme.spring(MaterialMotionRole::FastSpatial));
                states[1].set_spec(scheme.spring(MaterialMotionRole::FastSpatial));
                states[2].set_spec(scheme.spring(MaterialMotionRole::DefaultEffects));
            }
            this.advance_motion(0.0);
            this.update_clock();
        }
    }
}
impl VisualImpl for MaterialSegmentedContent {
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        let weak = this.to_ref().downgrade();
        let subscription = this.is_effectively_visible_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.update_clock();
            }
        });
        if let Some(old) = this.visibility.replace(Some(subscription)) {
            old.dispose();
        }
        this.publish_frame();
        this.update_clock();
    }
    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        this.stop_clock();
        if let Some(subscription) = this.visibility.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, event);
    }
}
impl LayoutableImpl for MaterialSegmentedContent {
    fn measure_override(this: &Self, available: Size) -> Size {
        let children = this.children().snapshot();
        if let Some(icon) = children.first() {
            icon.measure(Size::new(18.0, 18.0));
        }
        let label = children.get(1);
        if let Some(label) = label {
            label.measure(Size::new(
                (available.width - 26.0).max(0.0),
                available.height,
            ));
        }
        let label = label.map_or(Size::default(), |label| label.desired_size());
        Size::new(label.width + 26.0, label.height.max(18.0))
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        if let Some(icon) = children.first() {
            icon.arrange(Rect::new(0.0, (final_size.height - 18.0) / 2.0, 18.0, 18.0));
        }
        if let Some(label) = children.get(1) {
            let size = label.desired_size();
            label.arrange(Rect::new(
                13.0 + 13.0 * this.label_progress(),
                (final_size.height - size.height) / 2.0,
                size.width,
                size.height,
            ));
        }
        final_size
    }
}
ferro_properties! { impl MaterialSegmentedContent {
    pub fn is_selected_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialSegmentedContent, _>("IsSelected", false) }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> { FerroProperty::register::<MaterialSegmentedContent, _>("MotionScheme", MaterialMotionScheme::Expressive) }
    pub fn reduced_motion_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialSegmentedContent, _>("ReducedMotion", false) }
    pub fn label_progress_property() -> DirectProperty<MaterialSegmentedContent, f64> { FerroProperty::register_direct::<MaterialSegmentedContent, _>("LabelProgress", |owner| owner.label_progress(), None, 0.0) }
    pub fn icon_scale_property() -> DirectProperty<MaterialSegmentedContent, f64> { FerroProperty::register_direct::<MaterialSegmentedContent, _>("IconScale", |owner| owner.icon_scale(), None, 0.0) }
    pub fn icon_opacity_property() -> DirectProperty<MaterialSegmentedContent, f64> { FerroProperty::register_direct::<MaterialSegmentedContent, _>("IconOpacity", |owner| owner.icon_opacity(), None, 0.0) }
}}
impl MaterialSegmentedContent {
    pub fn construct() -> Self {
        let scheme = MaterialMotionScheme::Expressive;
        Self {
            base: Panel::construct(),
            states: RefCell::new([
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::FastSpatial), 0.0),
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::FastSpatial), 0.0),
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::DefaultEffects), 0.0),
            ]),
            label_progress: Cell::new(0.0),
            icon_scale: Cell::new(0.0),
            icon_opacity: Cell::new(0.0),
            clock: RefCell::new(None),
            visibility: RefCell::new(None),
            last_frame: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }
    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value);
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
    pub fn label_progress(&self) -> f64 {
        self.label_progress.get()
    }
    pub fn icon_scale(&self) -> f64 {
        self.icon_scale.get()
    }
    pub fn icon_opacity(&self) -> f64 {
        self.icon_opacity.get()
    }
    fn motion_pending(&self) -> bool {
        self.states
            .borrow()
            .iter()
            .any(|state| !state.is_at_rest(0.01, 0.625))
    }
    fn publish_frame(&self) {
        let samples = self.states.borrow().map(|state| state.current().value);
        self.set_and_raise_cell(
            Self::label_progress_property(),
            &self.label_progress,
            samples[0],
        );
        self.set_and_raise_cell(
            Self::icon_scale_property(),
            &self.icon_scale,
            samples[1].max(0.0),
        );
        self.set_and_raise_cell(
            Self::icon_opacity_property(),
            &self.icon_opacity,
            samples[2].clamp(0.0, 1.0),
        );
        self.invalidate_arrange();
    }
    fn advance_motion(&self, seconds: f64) {
        {
            let mut states = self.states.borrow_mut();
            for state in states.iter_mut() {
                state.set_reduced_motion(!animations_enabled(self.reduced_motion()));
                state.advance(seconds);
                if state.is_at_rest(0.01, 0.625) {
                    state.finish();
                }
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
        if self.clock.borrow().is_some() {
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
                let Some(this) = weak.upgrade() else {
                    return;
                };
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
            })));
        *self.clock.borrow_mut() = Some(subscription);
    }
    fn stop_clock(&self) {
        if let Some(subscription) = self.clock.borrow_mut().take() {
            subscription.dispose();
        }
        self.last_frame.set(None);
    }
}
impl Drop for MaterialSegmentedContent {
    fn drop(&mut self) {
        if let Some(subscription) = self.clock.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility.get_mut().take() {
            subscription.dispose();
        }
    }
}
