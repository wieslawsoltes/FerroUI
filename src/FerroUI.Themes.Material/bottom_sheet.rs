// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! In-window Material modal bottom sheets, with deterministic anchors, dragging,
//! focus containment, back navigation and source-defined spring roles.

use crate::motion::animations_enabled;
use crate::{
    MaterialAppBarFling, MaterialMotionRole, MaterialMotionScheme, MaterialSheetAnchors,
    MaterialSheetState, MaterialSpringSpec,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    FocusManager, IPointer, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    KeyboardNavigation, KeyboardNavigationMode, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{MediaContext, TranslateTransform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Point, Ref, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness, Vector,
    Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef, ferro_class,
    ferro_class_info, ferro_impl_classes, ferro_properties, ferro_routed_event, instantiate,
};
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::{Border, ContentControl, ContentControlImpl, ControlImpl, ScrollViewer};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A modal sheet placed as the last child of the application's overlay `Grid`.
/// Its `Content` is the sheet body. `State` is the requested semantic state;
/// [`Self::current_state`] changes when the transition settles.
///
/// The host owns scrim input, sheet/scroll-content pointer capture, Escape/back navigation,
/// focus containment/restoration and completion events. Supply the platform's
/// insets through `SafeInsets`; no extra native window is created.
#[repr(C)]
pub struct MaterialBottomSheetHost {
    base: ContentControl,
    sheet: RefCell<Option<Ref<Border>>>,
    scrim: RefCell<Option<Ref<Border>>>,
    content_scroll: RefCell<Option<Ref<ScrollViewer>>>,
    translation: RefCell<Option<Ref<TranslateTransform>>>,
    anchors: Cell<MaterialSheetAnchors>,
    current: Cell<MaterialSheetState>,
    offset: Cell<f64>,
    velocity: Cell<f64>,
    scrim_opacity: Cell<f64>,
    animation: RefCell<Option<SheetAnimation>>,
    content_fling: RefCell<Option<SheetContentFling>>,
    drag: Cell<Option<DragState>>,
    touch: RefCell<Option<SheetTouch>>,
    scrim_press: Cell<Option<i32>>,
    previous_focus: RefCell<Option<WeakRef<InputElement>>>,
    confirm: RefCell<Option<Rc<dyn Fn(MaterialSheetState) -> bool>>>,
    changing: Cell<bool>,
    frame_pending: Cell<bool>,
    frame_generation: Cell<u64>,
    last_frame: Cell<Option<TimeSpan>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(MaterialBottomSheetHost: ContentControl);
ferro_class_info!(MaterialBottomSheetHost {
    new: MaterialBottomSheetHost::new,
    markup: { attributes: [PseudoClasses(":open", ":dragging", ":expanded", ":partial")] },
});
ferro_impl_classes!(MaterialBottomSheetHost: StyledElementImpl, InteractiveImpl, ContentControlImpl);

ferro_properties! { impl MaterialBottomSheetHost {
    /// Requested state. Bind this property two-way to application state.
    pub fn state_property() -> StyledProperty<MaterialSheetState> {
        FerroProperty::register_with::<Self, _>("State",
            StyledPropertyOptions::new(MaterialSheetState::Hidden).default_binding_mode(BindingMode::TwoWay))
    }
    /// Whether the deterministic partial anchor is available.
    pub fn is_partial_enabled_property() -> StyledProperty<bool> {
        FerroProperty::register::<Self, _>("IsPartialEnabled", true)
    }
    /// Whether scrim, drag and Escape may dismiss the sheet.
    pub fn is_dismiss_enabled_property() -> StyledProperty<bool> {
        FerroProperty::register::<Self, _>("IsDismissEnabled", true)
    }
    /// Safe area supplied by the host, including system bars and keyboard insets.
    pub fn safe_insets_property() -> StyledProperty<Thickness> {
        FerroProperty::register_with::<Self, _>("SafeInsets", StyledPropertyOptions::new(Thickness::default())
            .validate(|value: &Thickness| [value.left, value.top, value.right, value.bottom]
                .into_iter().all(|side| side.is_finite() && side >= 0.0)))
    }
    pub fn reduced_motion_property() -> StyledProperty<bool> {
        FerroProperty::register::<Self, _>("ReducedMotion", false)
    }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> {
        FerroProperty::register::<Self, _>("MotionScheme", MaterialMotionScheme::Expressive)
    }
} }

impl ControlImpl for MaterialBottomSheetHost {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        crate::MaterialBottomSheetAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for MaterialBottomSheetHost {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "State" if !this.changing.get() => {
                let old = change
                    .get_old_value::<MaterialSheetState>()
                    .unwrap_or_default();
                let mut target = this.state();
                if target == MaterialSheetState::PartiallyExpanded && !this.is_partial_enabled() {
                    target = MaterialSheetState::Expanded;
                }
                if !this.permits(target) {
                    this.set_state_without_transition(old);
                    return;
                }
                if target != this.state() {
                    this.set_state_without_transition(target);
                }
                this.transition_to(target, false);
            }
            "IsPartialEnabled"
                if !this.is_partial_enabled()
                    && this.state() == MaterialSheetState::PartiallyExpanded =>
            {
                this.set_current_value(Self::state_property(), MaterialSheetState::Expanded);
            }
            "ReducedMotion" | "MotionScheme" => {
                if this.content_fling.borrow().is_some()
                    && !animations_enabled(this.reduced_motion())
                {
                    this.stop_content_fling();
                    this.settle_drag(0.0);
                } else if this.animation.borrow().is_some() {
                    this.transition_to(this.state(), false);
                }
            }
            "SafeInsets" => this.invalidate_arrange(),
            "IsEffectivelyEnabled" if !this.is_effectively_enabled() => this.cancel_drag(),
            _ => {}
        }
    }
}

impl VisualImpl for MaterialBottomSheetHost {
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        let weak = this.to_ref().downgrade();
        *this.visibility_subscription.borrow_mut() =
            Some(this.is_effectively_visible_changed(move || {
                if let Some(this) = weak.upgrade() {
                    if this.is_effectively_visible() {
                        this.request_frame();
                    } else {
                        this.invalidate_frame();
                        this.content_fling.borrow_mut().take();
                        this.cancel_drag();
                    }
                }
            }));
        if this.state() != MaterialSheetState::Hidden {
            this.transition_to(this.state(), false);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        this.invalidate_frame();
        this.drag.set(None);
        this.release_touch();
        this.scrim_press.set(None);
        this.animation.borrow_mut().take();
        this.content_fling.borrow_mut().take();
        // No completion event for an interrupted, detached visual.
        this.restore_focus();
        Self::parent_on_detached_from_visual_tree(this, event);
    }
}

impl LayoutableImpl for MaterialBottomSheetHost {
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let result = Self::parent_arrange_override(this, final_size);
        let insets = this.safe_insets();
        let height = (final_size.height - insets.top - insets.bottom).max(0.0);
        let measured = this
            .sheet
            .borrow()
            .as_ref()
            .map_or(0.0, |sheet| sheet.bounds().height);
        let anchors = MaterialSheetAnchors::new(height, measured);
        let previous = this.anchors.replace(anchors);
        if previous != anchors {
            let target = if previous.hidden > 0.0 {
                anchors.reconcile_target(previous, this.state(), this.is_partial_enabled())
            } else {
                this.state()
            };
            if target != this.state() {
                this.set_state_without_transition(target);
            }
            if this.drag.get().is_some()
                || this
                    .touch
                    .borrow()
                    .as_ref()
                    .is_some_and(|touch| touch.dragging)
            {
                this.offset
                    .set(this.offset.get().clamp(anchors.expanded, anchors.hidden));
                this.apply_visuals();
            } else if target == MaterialSheetState::Hidden && this.animation.borrow().is_none() {
                this.offset.set(anchors.hidden);
                this.apply_visuals();
            } else {
                // Before the first measured frame the sheet starts at the bottom edge.
                if previous.hidden == 0.0 {
                    this.offset.set(anchors.hidden);
                }
                this.transition_to(target, false);
            }
        }
        result
    }
}

