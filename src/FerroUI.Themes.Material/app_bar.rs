// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! Collapsing Material top app bars with native scroll arbitration and title semantics.
//! Reference: AndroidX Material3 AppBar.kt, revision
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff (Apache-2.0).

use crate::app_bar_layout::MaterialAppBarLayout;
use crate::app_bar_state::{
    MaterialAppBarFling, MaterialAppBarScrollBehavior, MaterialAppBarSize, MaterialAppBarState,
    cubic_bezier, interpolate_container,
};
use crate::motion::{
    MaterialMotionRole, MaterialMotionScheme, MaterialSpringState, animations_enabled,
};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, KeyModifiers, PointerCaptureLostEventArgs,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{FontWeight, IBrush, SolidColorBrush, TextAlignment};
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    BoxedValue, DirectProperty, ElementRef, FerroLocator, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions, Point, Ref, StyledElement,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, Thickness, Vector, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::{Border, Control, ControlImpl, ScrollViewer, StackPanel, TextBlock};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

struct Parts {
    layout: Ref<MaterialAppBarLayout>,
    container: Option<Ref<Border>>,
    compact: Option<Ref<StackPanel>>,
    expanded: Option<Ref<StackPanel>>,
    compact_title: Option<Ref<TextBlock>>,
    compact_subtitle: Option<Ref<TextBlock>>,
    expanded_title: Option<Ref<TextBlock>>,
    expanded_subtitle: Option<Ref<TextBlock>>,
    navigation: Option<Ref<ContentPresenter>>,
    actions: Option<Ref<ContentPresenter>>,
}
enum HeightAnimation {
    Fling {
        curve: MaterialAppBarFling,
        elapsed: f64,
        previous: f64,
    },
    ContentFling {
        curve: MaterialAppBarFling,
        elapsed: f64,
        previous: f64,
    },
    Snap(MaterialSpringState),
}
struct SourceHandlers {
    source: WeakRef<ScrollViewer>,
    changed: RoutedEventHandlerToken,
    wheel: RoutedEventHandlerToken,
    pressed: RoutedEventHandlerToken,
    moved: RoutedEventHandlerToken,
    released: RoutedEventHandlerToken,
    capture_lost: RoutedEventHandlerToken,
}
impl Drop for SourceHandlers {
    fn drop(&mut self) {
        if let Some(source) = self.source.upgrade() {
            source.remove_handler(ScrollViewer::scroll_changed_event(), self.changed);
            source.remove_handler(InputElement::pointer_wheel_changed_event(), self.wheel);
            source.remove_handler(InputElement::pointer_pressed_event(), self.pressed);
            source.remove_handler(InputElement::pointer_moved_event(), self.moved);
            source.remove_handler(InputElement::pointer_released_event(), self.released);
            source.remove_handler(
                InputElement::pointer_capture_lost_event(),
                self.capture_lost,
            );
        }
    }
}
#[derive(Clone, Copy)]
struct Drag {
    id: i32,
    origin: Point,
    previous: Point,
    timestamp: u64,
    velocity: f64,
    active: bool,
    from_content: bool,
}

/// Place this bar above its `ScrollSource` in an Auto,* grid. Navigation and Actions are
/// ordinary controls: commands, keyboard focus and automation remain their own.
/// The scroll source is weak. A drag consumes app-bar pre-scroll, then child scroll,
/// then leftover app-bar post-scroll; horizontal gestures remain with the child.
#[repr(C)]
pub struct MaterialAppBar {
    base: TemplatedControl,
    state: Cell<MaterialAppBarState>,
    height_offset: Cell<f64>,
    collapsed_fraction: Cell<f64>,
    overlapped_fraction: Cell<f64>,
    top_height: Cell<f64>,
    bottom_height: Cell<f64>,
    parts: RefCell<Option<Parts>>,
    source_handlers: RefCell<Option<SourceHandlers>>,
    last_source_offset: Cell<f64>,
    drag: Cell<Option<Drag>>,
    animation: RefCell<Option<HeightAnimation>>,
    color_motion: RefCell<MaterialSpringState>,
    color_brush: RefCell<Option<Ref<SolidColorBrush>>>,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    idle_timer: RefCell<Option<Rc<dyn IDisposable>>>,
    last_frame: Cell<Option<f64>>,
}
ferro_class!(MaterialAppBar: TemplatedControl);
ferro_class_info!(MaterialAppBar {
    new: MaterialAppBar::new,
    markup: { attributes: [PseudoClasses(":collapsed", ":scrolled", ":dragging")] },
});
ferro_impl_classes!(MaterialAppBar: StyledElementImpl, InteractiveImpl, LayoutableImpl, ControlImpl);

