// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! Native chip slot geometry and retained icon entry/exit, from AndroidX Chip.kt.
use crate::motion::{
    MaterialMotionRole, MaterialMotionScheme, MaterialSpringState, animations_enabled,
};
use crate::{MaterialChip, MaterialChipKind};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    BoxedValue, CornerRadius, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::{Button, ControlImpl, Panel, PanelImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// Children: painted container, leading clip, label, trailing clip.
// Leading mode: none, icon, avatar, selected check. Trailing: none, icon, dismiss.
#[repr(C)]
pub struct MaterialChipLayout {
    base: Panel,
    states: RefCell<[MaterialSpringState; 4]>,
    leading_mode: Cell<u8>,
    trailing_mode: Cell<u8>,
    wanted_leading: Cell<u8>,
    wanted_trailing: Cell<u8>,
    painted_height: Cell<f64>,
    clock: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility: RefCell<Option<Rc<dyn IDisposable>>>,
    last_frame: Cell<Option<f64>>,
}
ferro_class!(MaterialChipLayout: Panel);
ferro_class_info!(MaterialChipLayout {
    new: MaterialChipLayout::new
});
ferro_impl_classes!(MaterialChipLayout: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);
impl FerroObjectImpl for MaterialChipLayout {
    fn on_property_changed(this: &Self, event: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, event);
        let property = event.property();
        if [
            Self::kind_property().as_property(),
            Self::leading_icon_property().as_property(),
            Self::avatar_property().as_property(),
            Self::trailing_icon_property().as_property(),
            Self::is_dismissible_property().as_property(),
            Self::is_selected_property().as_property(),
            Self::show_selected_icon_property().as_property(),
            Self::is_expressive_property().as_property(),
            Self::motion_scheme_property().as_property(),
            Self::reduced_motion_property().as_property(),
        ]
        .contains(&property)
        {
            this.refresh_content();
        }
    }
}
impl VisualImpl for MaterialChipLayout {
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
impl LayoutableImpl for MaterialChipLayout {
    fn measure_override(this: &Self, available: Size) -> Size {
        let children = this.children().snapshot();
        let (start, end, leading_gap, trailing_gap) = this.insets();
        let widths = this.slot_widths();
        if let Some(leading) = children.get(1) {
            leading.measure(Size::new(this.leading_content_width(), available.height));
        }
        if let Some(trailing) = children.get(3) {
            trailing.measure(Size::new(this.trailing_content_width(), available.height));
        }
        let horizontal = start + end + widths.0 + widths.1 + leading_gap + trailing_gap;
        if let Some(label) = children.get(2) {
            label.measure(Size::new(
                (available.width - horizontal).max(0.0),
                available.height,
            ));
        }
        let label = children
            .get(2)
            .map_or(Size::default(), |label| label.desired_size());
        let leading_height = if this.leading_mode.get() == 0 {
            0.0
        } else {
            this.leading_content_width()
        };
        let trailing_height = if this.trailing_mode.get() == 0 {
            0.0
        } else {
            this.trailing_content_width()
        };
        let height = 32.0_f64
            .max(label.height)
            .max(leading_height)
            .max(trailing_height);
        this.painted_height.set(height);
        this.publish_full_shape((label.width + horizontal).max(48.0), height);
        if let Some(surface) = children.first() {
            surface.measure(Size::new(label.width + horizontal, height));
        }
        Size::new((label.width + horizontal).max(48.0), height.max(48.0))
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        let (start, end, leading_gap, trailing_gap) = this.insets();
        let widths = this.slot_widths();
        let painted_height = this.painted_height.get();
        this.publish_full_shape(final_size.width, painted_height);
        if let Some(surface) = children.first() {
            surface.arrange(Rect::new(
                0.0,
                (final_size.height - painted_height) / 2.0,
                final_size.width,
                painted_height,
            ));
        }
        if let Some(leading) = children.get(1) {
            let height = this.leading_content_width();
            leading.arrange(Rect::new(
                start,
                (final_size.height - height) / 2.0,
                widths.0,
                height,
            ));
        }
        if let Some(label) = children.get(2) {
            let height = label.desired_size().height;
            let gap = if widths.0 > 0.0 {
                leading_gap
            } else {
                trailing_gap
            };
            label.arrange(Rect::new(
                start + widths.0 + gap,
                (final_size.height - height) / 2.0,
                (final_size.width - start - end - widths.0 - widths.1 - leading_gap - trailing_gap)
                    .max(0.0),
                height,
            ));
        }
        if let Some(trailing) = children.get(3) {
            let height = this.trailing_content_width();
            trailing.arrange(Rect::new(
                final_size.width - end - widths.1,
                (final_size.height - height) / 2.0,
                widths.1,
                height,
            ));
        }
        final_size
    }
}
ferro_properties! { impl MaterialChipLayout {
    pub fn kind_property() -> StyledProperty<MaterialChipKind> { FerroProperty::register::<MaterialChipLayout, _>("Kind", MaterialChipKind::Assist) }
    pub fn leading_icon_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<MaterialChipLayout, _>("LeadingIcon", None) }
    pub fn avatar_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<MaterialChipLayout, _>("Avatar", None) }
    pub fn trailing_icon_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<MaterialChipLayout, _>("TrailingIcon", None) }
    pub fn is_selected_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("IsSelected", false) }
    pub fn show_selected_icon_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("ShowSelectedIcon", true) }
    pub fn is_expressive_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("IsExpressive", false) }
    pub fn is_dismissible_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("IsDismissible", false) }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> { FerroProperty::register::<MaterialChipLayout, _>("MotionScheme", MaterialMotionScheme::Expressive) }
    pub fn reduced_motion_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("ReducedMotion", false) }
    pub fn displayed_leading_icon_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<MaterialChipLayout, _>("DisplayedLeadingIcon", None) }
    pub fn displayed_avatar_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<MaterialChipLayout, _>("DisplayedAvatar", None) }
    pub fn displayed_trailing_icon_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<MaterialChipLayout, _>("DisplayedTrailingIcon", None) }
    pub fn show_leading_icon_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("ShowLeadingIcon", false) }
    pub fn show_avatar_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("ShowAvatar", false) }
    pub fn show_check_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("ShowCheck", false) }
    pub fn show_trailing_icon_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("ShowTrailingIcon", false) }
    pub fn show_dismiss_property() -> StyledProperty<bool> { FerroProperty::register::<MaterialChipLayout, _>("ShowDismiss", false) }
    pub fn leading_content_width_property() -> StyledProperty<f64> { FerroProperty::register::<MaterialChipLayout, _>("LeadingContentWidth", 18.0) }
    pub fn trailing_content_width_property() -> StyledProperty<f64> { FerroProperty::register::<MaterialChipLayout, _>("TrailingContentWidth", 18.0) }
}}
impl MaterialChipLayout {
    pub fn kind(&self) -> MaterialChipKind {
        self.get_value(Self::kind_property())
    }
    pub fn set_kind(&self, value: MaterialChipKind) {
        self.set_value(Self::kind_property(), value);
    }
    pub fn leading_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::leading_icon_property())
    }
    pub fn set_leading_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::leading_icon_property(), value);
    }
    pub fn avatar(&self) -> Option<BoxedValue> {
        self.get_value(Self::avatar_property())
    }
    pub fn set_avatar(&self, value: Option<BoxedValue>) {
        self.set_value(Self::avatar_property(), value);
    }
    pub fn trailing_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::trailing_icon_property())
    }
    pub fn set_trailing_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::trailing_icon_property(), value);
    }
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }
    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value);
    }
    pub fn show_selected_icon(&self) -> bool {
        self.get_value(Self::show_selected_icon_property())
    }
    pub fn set_show_selected_icon(&self, value: bool) {
        self.set_value(Self::show_selected_icon_property(), value);
    }
    pub fn is_expressive(&self) -> bool {
        self.get_value(Self::is_expressive_property())
    }
    pub fn set_is_expressive(&self, value: bool) {
        self.set_value(Self::is_expressive_property(), value);
    }
    pub fn is_dismissible(&self) -> bool {
        self.get_value(Self::is_dismissible_property())
    }
    pub fn set_is_dismissible(&self, value: bool) {
        self.set_value(Self::is_dismissible_property(), value);
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
    pub fn displayed_leading_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::displayed_leading_icon_property())
    }
    pub fn displayed_avatar(&self) -> Option<BoxedValue> {
        self.get_value(Self::displayed_avatar_property())
    }
    pub fn displayed_trailing_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::displayed_trailing_icon_property())
    }
    pub fn show_leading_icon(&self) -> bool {
        self.get_value(Self::show_leading_icon_property())
    }
    pub fn show_avatar(&self) -> bool {
        self.get_value(Self::show_avatar_property())
    }
    pub fn show_check(&self) -> bool {
        self.get_value(Self::show_check_property())
    }
    pub fn show_trailing_icon(&self) -> bool {
        self.get_value(Self::show_trailing_icon_property())
    }
    pub fn show_dismiss(&self) -> bool {
        self.get_value(Self::show_dismiss_property())
    }
    pub fn leading_content_width(&self) -> f64 {
        self.get_value(Self::leading_content_width_property())
    }
    pub fn trailing_content_width(&self) -> f64 {
        self.get_value(Self::trailing_content_width_property())
    }
    pub fn construct() -> Self {
        let scheme = MaterialMotionScheme::Expressive;
        Self {
            base: Panel::construct(),
            states: RefCell::new([
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::FastSpatial), 0.0),
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::SlowEffects), 0.0),
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::FastSpatial), 0.0),
                MaterialSpringState::new(scheme.spring(MaterialMotionRole::SlowEffects), 0.0),
            ]),
            leading_mode: Cell::new(0),
            trailing_mode: Cell::new(0),
            wanted_leading: Cell::new(0),
            wanted_trailing: Cell::new(0),
            painted_height: Cell::new(32.0),
            clock: RefCell::new(None),
            visibility: RefCell::new(None),
            last_frame: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn slot_widths(&self) -> (f64, f64) {
        let states = self.states.borrow();
        (
            states[0].current().value.max(0.0),
            states[2].current().value.max(0.0),
        )
    }
    fn publish_full_shape(&self, width: f64, height: f64) {
        if let Some(owner) = self
            .templated_parent()
            .and_then(|parent| parent.cast::<Button>())
        {
            owner.set_current_value(
                MaterialChip::full_corner_radius_property(),
                CornerRadius::uniform(width.min(height) / 2.0),
            );
        }
    }
    pub fn slot_opacities(&self) -> (f64, f64) {
        let states = self.states.borrow();
        (
            states[1].current().value.clamp(0.0, 1.0),
            states[3].current().value.clamp(0.0, 1.0),
        )
    }
    fn selectable(&self) -> bool {
        matches!(
            self.kind(),
            MaterialChipKind::Filter | MaterialChipKind::Input
        )
    }
    fn insets(&self) -> (f64, f64, f64, f64) {
        let leading = self.wanted_leading.get();
        let trailing = self.wanted_trailing.get();
        let (start, end) = if self.kind() == MaterialChipKind::Input {
            (
                if leading == 2 || leading == 0 {
                    4.0
                } else {
                    8.0
                },
                if trailing == 0 { 4.0 } else { 8.0 },
            )
        } else {
            (8.0, 8.0)
        };
        let compact = self.is_expressive() && self.selectable();
        (
            start,
            end,
            if compact && leading != 0 { 4.0 } else { 8.0 },
            if compact && trailing != 0 { 4.0 } else { 8.0 },
        )
    }
    fn refresh_content(&self) {
        let leading = if self.kind() == MaterialChipKind::Input && self.avatar().is_some() {
            2
        } else if self.selectable() && self.is_selected() && self.show_selected_icon() {
            3
        } else if self.leading_icon().is_some() {
            1
        } else {
            0
        };
        let trailing = if self.is_dismissible() {
            2
        } else if self.trailing_icon().is_some() {
            1
        } else {
            0
        };
        self.wanted_leading.set(leading);
        self.wanted_trailing.set(trailing);
        if leading != 0 {
            // Clear the previous presenter before transferring any actual Control to the other slot.
            if leading != self.leading_mode.get() {
                self.set_current_value(Self::displayed_leading_icon_property(), None);
                self.set_current_value(Self::displayed_avatar_property(), None);
            }
            self.leading_mode.set(leading);
            self.set_current_value(
                Self::displayed_leading_icon_property(),
                if leading == 1 {
                    self.leading_icon()
                } else {
                    None
                },
            );
            self.set_current_value(
                Self::displayed_avatar_property(),
                if leading == 2 { self.avatar() } else { None },
            );
            self.set_current_value(
                Self::leading_content_width_property(),
                if leading == 2 { 24.0 } else { 18.0 },
            );
        }
        if trailing != 0 {
            self.trailing_mode.set(trailing);
            self.set_current_value(
                Self::displayed_trailing_icon_property(),
                if trailing == 1 {
                    self.trailing_icon()
                } else {
                    None
                },
            );
            self.set_current_value(
                Self::trailing_content_width_property(),
                if trailing == 2 { 48.0 } else { 18.0 },
            );
        }
        let specs = self.motion_scheme();
        {
            let mut states = self.states.borrow_mut();
            for (slot, visible, width) in [
                (0, leading != 0, self.leading_content_width()),
                (2, trailing != 0, self.trailing_content_width()),
            ] {
                let reduced = !animations_enabled(self.reduced_motion()) || !self.selectable();
                states[slot].set_reduced_motion(reduced);
                states[slot + 1].set_reduced_motion(reduced);
                let spatial_role = if self.is_expressive() || visible {
                    MaterialMotionRole::FastSpatial
                } else {
                    MaterialMotionRole::DefaultEffects
                };
                let alpha_role = if self.is_expressive() {
                    MaterialMotionRole::DefaultEffects
                } else if visible {
                    MaterialMotionRole::SlowEffects
                } else {
                    MaterialMotionRole::FastEffects
                };
                states[slot].set_spec(specs.spring(spatial_role));
                states[slot + 1].set_spec(specs.spring(alpha_role));
                states[slot].retarget(if visible { width } else { 0.0 });
                states[slot + 1].retarget(if visible { 1.0 } else { 0.0 });
            }
            if !self.is_attached_to_visual_tree() || !self.selectable() {
                for state in states.iter_mut() {
                    state.finish();
                }
            }
        }
        self.publish_frame();
        self.update_clock();
    }
    fn at_rest(index: usize, state: &MaterialSpringState) -> bool {
        if index % 2 == 0 {
            state.is_at_rest(1.0, 62.5)
        } else {
            state.is_at_rest(0.01, 0.625)
        }
    }
    fn motion_pending(&self) -> bool {
        self.states
            .borrow()
            .iter()
            .enumerate()
            .any(|(index, state)| !Self::at_rest(index, state))
    }
    fn publish_frame(&self) {
        let samples = self.states.borrow().map(|state| state.current().value);
        if self.wanted_leading.get() == 0 && samples[0] == 0.0 && samples[1] == 0.0 {
            self.leading_mode.set(0);
            self.set_current_value(Self::displayed_leading_icon_property(), None);
            self.set_current_value(Self::displayed_avatar_property(), None);
        }
        if self.wanted_trailing.get() == 0 && samples[2] == 0.0 && samples[3] == 0.0 {
            self.trailing_mode.set(0);
            self.set_current_value(Self::displayed_trailing_icon_property(), None);
        }
        self.set_current_value(
            Self::show_leading_icon_property(),
            self.leading_mode.get() == 1,
        );
        self.set_current_value(Self::show_avatar_property(), self.leading_mode.get() == 2);
        self.set_current_value(Self::show_check_property(), self.leading_mode.get() == 3);
        self.set_current_value(
            Self::show_trailing_icon_property(),
            self.trailing_mode.get() == 1,
        );
        self.set_current_value(Self::show_dismiss_property(), self.trailing_mode.get() == 2);
        let children = self.children().snapshot();
        if let Some(leading) = children.get(1) {
            leading.set_opacity(samples[1].clamp(0.0, 1.0));
        }
        if let Some(trailing) = children.get(3) {
            trailing.set_opacity(samples[3].clamp(0.0, 1.0));
        }
        self.invalidate_measure();
    }
    fn advance_motion(&self, seconds: f64) {
        {
            let mut states = self.states.borrow_mut();
            for (index, state) in states.iter_mut().enumerate() {
                state.set_reduced_motion(
                    !animations_enabled(self.reduced_motion()) || !self.selectable(),
                );
                state.advance(seconds);
                if Self::at_rest(index, state) {
                    state.finish();
                }
            }
        }
        self.publish_frame();
    }
    fn update_clock(&self) {
        if !animations_enabled(self.reduced_motion()) || !self.selectable() {
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
impl Drop for MaterialChipLayout {
    fn drop(&mut self) {
        if let Some(subscription) = self.clock.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility.get_mut().take() {
            subscription.dispose();
        }
    }
}