impl TemplatedControlImpl for MaterialBottomSheetHost {
    fn on_apply_template(this: &Self, event: &TemplateAppliedEventArgs) {
        let was_flinging = this.content_fling.borrow().is_some();
        if was_flinging {
            this.stop_content_fling();
            this.settle_drag(0.0);
        }
        Self::parent_on_apply_template(this, event);
        let sheet = event.name_scope().find_as::<Border>("PART_Sheet");
        let transform = TranslateTransform::new();
        if let Some(sheet) = &sheet {
            sheet.set_render_transform(Some(transform.clone().into()));
        }
        *this.sheet.borrow_mut() = sheet;
        *this.translation.borrow_mut() = Some(transform);
        *this.scrim.borrow_mut() = event.name_scope().find_as::<Border>("PART_Scrim");
        *this.content_scroll.borrow_mut() = event
            .name_scope()
            .find_as::<ScrollViewer>("PART_ContentScroll");
        this.apply_visuals();
    }
}

impl InputElementImpl for MaterialBottomSheetHost {
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        if !event.handled()
            && event.key == Key::Escape
            && this.state() != MaterialSheetState::Hidden
        {
            this.handle_back();
            event.set_handled(true);
        }
        Self::parent_on_key_down(this, event);
    }

    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, event);
        if event.handled()
            || !this.is_effectively_enabled()
            || this.state() == MaterialSheetState::Hidden
            || (event.pointer().type_() == PointerType::Mouse
                && !event.properties().is_left_button_pressed)
        {
            return;
        }
        if this.source_is_part(event.source(), "PART_DragHandle") {
            let point = event.get_position(Some(this));
            this.begin_drag(point.y, event.timestamp(), event.pointer().id());
            event.pointer().capture(Some(&this.to_ref().upcast()));
            event.prevent_gesture_recognition();
            event.set_handled(true);
        } else if this.source_is_part(event.source(), "PART_Scrim") {
            this.scrim_press.set(Some(event.pointer().id()));
            event.set_handled(true);
        }
    }

    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, event);
        if this
            .drag
            .get()
            .is_some_and(|drag| drag.pointer_id == event.pointer().id())
        {
            this.update_drag(event.get_position(Some(this)).y, event.timestamp());
            event.set_handled(true);
        }
    }

    fn on_pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, event);
        if this
            .drag
            .get()
            .is_some_and(|drag| drag.pointer_id == event.pointer().id())
        {
            this.update_drag(event.get_position(Some(this)).y, event.timestamp());
            this.end_drag();
            event.pointer().capture(None);
            event.set_handled(true);
        } else if this.scrim_press.take() == Some(event.pointer().id()) {
            if this.source_is_part(event.source(), "PART_Scrim") && this.is_dismiss_enabled() {
                this.hide();
            }
            event.set_handled(true);
        }
    }

    fn on_pointer_capture_lost(this: &Self, event: &PointerCaptureLostEventArgs) {
        if this.drag.get().is_some()
            || this
                .touch
                .borrow()
                .as_ref()
                .is_some_and(|touch| touch.dragging)
        {
            this.cancel_drag();
        }
        Self::parent_on_pointer_capture_lost(this, event);
    }
}