ferro_properties! { impl MaterialAppBar {
    pub fn title_property() -> StyledProperty<Option<String>> { FerroProperty::register::<Self, _>("Title", None) }
    pub fn subtitle_property() -> StyledProperty<Option<String>> { FerroProperty::register::<Self, _>("Subtitle", None) }
    pub fn navigation_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<Self, _>("Navigation", None) }
    pub fn actions_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<Self, _>("Actions", None) }
    pub fn size_variant_property() -> StyledProperty<MaterialAppBarSize> { FerroProperty::register::<Self, _>("SizeVariant", MaterialAppBarSize::Small) }
    pub fn scroll_behavior_property() -> StyledProperty<MaterialAppBarScrollBehavior> { FerroProperty::register::<Self, _>("ScrollBehavior", MaterialAppBarScrollBehavior::Pinned) }
    pub fn scroll_source_property() -> StyledProperty<Option<ElementRef<ScrollViewer>>> {
        ValueTypes::register_element_ref::<ScrollViewer>();
        FerroProperty::register::<Self, _>("ScrollSource", None)
    }
    pub fn collapsed_height_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self, _>("CollapsedHeight", StyledPropertyOptions::new(64.0_f64).validate(|v| v.is_finite() && *v >= 0.0)) }
    /// NaN selects the source-defined height for the variant and subtitle presence.
    pub fn expanded_height_property() -> StyledProperty<f64> { FerroProperty::register_with::<Self, _>("ExpandedHeight", StyledPropertyOptions::new(f64::NAN).validate(|v| v.is_nan() || (v.is_finite() && *v >= 0.0))) }
    pub fn center_title_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("CenterTitle", false) }
    pub fn safe_insets_property() -> StyledProperty<Thickness> { FerroProperty::register::<Self, _>("SafeInsets", Thickness::default()) }
    pub fn scrolled_background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> { FerroProperty::register::<Self, _>("ScrolledBackground", None) }
    pub fn can_scroll_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("CanScroll", true) }
    pub fn is_snap_enabled_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("IsSnapEnabled", true) }
    pub fn reduced_motion_property() -> StyledProperty<bool> { FerroProperty::register::<Self, _>("ReducedMotion", false) }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> { FerroProperty::register::<Self, _>("MotionScheme", MaterialMotionScheme::Expressive) }
    pub fn height_offset_property() -> DirectProperty<MaterialAppBar, f64> { FerroProperty::register_direct::<Self, _>("HeightOffset", |o| o.height_offset(), Some(|o, v| o.set_height_offset(v)), 0.0) }
    pub fn collapsed_fraction_property() -> DirectProperty<MaterialAppBar, f64> { FerroProperty::register_direct::<Self, _>("CollapsedFraction", |o| o.collapsed_fraction(), None, 0.0) }
    pub fn overlapped_fraction_property() -> DirectProperty<MaterialAppBar, f64> { FerroProperty::register_direct::<Self, _>("OverlappedFraction", |o| o.overlapped_fraction(), None, 0.0) }
} }

impl FerroObjectImpl for MaterialAppBar {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "Navigation" | "Actions" => {
                let (old, new) = change.get_old_and_new_value::<Option<BoxedValue>>();
                if let Some(old) = old.and_then(|v| Control::logical_from_boxed(&v)) {
                    StyledElement::logical_children(this).remove(&old);
                }
                if let Some(new) = new.and_then(|v| Control::logical_from_boxed(&v)) {
                    StyledElement::logical_children(this).add(new);
                }
            }
            "ScrollSource" => this.connect_source(),
            "Title" | "Subtitle" | "SizeVariant" | "ExpandedHeight" | "CollapsedHeight"
            | "SafeInsets" | "CenterTitle" => {
                this.update_metrics();
                this.configure_parts();
                this.publish_state();
            }
            "ScrollBehavior" => {
                this.cancel_animation();
                if this.scroll_behavior() == MaterialAppBarScrollBehavior::Pinned {
                    this.set_height_offset(0.0);
                }
                this.publish_state();
            }
            "Background" | "ScrolledBackground" => this.apply_color(),
            "MotionScheme" | "ReducedMotion" => {
                this.color_motion.borrow_mut().set_spec(
                    this.motion_scheme()
                        .spring(MaterialMotionRole::DefaultEffects),
                );
                this.advance(0.0);
                this.update_clock();
            }
            "CanScroll" | "IsEffectivelyEnabled" if !this.scroll_enabled() => {
                this.drag.set(None);
                this.cancel_animation();
            }
            _ => {}
        }
    }
}

