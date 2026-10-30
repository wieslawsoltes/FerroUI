//! Original complete inspector state and native source window lifecycle.
use crate::{
    controls::{RootDockDebug, RootDockDebugExtensions},
    view_models::{IEventLogClock, RootDockDebugViewModel},
};
use ferrodock_model::{collections::ListHandle, core::*};
use ferrodock_model_reactiveui::{
    controls::{RootDock, Tool},
    core::DockWindow,
    Factory,
};
use ferroui_base::{
    controls::NameScopeExtensions,
    data::core::ValueTypes,
    input::{
        InputElement, Key, KeyEventArgs, KeyGesture, KeyModifiers, PhysicalKey, RawInputModifiers,
    },
    threading::{CancellationToken, Dispatcher},
    BoxedValue, Ref,
};
use ferroui_controls::{
    application_lifetimes::{ClassicDesktopStyleApplicationLifetime, ISetupApplicationLifetime},
    AppBuilder, Application, Button, Control, ListBox, ShutdownMode, TabControl, TreeView, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct FixedClock;
impl IEventLogClock for FixedClock {
    fn local_timestamp(&self) -> String {
        "12:34:56.789".into()
    }
}
fn build() -> AppBuilder {
    use ferroui_fonts_inter::AppBuilderExtension;
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with_inter_font()
        .after_setup(|_| {
            let app = Application::current().unwrap();
            app.styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style());
            app.styles()
                .add(ferrodock_ferro_themes_fluent::DockFluentTheme::new().as_style());
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "complete-original-root-inspector",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn drain(window: &Window) {
    for _ in 0..4 {
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
    }
}
fn desktop() -> Rc<ClassicDesktopStyleApplicationLifetime> {
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    lifetime.set_shutdown_mode(ShutdownMode::OnExplicitShutdown);
    lifetime.before_app_init();
    lifetime
}
fn key(window: &Window, key: Key, modifiers: KeyModifiers) {
    let mut event = KeyEventArgs::new();
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.key = key;
    event.key_modifiers = modifiers;
    window.raise_event(&event);
    drain(window);
}
fn context(model: Rc<dyn IDockable>) -> BoxedValue {
    Rc::new(model)
}
fn show(view: &Ref<RootDockDebug>, model: Rc<dyn IDockable>) -> Ref<Window> {
    view.set_data_context(Some(context(model)));
    let window = Window::new();
    window.set_width(500.0);
    window.set_height(700.0);
    window.set_content(Some(Control::boxed(view.clone())));
    window.show();
    drain(&window);
    window
}
#[test]
fn original_factory_events_have_exact_names_titles_operations_and_local_timestamps() {
    let model = RootDockDebugViewModel::new(Rc::new(FixedClock));
    let factory = Factory::new();
    model.set_factory(Some(factory.clone()));
    let tool = Tool::new();
    tool.set_title("Tool".into());
    let tool: Rc<dyn IDockable> = tool;
    let window = DockWindow::new();
    window.set_title("Window".into());
    let window: Rc<dyn IDockWindow> = window;
    factory.on_active_dockable_changed(Some(tool.clone()));
    factory.on_focused_dockable_changed(Some(tool.clone()));
    factory.on_dockable_init(Some(tool.clone()));
    factory.on_dockable_added(Some(tool.clone()));
    factory.on_dockable_removed(Some(tool.clone()));
    factory.on_dockable_closed(Some(tool.clone()));
    factory.on_dockable_moved(Some(tool.clone()));
    factory.on_dockable_docked(Some(tool.clone()), DockOperation::Right);
    factory.on_dockable_undocked(Some(tool.clone()), DockOperation::Left);
    factory.on_dockable_swapped(Some(tool.clone()));
    factory.on_dockable_pinned(Some(tool.clone()));
    factory.on_dockable_unpinned(Some(tool.clone()));
    factory.on_dockable_hidden(Some(tool.clone()));
    factory.on_dockable_restored(Some(tool.clone()));
    factory.on_window_opened(Some(window.clone()));
    factory.on_window_closing(Some(window.clone()));
    factory.on_window_closed(Some(window.clone()));
    factory.on_window_added(Some(window.clone()));
    factory.on_window_removed(Some(window.clone()));
    factory.on_window_move_drag_begin(Some(window.clone()));
    factory.on_window_move_drag(Some(window.clone()));
    factory.on_window_move_drag_end(Some(window.clone()));
    factory.on_window_activated(Some(window.clone()));
    factory.on_dockable_activated(Some(tool.clone()));
    factory.on_window_deactivated(Some(window.clone()));
    factory.on_dockable_deactivated(Some(tool));
    let expected = [
        "ActiveDockableChanged Tool",
        "FocusedDockableChanged Tool",
        "DockableInit Tool",
        "DockableAdded Tool",
        "DockableRemoved Tool",
        "DockableClosed Tool",
        "DockableMoved Tool",
        "DockableDocked Tool Right",
        "DockableUndocked Tool Left",
        "DockableSwapped Tool",
        "DockablePinned Tool",
        "DockableUnpinned Tool",
        "DockableHidden Tool",
        "DockableRestored Tool",
        "WindowOpened Window",
        "WindowClosing Window",
        "WindowClosed Window",
        "WindowAdded Window",
        "WindowRemoved Window",
        "WindowMoveDragBegin Window",
        "WindowMoveDrag Window",
        "WindowMoveDragEnd Window",
        "WindowActivated Window",
        "DockableActivated Tool",
        "WindowDeactivated Window",
        "DockableDeactivated Tool",
    ];
    assert_eq!(
        model.events().to_vec(),
        expected.map(|message| format!("12:34:56.789 {message}"))
    );
    let source: Vec<_> = include_str!("../../../tests/Dock/data/inspector-view-grids.tsv")
        .lines()
        .filter_map(|line| line.strip_prefix("# log\t"))
        .collect();
    let actual: Vec<_> = model
        .events()
        .to_vec()
        .into_iter()
        .map(|line| line[13..].to_owned())
        .collect();
    assert_eq!(actual, source);
    factory.on_dockable_added(None);
    assert_eq!(model.events().get(26), "12:34:56.789 DockableAdded ");
    model.set_factory(None);
}
#[test]
fn source_filter_is_lazy_ordinal_and_notifies_in_original_order() {
    let model = RootDockDebugViewModel::new(Rc::new(FixedClock));
    let events = model.events();
    events.add("First Alpha".into());
    events.add("Second Βήτα".into());
    let all = model.filtered_events();
    assert!(all.is_unfiltered());
    assert!(all == model.filtered_events());
    let changes = Rc::new(RefCell::new(Vec::new()));
    let log = changes.clone();
    let token = model
        .property_changed()
        .add(Rc::new(move |name| log.borrow_mut().push(name.to_owned())));
    model.set_filter(Some("alpha".into()));
    assert_eq!(&*changes.borrow(), &["Filter", "FilteredEvents"]);
    let query = model.filtered_events();
    assert_eq!(query.count(), 1);
    assert_eq!(query.get(0), "First Alpha");
    model.set_filter(Some("ALPHA".into()));
    assert_eq!(query.count(), 1);
    events.add("Third ALPHA".into());
    assert_eq!(query.count(), 2);
    assert_eq!(all.count(), 3);
    model.set_filter(Some("βΉΤΑ".into()));
    assert_eq!(query.get(0), "Second Βήτα");
    let length = changes.borrow().len();
    model.set_filter(Some("βΉΤΑ".into()));
    assert_eq!(changes.borrow().len(), length);
    model.clear_events();
    assert!(model.events().ptr_eq(&events));
    assert_eq!(query.count(), 0);
    assert_eq!(changes.borrow().last().unwrap(), "FilteredEvents");
    model.property_changed().remove(token);
}
#[test]
fn replacing_context_factory_disposes_all_previous_subscriptions() {
    let model = RootDockDebugViewModel::new(Rc::new(FixedClock));
    let first = Factory::new();
    let second = Factory::new();
    model.set_factory(Some(first.clone()));
    first.on_dockable_added(None);
    model.set_factory(Some(second.clone()));
    first.on_dockable_added(None);
    second.on_dockable_added(None);
    assert_eq!(model.events().count(), 2);
    model.set_factory(Some(second.clone()));
    second.on_dockable_added(None);
    assert_eq!(model.events().count(), 3);
    model.set_factory(None);
    second.on_dockable_added(None);
    assert_eq!(model.events().count(), 3);
}
#[test]
fn complete_compiled_inspector_has_all_tabs_templates_tree_and_window_bindings() {
    run(|| {
        let factory = Factory::new();
        let root = RootDock::new();
        let nested = RootDock::new();
        nested.set_title("Nested".into());
        let tool = Tool::new();
        tool.set_title("Actual Tool".into());
        nested.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            tool.clone() as Rc<dyn IDockable>
        )])));
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            nested.clone() as Rc<dyn IDockable>
        )])));
        factory.init_layout(root.clone());
        let floating = DockWindow::new();
        floating.set_title("Floating".into());
        floating.set_layout(Some(nested.clone()));
        nested.set_windows(Some(ListHandle::from_items(vec![Some(
            floating as Rc<dyn IDockWindow>,
        )])));
        let view = RootDockDebug::new();
        let window = show(&view, root);

        assert_eq!(view.data_templates().count(), 18);
        let tabs = view.find_descendant_of_type::<TabControl>(false).unwrap();
        assert_eq!(tabs.item_count(), 5);
        let tree = NameScopeExtensions::find::<TreeView>(&view, "Visible").unwrap();
        assert_eq!(tree.item_count(), 1);
        let container = tree.container_from_index(0).unwrap();
        let item = container.cast::<ferroui_controls::TreeViewItem>().unwrap();
        assert!(item.is_expanded());
        assert_eq!(item.item_count(), 1);
        // A checked IDockable -> IDock conversion must not make a Tool match IDock.
        tree.set_selected_item(Some(context(tool.clone())));
        tabs.set_selected_index(1);
        drain(&window);
        assert!(view
            .find_descendant_of_type::<crate::controls::DockableDebugView>(false)
            .is_some());
        assert!(view
            .find_descendant_of_type::<crate::controls::DockDebugView>(false)
            .is_none());
        tree.set_selected_item(Some(context(nested)));
        tabs.set_selected_index(2);
        drain(&window);
        let windows = NameScopeExtensions::find::<TreeView>(&view, "Windows").unwrap();
        assert_eq!(windows.item_count(), 1);
        assert_eq!(view.events().count(), 0);
        factory.on_dockable_added(Some(tool));
        assert_eq!(view.events().count(), 1);
        window.close();
        view.set_data_context(None);
    });
}
#[test]
fn source_events_tab_filters_and_clears_through_real_native_controls() {
    run(|| {
        let factory = Factory::new();
        let root = RootDock::new();
        root.set_factory(Some(factory.clone()));
        let view = RootDockDebug::new();
        let window = show(&view, root);

        let tabs = view.find_descendant_of_type::<TabControl>(false).unwrap();
        tabs.set_selected_index(3);
        drain(&window);
        let filter = view
            .find_descendant_of_type::<ferroui_controls::TextBox>(false)
            .unwrap();
        let list = view.find_descendant_of_type::<ListBox>(false).unwrap();
        factory.on_dockable_added(None);
        factory.on_dockable_removed(None);
        drain(&window);
        assert_eq!(list.item_count(), 2);
        filter.set_text(Some("removed"));
        drain(&window);
        assert_eq!(view.filter().as_deref(), Some("removed"));
        assert_eq!(list.item_count(), 1);
        factory.on_dockable_removed(None);
        drain(&window);
        assert_eq!(list.item_count(), 2);
        let reset = view.find_descendant_of_type::<Button>(false).unwrap();
        assert!(reset.focus());
        window.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::empty());
        window.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::empty());
        drain(&window);
        assert_eq!(view.events().count(), 0);
        assert_eq!(list.item_count(), 0);
        assert_eq!(view.filter().as_deref(), Some("removed"));
        window.close();
        view.set_data_context(None);
    });
}
#[test]
fn default_f11_resolves_current_layout_lazily_and_builds_the_actual_inspector() {
    run(|| {
        let lifetime = desktop();
        let main = Window::new();
        main.show();
        let current: Rc<RefCell<Option<BoxedValue>>> = Rc::new(RefCell::new(None));
        let count = Rc::new(Cell::new(0));
        let calls = count.clone();
        let provider = current.clone();
        let cleanup = RootDockDebugExtensions::attach_dock_debug(
            main.clone().upcast(),
            Rc::new(move || {
                calls.set(calls.get() + 1);
                provider.borrow().clone()
            }),
            None,
        );
        key(&main, Key::F10, KeyModifiers::empty());
        assert_eq!(count.get(), 0);
        key(&main, Key::F11, KeyModifiers::empty());
        assert_eq!(count.get(), 1);
        assert_eq!(lifetime.windows().len(), 1);
        let root = RootDock::new();
        current.replace(Some(context(root.clone())));
        key(&main, Key::F11, KeyModifiers::empty());
        assert_eq!(count.get(), 2);
        let debug = lifetime
            .windows()
            .into_iter()
            .find(|window| !window.ptr_eq(&main))
            .unwrap();
        assert_eq!(debug.width(), 400.0);
        assert_eq!(debug.height(), 680.0);
        let inspector = ValueTypes::as_object(&*debug.content().unwrap())
            .unwrap()
            .cast::<RootDockDebug>()
            .unwrap();
        assert!(Rc::ptr_eq(
            inspector
                .data_context()
                .unwrap()
                .downcast_ref::<Rc<dyn IDockable>>()
                .unwrap(),
            &(root as Rc<dyn IDockable>)
        ));
        key(&main, Key::F11, KeyModifiers::empty());
        assert!(!debug.is_visible());
        assert_eq!(count.get(), 2);
        cleanup.dispose();
        cleanup.dispose();
        key(&main, Key::F11, KeyModifiers::empty());
        assert_eq!(count.get(), 2);
        main.close();
    });
}
#[test]
fn custom_factory_gesture_user_close_and_main_closing_follow_source_lifecycle() {
    run(|| {
        let _lifetime = desktop();
        let main = Window::new();
        main.show();
        let created = Rc::new(RefCell::new(Vec::new()));
        let output = created.clone();
        let owner = main.clone();
        let cleanup = RootDockDebugExtensions::attach_dock_debug_with_window_factory(
            main.clone().upcast(),
            Rc::new(|| Some(context(RootDock::new()))),
            Some(Rc::new(move |top| {
                assert!(top.ptr_eq(&owner));
                let window = Window::new();
                window.set_width(512.0);
                output.borrow_mut().push(window.clone());
                window
            })),
            Some(KeyGesture::new(Key::F8, KeyModifiers::CONTROL)),
        );
        key(&main, Key::F11, KeyModifiers::empty());
        key(&main, Key::F8, KeyModifiers::empty());
        assert!(created.borrow().is_empty());
        key(&main, Key::F8, KeyModifiers::CONTROL);
        let first = created.borrow()[0].clone();
        assert_eq!(first.width(), 512.0);
        first.close();
        key(&main, Key::F8, KeyModifiers::CONTROL);
        let second = created.borrow()[1].clone();
        assert!(second.is_visible());
        main.close();
        assert!(!second.is_visible());
        cleanup.dispose();
    });
}