impl MaterialBottomSheetHost {
    fn static_constructor() {
        Visual::is_visible_property().override_default_value::<Self>(false);
        InputElement::focusable_property().override_default_value::<Self>(true);
    }

    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            sheet: RefCell::new(None),
            scrim: RefCell::new(None),
            content_scroll: RefCell::new(None),
            translation: RefCell::new(None),
            anchors: Cell::new(MaterialSheetAnchors::default()),
            current: Cell::new(MaterialSheetState::Hidden),
            offset: Cell::new(0.0),
            velocity: Cell::new(0.0),
            scrim_opacity: Cell::new(0.0),
            animation: RefCell::new(None),
            content_fling: RefCell::new(None),
            drag: Cell::new(None),
            touch: RefCell::new(None),
            scrim_press: Cell::new(None),
            previous_focus: RefCell::new(None),
            confirm: RefCell::new(None),
            changing: Cell::new(false),
            frame_pending: Cell::new(false),
            frame_generation: Cell::new(0),
            last_frame: Cell::new(None),
            visibility_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        KeyboardNavigation::set_tab_navigation(&this, KeyboardNavigationMode::Cycle);
        let weak = this.downgrade();
        this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, event| {
                if let Some(this) = weak.upgrade() {
                    this.touch_pressed(event);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = this.downgrade();
        this.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, event| {
                if let Some(this) = weak.upgrade() {
                    this.touch_moved(event);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = this.downgrade();
        this.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, event| {
                if let Some(this) = weak.upgrade() {
                    this.touch_released(event);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        this
    }

    pub fn state(&self) -> MaterialSheetState {
        self.get_value(Self::state_property())
    }
    pub fn set_state(&self, value: MaterialSheetState) {
        self.set_value(Self::state_property(), value);
    }
    pub fn is_partial_enabled(&self) -> bool {
        self.get_value(Self::is_partial_enabled_property())
    }
    pub fn set_is_partial_enabled(&self, value: bool) {
        self.set_value(Self::is_partial_enabled_property(), value);
    }
    pub fn is_dismiss_enabled(&self) -> bool {
        self.get_value(Self::is_dismiss_enabled_property())
    }
    pub fn set_is_dismiss_enabled(&self, value: bool) {
        self.set_value(Self::is_dismiss_enabled_property(), value);
    }
    pub fn safe_insets(&self) -> Thickness {
        self.get_value(Self::safe_insets_property())
    }
    pub fn set_safe_insets(&self, value: Thickness) {
        self.set_value(Self::safe_insets_property(), value);
    }
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }
    pub fn motion_scheme(&self) -> MaterialMotionScheme {
        self.get_value(Self::motion_scheme_property())
    }
    pub fn set_motion_scheme(&self, value: MaterialMotionScheme) {
        self.set_value(Self::motion_scheme_property(), value);
    }
    pub fn current_state(&self) -> MaterialSheetState {
        self.current.get()
    }
    pub fn anchors(&self) -> MaterialSheetAnchors {
        self.anchors.get()
    }
    pub fn offset(&self) -> f64 {
        self.offset.get()
    }
    pub fn is_animating(&self) -> bool {
        self.animation.borrow().is_some() || self.content_fling.borrow().is_some()
    }
    #[cfg(test)]
    pub(crate) fn has_content_fling(&self) -> bool {
        self.content_fling.borrow().is_some()
    }
    #[cfg(test)]
    pub(crate) fn advance_for_test(&self, seconds: f64) {
        self.advance(seconds);
    }

    /// A synchronous veto applied to programmatic state changes and user gestures.
    /// The callback must not change this host's state reentrantly.
    pub fn set_confirm_state_change(
        &self,
        confirm: Option<Rc<dyn Fn(MaterialSheetState) -> bool>>,
    ) {
        *self.confirm.borrow_mut() = confirm;
    }

    pub fn show(&self) -> bool {
        self.request_state(if self.is_partial_enabled() {
            MaterialSheetState::PartiallyExpanded
        } else {
            MaterialSheetState::Expanded
        })
    }
    pub fn expand(&self) -> bool {
        self.request_state(MaterialSheetState::Expanded)
    }
    pub fn hide(&self) -> bool {
        self.request_state(MaterialSheetState::Hidden)
    }

    /// Expanded → partial → hidden, following the modal back behavior.
    pub fn handle_back(&self) -> bool {
        if self.state() == MaterialSheetState::Hidden {
            return false;
        }
        if self.state() == MaterialSheetState::Expanded
            && self.is_partial_enabled()
            && !self.anchors.get().partial_matches_expanded()
        {
            self.request_state(MaterialSheetState::PartiallyExpanded)
        } else if self.is_dismiss_enabled() {
            self.hide()
        } else {
            false
        }
    }

    pub fn request_state(&self, state: MaterialSheetState) -> bool {
        let state = if state == MaterialSheetState::PartiallyExpanded && !self.is_partial_enabled()
        {
            MaterialSheetState::Expanded
        } else {
            state
        };
        if self.state() == state {
            return true;
        }
        self.set_current_value(Self::state_property(), state);
        self.state() == state
    }

    ferro_routed_event!(
        pub fn dismissed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Self, _>("Dismissed", RoutingStrategies::BUBBLE)
        }
    );
    ferro_routed_event!(
        pub fn settled_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Self, _>("Settled", RoutingStrategies::BUBBLE)
        }
    );
    pub fn dismissed(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::dismissed_event(), handler)
    }
    pub fn settled(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::settled_event(), handler)
    }

