// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

//! Original managed document behavior using the existing reactive window and dock models.
use super::*;
use crate::controls::ManagedWindowDock;
use crate::internal::managed_host_window_access::ManagedHostWindowAccess;
use ferrodock_model_reactiveui::{
    controls::{Document, RootDock, ToolDock},
    core::DockWindow,
    Factory,
};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_controls::{AppBuilder, Application, Button, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{any::TypeId, cell::Cell};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "managed-document",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                ferroui_markup_xaml::register_types();
                f()
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn window() -> (Rc<DockWindow>, Rc<RootDock>) {
    let root = RootDock::new();
    let window = DockWindow::new();
    window.set_id("window".into());
    window.set_title("Window title".into());
    window.set_layout(Some(root.clone()));
    (window, root)
}
fn events(model: &dyn INotifyPropertyChanged) -> Rc<RefCell<Vec<String>>> {
    let names = Rc::new(RefCell::new(Vec::new()));
    let observed = names.clone();
    model.property_changed().add(Rc::new(move |name| {
        observed.borrow_mut().push(name.to_owned())
    }));
    names
}
#[test]
fn defaults_identity_nullable_members_and_source_notifications_follow_original_window() {
    let (window, root) = window();
    window.set_window_state(DockWindowState::Maximized);
    let document = ManagedDockWindowDocument::new(window.clone());
    let contract: Rc<dyn IDockWindow> = window.clone();
    assert!(Rc::ptr_eq(&document.window().unwrap(), &contract));
    assert_eq!(document.id(), "window");
    assert_eq!(document.title(), "Window title");
    assert!(Rc::ptr_eq(
        document
            .context()
            .unwrap()
            .downcast_ref::<Rc<dyn IDockWindow>>()
            .unwrap(),
        &contract
    ));
    assert_eq!(document.mdi_bounds(), DockRect::default());
    assert_eq!(document.mdi_state(), MdiWindowState::Maximized);
    assert_eq!(document.mdi_z_index(), 0);
    assert!(document.content().is_none());
    assert!(document.data_type().is_none());
    assert!(!document.defer_content_presentation());
    assert!(!document.is_tool_window());
    assert!(document.tool_dock().is_none());
    let tool = ToolDock::new();
    root.set_active_dockable(Some(tool.clone()));
    assert!(document.is_tool_window());
    let expected: Rc<dyn IToolDock> = tool;
    assert!(Rc::ptr_eq(&document.tool_dock().unwrap(), &expected));
    let names = events(&*document);
    let value: ModelValue = Rc::new("content".to_owned());
    document.set_content(Some(value.clone()));
    document.set_content(Some(value.clone()));
    assert!(Rc::ptr_eq(&document.content().unwrap(), &value));
    document.set_mdi_z_index(42);
    document.set_mdi_z_index(42);
    document.set_data_type(Some(ValueType::of::<String>()));
    document.set_data_type(Some(ValueType::of::<String>()));
    assert_eq!(&*names.borrow(), &["Content", "MdiZIndex", "DataType"]);
    document.dispose();
    assert!(document.window().is_none());
    assert!(document.content().is_none());
    assert!(document.context().is_some());
    assert_eq!(document.mdi_z_index(), 42);
    assert_eq!(document.data_type(), Some(ValueType::of::<String>()));
    assert!(document.on_close());
    document.dispose();
}
#[test]
fn focused_title_replacement_detaches_old_sources_and_window_state_retains_fullscreen_guard() {
    let (window, root) = window();
    let first = Document::new();
    first.set_title("Focused".into());
    root.set_focused_dockable(Some(first.clone()));
    let document = ManagedDockWindowDocument::new(window.clone());
    assert_eq!(document.title(), "Focused");
    assert_eq!(window.title(), "Focused");
    first.set_title("Changed".into());
    assert_eq!(document.title(), "Changed");
    window.set_title("External".into());
    assert_eq!(document.title(), "Changed");
    let second = Document::new();
    second.set_title("Second".into());
    root.set_focused_dockable(Some(second.clone()));
    assert_eq!(document.title(), "Second");
    first.set_title("Stale".into());
    assert_eq!(document.title(), "Second");
    second.set_title(" \t".into());
    assert_eq!(document.title(), " \t");
    let replacement = RootDock::new();
    let third = Document::new();
    third.set_title("Replacement".into());
    replacement.set_focused_dockable(Some(third.clone()));
    window.set_layout(Some(replacement.clone()));
    assert_eq!(document.title(), "Replacement");
    second.set_title("Detached".into());
    assert_eq!(document.title(), "Replacement");
    window.set_window_state(DockWindowState::FullScreen);
    assert_eq!(document.mdi_state(), MdiWindowState::Normal);
    document.set_mdi_state(MdiWindowState::Normal);
    assert_eq!(window.window_state(), DockWindowState::FullScreen);
    document.set_mdi_state(MdiWindowState::Minimized);
    assert_eq!(window.window_state(), DockWindowState::Minimized);
    document.set_id("new-id".into());
    assert_eq!(window.id(), "new-id");
    document.set_title("Manual".into());
    assert_eq!(window.title(), "Manual");
    replacement.set_focused_dockable(None);
    window.set_title("No focus".into());
    assert_eq!(document.title(), "No focus");
    document.dispose();
    third.set_title("After dispose".into());
    window.set_title("After dispose".into());
    assert_eq!(document.title(), "No focus");
    assert_eq!(first.property_changed().handler_count(), 0);
}
#[test]
fn dispose_releases_document_while_original_window_layout_focus_and_factory_remain_alive() {
    let factory = Factory::new();
    let (window, root) = window();
    window.set_factory(Some(factory.clone()));
    root.set_factory(Some(factory.clone()));
    let focused = Document::new();
    focused.set_factory(Some(factory.clone()));
    root.set_focused_dockable(Some(focused.clone()));
    let before = (
        window.property_changed().handler_count(),
        root.property_changed().handler_count(),
        focused.property_changed().handler_count(),
    );
    let document = ManagedDockWindowDocument::new(window.clone());
    document.set_factory(Some(factory.clone()));
    let weak = Rc::downgrade(&document);
    assert!(window.property_changed().handler_count() > 0);
    assert!(root.property_changed().handler_count() > 0);
    assert!(focused.property_changed().handler_count() > 0);
    drop(document);
    assert!(weak.upgrade().is_some());
    let document = weak.upgrade().unwrap();
    document.dispose();
    drop(document);
    assert!(weak.upgrade().is_none());
    assert!(window.layout().is_some());
    assert!(root.focused_dockable().is_some());
    assert!(focused.factory().is_some());
    assert_eq!(
        (
            window.property_changed().handler_count(),
            root.property_changed().handler_count(),
            focused.property_changed().handler_count()
        ),
        before
    );
}
struct ProbeHost {
    managed: bool,
    canceled: Cell<bool>,
    exits: Cell<usize>,
    bounds: RefCell<Vec<DockRect>>,
    position: Cell<(f64, f64)>,
    size: Cell<(f64, f64)>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    tracked: Cell<bool>,
}
impl ProbeHost {
    fn new(managed: bool) -> Rc<Self> {
        Rc::new(Self {
            managed,
            canceled: Cell::new(false),
            exits: Cell::new(0),
            bounds: RefCell::new(Vec::new()),
            position: Cell::new((0.0, 0.0)),
            size: Cell::new((400.0, 300.0)),
            window: RefCell::new(None),
            tracked: Cell::new(false),
        })
    }
}
impl ManagedHostWindowAccess for ProbeHost {
    fn managed_z_index(&self) -> i32 {
        0
    }
    fn process_drag(&self, _: ferroui_base::PixelPoint, _: crate::internal::event_type::EventType) {
    }

    fn last_close_canceled(&self) -> bool {
        self.canceled.get()
    }
    fn update_bounds_from_document(&self, bounds: DockRect) {
        self.bounds.borrow_mut().push(bounds);
    }
}
impl IHostWindow for ProbeHost {
    fn boxed_interface(self: Rc<Self>, target: TypeId) -> Option<ModelValue> {
        if self.managed && target == TypeId::of::<Rc<dyn ManagedHostWindowAccess>>() {
            let access: Rc<dyn ManagedHostWindowAccess> = self;
            Some(Rc::new(access))
        } else {
            None
        }
    }
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        None
    }
    fn is_tracked(&self) -> bool {
        self.tracked.get()
    }
    fn set_is_tracked(&self, v: bool) {
        self.tracked.set(v)
    }
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    fn set_window(&self, v: Option<Rc<dyn IDockWindow>>) {
        self.window.replace(v);
    }
    fn present(&self, _: bool) {}
    fn exit(&self) {
        self.exits.set(self.exits.get() + 1);
    }
    fn set_position(&self, x: f64, y: f64) {
        self.position.set((x, y));
    }
    fn get_position(&self) -> (f64, f64) {
        self.position.get()
    }
    fn set_size(&self, w: f64, h: f64) {
        self.size.set((w, h));
    }
    fn get_size(&self) -> (f64, f64) {
        self.size.get()
    }
    fn set_window_state(&self, _: DockWindowState) {}
    fn get_window_state(&self) -> DockWindowState {
        DockWindowState::Normal
    }
    fn set_title(&self, _: Option<String>) {}
    fn set_layout(&self, _: Rc<dyn IDock>) {}
    fn set_active(&self) {}
}
#[test]
fn original_close_and_bounds_branches_dispatch_managed_members_and_ordinary_host_api() {
    for managed in [false, true] {
        let (window, _root) = window();
        let host = ProbeHost::new(managed);
        window.set_host(Some(host.clone()));
        let document = ManagedDockWindowDocument::new(window.clone());
        let bounds = DockRect::new(10.0, 20.0, 320.0, 180.0);
        document.set_mdi_bounds(bounds);
        assert_eq!(
            (window.x(), window.y(), window.width(), window.height()),
            (10.0, 20.0, 320.0, 180.0)
        );
        if managed {
            assert_eq!(&*host.bounds.borrow(), &[bounds]);
            assert_eq!(host.position.get(), (0.0, 0.0));
            host.canceled.set(true);
            assert!(!document.on_close());
            host.canceled.set(false);
            assert!(document.on_close());
            assert_eq!(host.exits.get(), 2);
        } else {
            assert!(host.bounds.borrow().is_empty());
            assert_eq!(host.position.get(), (10.0, 20.0));
            assert_eq!(host.size.get(), (320.0, 180.0));
            assert!(document.on_close());
            assert_eq!(host.exits.get(), 1);
        }
        document.dispose();
        window.set_host(None);
    }
}
#[test]
fn shared_native_matching_accepts_null_without_type_and_honors_native_assignability() {
    run(|| {
        let (window, _root) = window();
        let document = ManagedDockWindowDocument::new(window);
        assert!(document.match_(None));
        document.set_data_type(Some(ValueType::of::<String>()));
        assert!(!document.match_(None));
        let text: BoxedValue = Rc::new("text".to_owned());
        assert!(document.match_(Some(&text)));
        let number: BoxedValue = Rc::new(3);
        assert!(!document.match_(Some(&number)));
        document.set_data_type(Some(ValueType::of::<Ref<Control>>()));
        assert!(document.match_(Some(&Control::boxed(Button::new()))));
        document.set_data_type(None);
        assert!(document.match_(Some(&number)));
        document.dispose();
    });
}
fn native(value: Ref<Control>) -> ModelValue {
    ferrodock_controls_recycling::to_recycling_value(Control::boxed(value))
}
#[test]
fn content_build_preserves_existing_and_detaches_all_original_supported_parent_kinds() {
    run(|| {
        let (window, _root) = window();
        let document = ManagedDockWindowDocument::new(window);
        assert!(document.build(None).is_none());
        let child = Button::new();
        document.set_content(Some(native(child.clone().upcast())));
        assert_eq!(
            document.build_with_existing(None, Some(child.clone().upcast())),
            Some(child.clone().upcast())
        );
        let panel = Panel::new();
        panel.children().add(child.clone().upcast::<Control>());
        assert_eq!(document.build(None), Some(child.clone().upcast()));
        assert_eq!(panel.children().count(), 0);
        let decorator = Decorator::new();
        decorator.set_child(Some(child.clone().upcast()));
        assert_eq!(document.build(None), Some(child.clone().upcast()));
        assert!(decorator.child().is_none());
        let content = ContentControl::new();
        content.set_content(Some(Control::boxed(child.clone())));
        let host = Window::new();
        host.set_content(Some(Control::boxed(content.clone())));
        host.show();
        Dispatcher::ui_thread().run_jobs(None);
        host.update_layout();
        assert_eq!(document.build(None), Some(child.clone().upcast()));
        assert!(content.content().is_none());
        let presenter = ContentPresenter::new();
        presenter.set_content(Some(Control::boxed(child.clone())));
        presenter.update_child();
        assert_eq!(document.build(None), Some(child.clone().upcast()));
        assert!(presenter.child().is_none());
        host.close();
        document.dispose();
    });
}
#[test]
fn template_result_and_recursive_delegate_load_original_content_without_cloning_controls() {
    run(|| {
        use ferroui_base::controls::NameScope;
        let (window, _root) = window();
        let document = ManagedDockWindowDocument::new(window);
        let child = Button::new().upcast::<Control>();
        let result = TemplateResult::new(
            child.clone(),
            ferroui_base::controls::NameScopeRef(Rc::new(NameScope::new())),
        );
        document.set_content(Some(Rc::new(result)));
        assert_eq!(document.build(None), Some(child.clone()));
        let count = Rc::new(Cell::new(0));
        let calls = count.clone();
        let content = child.clone();
        let factory: ManagedTemplateContentFactory = Rc::new(move |provider| {
            assert!(provider.is_none());
            calls.set(calls.get() + 1);
            Some(native(content.clone()))
        });
        document.set_content(Some(Rc::new(factory)));
        assert_eq!(document.build(None), Some(child));
        assert_eq!(count.get(), 1);
        document.dispose();
    });
}
#[test]
fn preview_build_uses_visual_brush_without_detaching_original_and_nonvisual_returns_panel() {
    run(|| {
        let (window, _root) = window();
        let document = ManagedDockWindowDocument::new(window);
        let child = Button::new();
        let parent = Panel::new();
        parent.children().add(child.clone().upcast::<Control>());
        document.set_content(Some(native(child.clone().upcast())));
        let contract: Rc<dyn IDockable> = document.clone();
        DragPreviewContext::set_dockable(Some(contract));
        DragPreviewContext::set_is_active(true);
        let preview = document.build(None).unwrap().cast::<Border>().unwrap();
        assert!(preview.clip_to_bounds());
        assert_eq!(parent.children().count(), 1);
        assert!(preview.background().unwrap().as_tile_brush().is_some());
        document.set_content(None);
        assert!(document.build(None).unwrap().is::<Panel>());
        DragPreviewContext::clear();
        document.dispose();
    });
}
#[test]
fn managed_dock_source_defaults_noop_identity_add_remove_and_original_notification_order() {
    let dock = ManagedWindowDock::new();
    assert!(!dock.is_collapsable());
    assert_eq!(dock.visible_dockables().unwrap().count(), 0);
    assert!(dock.active_dockable().is_none());
    assert!(dock.default_dockable().is_none());
    assert!(dock.focused_dockable().is_none());
    assert!(!dock.is_active());
    assert_eq!(dock.opened_dockables_count(), 0);
    assert!(dock.can_close_last_dockable());
    assert!(dock.dock_capability_policy().is_none());
    assert!(dock.enable_global_docking());
    assert!(!dock.can_go_back());
    assert!(!dock.can_go_forward());
    let commands = [
        dock.go_back(),
        dock.go_forward(),
        dock.navigate(),
        dock.close(),
        ManagedWindowDock::new().close(),
    ];
    for command in &commands {
        assert!(Rc::ptr_eq(command, &commands[0]));
        assert!(!command.can_execute(None));
        command.execute(None);
        command
            .can_execute_changed(Rc::new(|| panic!("no-op never raises")))
            .dispose();
    }
    let (first_window, _) = window();
    let (second_window, _) = window();
    let first = ManagedDockWindowDocument::new(first_window);
    let second = ManagedDockWindowDocument::new(second_window);
    let names = events(&*dock);
    dock.add_window(None);
    dock.add_window(Some(first.clone()));
    dock.add_window(Some(first.clone()));
    assert_eq!(dock.visible_dockables().unwrap().count(), 1);
    assert_eq!(dock.opened_dockables_count(), 1);
    assert_eq!(
        &*names.borrow(),
        &["OpenedDockablesCount", "ActiveDockable"]
    );
    let owner: Rc<dyn IDockable> = dock.clone();
    assert!(Rc::ptr_eq(&first.owner().unwrap(), &owner));
    dock.add_window(Some(second.clone()));
    dock.remove_window(Some(second.clone()));
    assert!(Rc::ptr_eq(
        &dock.active_dockable().unwrap(),
        &(first.clone() as Rc<dyn IDockable>)
    ));
    assert!(second.owner().is_some());
    dock.remove_window(Some(first.clone()));
    assert!(dock.active_dockable().is_none());
    assert_eq!(dock.opened_dockables_count(), 0);
    dock.set_visible_dockables(None);
    dock.remove_window(None);
    dock.add_window(Some(first.clone()));
    assert_eq!(dock.visible_dockables().unwrap().count(), 1);
    dock.remove_window(Some(first.clone()));
    first.dispose();
    second.dispose();
}
#[test]
fn managed_dock_calls_original_factory_activation_events_in_source_order() {
    let factory = Factory::new();
    let dock = ManagedWindowDock::new();
    dock.set_factory(Some(factory.clone()));
    let mut documents = Vec::new();
    for title in ["first", "second"] {
        let (window, root) = window();
        let child = Document::new();
        child.set_title(title.into());
        root.set_active_dockable(Some(child));
        documents.push(ManagedDockWindowDocument::new(window));
    }
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut subscriptions = Vec::new();
    let list = calls.clone();
    subscriptions.push(factory.window_deactivated(Rc::new(move |_| {
        list.borrow_mut().push("window-deactivated")
    })));
    let list = calls.clone();
    subscriptions.push(factory.dockable_deactivated(Rc::new(move |_| {
        list.borrow_mut().push("dockable-deactivated")
    })));
    let list = calls.clone();
    subscriptions.push(
        factory.window_activated(Rc::new(move |_| list.borrow_mut().push("window-activated"))),
    );
    let list = calls.clone();
    subscriptions.push(factory.dockable_activated(Rc::new(move |_| {
        list.borrow_mut().push("dockable-activated")
    })));
    dock.add_window(Some(documents[0].clone()));
    assert_eq!(
        &*calls.borrow(),
        &["window-activated", "dockable-activated"]
    );
    calls.borrow_mut().clear();
    dock.add_window(Some(documents[1].clone()));
    assert_eq!(
        &*calls.borrow(),
        &[
            "window-deactivated",
            "dockable-deactivated",
            "window-activated",
            "dockable-activated"
        ]
    );
    calls.borrow_mut().clear();
    dock.set_active_dockable(Some(documents[1].clone()));
    assert!(calls.borrow().is_empty());
    for document in documents {
        dock.remove_window(Some(document.clone()));
        document.dispose();
    }
    for subscription in subscriptions {
        subscription.dispose();
    }
}
#[test]
fn unsupported_parent_uses_original_two_attempt_fallback_and_retains_existing_when_detach_fails() {
    run(|| {
        let stuck = Button::new().upcast::<Control>();
        let host = Control::new();
        host.visual_children()
            .add(stuck.clone().upcast::<ferroui_base::Visual>());
        let existing = Button::new().upcast::<Control>();
        assert_eq!(
            ManagedDockWindowDocument::detach_or_fallback(
                Some(stuck.clone()),
                Some(stuck.clone()),
                None
            ),
            Some(stuck.clone())
        );
        assert_eq!(host.visual_children().count(), 1);
        assert_eq!(
            ManagedDockWindowDocument::detach_or_fallback(
                Some(stuck.clone()),
                Some(existing.clone()),
                None
            ),
            Some(existing.clone())
        );
        assert_eq!(
            ManagedDockWindowDocument::detach_or_fallback(
                Some(stuck.clone()),
                Some(existing.clone()),
                Some(&|| None)
            ),
            Some(existing.clone())
        );
        let other = Button::new().upcast::<Control>();
        let other_host = Control::new();
        other_host
            .visual_children()
            .add(other.clone().upcast::<ferroui_base::Visual>());
        assert_eq!(
            ManagedDockWindowDocument::detach_or_fallback(
                Some(stuck.clone()),
                Some(existing.clone()),
                Some(&|| Some(other.clone()))
            ),
            Some(existing.clone())
        );
        assert_eq!(
            ManagedDockWindowDocument::detach_or_fallback(
                Some(stuck.clone()),
                Some(existing.clone()),
                Some(&|| Some(existing.clone()))
            ),
            Some(existing.clone())
        );
        let replacement = Button::new().upcast::<Control>();
        let panel = Panel::new();
        panel.children().add(replacement.clone());
        assert_eq!(
            ManagedDockWindowDocument::detach_or_fallback(
                Some(stuck.clone()),
                Some(existing.clone()),
                Some(&|| Some(replacement.clone()))
            ),
            Some(replacement)
        );
        assert_eq!(panel.children().count(), 0);
        let (window, _) = window();
        let model = ManagedDockWindowDocument::new(window);
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let first = stuck.clone();
        let result = existing.clone();
        let factory: ManagedTemplateContentFactory = Rc::new(move |_| {
            let call = count.get();
            count.set(call + 1);
            Some(native(if call == 0 {
                first.clone()
            } else {
                result.clone()
            }))
        });
        model.set_content(Some(Rc::new(factory)));
        assert_eq!(
            model.build_with_existing(None, Some(existing.clone())),
            Some(existing)
        );
        assert_eq!(calls.get(), 2);
        assert_eq!(host.visual_children().count(), 1);
        model.dispose();
        host.visual_children().clear();
        other_host.visual_children().clear();
    });
}
#[test]
fn recursive_template_delegate_returns_none_when_it_returns_its_own_original_object() {
    run(|| {
        let (window, _) = window();
        let model = ManagedDockWindowDocument::new(window);
        let value = Rc::new(RefCell::new(None::<ModelValue>));
        let returned = value.clone();
        let factory: ManagedTemplateContentFactory = Rc::new(move |provider| {
            assert!(provider.is_none());
            returned.borrow().clone()
        });
        let boxed: ModelValue = Rc::new(factory);
        value.replace(Some(boxed.clone()));
        model.set_content(Some(boxed));
        assert!(model.build(None).is_none());
        value.replace(None);
        model.dispose();
    });
}
#[test]
fn managed_dock_mutable_fields_notify_original_names_and_focus_callback_runs_once() {
    let dock = ManagedWindowDock::new();
    let factory = Factory::new();
    dock.set_factory(Some(factory.clone()));
    let focused = Document::new();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let subscription =
        factory.focused_dockable_changed(Rc::new(move |_| count.set(count.get() + 1)));
    let names = events(&*dock);
    dock.set_default_dockable(Some(focused.clone()));
    dock.set_focused_dockable(Some(focused.clone()));
    dock.set_focused_dockable(Some(focused.clone()));
    dock.set_is_active(true);
    dock.set_can_close_last_dockable(false);
    dock.set_enable_global_docking(false);
    let policy = Rc::new(DockCapabilityPolicy::default());
    dock.set_dock_capability_policy(Some(policy.clone()));
    assert_eq!(calls.get(), 1);
    assert_eq!(
        &*names.borrow(),
        &[
            "DefaultDockable",
            "FocusedDockable",
            "IsActive",
            "CanCloseLastDockable",
            "EnableGlobalDocking",
            "DockCapabilityPolicy"
        ]
    );
    let expected: Rc<dyn DockCapabilityPolicyImpl> = policy;
    assert!(Rc::ptr_eq(
        &dock.dock_capability_policy().unwrap(),
        &expected
    ));
    assert!(dock.default_dockable().is_some());
    assert!(dock.focused_dockable().is_some());
    assert!(dock.is_active());
    assert!(!dock.can_close_last_dockable());
    assert!(!dock.enable_global_docking());
    dock.set_focused_dockable(None);
    assert_eq!(calls.get(), 2);
    dock.set_dock_capability_policy(None);
    assert!(dock.dock_capability_policy().is_none());
    subscription.dispose();
}
#[test]
fn canonical_interface_casts_recover_same_original_model_and_reject_unrelated_documents() {
    run(|| {
        use super::super::ManagedDockableBaseReference;
        use ferroui_base::data::core::ValueTypes;
        let (window, _) = window();
        let model = ManagedDockWindowDocument::new(window);
        let dockable: Rc<dyn IDockable> = model.clone();
        let mdi: Rc<dyn IMdiDocument> = model.clone();
        for value in [Rc::new(dockable) as BoxedValue, Rc::new(mdi) as BoxedValue] {
            let cast =
                ValueTypes::try_cast(&value, ValueType::of::<Rc<ManagedDockWindowDocument>>())
                    .unwrap();
            assert!(Rc::ptr_eq(
                cast.downcast_ref::<Rc<ManagedDockWindowDocument>>()
                    .unwrap(),
                &model
            ));
            let cast =
                ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRecyclingDataTemplate>>())
                    .unwrap();
            let template = cast
                .downcast_ref::<Rc<dyn IRecyclingDataTemplate>>()
                .unwrap();
            assert!(std::ptr::eq(
                template
                    .as_any()
                    .unwrap()
                    .downcast_ref::<ManagedDockWindowDocument>()
                    .unwrap(),
                model.as_ref()
            ));
            let cast = ValueTypes::try_cast(
                &value,
                ValueType::of::<Rc<dyn IDeferredContentPresentation>>(),
            )
            .unwrap();
            assert!(!cast
                .downcast_ref::<Rc<dyn IDeferredContentPresentation>>()
                .unwrap()
                .defer_content_presentation());
            let cast =
                ValueTypes::try_cast(&value, ValueType::of::<ManagedDockableBaseReference>())
                    .unwrap();
            assert!(std::ptr::eq(
                cast.downcast_ref::<ManagedDockableBaseReference>()
                    .unwrap()
                    .model()
                    .property_changed(),
                model.property_changed()
            ));
        }
        model.set_data_type(Some(ValueType::of::<
            Rc<dyn ferrodock_model::controls::IRootDock>,
        >()));
        let root: Rc<dyn ferrodock_model::controls::IRootDock> =
            ferrodock_model_reactiveui::controls::RootDock::new();
        assert!(model.match_(Some(&crate::model_projection::to_root_value(root.clone()))));
        assert!(
            !model.match_(Some(&crate::model_projection::to_dockable_value(
                Document::new()
            )))
        );
        model.set_data_type(Some(ValueType::of::<
            Rc<dyn ferrodock_model::controls::IDocumentDock>,
        >()));
        assert!(!model.match_(Some(&crate::model_projection::to_root_value(root))));
        model.set_data_type(Some(ValueType::of::<Rc<ManagedDockWindowDocument>>()));
        let original: Rc<dyn IDockable> = model.clone();
        assert!(model.match_(Some(&crate::model_projection::to_dockable_value(original))));
        let unrelated = Document::new();
        let ordinary: Rc<dyn IDockable> = unrelated;
        let value: BoxedValue = Rc::new(ordinary);
        assert!(!model.match_(Some(&value)));
        assert!(
            ValueTypes::try_cast(&value, ValueType::of::<Rc<ManagedDockWindowDocument>>())
                .is_none()
        );
        assert!(
            ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRecyclingDataTemplate>>())
                .is_none()
        );
        let dock = ManagedWindowDock::new();
        let source: Rc<dyn IDock> = dock.clone();
        let value: BoxedValue = Rc::new(source);
        let cast = ValueTypes::try_cast(&value, ValueType::of::<Rc<ManagedWindowDock>>()).unwrap();
        assert!(Rc::ptr_eq(
            cast.downcast_ref::<Rc<ManagedWindowDock>>().unwrap(),
            &dock
        ));
        model.dispose();
    });
}
// An application-owned model proves canonical casts preserve the source extension point.
struct ApplicationTemplateModel {
    base: ManagedDockableBase,
    calls: Cell<usize>,
}
crate::impl_managed_dockable!(
    ApplicationTemplateModel,
    fn boxed_interface(self: Rc<Self>, target: TypeId) -> Option<ModelValue> {
        if target == TypeId::of::<Rc<dyn IRecyclingDataTemplate>>() {
            let contract: Rc<dyn IRecyclingDataTemplate> = self;
            return Some(Rc::new(contract));
        }
        if target == TypeId::of::<Rc<dyn IDataTemplate>>() {
            let contract: Rc<dyn IDataTemplate> = self;
            return Some(Rc::new(contract));
        }
        None
    }
);
impl ManagedDockableBaseImpl for ApplicationTemplateModel {}
impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ApplicationTemplateModel {
    fn build(&self, _: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(None, None)
    }
}
impl IDataTemplate for ApplicationTemplateModel {
    fn match_(&self, _: Option<&BoxedValue>) -> bool {
        true
    }
    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}