impl VisualImpl for MaterialAppBar {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.connect_source();
        let weak = this.to_ref().downgrade();
        *this.visibility_subscription.borrow_mut() =
            Some(this.is_effectively_visible_changed(move || {
                if let Some(this) = weak.upgrade() {
                    if this.is_effectively_visible() {
                        this.update_clock();
                    } else {
                        this.stop_clock();
                        this.stop_idle_timer();
                        this.drag.set(None);
                    }
                }
            }));
        this.update_clock();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.source_handlers.borrow_mut().take();
        this.stop_clock();
        this.stop_idle_timer();
        this.drag.set(None);
        this.animation.borrow_mut().take();
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl TemplatedControlImpl for MaterialAppBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let Some(layout) = e
            .name_scope()
            .find_as::<MaterialAppBarLayout>("PART_Layout")
        else {
            this.parts.borrow_mut().take();
            return;
        };
        layout.set_owner(this);
        *this.parts.borrow_mut() = Some(Parts {
            layout,
            container: e.name_scope().find_as("PART_Container"),
            compact: e.name_scope().find_as("PART_CompactTitle"),
            expanded: e.name_scope().find_as("PART_ExpandedTitle"),
            compact_title: e.name_scope().find_as("PART_CompactTitleText"),
            compact_subtitle: e.name_scope().find_as("PART_CompactSubtitleText"),
            expanded_title: e.name_scope().find_as("PART_ExpandedTitleText"),
            expanded_subtitle: e.name_scope().find_as("PART_ExpandedSubtitleText"),
            navigation: e.name_scope().find_as("PART_Navigation"),
            actions: e.name_scope().find_as("PART_Actions"),
        });
        this.configure_parts();
        this.publish_state();
    }
}

impl InputElementImpl for MaterialAppBar {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if e.handled()
            || !this.scroll_enabled()
            || this.scroll_behavior() == MaterialAppBarScrollBehavior::Pinned
            || (e.pointer().type_() == PointerType::Mouse && !e.properties().is_left_button_pressed)
        {
            return;
        }
        if let Some(source) = e
            .source()
            .and_then(|source| source.cast::<ferroui_base::Visual>())
        {
            if this.parts.borrow().as_ref().is_some_and(|p| {
                [p.navigation.as_ref(), p.actions.as_ref()]
                    .iter()
                    .flatten()
                    .any(|slot| {
                        slot.is_visual_ancestor_of(&source)
                            || slot.to_ref().upcast::<ferroui_base::Visual>() == source
                    })
            }) {
                return;
            }
        }
        this.begin_pointer(e, false);
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        if this.drag.get().is_some_and(|d| !d.from_content) {
            this.move_pointer(e);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.drag.get().is_some_and(|d| !d.from_content) {
            this.release_pointer(e);
        }
    }
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        if this.drag.get().is_some_and(|d| d.id == e.pointer().id()) {
            this.drag.set(None);
            this.settle(0.0);
        }
        Self::parent_on_pointer_capture_lost(this, e);
    }
}

