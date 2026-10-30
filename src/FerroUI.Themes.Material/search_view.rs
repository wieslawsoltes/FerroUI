// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! Docked Material search with one persistent FerroUI text editor.
//! Reference: AndroidX Material3 SearchBar.kt at e4d933c98df7baa7ea2b83eea6e086915961e0ff.

use crate::motion::animations_enabled;
use crate::{MaterialMotionRole, MaterialMotionScheme, MaterialSpringState};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
};
use ferroui_base::interactivity::{
    InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::MediaContext;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, ferro_routed_event, instantiate,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::{
    Button, ContentControl, ContentControlImpl, ControlImpl, ScrollViewer, TextBox, TopLevel,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A 56dp search field with an animated docked results surface.
/// `Content` supplies the application's results; `Query` is two-way bindable.
/// Query submission is a routed event, so filtering can remain in the view model.
/// The same TextBox survives expansion, preserving composition, undo and selection.
#[repr(C)]
pub struct MaterialSearchView {
    base: ContentControl,
    editor: RefCell<Option<Ref<TextBox>>>,
    results: RefCell<Option<Ref<ScrollViewer>>>,
    result_content: RefCell<Option<Ref<ContentPresenter>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    progress: RefCell<MaterialSpringState>,
    result_height: Cell<f64>,
    maximum_height: Cell<f64>,
    parent_height: Cell<f64>,
    synchronizing: Cell<bool>,
    suppress_expand: Cell<bool>,
    frame_pending: Cell<bool>,
    frame_generation: Cell<u64>,
    last_frame: Cell<Option<TimeSpan>>,
}

ferro_class!(MaterialSearchView: ContentControl);
ferro_class_info!(MaterialSearchView {
    new: MaterialSearchView::new,
    markup: { attributes: [PseudoClasses(":expanded", ":query")] },
});
ferro_impl_classes!(MaterialSearchView: StyledElementImpl, InteractiveImpl, ControlImpl, ContentControlImpl);

ferro_properties! { impl MaterialSearchView {
    pub fn query_property() -> StyledProperty<Option<String>> {
        FerroProperty::register_with::<MaterialSearchView, _>("Query",
            StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay))
    }
    pub fn placeholder_text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<MaterialSearchView, _>("PlaceholderText", Some("Search".to_string()))
    }
    pub fn is_expanded_property() -> StyledProperty<bool> {
        FerroProperty::register_with::<MaterialSearchView, _>("IsExpanded",
            StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay))
    }
    pub fn expand_on_focus_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialSearchView, _>("ExpandOnFocus", true)
    }
    /// Optional tighter limit; the native maximum is two thirds of the window height.
    pub fn max_expanded_height_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<MaterialSearchView, _>("MaxExpandedHeight",
            StyledPropertyOptions::new(f64::INFINITY).validate(|v| !v.is_nan() && *v >= 56.0))
    }
    pub fn reduced_motion_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialSearchView, _>("ReducedMotion", false)
    }
    pub fn motion_scheme_property() -> StyledProperty<MaterialMotionScheme> {
        FerroProperty::register::<MaterialSearchView, _>("MotionScheme", MaterialMotionScheme::Expressive)
    }
} }

impl FerroObjectImpl for MaterialSearchView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        match change.property().name() {
            "Query" => {
                this.pseudo_classes()
                    .set(":query", this.query().is_some_and(|text| !text.is_empty()));
                if !this.synchronizing.get() {
                    if let Some(editor) = this.editor.borrow().clone() {
                        this.synchronizing.set(true);
                        editor.set_current_value(TextBox::text_property(), this.query());
                        this.synchronizing.set(false);
                    }
                }
            }
            "IsExpanded" | "ReducedMotion" | "MotionScheme" => this.retarget(),
            "MaxExpandedHeight" | "Content" => this.invalidate_measure(),
            _ => {}
        }
    }
}

impl VisualImpl for MaterialSearchView {
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
                    }
                }
            }));
        this.retarget();
    }
    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        this.invalidate_frame();
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        this.progress.borrow_mut().finish();
        Self::parent_on_detached_from_visual_tree(this, event);
    }
}