impl IRecyclingDataTemplate for ApplicationTemplateModel {
    fn build_with_existing(
        &self,
        _: Option<&BoxedValue>,
        existing: Option<Ref<Control>>,
    ) -> Option<Ref<Control>> {
        self.calls.set(self.calls.get() + 1);
        existing.or_else(|| Some(Button::new().upcast()))
    }
}
#[test]
fn application_owned_recycling_template_works_through_original_dockable_interface() {
    run(|| {
        let model = Rc::new_cyclic(|runtime| ApplicationTemplateModel {
            base: ManagedDockableBase::construct(runtime.clone()),
            calls: Cell::new(0),
        });
        let contract: Rc<dyn IDockable> = model.clone();
        let value = crate::model_projection::to_dockable_value(contract);
        let template = ferrodock_controls_recycling::ControlRecyclingDataTemplate::new();
        let control = template.build(Some(&value)).unwrap();
        assert!(control.is::<Button>());
        let recycled = template
            .build_with_existing(Some(&value), Some(control.clone()))
            .unwrap();
        assert_eq!(control, recycled);
        assert_eq!(model.calls.get(), 2);
    });
}

#[test]
fn original_managed_document_local_target_query_preserves_document_identity() {
    run(|| {
        let (window, _root) = window();
        let document = ManagedDockWindowDocument::new(window);
        let model: Rc<dyn IDockable> = document.clone();
        let target = model.clone().as_local_target().unwrap();
        assert_eq!(
            Rc::as_ptr(&model) as *const (),
            Rc::as_ptr(&target) as *const ()
        );
        assert!(model.clone().as_global_target().is_none());
        assert_eq!(model.runtime_type_name(), "ManagedDockWindowDocument");
        document.dispose();
    });
}