    fn permits(&self, state: MaterialSheetState) -> bool {
        let confirm = self.confirm.borrow().clone();
        confirm.is_none_or(|confirm| confirm(state))
    }

    fn set_state_without_transition(&self, state: MaterialSheetState) {
        self.changing.set(true);
        self.set_current_value(Self::state_property(), state);
        self.changing.set(false);
    }

    fn transition_to(&self, target: MaterialSheetState, from_drag: bool) {
        self.stop_content_fling();
        if target != MaterialSheetState::Hidden {
            if self.previous_focus.borrow().is_none() {
                *self.previous_focus.borrow_mut() = FocusManager::get_focus_manager(self)
                    .and_then(|manager| manager.get_focused_element())
                    .map(|input| input.downgrade());
            }
            self.set_current_value(Visual::is_visible_property(), true);
            if !self.is_keyboard_focus_within() {
                self.focus();
            }
        }
        let anchors = self.anchors.get();
        let role = if target == MaterialSheetState::Hidden && !from_drag {
            MaterialMotionRole::FastEffects
        } else {
            MaterialMotionRole::DefaultSpatial
        };
        let old = self.animation.borrow_mut().take();
        let alpha_velocity = old.as_ref().map_or(0.0, |animation| {
            animation.alpha_sample(self.motion_scheme()).1
        });
        *self.animation.borrow_mut() = Some(SheetAnimation {
            spec: self.motion_scheme().spring(role),
            from: self.offset.get(),
            to: anchors.offset(target),
            velocity: self.velocity.get(),
            elapsed: 0.0,
            alpha_from: self.scrim_opacity.get(),
            alpha_to: if target == MaterialSheetState::Hidden {
                0.0
            } else {
                0.32
            },
            alpha_velocity,
        });
        self.pseudo_classes()
            .set(":open", target != MaterialSheetState::Hidden);
        self.pseudo_classes()
            .set(":expanded", target == MaterialSheetState::Expanded);
        self.pseudo_classes()
            .set(":partial", target == MaterialSheetState::PartiallyExpanded);
        if !animations_enabled(self.reduced_motion()) || !self.is_attached_to_visual_tree() {
            self.finish_transition();
        } else {
            self.request_frame();
        }
    }

