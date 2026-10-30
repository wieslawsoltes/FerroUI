// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::deferred_content_control::{changed_schedule, same_content, same_template};
use crate::{presentation, reveal, scheduling, target::TargetState, DeferredContentControl};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, Size, StyledElementImpl, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{presenters::ContentPresenter, templates::IDataTemplate, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
/// A native ContentPresenter batching materialization on a presentation timeline.
#[repr(C)]
pub struct DeferredContentPresenter {
    base: ContentPresenter,
    suppress: Cell<bool>,
    requested_content: RefCell<Option<BoxedValue>>,
    requested_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    applied_content: RefCell<Option<BoxedValue>>,
    applied_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    requested_version: Cell<i64>,
    applied_version: Cell<i64>,
    last_desired_size: Cell<Size>,
    pub(crate) target: TargetState,
}
ferro_class!(DeferredContentPresenter:ContentPresenter);
ferro_class_info!(DeferredContentPresenter {
    new: DeferredContentPresenter::new
});
ferro_impl_classes!(DeferredContentPresenter:StyledElementImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl FerroObjectImpl for DeferredContentPresenter {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == ContentPresenter::content_property().as_property()
            || change.property() == ContentPresenter::content_template_property().as_property()
        {
            if this.suppress.get() {
                return;
            }
            if change.property() == ContentPresenter::content_property().as_property() {
                this.requested_content.replace(change.get_new_value());
            } else {
                this.requested_template.replace(change.get_new_value());
            }
            this.requested_version
                .set(this.requested_version.get().wrapping_add(1));
            this.restore_applied_state();
            this.queue_deferred_presentation();
            return;
        }
        Self::parent_on_property_changed(this, change);
        if changed_schedule(change) {
            this.queue_deferred_presentation();
        }
    }
}
impl VisualImpl for DeferredContentPresenter {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.queue_deferred_presentation();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.remove_queued_presentation();
    }
}
impl LayoutableImpl for DeferredContentPresenter {
    fn measure_override(this: &Self, available: Size) -> Size {
        if this.should_defer_measure() {
            return this.last_desired_size.get();
        }
        let desired = Self::parent_measure_override(this, available);
        this.last_desired_size.set(desired);
        desired
    }
}
struct Suppression<'a>(&'a Cell<bool>);
impl Drop for Suppression<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl DeferredContentPresenter {
    /// Constructs this or a derived native presenter with source defaults.
    pub fn construct() -> Self {
        Self {
            base: ContentPresenter::construct(),
            suppress: Cell::new(false),
            requested_content: RefCell::new(None),
            requested_template: RefCell::new(None),
            applied_content: RefCell::new(None),
            applied_template: RefCell::new(None),
            requested_version: Cell::new(0),
            applied_version: Cell::new(-1),
            last_desired_size: Cell::new(Size::default()),
            target: TargetState::new(),
        }
    }
    /// Creates a native deferred presenter.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn apply_deferred_state(
        &self,
        content: Option<BoxedValue>,
        template: Option<Rc<dyn IDataTemplate>>,
        duration: TimeSpan,
    ) {
        let had_child = self.child().is_some();
        let has_content = content.is_some();
        self.suppress.set(true);
        {
            let _reset = Suppression(&self.suppress);
            self.set_current_value(ContentPresenter::content_template_property(), template);
            self.set_current_value(ContentPresenter::content_property(), content);
        }
        self.update_presented_child();
        reveal::apply(
            self,
            duration,
            had_child && has_content && self.child().is_some(),
        );
    }
    pub(crate) fn apply_deferred_presentation(&self, animate: bool) -> bool {
        if !self.ready() {
            return false;
        }
        let content = self.requested_content.borrow().clone();
        let template = self.requested_template.borrow().clone();
        if self.applied_version.get() == self.requested_version.get()
            && same_content(&self.applied_content.borrow(), &content)
            && same_template(&self.applied_template.borrow(), &template)
        {
            return true;
        }
        let duration = if animate {
            self.target
                .timeline
                .borrow()
                .upgrade()
                .unwrap_or_else(|| scheduling::resolve_timeline(self))
                .reveal_duration()
        } else {
            TimeSpan::ZERO
        };
        self.apply_deferred_state(content.clone(), template.clone(), duration);
        self.applied_content.replace(content);
        self.applied_template.replace(template);
        self.applied_version.set(self.requested_version.get());
        true
    }
    pub(crate) fn remove_queued_presentation(&self) {
        self.target.remove(self);
    }
    fn hosted(&self) -> bool {
        self.templated_parent()
            .is_some_and(|p| p.cast::<DeferredContentControl>().is_some())
    }
    fn queue_deferred_presentation(&self) {
        if self.hosted() || !self.ready() {
            self.remove_queued_presentation();
            return;
        }
        if !presentation::should_defer(self.requested_content.borrow().as_ref()) {
            self.remove_queued_presentation();
            self.apply_deferred_presentation(false);
            return;
        }
        let timeline = scheduling::resolve_timeline(self);
        let current = self.target.timeline.borrow().upgrade();
        if !current
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &timeline))
        {
            self.remove_queued_presentation();
        }
        timeline.enqueue(
            self.target.project(self),
            scheduling::resolve_delay(self),
            scheduling::resolve_order(self),
        );
    }
    fn ready(&self) -> bool {
        self.visual_root().is_some() && self.is_attached_to_logical_tree()
    }
    fn should_defer_measure(&self) -> bool {
        !self.suppress.get()
            && !self.hosted()
            && self.ready()
            && presentation::should_defer(self.content().as_ref())
            && self.applied_version.get() != self.requested_version.get()
    }
    fn restore_applied_state(&self) {
        self.suppress.set(true);
        {
            let _reset = Suppression(&self.suppress);
            let content = self.applied_content.borrow().clone();
            let template = self.applied_template.borrow().clone();
            self.set_current_value(ContentPresenter::content_template_property(), template);
            self.set_current_value(ContentPresenter::content_property(), content);
        }
        self.update_presented_child();
    }
    fn update_presented_child(&self) {
        self.update_child();
        self.pseudo_classes()
            .set(":empty", self.content().is_none());
        self.invalidate_measure();
    }
}
