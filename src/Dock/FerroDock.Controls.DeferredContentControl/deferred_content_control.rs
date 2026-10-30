// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{presentation, reveal, scheduling, target::TargetState};
use crate::{DeferredContentPresenter, DeferredContentScheduling};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::templates::{FuncControlTemplate, IDataTemplate};
use ferroui_controls::{boxed_reference_equals, ContentControl, ContentControlImpl, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
/// A native ContentControl that batches materialization on a presentation timeline.
#[repr(C)]
pub struct DeferredContentControl {
    base: ContentControl,
    presenter: RefCell<Option<Ref<ContentPresenter>>>,
    applied_content: RefCell<Option<BoxedValue>>,
    applied_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    requested_version: Cell<i64>,
    applied_version: Cell<i64>,
    pub(crate) target: TargetState,
}
ferro_class!(DeferredContentControl:ContentControl);
ferro_class_info!(DeferredContentControl{new:DeferredContentControl::new,markup:{attributes:[TemplatePart("PART_ContentPresenter",type(Ref<ContentPresenter>))]}});
ferro_impl_classes!(DeferredContentControl:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ContentControlImpl);
impl FerroObjectImpl for DeferredContentControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == ContentControl::content_property().as_property()
            || change.property() == ContentControl::content_template_property().as_property()
        {
            this.requested_version
                .set(this.requested_version.get().wrapping_add(1));
            this.queue_deferred_presentation();
            return;
        }
        if changed_schedule(change) {
            this.queue_deferred_presentation();
        }
    }
}
impl VisualImpl for DeferredContentControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.queue_deferred_presentation();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.remove_queued_presentation();
    }
}
impl TemplatedControlImpl for DeferredContentControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let old = this.presenter.borrow().clone();
        if let Some(old) = old.and_then(|p| p.cast::<DeferredContentPresenter>()) {
            old.remove_queued_presentation();
        }
        let presenter = e
            .name_scope()
            .find_as::<ContentPresenter>("PART_ContentPresenter");
        *this.presenter.borrow_mut() = presenter.clone();
        if let Some(presenter) = presenter.and_then(|p| p.cast::<DeferredContentPresenter>()) {
            presenter.remove_queued_presentation();
        }
        this.applied_version.set(-1);
        this.queue_deferred_presentation();
    }
}
impl DeferredContentControl {
    fn static_constructor() {
        TemplatedControl::template_property().override_default_value::<DeferredContentControl>(
            Some(FuncControlTemplate::for_type::<DeferredContentControl>(
                |_, scope| {
                    let presenter = DeferredContentPresenter::new();
                    presenter.set_name(Some("PART_ContentPresenter".to_owned()));
                    macro_rules! bind {
                        ($target:ident,$owner:ident,$source:ident) => {
                            presenter.bind_binding(
                                ContentPresenter::$target().as_property(),
                                &TemplateBinding::new($owner::$source().as_property()),
                            );
                        };
                    }
                    bind!(background_property, TemplatedControl, background_property);
                    bind!(
                        background_sizing_property,
                        TemplatedControl,
                        background_sizing_property
                    );
                    bind!(
                        border_brush_property,
                        TemplatedControl,
                        border_brush_property
                    );
                    bind!(
                        border_thickness_property,
                        TemplatedControl,
                        border_thickness_property
                    );
                    bind!(
                        corner_radius_property,
                        TemplatedControl,
                        corner_radius_property
                    );
                    bind!(padding_property, TemplatedControl, padding_property);
                    bind!(
                        vertical_content_alignment_property,
                        ContentControl,
                        vertical_content_alignment_property
                    );
                    bind!(
                        horizontal_content_alignment_property,
                        ContentControl,
                        horizontal_content_alignment_property
                    );
                    scope.register("PART_ContentPresenter", presenter.clone().upcast());
                    presenter.upcast()
                },
            )),
        );
    }
    /// Constructs this or a derived native deferred control with source defaults.
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            presenter: RefCell::new(None),
            applied_content: RefCell::new(None),
            applied_template: RefCell::new(None),
            requested_version: Cell::new(0),
            applied_version: Cell::new(-1),
            target: TargetState::new(),
        }
    }
    /// Creates a native deferred content control.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Releases materialized content and cancels queued work while preserving
    /// Content and ContentTemplate. Property change or reattachment realizes it again.
    pub fn clear_presentation(&self) {
        self.remove_queued_presentation();
        let presenter = self.presenter.borrow().clone();
        if let Some(presenter) = presenter {
            apply_state(&presenter, None, None, TimeSpan::ZERO);
        }
        self.applied_content.replace(None);
        self.applied_template.replace(None);
        self.applied_version.set(-1);
    }
    fn ready(&self) -> bool {
        self.presenter.borrow().is_some()
            && self.visual_root().is_some()
            && self.is_attached_to_logical_tree()
    }
    pub(crate) fn apply_deferred_presentation(&self, animate: bool) -> bool {
        if !self.ready() {
            return false;
        }
        let content = self.content();
        let template = self.content_template();
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
        let presenter = self.presenter.borrow().clone().expect("ready presenter");
        apply_state(&presenter, content.clone(), template.clone(), duration);
        self.applied_content.replace(content);
        self.applied_template.replace(template);
        self.applied_version.set(self.requested_version.get());
        true
    }
    fn queue_deferred_presentation(&self) {
        if !self.ready() {
            self.remove_queued_presentation();
            return;
        }
        if !presentation::should_defer(self.content().as_ref()) {
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
    fn remove_queued_presentation(&self) {
        self.target.remove(self);
    }
}
pub(crate) fn same_content(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => boxed_reference_equals(a, b),
        _ => false,
    }
}
pub(crate) fn same_template(
    a: &Option<Rc<dyn IDataTemplate>>,
    b: &Option<Rc<dyn IDataTemplate>>,
) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}
pub(crate) fn changed_schedule(change: &FerroPropertyChangedEventArgs<'_>) -> bool {
    change.property() == DeferredContentScheduling::timeline_property().as_property()
        || change.property() == DeferredContentScheduling::delay_property().as_property()
        || change.property() == DeferredContentScheduling::order_property().as_property()
}
fn apply_state(
    presenter: &ContentPresenter,
    content: Option<BoxedValue>,
    template: Option<Rc<dyn IDataTemplate>>,
    duration: TimeSpan,
) {
    if let Some(deferred) = presenter.to_ref().cast::<DeferredContentPresenter>() {
        deferred.apply_deferred_state(content, template, duration);
        return;
    }
    let had_child = presenter.child().is_some();
    let has_content = content.is_some();
    presenter.set_current_value(ContentPresenter::content_template_property(), template);
    presenter.set_current_value(ContentPresenter::content_property(), content);
    reveal::apply(
        presenter,
        duration,
        had_child && has_content && presenter.child().is_some(),
    );
}