    fn finish_transition(&self) {
        let animation = self.animation.borrow_mut().take();
        if animation.is_none() {
            return;
        }
        self.invalidate_frame();
        let target = self.state();
        let previous = self.current.replace(target);
        self.offset.set(self.anchors.get().offset(target));
        self.velocity.set(0.0);
        self.scrim_opacity
            .set(if target == MaterialSheetState::Hidden {
                0.0
            } else {
                0.32
            });
        self.apply_visuals();
        if target == MaterialSheetState::Hidden {
            self.set_current_value(Visual::is_visible_property(), false);
            self.restore_focus();
        }
        if previous != target {
            self.raise_event(&RoutedEventArgs::with_event_and_source(
                Self::settled_event(),
                self.to_ref(),
            ));
            if target == MaterialSheetState::Hidden {
                self.raise_event(&RoutedEventArgs::with_event_and_source(
                    Self::dismissed_event(),
                    self.to_ref(),
                ));
            }
        }
    }

    fn advance(&self, seconds: f64) {
        if self.content_fling.borrow().is_some() {
            self.advance_content_fling(seconds);
            return;
        }
        if !animations_enabled(self.reduced_motion()) {
            self.finish_transition();
            return;
        }
        let done = {
            let mut slot = self.animation.borrow_mut();
            let Some(animation) = slot.as_mut() else {
                return;
            };
            animation.elapsed += if seconds.is_finite() {
                seconds.max(0.0)
            } else {
                0.0
            };
            let value = animation.spec.sample(
                animation.elapsed,
                animation.from,
                animation.to,
                animation.velocity,
            );
            let (alpha, alpha_velocity) = animation.alpha_sample(self.motion_scheme());
            self.offset.set(
                value
                    .value
                    .clamp(self.anchors.get().expanded, self.anchors.get().hidden),
            );
            self.velocity.set(value.velocity);
            self.scrim_opacity.set(alpha.clamp(0.0, 0.32));
            ((value.value - animation.to).abs() < 0.05
                && value.velocity.abs() < 0.5
                && (alpha - animation.alpha_to).abs() < 0.001
                && alpha_velocity.abs() < 0.01)
                || animation.elapsed > 5.0
        };
        self.apply_visuals();
        if done {
            self.finish_transition();
        }
    }

    fn apply_visuals(&self) {
        if let Some(transform) = self.translation.borrow().as_ref() {
            transform.set_y(self.offset.get());
        }
        if let Some(scrim) = self.scrim.borrow().as_ref() {
            scrim.set_opacity(self.scrim_opacity.get());
        }
    }

    fn restore_focus(&self) {
        let previous = self
            .previous_focus
            .borrow_mut()
            .take()
            .and_then(|weak| weak.upgrade());
        if let Some(previous) = previous {
            if previous.is_attached_to_visual_tree() && previous.is_effectively_visible() {
                previous.focus();
            }
        }
    }

    fn source_is_part(&self, source: Option<Ref<FerroObject>>, name: &str) -> bool {
        source
            .and_then(|source| source.cast::<Visual>())
            .is_some_and(|source| {
                let root = self.to_ref().upcast::<Visual>();
                source
                    .get_self_and_visual_ancestors()
                    .take_while(|node| *node != root)
                    .any(|node| node.name().as_deref() == Some(name))
            })
    }