impl LayoutableImpl for MaterialSearchView {
    fn measure_override(this: &Self, available: Size) -> Size {
        let first = Self::parent_measure_override(this, available);
        // The top level has no arranged bounds during its first measure. Use
        // the actual constraint then, so a search first measured while closed
        // does not retain a zero-height results area when it opens.
        let viewport = TopLevel::get_top_level(Some(this))
            .map(|top| top.bounds().height)
            .filter(|height| height.is_finite() && *height > 56.0)
            .unwrap_or_else(|| {
                if available.height.is_finite() && available.height > 56.0 {
                    available.height
                } else {
                    720.0
                }
            });
        let maximum = (viewport * 2.0 / 3.0)
            .min(this.max_expanded_height())
            .max(56.0);
        let natural = this
            .result_content
            .borrow()
            .as_ref()
            .map_or(0.0, |presenter| {
                presenter.measure(Size::new(
                    available.width.min(720.0).max(0.0),
                    f64::INFINITY,
                ));
                presenter.desired_size().height
            });
        let old_natural = this.result_height.replace(natural + 1.0); // Native divider.
        let old_maximum = this.maximum_height.replace(maximum);
        let old_parent = this.parent_height.replace(available.height.max(56.0));
        if (old_natural - natural - 1.0).abs() > 0.001
            || old_maximum != maximum
            || old_parent != this.parent_height.get()
        {
            this.apply_progress();
            Self::parent_measure_override(this, available)
        } else {
            first
        }
    }
}

impl TemplatedControlImpl for MaterialSearchView {
    fn on_apply_template(this: &Self, event: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, event);
        for subscription in this.subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        let editor = event.name_scope().find_as::<TextBox>("PART_QueryBox");
        *this.editor.borrow_mut() = editor.clone();
        *this.results.borrow_mut() = event.name_scope().find_as::<ScrollViewer>("PART_Results");
        *this.result_content.borrow_mut() = event
            .name_scope()
            .find_as::<ContentPresenter>("PART_ContentPresenter");
        if let Some(editor) = editor {
            editor.set_current_value(TextBox::text_property(), this.query());
            let weak = this.to_ref().downgrade();
            let editor_weak = editor.downgrade();
            this.subscriptions
                .borrow_mut()
                .push(editor.property_changed(move |change| {
                    if change.property() != TextBox::text_property().as_property() {
                        return;
                    }
                    if let (Some(this), Some(editor)) = (weak.upgrade(), editor_weak.upgrade()) {
                        if !this.synchronizing.get() {
                            this.synchronizing.set(true);
                            this.set_current_value(Self::query_property(), editor.text());
                            this.synchronizing.set(false);
                        }
                    }
                }));
        }
        for (part, action) in [("PART_BackButton", false), ("PART_ClearButton", true)] {
            if let Some(button) = event.name_scope().find_as::<Button>(part) {
                let weak = this.to_ref().downgrade();
                this.subscriptions
                    .borrow_mut()
                    .push(button.add_disposable_handler_with(
                        Button::click_event(),
                        move |_, args| {
                            if let Some(this) = weak.upgrade() {
                                if action {
                                    this.clear_query();
                                } else {
                                    this.collapse();
                                }
                                args.set_handled(true);
                            }
                        },
                        RoutingStrategies::BUBBLE,
                        false,
                    ));
            }
        }
        this.apply_progress();
    }
}

impl InputElementImpl for MaterialSearchView {
    fn on_got_focus(this: &Self, event: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, event);
        if this.expand_on_focus()
            && !this.suppress_expand.get()
            && event
                .source()
                .is_some_and(|source| source.cast::<TextBox>().is_some())
        {
            this.expand();
        }
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        Self::parent_on_key_down(this, event);
        if event.handled() {
            return;
        }
        match event.key {
            Key::Escape if this.is_expanded() => {
                this.collapse();
                event.set_handled(true);
            }
            Key::Enter
                if this
                    .editor
                    .borrow()
                    .as_ref()
                    .is_some_and(|editor| editor.is_keyboard_focus_within()) =>
            {
                this.submit_query();
                event.set_handled(true);
            }
            Key::Down
                if this.is_expanded()
                    && this
                        .editor
                        .borrow()
                        .as_ref()
                        .is_some_and(|editor| editor.is_keyboard_focus_within()) =>
            {
                if let Some(results) = this.results.borrow().as_ref() {
                    if let Some(first) = results
                        .get_visual_descendants()
                        .filter_map(|visual| visual.cast::<InputElement>())
                        .find(|input| {
                            input.focusable()
                                && input.is_effectively_enabled()
                                && input.is_effectively_visible()
                        })
                    {
                        first.focus();
                        event.set_handled(true);
                    }
                }
            }
            _ => {}
        }
    }
}