#[test]
fn selected_tree_item_uses_actual_factory_controls_and_detach_removes_highlight() {
    run(|| {
        use ferrodock_ferro::{controls::DockableControl, diagnostics::DockDiagnosticEvents};
        use ferroui_controls::{
            primitives::{AdornerLayer, VisualLayerManager},
            Canvas,
        };
        let factory = Factory::new();
        let model = Tool::new();
        model.set_factory(Some(factory.clone()));
        let dockable: Rc<dyn IDockable> = model;
        let root = Canvas::new();
        let control = DockableControl::new();
        control.set_width(100.0);
        control.set_height(40.0);
        root.children().add(control.clone());
        let layers = VisualLayerManager::new();
        layers.set_child(Some(root.clone().upcast::<Control>()));
        let main = Window::new();
        main.set_width(400.0);
        main.set_height(300.0);
        main.set_content(Some(Control::boxed(layers)));
        main.show();
        drain(&main);
        factory.visible_dockable_controls().borrow_mut().insert(
            dockable.clone(),
            DockableControl::as_dockable_control(control.clone()),
        );
        factory
            .visible_root_controls()
            .borrow_mut()
            .insert(dockable.clone(), Rc::new(root.clone()));
        let inspector = RootDockDebug::new();
        let root_model = RootDock::new();
        root_model.set_factory(Some(factory.clone()));
        root_model
            .set_visible_dockables(Some(ListHandle::from_items(vec![Some(dockable.clone())])));
        let before = DockDiagnosticEvents::select_dockable_requested().handler_count();
        let window = show(&inspector, root_model);
        let tree = NameScopeExtensions::find::<TreeView>(&inspector, "Visible").unwrap();
        assert_eq!(
            DockDiagnosticEvents::select_dockable_requested().handler_count(),
            before + 1
        );
        DockDiagnosticEvents::select_dockable_requested().raise(&Some(dockable.clone()));
        drain(&window);
        let selected = tree.selected_item().unwrap();
        assert!(Rc::ptr_eq(
            selected.downcast_ref::<Rc<dyn IDockable>>().unwrap(),
            &dockable
        ));
        let layer = AdornerLayer::get_adorner_layer(&root).unwrap();
        let adorner = layer
            .children()
            .snapshot()
            .iter()
            .find_map(|child| child.cast::<crate::controls::SelectionOverlayAdorner>())
            .unwrap();
        assert!(adorner.highlight_rect().is_some());
        tree.set_selected_item(None);
        assert_eq!(adorner.highlight_rect(), None);
        tree.set_selected_item(Some(context(dockable)));
        assert!(adorner.highlight_rect().is_some());
        window.set_content(None);
        drain(&window);
        assert_eq!(
            DockDiagnosticEvents::select_dockable_requested().handler_count(),
            before
        );
        assert!(!layer
            .children()
            .snapshot()
            .iter()
            .any(|child| child.is::<crate::controls::SelectionOverlayAdorner>()));
        let count = inspector.events().count();
        factory.on_dockable_added(None);
        assert_eq!(
            inspector.events().count(),
            count + 1,
            "source logging remains subscribed after visual detach"
        );
        inspector.set_data_context(None);
        factory.on_dockable_added(None);
        assert_eq!(inspector.events().count(), count + 1);
        window.close();
        main.close();
    });
}
#[test]
fn f11_cancelled_close_and_explicit_disposal_keep_source_distinctions() {
    run(|| {
        let lifetime = desktop();
        let main = Window::new();
        main.show();
        let cleanup = RootDockDebugExtensions::attach_dock_debug(
            main.clone().upcast(),
            Rc::new(|| Some(context(RootDock::new()))),
            None,
        );
        key(&main, Key::F11, KeyModifiers::empty());
        let debug = lifetime
            .windows()
            .into_iter()
            .find(|window| !window.ptr_eq(&main))
            .unwrap();
        let cancel = debug.closing(|event| event.set_cancel(true));
        key(&main, Key::F11, KeyModifiers::empty());
        assert!(debug.is_visible());
        assert_eq!(lifetime.windows().len(), 2);
        cleanup.dispose();
        assert!(
            debug.is_visible(),
            "source disposal clears its reference even if close is cancelled"
        );
        key(&main, Key::F11, KeyModifiers::empty());
        assert_eq!(lifetime.windows().len(), 2);
        cancel.dispose();
        debug.close();
        main.close();
    });
}
#[test]
fn title_converter_uses_original_titles_concrete_names_unset_and_shared_instance() {
    run(|| {
        use ferrodock_ferro::converters::TitleOrTypeNameConverter;
        use ferroui_base::{
            data::{converters::IValueConverter, core::ValueType},
            utilities::CultureInfo,
            FerroProperty,
        };
        let converter = TitleOrTypeNameConverter::instance();
        assert!(Rc::ptr_eq(
            &converter,
            &TitleOrTypeNameConverter::instance()
        ));
        let tool = Tool::new();
        let original = context(tool.clone());
        assert!(ValueTypes::try_cast(&original, ValueType::of::<Rc<dyn ILocalTarget>>()).is_some());
        assert!(ValueTypes::try_cast(
            &original,
            ValueType::of::<Rc<dyn ferrodock_model::controls::ITool>>()
        )
        .is_some());
        assert!(
            ValueTypes::try_cast(
                &original,
                ValueType::of::<Rc<dyn ferrodock_model::controls::IDocument>>()
            )
            .is_some(),
            "the source Tool also implements IDocument"
        );
        assert!(ValueTypes::try_cast(
            &original,
            ValueType::of::<Rc<dyn ferrodock_model::controls::IGridDockSplitter>>()
        )
        .is_none());
        let window = DockWindow::new();
        window.set_title(String::new());
        for (value, expected) in [
            (context(tool.clone()), "Tool"),
            (
                Rc::new(window.clone() as Rc<dyn IDockWindow>) as BoxedValue,
                "DockWindow",
            ),
            (Rc::new("value".to_owned()) as BoxedValue, "String"),
            (Rc::new(13_i32) as BoxedValue, "Int32"),
            (Rc::new(true) as BoxedValue, "Boolean"),
        ] {
            let result = converter
                .convert(
                    Some(&value),
                    ValueType::of::<String>(),
                    None,
                    &CultureInfo::invariant_culture(),
                )
                .unwrap()
                .unwrap();
            assert_eq!(result.downcast_ref::<String>().unwrap(), expected);
        }
        tool.set_title(" ".into());
        let value = context(tool);
        assert_eq!(
            converter
                .convert(
                    Some(&value),
                    ValueType::of::<String>(),
                    None,
                    &CultureInfo::invariant_culture()
                )
                .unwrap()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some(" ")
        );
        assert!(converter
            .convert(
                None,
                ValueType::of::<String>(),
                None,
                &CultureInfo::invariant_culture()
            )
            .unwrap()
            .unwrap()
            .any_value_eq(&*FerroProperty::unset_value()));
        assert!(converter
            .convert_back(
                None,
                ValueType::of::<String>(),
                None,
                &CultureInfo::invariant_culture()
            )
            .is_err());
    });
}