    // Touch is acquired only after vertical slop. Until then the child's normal
    // click, text-selection and horizontal gesture recognition remains active.
    fn touch_pressed(&self, event: &PointerPressedEventArgs) {
        if !matches!(
            event.pointer().type_(),
            PointerType::Touch | PointerType::Pen
        ) || !self.is_effectively_enabled()
            || self.state() == MaterialSheetState::Hidden
            || self.touch.borrow().is_some()
            || self.drag.get().is_some()
            || !self.source_is_part(event.source(), "PART_Sheet")
            || self.source_is_part(event.source(), "PART_DragHandle")
        {
            return;
        }
        // A new contact interrupts child inertia immediately, before drag slop.
        self.stop_content_fling();
        self.request_frame();
        let scroll = event
            .source()
            .and_then(|source| source.cast::<Visual>())
            .and_then(|source| {
                source
                    .get_self_and_visual_ancestors()
                    .take_while(|visual| *visual != self.to_ref().upcast::<Visual>())
                    .find_map(|visual| visual.cast::<ScrollViewer>())
            })
            .or_else(|| self.content_scroll.borrow().clone());
        let point = event.get_position(Some(self));
        *self.touch.borrow_mut() = Some(SheetTouch {
            pointer: event.pointer().clone(),
            origin: point,
            last: point,
            timestamp: event.timestamp(),
            velocity: 0.0,
            dragging: false,
            scroll: scroll.map(|scroll| scroll.downgrade()),
        });
    }

    fn touch_moved(&self, event: &PointerEventArgs) {
        if !self
            .touch
            .borrow()
            .as_ref()
            .is_some_and(|touch| touch.pointer.id() == event.pointer().id())
        {
            return;
        }
        let Some(mut touch) = self.touch.borrow_mut().take() else {
            return;
        };
        let point = event.get_position(Some(self));
        if !point.x.is_finite() || !point.y.is_finite() {
            *self.touch.borrow_mut() = Some(touch);
            return;
        }
        let mut delta = point.y - touch.last.y;
        if !touch.dragging {
            let movement = point - touch.origin;
            let slop = self.get_platform_settings().map_or(8.0, |settings| {
                (settings.get_tap_size(event.pointer().type_()).height / 2.0).max(1.0)
            });
            if movement.x.abs() > slop && movement.x.abs() > movement.y.abs() {
                return; // The child owns a horizontal gesture.
            }
            if movement.y.abs() <= slop {
                *self.touch.borrow_mut() = Some(touch);
                return;
            }
            touch.dragging = true;
            delta = movement.y - movement.y.signum() * slop;
            self.animation.borrow_mut().take();
            self.invalidate_frame();
            self.velocity.set(0.0);
            self.pseudo_classes().set(":dragging", true);
            touch.pointer.capture(Some(&self.to_ref().upcast()));
        }
        let elapsed = event.timestamp().saturating_sub(touch.timestamp) as f64 / 1000.0;
        if elapsed > 0.0 {
            touch.velocity = if elapsed <= 0.15 {
                delta / elapsed
            } else {
                0.0
            };
        }
        touch.last = point;
        touch.timestamp = event.timestamp();
        let scroll = touch.scroll.as_ref().and_then(WeakRef::upgrade);
        self.dispatch_nested_delta(delta, scroll.as_deref());
        *self.touch.borrow_mut() = Some(touch);
        event.prevent_gesture_recognition();
        event.set_handled(true);
    }

    fn touch_released(&self, event: &PointerReleasedEventArgs) {
        if !self
            .touch
            .borrow()
            .as_ref()
            .is_some_and(|touch| touch.pointer.id() == event.pointer().id())
        {
            return;
        }
        let Some(touch) = self.touch.borrow_mut().take() else {
            return;
        };
        if touch.dragging {
            self.pseudo_classes().set(":dragging", false);
            let velocity = if event.timestamp().saturating_sub(touch.timestamp) <= 150 {
                touch.velocity
            } else {
                0.0
            };
            let scroll = touch.scroll.as_ref().and_then(WeakRef::upgrade);
            self.release_nested_fling(velocity, scroll.as_deref());
            touch.pointer.capture(None);
            event.prevent_gesture_recognition();
            event.set_handled(true);
        }
    }

    /// Pinned SheetDefaults nested-scroll order, in finger-motion coordinates:
    /// upward input expands the sheet first; the child consumes the remainder;
    /// only input left by the child can collapse the sheet.
    fn dispatch_nested_delta(&self, delta: f64, scroll: Option<&ScrollViewer>) {
        let mut available = delta;
        if delta < 0.0 {
            available -= self.consume_sheet_delta(delta);
        }
        if let Some(scroll) = scroll {
            let old = scroll.offset();
            scroll.set_current_value(
                ScrollViewer::offset_property(),
                Vector::new(old.x, old.y - available),
            );
            available -= old.y - scroll.offset().y;
        }
        if available != 0.0 {
            self.consume_sheet_delta(available);
        }
    }

