// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{managed_dockable_base::ContextValue, ManagedDockableBase, ManagedDockableBaseImpl};
use crate::{
    impl_managed_dockable,
    internal::{
        dock_window_state_helper::DockWindowStateHelper as States,
        drag_preview_context::DragPreviewContext, managed_host_window_access::managed_host,
    },
};
use ferrodock_controls_deferred_content_control::IDeferredContentPresentation;
use ferrodock_controls_recycling::from_recycling_value;
use ferrodock_model::{controls::*, core::*, BoxedValue as ModelValue};
use ferroui_base::{
    data::core::ValueType,
    media::{Stretch, VisualBrush},
    metadata::IServiceProvider,
    BoxedValue, Ref,
};
use ferroui_controls::{
    presenters::ContentPresenter,
    templates::{IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam, TemplateResult},
    Border, ContentControl, Control, Decorator, Panel,
};
use ferroui_markup_xaml::templates::{DataTemplate, TemplateContent};
use ferroui_system::IDisposable;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
/// Original template-content delegate. The source passes a null service provider.
pub type ManagedTemplateContentFactory =
    Rc<dyn Fn(Option<Rc<dyn IServiceProvider>>) -> Option<ModelValue>>;
struct Subscription {
    unsubscribe: Box<dyn Fn()>,
}
impl Subscription {
    fn dispose(self) {
        (self.unsubscribe)();
    }
}
/// Original managed floating-window document, with live window/layout/focus subscriptions.
pub struct ManagedDockWindowDocument {
    base: ManagedDockableBase,
    runtime: Weak<Self>,
    mdi_bounds: RefCell<DockRect>,
    mdi_state: RefCell<MdiWindowState>,
    mdi_z_index: RefCell<i32>,
    content: RefCell<ContextValue>,
    matcher: Rc<DataTemplate>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    window_subscription: RefCell<Option<Subscription>>,
    layout_subscription: RefCell<Option<Subscription>>,
    focused_subscription: RefCell<Option<Subscription>>,
    focused: RefCell<Option<Rc<dyn IDockable>>>,
}
impl_managed_dockable!(
    ManagedDockWindowDocument,
    fn as_local_target(self: Rc<Self>) -> Option<Rc<dyn ILocalTarget>> {
        Some(self)
    },
    fn boxed_reference(self: Rc<Self>) -> ModelValue {
        Rc::new(self)
    },
    fn as_document(self: Rc<Self>) -> Option<Rc<dyn IDocument>> {
        Some(self)
    },
    fn as_mdi_document(self: Rc<Self>) -> Option<Rc<dyn IMdiDocument>> {
        Some(self)
    },
    fn as_document_content(self: Rc<Self>) -> Option<Rc<dyn IDocumentContent>> {
        Some(self)
    }
);
impl ILocalTarget for ManagedDockWindowDocument {}
impl IDocument for ManagedDockWindowDocument {}
impl ManagedDockableBaseImpl for ManagedDockWindowDocument {
    fn on_close(this: &Self) -> bool {
        let Some(window) = this.window() else {
            return true;
        };
        if let Some(host) = window.host() {
            if let Some(managed) = managed_host(&host) {
                host.exit();
                return !managed.last_close_canceled();
            }
        }
        window.exit();
        true
    }
    fn on_property_changed(this: &Self, name: &str) {
        this.base.raise_property_changed(name);
        let Some(window) = this.window() else {
            return;
        };
        match name {
            "Title" => window.set_title(this.title()),
            "Id" => window.set_id(this.id()),
            "MdiState" => {
                let mapped = States::to_dock_window_state_from_mdi(this.mdi_state());
                if !(window.window_state() == DockWindowState::FullScreen
                    && mapped == DockWindowState::Normal)
                {
                    window.set_window_state(mapped);
                }
            }
            _ => {}
        }
    }
}
impl ManagedDockWindowDocument {
    /// Creates the original model and attaches its optional source notifications.
    pub fn new(window: Rc<dyn IDockWindow>) -> Rc<Self> {
        let this = Rc::new_cyclic(|runtime| Self {
            base: ManagedDockableBase::construct(runtime.clone()),
            runtime: runtime.clone(),
            mdi_bounds: RefCell::new(DockRect::default()),
            mdi_state: RefCell::new(MdiWindowState::Normal),
            mdi_z_index: RefCell::new(0),
            content: RefCell::new(ContextValue(None)),
            matcher: DataTemplate::new(),
            window: RefCell::new(Some(window.clone())),
            window_subscription: RefCell::new(None),
            layout_subscription: RefCell::new(None),
            focused_subscription: RefCell::new(None),
            focused: RefCell::new(None),
        });
        this.set_id(window.id());
        this.set_title(window.title());
        this.mdi_state
            .replace(States::to_mdi_window_state(window.window_state()));
        this.set_context(Some(Rc::new(window.clone())));
        this.attach_window(Some(window.clone()));
        this.attach_layout(window.layout().map(|root| root as Rc<dyn IDock>));
        this.update_title_from_layout();
        this
    }
    /// Gets the associated `Window`, cleared by Dispose.
    pub fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    /// Whether the original window's active dockable is a tool dock.
    pub fn is_tool_window(&self) -> bool {
        self.tool_dock().is_some()
    }
    /// Gets the tool dock hosted by the source window.
    pub fn tool_dock(&self) -> Option<Rc<dyn IToolDock>> {
        self.window()?.layout()?.active_dockable()?.as_tool_dock()
    }
    /// Gets `Content` with the original model object identity.
    pub fn content(&self) -> Option<ModelValue> {
        self.content.borrow().0.clone()
    }
    /// Sets Content and raises only the source PropertyChanged event.
    pub fn set_content(&self, value: Option<ModelValue>) {
        self.base
            .set_property(&self.content, ContextValue(value), Some("Content"));
    }
    /// Gets the native type handle used by original template matching.
    pub fn data_type(&self) -> Option<ValueType> {
        self.matcher.data_type()
    }
    /// Sets DataType before raising the original notification.
    pub fn set_data_type(&self, value: Option<ValueType>) {
        if self.data_type() != value {
            self.matcher.set_data_type(value);
            self.base.on_property_changed("DataType");
        }
    }
    /// Gets original MDI bounds.
    pub fn mdi_bounds(&self) -> DockRect {
        *self.mdi_bounds.borrow()
    }
    /// Sets MdiBounds, then synchronizes source window and host geometry.
    pub fn set_mdi_bounds(&self, value: DockRect) {
        if self
            .base
            .set_property(&self.mdi_bounds, value, Some("MdiBounds"))
        {
            self.sync_bounds();
        }
    }
    /// Gets original MDI state.
    pub fn mdi_state(&self) -> MdiWindowState {
        *self.mdi_state.borrow()
    }
    /// Sets MdiState and preserves the full-screen/normal mapping guard.
    pub fn set_mdi_state(&self, value: MdiWindowState) {
        self.base
            .set_property(&self.mdi_state, value, Some("MdiState"));
    }
    /// Gets original MDI z index.
    pub fn mdi_z_index(&self) -> i32 {
        *self.mdi_z_index.borrow()
    }
    /// Sets original MDI z index.
    pub fn set_mdi_z_index(&self, value: i32) {
        self.base
            .set_property(&self.mdi_z_index, value, Some("MdiZIndex"));
    }
    /// Matches through the shared native type system.
    pub fn match_(&self, data: Option<&BoxedValue>) -> bool {
        // An opaque interface handle has a static Rust type. For the checked
        // cross-interface casts below, test the value rather than treating the
        // existence of a native cast registration as proof of membership.
        if let (Some(target), Some(value)) = (self.data_type(), data) {
            let opaque = value
                .downcast_ref::<Rc<dyn ferrodock_model::controls::IRootDock>>()
                .is_some()
                || value.downcast_ref::<Rc<dyn IDockable>>().is_some()
                || value.downcast_ref::<Rc<dyn IMdiDocument>>().is_some()
                || value.downcast_ref::<Rc<dyn IDock>>().is_some();
            let checked = target.is::<Rc<dyn ferrodock_model::controls::IRootDock>>()
                || target.is::<Rc<dyn IDock>>()
                || target.is::<Rc<dyn IDocumentDock>>()
                || target.is::<Rc<dyn IToolDock>>()
                || target.is::<Rc<dyn IMdiDocument>>()
                || target.is::<Rc<ManagedDockWindowDocument>>()
                || target.is::<ManagedDockWindowDocument>()
                || target.is::<Rc<super::ManagedWindowDock>>()
                || target.is::<super::ManagedWindowDock>()
                || target.is::<super::ManagedDockableBaseReference>()
                || target.is::<Rc<dyn IDataTemplate>>()
                || target.is::<Rc<dyn IRecyclingDataTemplate>>()
                || target.is::<Rc<dyn IDeferredContentPresentation>>();
            if opaque && checked {
                return ferroui_base::data::core::ValueTypes::try_cast(value, target).is_some();
            }
        }
        self.matcher.match_(data)
    }
    /// Builds original content without an existing control.
    pub fn build(&self, _data: Option<&BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(None, None)
    }
    /// Builds/recycles original content without stealing a control from unsupported parents.
    pub fn build_with_existing(
        &self,
        _data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        let dockable: Rc<dyn IDockable> = self
            .runtime
            .upgrade()
            .expect("managed document owns its template invocation");
        Self::build_content(self.content(), &dockable, existing)
    }
    /// Detaches focused/layout/window notifications and clears Window and Content.
    pub fn dispose(&self) {
        self.detach_focused_dockable();
        self.detach_layout();
        self.detach_window();
        self.window.replace(None);
        self.set_content(None);
    }
    fn attach_window(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.detach_window();
        let Some(window) = window else { return };
        let Some(notifier) = window.as_notify_property_changed() else {
            return;
        };
        let this = self.runtime.upgrade().expect("live managed document");
        let token = notifier
            .property_changed()
            .add(Rc::new(move |name| this.window_property_changed(name)));
        self.window_subscription.replace(Some(Subscription {
            unsubscribe: Box::new(move || {
                if let Some(notifier) = window.as_notify_property_changed() {
                    notifier.property_changed().remove(token);
                }
            }),
        }));
    }
    fn detach_window(&self) {
        let subscription = self.window_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
    fn window_property_changed(&self, name: &str) {
        if name == "Layout" {
            self.attach_layout(
                self.window()
                    .and_then(|w| w.layout())
                    .map(|r| r as Rc<dyn IDock>),
            );
            self.update_title_from_layout();
            return;
        }
        if name == "Title" {
            if self
                .window()
                .and_then(|w| w.layout())
                .and_then(|l| l.focused_dockable())
                .is_none()
            {
                self.update_title_from_layout();
            }
            return;
        }
        if name == "WindowState" {
            if let Some(window) = self.window() {
                self.set_mdi_state(States::to_mdi_window_state(window.window_state()));
            }
        }
    }
    fn attach_layout(&self, layout: Option<Rc<dyn IDock>>) {
        self.detach_layout();
        let Some(layout) = layout else { return };
        let Some(notifier) = layout.as_notify_property_changed() else {
            return;
        };
        let this = self.runtime.upgrade().expect("live managed document");
        let token = notifier
            .property_changed()
            .add(Rc::new(move |name| this.layout_property_changed(name)));
        self.layout_subscription.replace(Some(Subscription {
            unsubscribe: Box::new(move || {
                if let Some(notifier) = layout.as_notify_property_changed() {
                    notifier.property_changed().remove(token);
                }
            }),
        }));
    }
    fn detach_layout(&self) {
        let subscription = self.layout_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
    fn layout_property_changed(&self, name: &str) {
        if name.is_empty() || name == "FocusedDockable" {
            self.update_title_from_layout();
        }
    }
    fn update_title_from_layout(&self) {
        let focused = self
            .window()
            .and_then(|w| w.layout())
            .and_then(|l| l.focused_dockable());
        self.update_focused_subscription(focused.clone());
        if let Some(title) = focused.map(|f| f.title()).filter(|t| !t.is_empty()) {
            self.set_title(title);
            return;
        }
        if let Some(window) = self.window() {
            self.set_title(window.title());
        }
    }
    fn update_focused_subscription(&self, focused: Option<Rc<dyn IDockable>>) {
        let same = match (&*self.focused.borrow(), &focused) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if same {
            return;
        }
        self.detach_focused_dockable();
        self.focused.replace(focused.clone());
        let Some(focused) = focused else { return };
        let Some(notifier) = focused.as_notify_property_changed() else {
            return;
        };
        let this = self.runtime.upgrade().expect("live managed document");
        let token = notifier.property_changed().add(Rc::new(move |name| {
            if name.is_empty() || name == "Title" {
                this.update_title_from_layout();
            }
        }));
        self.focused_subscription.replace(Some(Subscription {
            unsubscribe: Box::new(move || {
                if let Some(notifier) = focused.as_notify_property_changed() {
                    notifier.property_changed().remove(token);
                }
            }),
        }));
    }
    fn detach_focused_dockable(&self) {
        let subscription = self.focused_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.focused.replace(None);
    }
    fn sync_bounds(&self) {
        if self.window().is_none() {
            return;
        }
        self.window()
            .expect("source window remains present")
            .set_x(self.mdi_bounds().x);
        self.window()
            .expect("source window remains present")
            .set_y(self.mdi_bounds().y);
        self.window()
            .expect("source window remains present")
            .set_width(self.mdi_bounds().width);
        self.window()
            .expect("source window remains present")
            .set_height(self.mdi_bounds().height);
        if let Some(host) = self.window().expect("source window remains present").host() {
            if let Some(managed) = managed_host(&host) {
                managed.update_bounds_from_document(self.mdi_bounds());
                return;
            }
        }
        if let Some(host) = self.window().expect("source window remains present").host() {
            host.set_position(self.mdi_bounds().x, self.mdi_bounds().y);
            host.set_size(self.mdi_bounds().width, self.mdi_bounds().height);
        }
    }
    fn build_content(
        content: Option<ModelValue>,
        dockable: &Rc<dyn IDockable>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        if DragPreviewContext::is_previewing(dockable) {
            return Some(Self::build_preview_content(content));
        }
        let content = content?;
        let native = from_recycling_value(&content);
        if let Some(control) = Control::from_boxed(&native) {
            return Self::detach_or_fallback(Some(control), existing, None);
        }
        Self::detach_or_fallback(
            Self::load_template_content(Some(content.clone())),
            existing,
            Some(&|| Self::load_template_content(Some(content.clone()))),
        )
    }
    fn detach_or_fallback(
        control: Option<Ref<Control>>,
        existing: Option<Ref<Control>>,
        fallback: Option<&dyn Fn() -> Option<Ref<Control>>>,
    ) -> Option<Ref<Control>> {
        let control = control?;
        if existing.as_ref() == Some(&control) {
            return Some(control);
        }
        if Self::try_detach_from_parent(&control) {
            return Some(control);
        }
        let Some(fallback) = fallback else {
            return existing;
        };
        let Some(fallback) = fallback() else {
            return existing;
        };
        if existing.as_ref() == Some(&fallback) {
            return Some(fallback);
        }
        if Self::try_detach_from_parent(&fallback) {
            Some(fallback)
        } else {
            existing
        }
    }
    fn try_detach_from_parent(control: &Control) -> bool {
        let parent = control
            .parent()
            .map(|p| p.upcast::<ferroui_base::FerroObject>())
            .or_else(|| control.visual_parent().map(|p| p.upcast()));
        let Some(parent) = parent else { return true };
        if let Some(panel) = parent.cast::<Panel>() {
            return panel.children().remove(control.to_ref());
        }
        if let Some(presenter) = parent.cast::<ContentPresenter>() {
            return Self::try_detach_from_content_presenter(&presenter, control);
        }
        if let Some(content) = parent.cast::<ContentControl>() {
            if content
                .content()
                .as_ref()
                .and_then(Control::from_boxed)
                .as_ref()
                == Some(&control.to_ref())
            {
                content.set_current_value(ContentControl::content_property(), None);
                return true;
            }
        }
        if let Some(decorator) = parent.cast::<Decorator>() {
            if decorator.child().as_ref() == Some(&control.to_ref()) {
                decorator.set_child(None);
                return true;
            }
        }
        false
    }
    fn try_detach_from_content_presenter(presenter: &ContentPresenter, control: &Control) -> bool {
        if presenter.child().as_ref() != Some(&control.to_ref()) {
            return false;
        }
        presenter.set_current_value(ContentPresenter::content_property(), None);
        presenter.update_child();
        control.visual_parent().is_none()
    }
    fn build_preview_content(content: Option<ModelValue>) -> Ref<Control> {
        let control = content
            .as_ref()
            .map(from_recycling_value)
            .as_ref()
            .and_then(Control::from_boxed);
        let Some(control) = control else {
            return Panel::new().upcast();
        };
        let brush = VisualBrush::with_visual(control);
        brush.set_stretch(Stretch::Uniform);
        let border = Border::new();
        border.set_background(Some(brush.into()));
        border.set_clip_to_bounds(true);
        border.upcast()
    }
    fn load_template_content(content: Option<ModelValue>) -> Option<Ref<Control>> {
        let content = content?;
        if let Some(result) = content.downcast_ref::<TemplateResult<Ref<Control>>>() {
            return Some(result.result().clone());
        }
        let native = from_recycling_value(&content);
        if let Some(result) = native.downcast_ref::<TemplateResult<Ref<Control>>>() {
            return Some(result.result().clone());
        }
        if let Some(control) = Control::from_boxed(&native) {
            return Some(control);
        }
        if let Some(factory) = content.downcast_ref::<ManagedTemplateContentFactory>() {
            let evaluated = factory(None);
            if evaluated
                .as_ref()
                .is_some_and(|value| Rc::ptr_eq(value, &content))
            {
                return None;
            }
            return Self::load_template_content(evaluated);
        }
        TemplateContent::load(Some(&native)).map(|result| result.result().clone())
    }
}
impl IMdiDocument for ManagedDockWindowDocument {
    fn mdi_bounds(&self) -> DockRect {
        self.mdi_bounds()
    }
    fn set_mdi_bounds(&self, value: DockRect) {
        self.set_mdi_bounds(value)
    }
    fn mdi_state(&self) -> MdiWindowState {
        self.mdi_state()
    }
    fn set_mdi_state(&self, value: MdiWindowState) {
        self.set_mdi_state(value)
    }
    fn mdi_z_index(&self) -> i32 {
        self.mdi_z_index()
    }
    fn set_mdi_z_index(&self, value: i32) {
        self.set_mdi_z_index(value)
    }
}
impl IDocumentContent for ManagedDockWindowDocument {
    fn content(&self) -> Option<ModelValue> {
        self.content()
    }
    fn set_content(&self, value: Option<ModelValue>) {
        self.set_content(value)
    }
}
impl IDeferredContentPresentation for ManagedDockWindowDocument {
    fn defer_content_presentation(&self) -> bool {
        false
    }
}
impl IDisposable for ManagedDockWindowDocument {
    fn dispose(&self) {
        self.dispose()
    }
}
impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ManagedDockWindowDocument {
    fn build(&self, data: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build(data.as_ref())
    }
}
impl IDataTemplate for ManagedDockWindowDocument {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        self.match_(data)
    }
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}
impl IRecyclingDataTemplate for ManagedDockWindowDocument {
    fn build_with_existing(
        &self,
        data: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        self.build_with_existing(data, existing)
    }
}
// The shared 0.3.0 typed-path probe selects a nonnotifying specialization here.
// Fallible setter metadata uses the standard compiled INPC accessor instead;
// source setters and their exceptions/notifications remain unchanged.
ferroui_base::ferro_markup_type!(class ManagedDockWindowDocument {
    namespace:"FerroDock.Ferro.Controls",this:Rc<ManagedDockWindowDocument>,handles:[Rc<ManagedDockWindowDocument>,Option<Rc<ManagedDockWindowDocument>>],
    base:super::ManagedDockableBaseReference,
    notify_property_changed:ManagedDockWindowDocument,
    interfaces:[Rc<dyn IMdiDocument>,Rc<dyn IDocumentContent>,Rc<dyn IRecyclingDataTemplate>,Rc<dyn IDeferredContentPresentation>],
    constructors:[(Rc<dyn IDockWindow>)=>ManagedDockWindowDocument::new],
    properties:[
        Window:Option<Rc<dyn IDockWindow>>{get:|this:&Rc<ManagedDockWindowDocument>|this.window()},
        IsToolWindow:bool{get:|this:&Rc<ManagedDockWindowDocument>|this.is_tool_window()},
        ToolDock:Option<Rc<dyn IToolDock>>{get:|this:&Rc<ManagedDockWindowDocument>|this.tool_dock()},
        Content:Option<BoxedValue>{get:|this:&Rc<ManagedDockWindowDocument>|this.content().as_ref().map(from_recycling_value),try_set:|this:&Rc<ManagedDockWindowDocument>,value:Option<BoxedValue>|{this.set_content(value.map(ferrodock_controls_recycling::to_recycling_value)); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        DataType:Option<ValueType>{get:|this:&Rc<ManagedDockWindowDocument>|this.data_type(),try_set:|this:&Rc<ManagedDockWindowDocument>,value|{this.set_data_type(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        MdiBounds:DockRect{get:|this:&Rc<ManagedDockWindowDocument>|this.mdi_bounds(),try_set:|this:&Rc<ManagedDockWindowDocument>,value|{this.set_mdi_bounds(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        MdiState:MdiWindowState{get:|this:&Rc<ManagedDockWindowDocument>|this.mdi_state(),try_set:|this:&Rc<ManagedDockWindowDocument>,value|{this.set_mdi_state(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        MdiZIndex:i32{get:|this:&Rc<ManagedDockWindowDocument>|this.mdi_z_index(),try_set:|this:&Rc<ManagedDockWindowDocument>,value|{this.set_mdi_z_index(value); Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
    methods:[fn Match(Option<BoxedValue>)->bool=>(|this:&Rc<ManagedDockWindowDocument>,data:Option<BoxedValue>|this.match_(data.as_ref())),fn Build(Option<BoxedValue>)->Option<Ref<Control>> =>(|this:&Rc<ManagedDockWindowDocument>,data:Option<BoxedValue>|this.build(data.as_ref())),fn Build(Option<BoxedValue>,Option<Ref<Control>>)->Option<Ref<Control>> =>(|this:&Rc<ManagedDockWindowDocument>,data:Option<BoxedValue>,existing|this.build_with_existing(data.as_ref(),existing)),fn Dispose()=>(|this:&Rc<ManagedDockWindowDocument>|this.dispose())],
});

#[cfg(test)]
#[path = "managed_document_tests.rs"]
mod tests;