impl MaterialAppBar {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            state: Cell::new(MaterialAppBarState::default()),
            height_offset: Cell::new(0.0),
            collapsed_fraction: Cell::new(0.0),
            overlapped_fraction: Cell::new(0.0),
            top_height: Cell::new(64.0),
            bottom_height: Cell::new(0.0),
            parts: RefCell::new(None),
            source_handlers: RefCell::new(None),
            last_source_offset: Cell::new(0.0),
            drag: Cell::new(None),
            animation: RefCell::new(None),
            color_motion: RefCell::new(MaterialSpringState::new(
                MaterialMotionScheme::Expressive.spring(MaterialMotionRole::DefaultEffects),
                0.0,
            )),
            color_brush: RefCell::new(None),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            idle_timer: RefCell::new(None),
            last_frame: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }
    pub fn set_title(&self, value: Option<String>) {
        self.set_value(Self::title_property(), value);
    }
    pub fn subtitle(&self) -> Option<String> {
        self.get_value(Self::subtitle_property())
    }
    pub fn set_subtitle(&self, value: Option<String>) {
        self.set_value(Self::subtitle_property(), value);
    }
    pub fn navigation(&self) -> Option<BoxedValue> {
        self.get_value(Self::navigation_property())
    }
    pub fn set_navigation(&self, value: Option<BoxedValue>) {
        self.set_value(Self::navigation_property(), value);
    }
    pub fn actions(&self) -> Option<BoxedValue> {
        self.get_value(Self::actions_property())
    }
    pub fn set_actions(&self, value: Option<BoxedValue>) {
        self.set_value(Self::actions_property(), value);
    }
    pub fn size_variant(&self) -> MaterialAppBarSize {
        self.get_value(Self::size_variant_property())
    }
    pub fn set_size_variant(&self, value: MaterialAppBarSize) {
        self.set_value(Self::size_variant_property(), value);
    }
    pub fn scroll_behavior(&self) -> MaterialAppBarScrollBehavior {
        self.get_value(Self::scroll_behavior_property())
    }
    pub fn set_scroll_behavior(&self, value: MaterialAppBarScrollBehavior) {
        self.set_value(Self::scroll_behavior_property(), value);
    }
    pub fn scroll_source(&self) -> Option<Ref<ScrollViewer>> {
        ElementRef::resolve(&self.get_value(Self::scroll_source_property()))
    }
    pub fn set_scroll_source(&self, value: Option<Ref<ScrollViewer>>) {
        self.set_value(
            Self::scroll_source_property(),
            ElementRef::from_nullable(value),
        );
    }
    pub fn collapsed_height(&self) -> f64 {
        self.get_value(Self::collapsed_height_property())
    }
    pub fn set_collapsed_height(&self, value: f64) {
        self.set_value(Self::collapsed_height_property(), value);
    }
    pub fn expanded_height(&self) -> f64 {
        self.get_value(Self::expanded_height_property())
    }
    pub fn set_expanded_height(&self, value: f64) {
        self.set_value(Self::expanded_height_property(), value);
    }
    pub fn center_title(&self) -> bool {
        self.get_value(Self::center_title_property())
    }
    pub fn set_center_title(&self, value: bool) {
        self.set_value(Self::center_title_property(), value);
    }
    pub fn safe_insets(&self) -> Thickness {
        self.get_value(Self::safe_insets_property())
    }
    pub fn set_safe_insets(&self, value: Thickness) {
        self.set_value(Self::safe_insets_property(), value);
    }
    pub fn scrolled_background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::scrolled_background_property())
    }
    pub fn set_scrolled_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::scrolled_background_property(), value);
    }
    pub fn can_scroll(&self) -> bool {
        self.get_value(Self::can_scroll_property())
    }
    pub fn set_can_scroll(&self, value: bool) {
        self.set_value(Self::can_scroll_property(), value);
    }
    pub fn is_snap_enabled(&self) -> bool {
        self.get_value(Self::is_snap_enabled_property())
    }
    pub fn set_is_snap_enabled(&self, value: bool) {
        self.set_value(Self::is_snap_enabled_property(), value);
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
    pub fn state(&self) -> MaterialAppBarState {
        self.state.get()
    }
    pub fn height_offset(&self) -> f64 {
        self.state.get().height_offset()
    }
    pub fn collapsed_fraction(&self) -> f64 {
        self.state.get().collapsed_fraction()
    }
    pub fn overlapped_fraction(&self) -> f64 {
        self.state.get().overlapped_fraction()
    }
    pub fn is_animating(&self) -> bool {
        self.animation.borrow().is_some()
    }
    #[cfg(test)]
    pub(crate) fn has_clock_subscription(&self) -> bool {
        self.clock_subscription.borrow().is_some()
    }
    pub fn set_height_offset(&self, value: f64) {
        self.cancel_animation();
        self.mutate_state(|state| state.set_height_offset(value));
    }
    pub fn restore_state(&self, value: MaterialAppBarState) {
        self.cancel_animation();
        let mut value = value;
        value.set_height_offset_limit(self.state.get().height_offset_limit());
        self.state.set(value);
        self.publish_state();
    }
    pub fn expand(&self) {
        self.animate_to(0.0);
    }
    pub fn collapse(&self) {
        self.animate_to(self.state.get().height_offset_limit());
    }

    /// For a custom nested-scroll container. Units are dp; upward finger movement is negative.
    pub fn pre_scroll(&self, available_y: f64) -> f64 {
        if !self.scroll_enabled() {
            return 0.0;
        }
        self.cancel_animation();
        let mut state = self.state.get();
        let consumed = state.pre_scroll(self.scroll_behavior(), available_y);
        self.state.set(state);
        self.publish_state();
        consumed
    }
    pub fn post_scroll(&self, consumed_y: f64, available_y: f64) -> f64 {
        if !self.scroll_enabled() {
            return 0.0;
        }
        let mut state = self.state.get();
        let consumed = state.post_scroll(self.scroll_behavior(), consumed_y, available_y);
        self.state.set(state);
        self.publish_state();
        consumed
    }
    /// Settles after a user scroll/fling, using native spline decay then DefaultEffects snap.
    pub fn settle(&self, velocity_y: f64) {
        self.stop_idle_timer();
        self.drag.set(None);
        self.pseudo_classes().set(":dragging", false);
        if !self.scroll_enabled() || self.scroll_behavior() == MaterialAppBarScrollBehavior::Pinned
        {
            return;
        }
        self.mutate_state(|s| s.reset_content_after_fling(velocity_y));
        if self.state.get().snap_target().is_none() {
            self.cancel_animation();
            return;
        }
        if animations_enabled(self.reduced_motion())
            && velocity_y.is_finite()
            && velocity_y.abs() > 1.0
        {
            *self.animation.borrow_mut() = Some(HeightAnimation::Fling {
                curve: MaterialAppBarFling::new(velocity_y),
                elapsed: 0.0,
                previous: 0.0,
            });
        } else {
            self.start_snap();
        }
        self.update_clock();
    }
    pub(crate) fn effective_expanded_height(&self) -> f64 {
        let explicit = self.expanded_height();
        let height = if explicit.is_nan() {
            self.size_variant()
                .expanded_height(self.subtitle().is_some())
        } else {
            explicit
        };
        if self.size_variant().has_two_rows() {
            height.max(self.collapsed_height())
        } else {
            height
        }
    }
    pub(crate) fn is_title_centered(&self) -> bool {
        self.center_title() || self.size_variant() == MaterialAppBarSize::CenterAligned
    }
    pub(crate) fn set_layout_heights(&self, top: f64, bottom: f64) {
        let changed = self.top_height.replace(top) != top || self.bottom_height.get() != bottom;
        self.bottom_height.set(bottom);
        let limit = -if self.size_variant().has_two_rows() {
            bottom
        } else {
            top
        };
        if changed || self.state.get().height_offset_limit() != limit {
            let mut state = self.state.get();
            state.set_height_offset_limit(limit);
            self.state.set(state);
            self.publish_state();
        }
    }
    fn update_metrics(&self) {
        let expanded = self.effective_expanded_height();
        self.set_layout_heights(
            if self.size_variant().has_two_rows() {
                self.collapsed_height()
            } else {
                expanded
            },
            if self.size_variant().has_two_rows() {
                expanded - self.collapsed_height()
            } else {
                0.0
            },
        );
    }
    fn scroll_enabled(&self) -> bool {
        self.can_scroll() && self.is_effectively_enabled()
    }
    fn mutate_state(&self, action: impl FnOnce(&mut MaterialAppBarState)) {
        let mut state = self.state.get();
        action(&mut state);
        self.state.set(state);
        self.publish_state();
    }
    fn configure_parts(&self) {
        let parts = self.parts.borrow();
        let Some(parts) = parts.as_ref() else {
            return;
        };
        for text in [
            &parts.compact_title,
            &parts.compact_subtitle,
            &parts.expanded_title,
            &parts.expanded_subtitle,
        ]
        .into_iter()
        .flatten()
        {
            text.set_text_alignment(if self.is_title_centered() {
                TextAlignment::Center
            } else {
                TextAlignment::Start
            });
        }
        for subtitle in [&parts.compact_subtitle, &parts.expanded_subtitle]
            .into_iter()
            .flatten()
        {
            subtitle.set_is_visible(self.subtitle().is_some());
        }
        if let Some(title) = &parts.expanded_title {
            let (font, line, _) = self.size_variant().title_metrics();
            title.set_font_size(font);
            title.set_line_height(line);
        }
        if let Some(subtitle) = &parts.expanded_subtitle {
            let large = self.size_variant() == MaterialAppBarSize::LargeFlexible;
            subtitle.set_font_size(if large { 16.0 } else { 14.0 });
            subtitle.set_line_height(if large { 24.0 } else { 20.0 });
            subtitle.set_font_weight(FontWeight::Medium);
        }
        parts.layout.invalidate_measure();
    }
    fn publish_state(&self) {
        let state = self.state.get();
        self.set_and_raise_cell(
            Self::height_offset_property(),
            &self.height_offset,
            state.height_offset(),
        );
        self.set_and_raise_cell(
            Self::collapsed_fraction_property(),
            &self.collapsed_fraction,
            state.collapsed_fraction(),
        );
        self.set_and_raise_cell(
            Self::overlapped_fraction_property(),
            &self.overlapped_fraction,
            state.overlapped_fraction(),
        );
        let two = self.size_variant().has_two_rows();
        let fraction = state.collapsed_fraction();
        self.pseudo_classes().set(":collapsed", fraction == 1.0);
        self.pseudo_classes().set(
            ":scrolled",
            state.overlapped_fraction() > 0.01 || fraction > 0.0,
        );
        if !two {
            self.color_motion
                .borrow_mut()
                .retarget(if state.overlapped_fraction() > 0.01 {
                    1.0
                } else {
                    0.0
                });
        }
        if !animations_enabled(self.reduced_motion()) {
            self.color_motion.borrow_mut().finish();
        }
        {
            let parts = self.parts.borrow();
            if let Some(parts) = parts.as_ref() {
                if let Some(title) = &parts.compact {
                    title.set_opacity(if two {
                        cubic_bezier(fraction, 0.8, 0.0, 0.8, 0.15)
                    } else {
                        1.0
                    });
                }
                if let Some(title) = &parts.expanded {
                    title.set_opacity(1.0 - fraction);
                }
                let compact_active = !two || fraction >= 0.5;
                for title in [&parts.compact_title, &parts.compact_subtitle]
                    .into_iter()
                    .flatten()
                {
                    AutomationProperties::set_accessibility_view(
                        title,
                        if compact_active {
                            AccessibilityView::Default
                        } else {
                            AccessibilityView::Raw
                        },
                    );
                }
                for title in [&parts.expanded_title, &parts.expanded_subtitle]
                    .into_iter()
                    .flatten()
                {
                    AutomationProperties::set_accessibility_view(
                        title,
                        if two && !compact_active {
                            AccessibilityView::Default
                        } else {
                            AccessibilityView::Raw
                        },
                    );
                }
                parts.layout.invalidate_measure();
            }
        }
        self.apply_color();
        self.update_clock();
    }
    fn apply_color(&self) {
        let container = self
            .parts
            .borrow()
            .as_ref()
            .and_then(|p| p.container.clone());
        let Some(container) = container else {
            return;
        };
        let (Some(from), Some(to)) = (self.background(), self.scrolled_background()) else {
            container.set_background(self.background());
            return;
        };
        let amount = if self.size_variant().has_two_rows() {
            cubic_bezier(self.collapsed_fraction(), 0.4, 0.0, 1.0, 1.0)
        } else {
            self.color_motion.borrow().current().value.clamp(0.0, 1.0)
        };
        if let (Some(a), Some(b)) = (from.as_solid_color_brush(), to.as_solid_color_brush()) {
            let color = interpolate_container(a.color(), b.color(), amount);
            let existing = self.color_brush.borrow().clone();
            let brush = existing.unwrap_or_else(|| {
                let brush = SolidColorBrush::with_color(color);
                *self.color_brush.borrow_mut() = Some(brush.clone());
                brush
            });
            brush.set_color(color);
            container.set_background(Some(brush.into()));
        } else {
            container.set_background(Some(if amount < 0.5 { from } else { to }));
        }
    }
    fn animate_to(&self, target: f64) {
        let target = target.clamp(self.state.get().height_offset_limit(), 0.0);
        let mut spring = MaterialSpringState::new(
            self.motion_scheme()
                .spring(MaterialMotionRole::DefaultEffects),
            self.height_offset(),
        );
        spring.retarget(target);
        *self.animation.borrow_mut() = Some(HeightAnimation::Snap(spring));
        self.advance(0.0);
        self.update_clock();
    }
    fn start_snap(&self) {
        if self.is_snap_enabled() {
            if let Some(target) = self.state.get().snap_target() {
                self.animate_to(target);
                return;
            }
        }
        self.animation.borrow_mut().take();
    }
    fn advance(&self, seconds: f64) {
        let reduced = !animations_enabled(self.reduced_motion());
        {
            let mut color = self.color_motion.borrow_mut();
            color.set_reduced_motion(reduced);
            color.advance(seconds);
            if color.is_at_rest(0.01, 0.625) {
                color.finish();
            }
        }
        let animation = self.animation.borrow_mut().take();
        let mut state = self.state.get();
        let mut snap_after = false;
        match animation {
            Some(HeightAnimation::Snap(mut spring)) => {
                spring.set_reduced_motion(reduced);
                spring.advance(seconds);
                if spring.is_at_rest(0.01, 0.625) {
                    spring.finish();
                }
                state.set_height_offset(spring.current().value);
                if !spring.is_at_rest(0.01, 0.625) {
                    *self.animation.borrow_mut() = Some(HeightAnimation::Snap(spring));
                }
            }
            Some(HeightAnimation::Fling {
                curve,
                elapsed,
                previous,
            }) => {
                if reduced {
                    snap_after = true;
                } else {
                    let elapsed = elapsed + seconds.max(0.0);
                    let (position, _) = curve.sample(elapsed);
                    let delta = position - previous;
                    let old = state.height_offset();
                    state.set_height_offset(old + delta);
                    let consumed = (old - state.height_offset()).abs();
                    // Keep the pinned settleAppBar comparison, including signed delta.
                    if elapsed >= curve.duration_seconds() || (delta - consumed).abs() > 0.5 {
                        snap_after = true;
                    } else {
                        *self.animation.borrow_mut() = Some(HeightAnimation::Fling {
                            curve,
                            elapsed,
                            previous: position,
                        });
                    }
                }
            }
            Some(HeightAnimation::ContentFling {
                curve,
                elapsed,
                previous,
            }) => {
                if reduced {
                    snap_after = true;
                } else {
                    let elapsed = elapsed + seconds.max(0.0);
                    let (position, velocity) = curve.sample(elapsed);
                    let delta = position - previous;
                    let consumed = self.scroll_content_by_core(delta);
                    state = self.state.get();
                    if elapsed >= curve.duration_seconds() || (delta - consumed).abs() > 0.5 {
                        state.reset_content_after_fling(velocity);
                        snap_after = true;
                    } else {
                        *self.animation.borrow_mut() = Some(HeightAnimation::ContentFling {
                            curve,
                            elapsed,
                            previous: position,
                        });
                    }
                }
            }
            None => {}
        }
        self.state.set(state);
        if snap_after {
            self.start_snap();
        }
        self.publish_state();
    }
    fn cancel_animation(&self) {
        self.animation.borrow_mut().take();
        self.stop_idle_timer();
        if self.color_motion.borrow().is_at_rest(0.01, 0.625) {
            self.stop_clock();
        }
    }
    fn motion_pending(&self) -> bool {
        self.animation.borrow().is_some() || !self.color_motion.borrow().is_at_rest(0.01, 0.625)
    }
    fn update_clock(&self) {
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
            return;
        };
        self.last_frame.set(None);
        let weak = self.to_ref().downgrade();
        let subscription =
            clock.subscribe(Rc::new(AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    if !this.is_attached_to_visual_tree() || !this.is_effectively_visible() {
                        this.stop_clock();
                        return;
                    }
                    let now = time.total_seconds();
                    if !now.is_finite() {
                        return;
                    }
                    let delta = this
                        .last_frame
                        .replace(Some(now))
                        .map_or(0.0, |last| (now - last).max(0.0));
                    this.advance(delta);
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
    fn stop_idle_timer(&self) {
        if let Some(timer) = self.idle_timer.borrow_mut().take() {
            timer.dispose();
        }
    }
    fn schedule_idle_settle(&self) {
        self.stop_idle_timer();
        if !self.is_attached_to_visual_tree() || !self.is_effectively_visible() {
            return;
        }
        let weak = self.to_ref().downgrade();
        *self.idle_timer.borrow_mut() = Some(DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.idle_timer.borrow_mut().take();
                    this.settle(0.0);
                }
            },
            Duration::from_millis(150),
            DispatcherPriority::BACKGROUND,
        ));
    }

    fn connect_source(&self) {
        self.source_handlers.borrow_mut().take();
        if !self.is_attached_to_visual_tree() {
            return;
        }
        let Some(source) = self.scroll_source() else {
            return;
        };
        self.last_source_offset.set(source.offset().y);
        self.mutate_state(|state| {
            state.set_content_offset(-source.offset().y);
            state.set_content_at_start(source.offset().y <= 0.0);
        });
        let weak = self.to_ref().downgrade();
        let changed = source.scroll_changed(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.source_scrolled();
            }
        });
        let weak = self.to_ref().downgrade();
        let wheel = source.add_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, e| {
                if let Some(this) = weak.upgrade() {
                    if !this.scroll_enabled()
                        || e.key_modifiers().contains(KeyModifiers::SHIFT)
                        || e.delta().y == 0.0
                    {
                        return;
                    }
                    let delta = e.delta().y * 50.0;
                    let mut consumed = this.pre_scroll(delta);
                    if consumed == 0.0
                        && delta > 0.0
                        && this.scroll_source().is_some_and(|s| s.offset().y <= 0.0)
                    {
                        consumed = this.post_scroll(0.0, delta);
                    }
                    if consumed != 0.0 {
                        e.set_handled(true);
                    }
                    this.schedule_idle_settle();
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = self.to_ref().downgrade();
        let pressed = source.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| {
                if let Some(this) = weak.upgrade() {
                    if e.pointer().type_() != PointerType::Mouse
                        && this.scroll_enabled()
                        && this.scroll_behavior() != MaterialAppBarScrollBehavior::Pinned
                    {
                        this.begin_pointer(e, true);
                    }
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = self.to_ref().downgrade();
        let moved = source.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.move_pointer(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = self.to_ref().downgrade();
        let released = source.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.release_pointer(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = self.to_ref().downgrade();
        let capture_lost =
            source.add_handler(InputElement::pointer_capture_lost_event(), move |_, e| {
                if let Some(this) = weak.upgrade() {
                    if this.drag.get().is_some_and(|d| d.id == e.pointer().id()) {
                        this.drag.set(None);
                        this.settle(0.0);
                    }
                }
            });
        *self.source_handlers.borrow_mut() = Some(SourceHandlers {
            source: source.downgrade(),
            changed,
            wheel,
            pressed,
            moved,
            released,
            capture_lost,
        });
    }
    fn source_scrolled(&self) {
        let Some(source) = self.scroll_source() else {
            return;
        };
        let current = source.offset().y;
        let delta = current - self.last_source_offset.replace(current);
        if !self.scroll_enabled() || delta == 0.0 {
            return;
        }
        self.cancel_animation();
        self.mutate_state(|state| {
            state.set_content_at_start(current <= 0.0);
            state.set_content_offset(-current);
            if self.scroll_behavior() == MaterialAppBarScrollBehavior::EnterAlways
                || (self.scroll_behavior() == MaterialAppBarScrollBehavior::ExitUntilCollapsed
                    && delta > 0.0)
            {
                state.set_height_offset(state.height_offset() - delta);
            }
        });
        self.schedule_idle_settle();
    }
    fn begin_pointer(&self, e: &PointerPressedEventArgs, from_content: bool) {
        self.cancel_animation();
        let point = e.get_position(None);
        self.drag.set(Some(Drag {
            id: e.pointer().id(),
            origin: point,
            previous: point,
            timestamp: e.timestamp(),
            velocity: 0.0,
            active: false,
            from_content,
        }));
    }
    fn move_pointer(&self, e: &PointerEventArgs) {
        let Some(mut drag) = self.drag.get().filter(|drag| drag.id == e.pointer().id()) else {
            return;
        };
        let point = e.get_position(None);
        let mut first_delta = false;
        if !drag.active {
            let x = (point.x - drag.origin.x).abs();
            let y = (point.y - drag.origin.y).abs();
            if x.max(y) <= 8.0 {
                return;
            }
            if x > y {
                self.drag.set(None);
                return;
            }
            drag.active = true;
            first_delta = true;
            e.prevent_gesture_recognition();
            let target: Ref<InputElement> = if drag.from_content {
                match self.scroll_source() {
                    Some(source) => source.upcast(),
                    None => return,
                }
            } else {
                self.to_ref().upcast()
            };
            e.pointer().capture(Some(&target));
            self.pseudo_classes().set(":dragging", true);
        }
        let movement = point.y - drag.previous.y;
        // Foundation draggable discards touch slop and delivers only the overslop delta.
        let delta = if first_delta {
            movement - 8.0 * movement.signum()
        } else {
            movement
        };
        let seconds = e.timestamp().saturating_sub(drag.timestamp) as f64 / 1000.0;
        if seconds > 0.0 {
            drag.velocity = (movement / seconds).clamp(-8000.0, 8000.0);
        }
        drag.previous = point;
        drag.timestamp = e.timestamp();
        self.drag.set(Some(drag));
        if drag.from_content {
            self.scroll_content_by(delta);
        } else {
            self.mutate_state(|state| state.set_height_offset(state.height_offset() + delta));
        }
        e.set_handled(true);
    }
    /// Runs the native pre/child/post ordering against the configured physical ScrollViewer.
    pub fn scroll_content_by(&self, delta_y: f64) -> f64 {
        self.cancel_animation();
        self.scroll_content_by_core(delta_y)
    }
    fn scroll_content_by_core(&self, delta_y: f64) -> f64 {
        let Some(source) = self.scroll_source() else {
            return 0.0;
        };
        if !self.scroll_enabled() || !delta_y.is_finite() {
            return 0.0;
        }
        let mut state = self.state.get();
        let pre = state.pre_scroll(self.scroll_behavior(), delta_y);
        let remaining = delta_y - pre;
        let old = source.offset();
        source.set_offset(Vector::new(old.x, old.y - remaining));
        let current = source.offset();
        let consumed = old.y - current.y;
        self.last_source_offset.set(current.y);
        state.set_content_at_start(current.y <= 0.0);
        let post = state.post_scroll(self.scroll_behavior(), consumed, remaining - consumed);
        self.state.set(state);
        self.publish_state();
        pre + consumed + post
    }
    fn release_pointer(&self, e: &PointerReleasedEventArgs) {
        let Some(drag) = self.drag.get().filter(|d| d.id == e.pointer().id()) else {
            return;
        };
        self.drag.set(None);
        if drag.active {
            let velocity = if e.timestamp().saturating_sub(drag.timestamp) > 40 {
                0.0
            } else {
                drag.velocity
            };
            e.pointer().capture(None);
            self.pseudo_classes().set(":dragging", false);
            if drag.from_content
                && animations_enabled(self.reduced_motion())
                && velocity.abs() > 1.0
            {
                *self.animation.borrow_mut() = Some(HeightAnimation::ContentFling {
                    curve: MaterialAppBarFling::new(velocity),
                    elapsed: 0.0,
                    previous: 0.0,
                });
                self.update_clock();
            } else {
                self.settle(velocity);
            }
            e.set_handled(true);
        }
    }
}

impl Drop for MaterialAppBar {
    fn drop(&mut self) {
        self.source_handlers.get_mut().take();
        if let Some(subscription) = self.clock_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(timer) = self.idle_timer.get_mut().take() {
            timer.dispose();
        }
    }
}