    /// SheetDefaults.kt onPreFling consumes an upward release in full until the
    /// sheet is expanded. Otherwise the child decays first, and onPostFling
    /// passes only its remaining velocity to the anchored sheet.
    fn release_nested_fling(&self, velocity: f64, scroll: Option<&ScrollViewer>) {
        if velocity < 0.0 && self.offset.get() > self.anchors.get().expanded {
            self.settle_drag(velocity);
            return;
        }
        let scroll = scroll.filter(|scroll| {
            let maximum = (scroll.extent().height - scroll.viewport().height).max(0.0);
            scroll.is_attached_to_visual_tree()
                && if velocity < 0.0 {
                    scroll.offset().y < maximum
                } else {
                    scroll.offset().y > 0.0
                }
        });
        if animations_enabled(self.reduced_motion()) && velocity.abs() > 1.0 {
            if let Some(scroll) = scroll {
                self.stop_content_fling();
                self.animation.borrow_mut().take();
                self.velocity.set(0.0);
                *self.content_fling.borrow_mut() = Some(SheetContentFling {
                    curve: MaterialAppBarFling::new(velocity),
                    elapsed: 0.0,
                    previous: 0.0,
                    scroll: scroll.to_ref().downgrade(),
                });
                self.request_frame();
                return;
            }
        }
        self.settle_drag(velocity);
    }

    fn advance_content_fling(&self, seconds: f64) {
        let Some(mut fling) = self.content_fling.borrow_mut().take() else {
            return;
        };
        if !animations_enabled(self.reduced_motion()) || !self.is_effectively_enabled() {
            self.settle_drag(0.0);
            return;
        }
        let Some(scroll) = fling
            .scroll
            .upgrade()
            .filter(|scroll| scroll.is_attached_to_visual_tree())
        else {
            self.settle_drag(0.0);
            return;
        };
        let generation = self.frame_generation.get();
        fling.elapsed += if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        let (position, velocity) = fling.curve.sample(fling.elapsed);
        let delta = position - fling.previous;
        fling.previous = position;
        let old = scroll.offset();
        // Inertial deltas are not UserInput. Sending them through the drag's
        // pre/post-scroll path would move the sheet before the child fling ends.
        scroll.set_current_value(
            ScrollViewer::offset_property(),
            Vector::new(old.x, old.y - delta),
        );
        let consumed = old.y - scroll.offset().y;
        // Offset observers can dismiss, detach, or start a new interaction.
        // Such reentrancy invalidates this generation and must not restore it.
        if generation != self.frame_generation.get() {
            return;
        }
        if !self.is_attached_to_visual_tree() || !self.is_effectively_visible() {
            self.invalidate_frame();
            return;
        }
        if !animations_enabled(self.reduced_motion()) || !self.is_effectively_enabled() {
            self.settle_drag(0.0);
            return;
        }
        if (delta - consumed).abs() > 0.5 {
            // DefaultFlingBehavior cancels decay after an unconsumed half pixel.
            // Keep that frame's velocity for the sheet's normal settle policy.
            self.settle_drag(velocity);
        } else if fling.elapsed >= fling.curve.duration_seconds() {
            self.settle_drag(0.0);
        } else {
            *self.content_fling.borrow_mut() = Some(fling);
        }
    }

    fn stop_content_fling(&self) {
        self.content_fling.borrow_mut().take();
        self.invalidate_frame();
    }

    fn consume_sheet_delta(&self, delta: f64) -> f64 {
        if !delta.is_finite() {
            return 0.0;
        }
        let anchors = self.anchors.get();
        let old = self.offset.get();
        let next = (old + delta).clamp(anchors.expanded, self.maximum_drag_offset());
        self.offset.set(next);
        self.apply_visuals();
        next - old
    }

    fn maximum_drag_offset(&self) -> f64 {
        let anchors = self.anchors.get();
        if self.is_dismiss_enabled() {
            anchors.hidden
        } else if self.is_partial_enabled() {
            anchors.partially_expanded
        } else {
            anchors.expanded
        }
    }

    fn release_touch(&self) -> bool {
        let touch = self.touch.borrow_mut().take();
        if let Some(touch) = touch {
            if touch.dragging {
                touch.pointer.capture(None);
            }
            touch.dragging
        } else {
            false
        }
    }