impl MaterialSearchView {
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            editor: RefCell::new(None),
            results: RefCell::new(None),
            result_content: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            visibility_subscription: RefCell::new(None),
            progress: RefCell::new(MaterialSpringState::new(
                MaterialMotionScheme::Expressive.spring(MaterialMotionRole::SlowSpatial),
                0.0,
            )),
            result_height: Cell::new(0.0),
            maximum_height: Cell::new(480.0),
            parent_height: Cell::new(f64::INFINITY),
            synchronizing: Cell::new(false),
            suppress_expand: Cell::new(false),
            frame_pending: Cell::new(false),
            frame_generation: Cell::new(0),
            last_frame: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn query(&self) -> Option<String> {
        self.get_value(Self::query_property())
    }
    pub fn set_query(&self, query: Option<String>) {
        self.set_value(Self::query_property(), query);
    }
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }
    pub fn set_placeholder_text(&self, value: Option<String>) {
        self.set_value(Self::placeholder_text_property(), value);
    }
    pub fn is_expanded(&self) -> bool {
        self.get_value(Self::is_expanded_property())
    }
    pub fn set_is_expanded(&self, value: bool) {
        self.set_value(Self::is_expanded_property(), value);
    }
    pub fn expand_on_focus(&self) -> bool {
        self.get_value(Self::expand_on_focus_property())
    }
    pub fn set_expand_on_focus(&self, value: bool) {
        self.set_value(Self::expand_on_focus_property(), value);
    }
    pub fn max_expanded_height(&self) -> f64 {
        self.get_value(Self::max_expanded_height_property())
    }
    pub fn set_max_expanded_height(&self, value: f64) {
        self.set_value(Self::max_expanded_height_property(), value);
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
    pub fn editor(&self) -> Option<Ref<TextBox>> {
        self.editor.borrow().clone()
    }
    pub fn expand(&self) {
        self.set_current_value(Self::is_expanded_property(), true);
    }
    pub fn collapse(&self) {
        self.set_current_value(Self::is_expanded_property(), false);
        self.suppress_expand.set(true);
        if let Some(editor) = self.editor.borrow().as_ref() {
            editor.focus();
        }
        self.suppress_expand.set(false);
    }
    pub fn clear_query(&self) {
        self.set_current_value(Self::query_property(), None);
        if let Some(editor) = self.editor.borrow().as_ref() {
            editor.focus();
        }
    }
    pub fn submit_query(&self) {
        self.raise_event(&RoutedEventArgs::with_event_and_source(
            Self::query_submitted_event(),
            self.to_ref(),
        ));
    }
    ferro_routed_event!(
        pub fn query_submitted_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MaterialSearchView, _>(
                "QuerySubmitted",
                RoutingStrategies::BUBBLE,
            )
        }
    );
    pub fn query_submitted(
        &self,
        handler: impl Fn(&ferroui_base::interactivity::Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::query_submitted_event(), handler)
    }

    fn retarget(&self) {
        let expanded = self.is_expanded();
        self.pseudo_classes().set(":expanded", expanded);
        let mut progress = self.progress.borrow_mut();
        progress.set_spec(self.motion_scheme().spring(if expanded {
            MaterialMotionRole::SlowSpatial
        } else {
            MaterialMotionRole::DefaultSpatial
        }));
        progress.set_reduced_motion(!animations_enabled(self.reduced_motion()));
        progress.retarget(if expanded { 1.0 } else { 0.0 });
        if !self.is_attached_to_visual_tree() {
            progress.finish();
        }
        drop(progress);
        self.apply_progress();
        self.request_frame();
    }
    fn apply_progress(&self) {
        let progress = self.progress.borrow().current().value.max(0.0);
        // DockedSearchBarLayoutImpl animates the maximum constraint toward the
        // screen cap. It does not multiply the final measured height by progress:
        // short result sets reach their native 240dp minimum before the spring
        // finishes, and spatial overshoot is retained for a full result list.
        let maximum = (56.0 + (self.maximum_height.get() - 56.0) * progress)
            .min(self.parent_height.get())
            .max(56.0);
        let height = (56.0 + self.result_height.get()).clamp(240.0_f64.min(maximum), maximum);
        if let Some(results) = self.results.borrow().as_ref() {
            results.set_height((height - 56.0).max(0.0));
            // The gapless native docked surface reveals opaque content by clipping.
            results.set_opacity(1.0);
            results.set_is_hit_test_visible(self.is_expanded());
            results.set_is_visible(progress > 0.0 || self.is_expanded());
        }
    }
    pub(crate) fn advance_motion(&self, seconds: f64) {
        {
            let mut progress = self.progress.borrow_mut();
            progress.set_reduced_motion(!animations_enabled(self.reduced_motion()));
            progress.advance(seconds);
            if progress.is_at_rest(0.001, 0.01) {
                progress.finish();
                self.last_frame.set(None);
            }
        }
        self.apply_progress();
    }
    fn invalidate_frame(&self) {
        self.frame_generation
            .set(self.frame_generation.get().wrapping_add(1));
        self.frame_pending.set(false);
        self.last_frame.set(None);
    }
    fn request_frame(&self) {
        if self.frame_pending.get()
            || !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
            || self.progress.borrow().is_at_rest(0.001, 0.01)
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
            this.advance_motion(elapsed);
            this.request_frame();
        });
    }
}