#[test]
fn all_original_extension_overloads_export_and_delegate_factory_null_uses_default_window() {
    run(|| {
        use ferroui_base::metadata::{MarkupDelegate, MarkupTyped};
        let metadata = RootDockDebugExtensions::MARKUP;
        let methods: Vec<_> = metadata.find_methods("AttachDockDebug").collect();
        assert_eq!(
            methods
                .iter()
                .map(|method| method.parameters.len())
                .collect::<Vec<_>>(),
            [3, 4, 4]
        );
        assert_eq!(
            RootDockDebugExtensions::TYPE.full_name(),
            "FerroDock.Ferro.Diagnostics.Controls.RootDockDebugExtensions"
        );
        let lifetime = desktop();
        let main = Window::new();
        main.show();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        let provider = Rc::new(MarkupDelegate::new(|_| Some(context(RootDock::new()))));
        let factory = Rc::new(MarkupDelegate::new(move |args| {
            assert_eq!(args.len(), 1);
            called.set(called.get() + 1);
            None
        }));
        let result = (methods[1].invoke)(&[
            Some(Rc::new(main.clone().upcast::<ferroui_controls::TopLevel>())),
            Some(Rc::new(provider)),
            None,
            Some(Rc::new(Some(factory))),
        ])
        .unwrap()
        .unwrap();
        let cleanup = result
            .downcast_ref::<Rc<dyn ferroui_system::IDisposable>>()
            .unwrap();
        key(&main, Key::F11, KeyModifiers::empty());
        assert_eq!(calls.get(), 1);
        let debug = lifetime
            .windows()
            .into_iter()
            .find(|window| !window.ptr_eq(&main))
            .unwrap();
        assert_eq!(debug.width(), 400.0);
        assert!(ValueTypes::as_object(&*debug.content().unwrap())
            .unwrap()
            .is::<RootDockDebug>());
        cleanup.dispose();
        assert!(!debug.is_visible());
        main.close();
    });
}