    fn begin_drag(&self, y: f64, timestamp: u64, pointer_id: i32) {
        self.stop_content_fling();
        self.animation.borrow_mut().take();
        self.velocity.set(0.0);
        self.drag.set(Some(DragState {
            pointer_id,
            start_y: y,
            start_offset: self.offset.get(),
            last_y: y,
            last_timestamp: timestamp,
            velocity: 0.0,
        }));
        self.pseudo_classes().set(":dragging", true);
    }

    fn update_drag(&self, y: f64, timestamp: u64) {
        let Some(mut drag) = self.drag.get() else {
            return;
        };
        if !y.is_finite() {
            return;
        }
        if timestamp > drag.last_timestamp {
            let elapsed = (timestamp - drag.last_timestamp) as f64 / 1000.0;
            drag.velocity = if elapsed <= 0.15 {
                (y - drag.last_y) / elapsed
            } else {
                0.0
            };
        }
        drag.last_y = y;
        drag.last_timestamp = timestamp;
        let anchors = self.anchors.get();
        let maximum = self.maximum_drag_offset();
        self.offset
            .set((drag.start_offset + y - drag.start_y).clamp(anchors.expanded, maximum));
        self.drag.set(Some(drag));
        self.apply_visuals();
    }

    fn end_drag(&self) {
        let Some(drag) = self.drag.take() else { return };
        self.pseudo_classes().set(":dragging", false);
        self.settle_drag(drag.velocity);
    }

    fn settle_drag(&self, velocity: f64) {
        let anchors = self.anchors.get();
        let velocity = anchors.damp_velocity(self.offset.get(), velocity, 8000.0);
        let target = anchors.settle_target(
            self.current.get(),
            self.offset.get(),
            velocity,
            self.is_partial_enabled(),
            self.is_dismiss_enabled(),
        );
        let target = if self.permits(target) {
            target
        } else {
            self.current.get()
        };
        self.set_state_without_transition(target);
        self.velocity.set(velocity);
        self.transition_to(target, true);
    }

    fn cancel_drag(&self) {
        let was_flinging = self.content_fling.borrow_mut().take().is_some();
        let touch_dragging = self.release_touch();
        if self.drag.take().is_some() || touch_dragging || was_flinging {
            self.pseudo_classes().set(":dragging", false);
            self.velocity.set(0.0);
            self.transition_to(self.state(), true);
        }
    }

    fn invalidate_frame(&self) {
        self.frame_generation
            .set(self.frame_generation.get().wrapping_add(1));
        self.frame_pending.set(false);
        self.last_frame.set(None);
    }

    fn request_frame(&self) {
        if self.frame_pending.get()
            || (self.animation.borrow().is_none() && self.content_fling.borrow().is_none())
            || !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
        {
            return;
        }
        self.frame_pending.set(true);
        let generation = self.frame_generation.get();
        let weak = self.to_ref().downgrade();
        MediaContext::instance().request_animation_frame(move |now| {
            let Some(this) = weak.upgrade() else { return };
            if generation != this.frame_generation.get() {
                return;
            }
            this.frame_pending.set(false);
            if !this.is_attached_to_visual_tree() || !this.is_effectively_visible() {
                return;
            }
            let elapsed = this
                .last_frame
                .replace(Some(now))
                .map_or(0.0, |last| (now - last).total_seconds().max(0.0));
            this.advance(elapsed);
            this.request_frame();
        });
    }
}

#[derive(Clone, Copy)]
struct DragState {
    pointer_id: i32,
    start_y: f64,
    start_offset: f64,
    last_y: f64,
    last_timestamp: u64,
    velocity: f64,
}

struct SheetTouch {
    pointer: Rc<dyn IPointer>,
    origin: Point,
    last: Point,
    timestamp: u64,
    velocity: f64,
    dragging: bool,
    scroll: Option<WeakRef<ScrollViewer>>,
}

struct SheetAnimation {
    spec: MaterialSpringSpec,
    from: f64,
    to: f64,
    velocity: f64,
    elapsed: f64,
    alpha_from: f64,
    alpha_to: f64,
    alpha_velocity: f64,
}

struct SheetContentFling {
    curve: MaterialAppBarFling,
    elapsed: f64,
    previous: f64,
    scroll: WeakRef<ScrollViewer>,
}
impl SheetAnimation {
    fn alpha_sample(&self, scheme: MaterialMotionScheme) -> (f64, f64) {
        let sample = scheme.spring(MaterialMotionRole::DefaultEffects).sample(
            self.elapsed,
            self.alpha_from,
            self.alpha_to,
            self.alpha_velocity,
        );
        (sample.value, sample.velocity)
    }
}
