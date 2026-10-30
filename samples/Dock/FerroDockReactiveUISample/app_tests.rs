//! Actual source application resources, passive theme controls and lifetime branches.
#[path = "native_file_workflow_tests.rs"]
mod native_file_workflow_tests;
use crate::{view_models::MainWindowViewModel, App};
use ferrodock_ferro::controls::DockControl;
use ferroui_base::{
    styling::ThemeVariant,
    threading::{CancellationToken, Dispatcher},
    BoxedValue, Ref,
};
use ferroui_controls::{
    application_lifetimes::{
        ClassicDesktopStyleApplicationLifetime, IApplicationLifetime,
        ISingleViewApplicationLifetime,
    },
    AppBuilder, Application, Button, ComboBox, Control, ShutdownMode, Window,
};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_reactiveui_controls::AppBuilderExtensions;
use std::{any::Any, cell::RefCell, rc::Rc};
fn build() -> AppBuilder {
    AppBuilder::configure::<App>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with_inter_font()
        .use_reactive_ui(|_| {})
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-native-app",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    run_with(&ASSEMBLY, test);
}
fn run_with(assembly: &'static FerroTestAssembly, test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(assembly))
        .dispatch(
            move || {
                let manager = App::theme_manager().unwrap();
                manager.switch(0);
                manager.switch_preset(0);
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
#[test]
fn native_file_storage_uses_weak_hosts_and_reports_missing_or_unsupported_pickers() {
    run(|| {
        use crate::{infrastructure::NativeLayoutFileStorage, services::ILayoutFileStorage};
        use ferroui_system::threading::tasks::Task;
        let orphan = Control::new();
        let service = NativeLayoutFileStorage::new(orphan.clone());
        let task = Task::spawn_local(async move {
            Ok::<_, ferroui_system::ObservableError>(
                service.open_layout().await.err().unwrap().to_string(),
            )
        });
        assert!(task.result().unwrap().contains("No window"));
        drop(orphan);
        let service = NativeLayoutFileStorage::new(Control::new());
        let task = Task::spawn_local(async move {
            Ok::<_, ferroui_system::ObservableError>(
                service
                    .save_layout(Vec::new())
                    .await
                    .err()
                    .unwrap()
                    .to_string(),
            )
        });
        assert!(task.result().unwrap().contains("closed"));
        let window = Window::new();
        let child = Control::new();
        window.set_content(Some(Control::boxed(child.clone())));
        window.show();
        drain(&window);
        let service = NativeLayoutFileStorage::new(child);
        let task = Task::spawn_local(async move {
            let open = service.open_layout().await.err().unwrap().to_string();
            let save = service
                .save_layout(Vec::new())
                .await
                .err()
                .unwrap()
                .to_string();
            Ok::<_, ferroui_system::ObservableError>((open, save))
        });
        let (open, save) = task.result().unwrap();
        assert!(open.contains("Opening files is unavailable"));
        assert!(save.contains("Saving files is unavailable"));
        window.close();
    });
}
#[test]
fn compiled_file_menu_commands_restore_native_layout_and_update_status_under_both_themes() {
    run(|| {
        use crate::layout_file_tests::TestLayoutFiles;
        use ferroui_controls::{MenuItem, TextBlock};
        assert!(Application::current().unwrap().cast::<App>().is_some());
        for theme in [0, 1] {
            let theme_model = App::create_theme_view_model(App::theme_manager().unwrap());
            theme_model.select_tab_theme().execute(Some(&theme));
            let model = App::create_main_view_model();
            let storage = TestLayoutFiles::new();
            model.attach_layout_files(storage.clone());
            let window = crate::views::MainWindow::new();
            window.set_data_context(Some(Rc::new(model.clone())));
            window.show();
            drain(&window);
            let menu = |name: &str| {
                window
                    .get_logical_descendants()
                    .filter_map(|v| v.cast::<MenuItem>())
                    .find(|v| v.name().as_deref() == Some(name))
                    .unwrap()
            };
            let save = menu("SaveLayoutMenuItem");
            let open = menu("OpenLayoutMenuItem");
            assert!(save.command().unwrap().can_execute(None));
            save.command().unwrap().execute(None);
            drain(&window);
            let bytes = storage.written.borrow()[0].clone();
            assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
            let original = model.layout().unwrap();
            storage.open_bytes(bytes);
            open.command().unwrap().execute(None);
            drain(&window);
            assert!(!Rc::ptr_eq(&original, &model.layout().unwrap()));
            let status = window
                .find_descendant_of_type_where::<TextBlock>(false, |v| {
                    v.name().as_deref() == Some("LayoutFileStatus")
                })
                .unwrap();
            assert_eq!(status.text().as_deref(), Some("Loaded layout.json"));
            let dock = window
                .find_descendant_of_type::<DockControl>(false)
                .unwrap();
            assert!(Rc::ptr_eq(
                &dock.layout().unwrap(),
                &(model.layout().unwrap() as Rc<dyn ferrodock_model::core::IDock>)
            ));
            assert!(window
                .find_descendant_of_type::<crate::views::documents::DocumentView>(false)
                .is_some());
            window.close();
        }
    });
}
#[test]
fn actual_app_resources_and_passive_main_view_switch_all_five_original_presets() {
    run(|| {
        let app = Application::current().unwrap();
        assert!(app.cast::<App>().is_some());
        assert_eq!(app.name().as_deref(), Some("FerroDock Demo"));
        let manager = App::theme_manager().unwrap();
        assert_eq!(manager.current_preset_index(), 0);
        let model = MainWindowViewModel::new();
        let window = crate::views::MainWindow::new();
        window.set_data_context(Some(Rc::new(model.clone())));
        window.show();
        drain(&window);
        let documents = window
            .find_descendant_of_type::<ferrodock_ferro::controls::DocumentControl>(false)
            .unwrap();
        let strip = documents
            .find_descendant_of_type::<ferrodock_ferro::controls::DocumentTabStrip>(false)
            .unwrap();
        let icon = strip
            .find_descendant_of_type_where::<ferroui_controls::presenters::ContentPresenter>(
                false,
                |p| p.name().as_deref() == Some("PART_IconPresenter"),
            )
            .unwrap();
        let boxed_icon = strip.icon_template().unwrap();
        let typed_icon = ferroui_base::data::core::ValueTypes::try_cast(
            &boxed_icon,
            ferroui_base::data::core::ValueType::of::<
                Option<Rc<dyn ferroui_controls::templates::IDataTemplate>>,
            >(),
        );
        assert!(
            typed_icon.is_some(),
            "the native cast of the object-valued template must succeed"
        );
        assert!(
            icon.content_template().is_some(),
            "source icon template must reach the presenter's typed property; source value type: {}",
            ferroui_base::data::core::ValueType::of_value(&*strip.icon_template().unwrap()).name()
        );
        assert!(
            icon.child()
                .unwrap()
                .cast::<ferroui_controls::Panel>()
                .is_some(),
            "the source empty icon template must not fall back to the application's document view"
        );
        let tool_strip = window
            .find_descendant_of_type::<ferrodock_ferro::controls::ToolTabStrip>(false)
            .unwrap();
        let tool_icon = tool_strip
            .find_descendant_of_type_where::<ferroui_controls::presenters::ContentPresenter>(
                false,
                |p| p.name().as_deref() == Some("PART_IconPresenter"),
            )
            .unwrap();
        assert!(tool_icon.content_template().is_some());
        assert!(tool_icon
            .child()
            .unwrap()
            .cast::<ferroui_controls::Panel>()
            .is_some());
        assert!(strip
            .find_descendant_of_type::<crate::views::documents::DocumentView>(false)
            .is_none());
        assert!(tool_strip
            .find_descendant_of_type::<crate::views::tools::Tool1View>(false)
            .is_none());
        let state = documents
            .find_descendant_of_type_where::<ComboBox>(false, |control| {
                control.items_view().count() == 11
            })
            .unwrap();
        let value = state.selected_item().unwrap();
        let display = ferroui_base::data::core::ValueTypes::to_display_string(Some(&value));
        assert_eq!(
            display,
            value
                .downcast_ref::<ferrodock_model::core::DockingWindowState>()
                .unwrap()
                .to_string()
        );
        assert!(!display.contains("::"));
        let button = window
            .find_descendant_of_type_where::<Button>(false, |control| {
                control.name().as_deref() == Some("ThemeButton")
            })
            .unwrap();
        let command = button
            .command()
            .expect("source theme action routed through ReactiveCommand");
        command.execute(None);
        drain(&window);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::dark()));
        button.command().unwrap().execute(None);
        drain(&window);
        assert_eq!(app.requested_theme_variant(), Some(ThemeVariant::light()));
        let preset = window
            .find_descendant_of_type_where::<ComboBox>(false, |control| {
                control.name().as_deref() == Some("PresetComboBox")
            })
            .unwrap();
        assert_eq!(preset.items_view().count(), 5);
        assert_eq!(preset.selected_index(), 0);
        for index in [1, 2, 3, 4, 0] {
            preset.set_selected_index(index);
            drain(&window);
            assert_eq!(manager.current_preset_index(), index);
        }
        let dock = window
            .find_descendant_of_type::<DockControl>(false)
            .unwrap();
        assert!(Rc::ptr_eq(
            &dock.layout().unwrap(),
            &(model.layout().unwrap() as Rc<dyn ferrodock_model::core::IDock>)
        ));
        assert!(window
            .find_descendant_of_type::<crate::views::documents::DocumentView>(false)
            .is_some());
        window.close();
    });
}

#[test]
fn sample_tab_selector_switches_live_browser_and_fluent_without_replacing_layout() {
    run(|| {
        use ferrodock_ferro::controls::{
            DelayedUniformTabPanel, DocumentTabStrip, DocumentTabStripItem,
        };
        use ferrodock_model::core::IFactory;
        use ferroui_base::{
            input::{MouseButton, RawInputModifiers},
            Point,
        };
        use ferroui_headless::HeadlessWindowExtensions;
        let app = Application::current().unwrap();
        let baseline_styles = app.styles().count();
        let model = MainWindowViewModel::new();
        let root = model.layout().unwrap();
        let window = crate::views::MainWindow::new();
        window.set_data_context(Some(Rc::new(model.clone())));
        window.show();
        drain(&window);
        let selector = window
            .find_descendant_of_type_where::<ComboBox>(false, |c| {
                c.name().as_deref() == Some("TabThemeComboBox")
            })
            .unwrap();
        assert_eq!(selector.items_view().count(), 2);
        assert_eq!(selector.selected_index(), 0);
        let original_layout = window
            .find_descendant_of_type::<DockControl>(false)
            .unwrap()
            .layout()
            .unwrap();
        for index in [1, 0, 1, 0] {
            selector.set_selected_index(index);
            let preset = window
                .find_descendant_of_type_where::<ComboBox>(false, |c| {
                    c.name().as_deref() == Some("PresetComboBox")
                })
                .unwrap();
            preset.set_selected_index(if index == 1 { 2 } else { 0 });
            if index == 1 {
                window
                    .find_descendant_of_type_where::<Button>(false, |c| {
                        c.name().as_deref() == Some("ThemeButton")
                    })
                    .unwrap()
                    .command()
                    .unwrap()
                    .execute(None);
            }
            drain(&window);
            assert_eq!(
                App::theme_manager().unwrap().current_preset_index(),
                if index == 1 { 2 } else { 0 }
            );
            assert_eq!(
                app.styles().count(),
                baseline_styles + usize::from(index == 1)
            );
            let dock = window
                .find_descendant_of_type::<DockControl>(false)
                .unwrap();
            assert!(Rc::ptr_eq(&dock.layout().unwrap(), &original_layout));
            assert!(Rc::ptr_eq(&model.layout().unwrap(), &root));
            let tabs = dock
                .find_descendant_of_type::<DocumentTabStrip>(false)
                .unwrap();
            assert_eq!(tabs.items().count(), 3);
            assert_eq!(
                tabs.find_descendant_of_type::<DelayedUniformTabPanel>(false)
                    .is_some(),
                index == 1
            );
            let item = tabs
                .container_from_index(1)
                .unwrap()
                .cast::<DocumentTabStripItem>()
                .unwrap();
            let point = item
                .translate_point(
                    Point::new(item.bounds().width / 2.0, item.bounds().height / 2.0),
                    &window,
                )
                .unwrap();
            window.mouse_move(point, RawInputModifiers::NONE);
            window.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
            window.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
            drain(&window);
            assert_eq!(tabs.selected_index(), 1);
            let factory: Rc<dyn IFactory> = root.factory().unwrap();
            let selected = factory
                .find_dockable(
                    Some(root.clone()),
                    Some(Rc::new(|v| v.is_some_and(|v| v.id() == "Document2"))),
                )
                .unwrap();
            assert!(Rc::ptr_eq(
                &selected
                    .owner()
                    .unwrap()
                    .as_dock()
                    .unwrap()
                    .active_dockable()
                    .unwrap(),
                &selected
            ));
        }
        window.close();
    });
}

#[test]
fn tab_theme_service_is_shared_across_main_views_and_repeated_commands_do_not_duplicate_styles() {
    run(|| {
        let app = Application::current().unwrap();
        let baseline = app.styles().count();
        let manager = App::theme_manager().unwrap();
        let first = App::create_theme_view_model(manager.clone());
        let second = App::create_theme_view_model(manager);
        for _ in 0..3 {
            first.select_tab_theme().execute(Some(&1_i32));
        }
        assert_eq!(app.styles().count(), baseline + 1);
        assert_eq!(second.selected_tab_theme(), 1);
        let view = crate::views::MainView::new();
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        drain(&window);
        let selector =
            ferroui_base::controls::NameScopeExtensions::get::<ComboBox>(&view, "TabThemeComboBox");
        assert_eq!(selector.selected_index(), 1);
        for index in [-1_i32, 2] {
            second.select_tab_theme().execute(Some(&index));
        }
        assert_eq!(app.styles().count(), baseline + 1);
        for _ in 0..3 {
            second.select_tab_theme().execute(Some(&0_i32));
        }
        assert_eq!(app.styles().count(), baseline);
        assert_eq!(first.selected_tab_theme(), 0);
        window.close();
    });
}

#[test]
fn injected_sample_json_restore_keeps_application_views_commands_and_both_tab_themes() {
    run(|| {
        use crate::view_models::{docks::CustomDocumentDock, documents::DocumentViewModel};
        use ferrodock_model::{controls::*, core::*};
        for style in [0, 1] {
            let model = App::create_main_view_model();
            assert!(model.can_persist_layout());
            let window = crate::views::MainWindow::new();
            window.set_data_context(Some(Rc::new(model.clone())));
            window.show();
            drain(&window);
            let selector = window
                .find_descendant_of_type_where::<ComboBox>(false, |c| {
                    c.name().as_deref() == Some("TabThemeComboBox")
                })
                .unwrap();
            selector.set_selected_index(style);
            drain(&window);
            let original = model.layout().unwrap();
            let wire = model.save_layout_json().unwrap();
            model.load_layout_json(&wire).unwrap();
            drain(&window);
            let loaded = model.layout().unwrap();
            assert!(!Rc::ptr_eq(&loaded, &original));
            let dock = window
                .find_descendant_of_type::<DockControl>(false)
                .unwrap();
            assert!(Rc::ptr_eq(
                &dock.layout().unwrap(),
                &(loaded.clone() as Rc<dyn IDock>)
            ));
            assert!(window
                .find_descendant_of_type::<crate::views::documents::DocumentView>(false)
                .is_some());
            assert!(window
                .find_descendant_of_type::<crate::views::tools::Tool1View>(false)
                .is_some());
            assert_eq!(selector.selected_index(), style);
            let factory = loaded.factory().unwrap();
            let documents = factory
                .find_dockable(
                    Some(loaded),
                    Some(Rc::new(|v| v.and_then(|v| v.as_document_dock()).is_some())),
                )
                .unwrap();
            let documents = documents
                .boxed_reference()
                .downcast_ref::<Rc<CustomDocumentDock>>()
                .unwrap()
                .clone();
            let first = documents.visible_dockables().unwrap().get(0).unwrap();
            assert!(first
                .clone()
                .boxed_reference()
                .is::<Rc<DocumentViewModel>>());
            assert!(first.context().is_some());
            documents.create_document().unwrap().execute(None);
            drain(&window);
            assert_eq!(documents.visible_dockables().unwrap().count(), 4);
            assert_eq!(documents.active_dockable().unwrap().id(), "Document4");
            assert!(model
                .save_layout_json()
                .unwrap()
                .contains("DockReactiveUISample.ViewModels.Documents.DocumentViewModel"));
            selector.set_selected_index(0);
            drain(&window);
            window.close();
        }
    });
}
#[test]
fn original_desktop_lifetime_constructs_main_window_and_closes_layout_on_both_source_events() {
    run_with(&DESKTOP_ASSEMBLY, || {
        let app = Application::current().unwrap();
        let lifetime_value = app.application_lifetime().unwrap();
        let lifetime = lifetime_value
            .as_any()
            .downcast_ref::<DesktopTestLifetime>()
            .unwrap();
        let window = lifetime.main_window().unwrap();
        assert!(window.cast::<crate::views::MainWindow>().is_some());
        let model = window
            .data_context()
            .unwrap()
            .downcast_ref::<Rc<MainWindowViewModel>>()
            .unwrap()
            .clone();
        assert!(model
            .layout_files()
            .unwrap()
            .save_layout()
            .can_execute(None));
        let attach_float = || {
            let floating = ferrodock_model_reactiveui::core::DockWindow::new();
            floating.set_host(Some(ferrodock_ferro::controls::ManagedHostWindow::new()));
            let windows: ferrodock_model::collections::ListHandle<
                Option<Rc<dyn ferrodock_model::core::IDockWindow>>,
            > = ferrodock_model::collections::ListHandle::from_items(vec![Some(floating.clone())]);
            model.layout().unwrap().set_windows(Some(windows));
            floating
        };
        let first = attach_float();
        window.show();
        drain(&window);
        window.close();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            first.host().is_none(),
            "source Closing executes the real layout Close/ExitWindows path"
        );
        let floating = attach_float();
        lifetime.raise_exit();
        assert!(floating.host().is_none(),"source Exit executes the real layout Close/ExitWindows path and releases its floating host");
    });
}
#[derive(Default)]
struct SingleLifetime {
    view: RefCell<Option<Ref<Control>>>,
}
impl IApplicationLifetime for SingleLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        Some(self)
    }
}
impl ISingleViewApplicationLifetime for SingleLifetime {
    fn main_view(&self) -> Option<Ref<Control>> {
        self.view.borrow().clone()
    }
    fn set_main_view(&self, value: Option<Ref<Control>>) {
        *self.view.borrow_mut() = value;
    }
}
#[test]
fn original_single_view_lifetime_receives_main_view_and_the_actual_main_model() {
    run_with(&SINGLE_ASSEMBLY, || {
        let app = Application::current().unwrap();
        let lifetime_value = app.application_lifetime().unwrap();
        let lifetime = lifetime_value
            .as_any()
            .downcast_ref::<SingleLifetime>()
            .unwrap();
        let view = lifetime.main_view().unwrap();
        assert!(view.cast::<crate::views::MainView>().is_some());
        let context: BoxedValue = view.data_context().unwrap();
        assert!(context.is::<Rc<MainWindowViewModel>>());
        assert!(context
            .downcast_ref::<Rc<MainWindowViewModel>>()
            .unwrap()
            .layout_files()
            .unwrap()
            .load_layout()
            .can_execute(None));
        let window = Window::new();
        window.set_content(Some(Control::boxed(view)));
        window.show();
        drain(&window);
        assert!(window
            .find_descendant_of_type::<DockControl>(false)
            .is_some());
        window.close();
    });
}

// These test applications install the desired contract before the ORIGINAL App
// service registration, Initialize and framework-completed hooks run. They do not change a live lifetime.
#[repr(C)]
struct DesktopTestApp {
    base: App,
}
ferroui_base::ferro_class!(DesktopTestApp:App);
ferroui_base::ferro_class_info!(DesktopTestApp {
    new: DesktopTestApp::new
});
ferroui_base::ferro_impl_classes!(DesktopTestApp:ferroui_base::FerroObjectImpl);
impl ferroui_controls::NewApplication for DesktopTestApp {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}
impl DesktopTestApp {
    fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self {
            base: App::construct(),
        })
    }
}
impl ferroui_controls::ApplicationImpl for DesktopTestApp {
    fn register_services(this: &Self) {
        let lifetime = DesktopTestLifetime::new();
        this.set_application_lifetime(Some(lifetime));
        <Self as ferroui_controls::ApplicationImplExt>::parent_register_services(this);
    }
}
#[repr(C)]
struct SingleTestApp {
    base: App,
}
ferroui_base::ferro_class!(SingleTestApp:App);
ferroui_base::ferro_class_info!(SingleTestApp {
    new: SingleTestApp::new
});
ferroui_base::ferro_impl_classes!(SingleTestApp:ferroui_base::FerroObjectImpl);
impl ferroui_controls::NewApplication for SingleTestApp {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}
impl SingleTestApp {
    fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self {
            base: App::construct(),
        })
    }
}
impl ferroui_controls::ApplicationImpl for SingleTestApp {
    fn register_services(this: &Self) {
        this.set_application_lifetime(Some(Rc::new(SingleLifetime::default())));
        <Self as ferroui_controls::ApplicationImplExt>::parent_register_services(this);
    }
}
fn build_desktop() -> AppBuilder {
    AppBuilder::configure::<DesktopTestApp>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with_inter_font()
        .use_reactive_ui(|_| {})
}
fn build_single() -> AppBuilder {
    AppBuilder::configure::<SingleTestApp>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with_inter_font()
        .use_reactive_ui(|_| {})
}
const DESKTOP_ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-desktop-app",
    test_application: Some(FerroTestApplicationAttribute::new(build_desktop)),
    test_isolation: None,
};
const SINGLE_ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-single-view-app",
    test_application: Some(FerroTestApplicationAttribute::new(build_single)),
    test_isolation: None,
};

#[cfg(debug_assertions)]
const DEBUG_OVERLAY_ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-original-app-debug-overlay",
    test_application: Some(FerroTestApplicationAttribute::new(build_desktop)),
    test_isolation: None,
};
#[cfg(debug_assertions)]
#[test]
fn actual_original_desktop_startup_registers_the_source_f9_and_f11_hotkeys() {
    run_with(&DEBUG_OVERLAY_ASSEMBLY, || {
        use ferroui_base::input::{InputElement, Key, KeyEventArgs};
        let app = Application::current().unwrap();
        let lifetime = app.application_lifetime().unwrap();
        let window = lifetime
            .as_classic_desktop_style_application_lifetime()
            .unwrap()
            .main_window()
            .unwrap();
        window.show();
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let count = || {
            window
                .get_visual_descendants()
                .filter(|visual| {
                    visual.get_type().full_name()
                        == "FerroDock.Ferro.Diagnostics.Controls.DebugOverlayAdorner"
                })
                .count()
        };
        assert_eq!(count(), 0);
        let mut key = KeyEventArgs::new();
        key.set_routed_event(Some(InputElement::key_down_event()));
        key.key = Key::F9;
        window.raise_event(&key);
        assert!(
            count() > 0,
            "the actual source App startup must attach the library hotkey"
        );
        window.raise_event(&key);
        assert_eq!(count(), 0);
        key = KeyEventArgs::new();
        key.set_routed_event(Some(InputElement::key_down_event()));
        key.key = Key::F11;
        window.raise_event(&key);
        drain(&window);
        let desktop = lifetime
            .as_classic_desktop_style_application_lifetime()
            .unwrap();
        let debug = desktop
            .windows()
            .into_iter()
            .find(|candidate| {
                candidate
                    .content()
                    .as_ref()
                    .and_then(|value| ferroui_base::data::core::ValueTypes::as_object(&**value))
                    .is_some_and(|content| {
                        content.is::<ferrodock_ferro_diagnostics::controls::RootDockDebug>()
                    })
            })
            .expect("the actual source App must attach the complete F11 inspector");
        let inspector = ferroui_base::data::core::ValueTypes::as_object(&*debug.content().unwrap())
            .unwrap()
            .cast::<ferrodock_ferro_diagnostics::controls::RootDockDebug>()
            .unwrap();
        assert_eq!(inspector.data_templates().count(), 18);
        window.raise_event(&key);
        assert!(!debug.is_visible());
        window.close();
    });
}

// Native dispatcher shutdown also stops the headless test runner. This test
// lifetime delegates window/state contracts to the native implementation and
// exposes the Exit event without shutting the runner down. The production App
// still receives only the original IClassicDesktopStyleApplicationLifetime.
struct DesktopTestLifetime {
    inner: Rc<ClassicDesktopStyleApplicationLifetime>,
    exits: Rc<
        ferroui_system::events::Event<
            ferroui_controls::application_lifetimes::ControlledApplicationLifetimeExitEventArgs,
        >,
    >,
}
impl DesktopTestLifetime {
    fn new() -> Rc<Self> {
        let inner = ClassicDesktopStyleApplicationLifetime::new();
        inner.set_shutdown_mode(ShutdownMode::OnExplicitShutdown);
        ferroui_controls::application_lifetimes::ISetupApplicationLifetime::before_app_init(
            &*inner,
        );
        Rc::new(Self {
            inner,
            exits: Rc::new(ferroui_system::events::Event::new()),
        })
    }
    fn main_window(&self) -> Option<Ref<Window>> {
        self.inner.main_window()
    }
    fn raise_exit(&self) {
        self.exits.raise(&ferroui_controls::application_lifetimes::ControlledApplicationLifetimeExitEventArgs::new(0));
    }
}
impl IApplicationLifetime for DesktopTestLifetime {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_controlled_application_lifetime(
        &self,
    ) -> Option<&dyn ferroui_controls::application_lifetimes::IControlledApplicationLifetime> {
        Some(self)
    }
    fn as_classic_desktop_style_application_lifetime(
        &self,
    ) -> Option<&dyn ferroui_controls::application_lifetimes::IClassicDesktopStyleApplicationLifetime>
    {
        Some(self)
    }
}
impl ferroui_controls::application_lifetimes::IControlledApplicationLifetime
    for DesktopTestLifetime
{
    fn startup(
        &self,
        handler:Rc<dyn Fn(&ferroui_controls::application_lifetimes::ControlledApplicationLifetimeStartupEventArgs)>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.inner.startup(move |e| handler(e))
    }
    fn exit(
        &self,
        handler: Rc<
            dyn Fn(
                &ferroui_controls::application_lifetimes::ControlledApplicationLifetimeExitEventArgs,
            ),
        >,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        let token = self.exits.add(handler);
        let event = self.exits.clone();
        ferroui_base::reactive::Disposable::create(move || {
            event.remove(token);
        })
    }
    fn shutdown(&self, code: i32) {
        self.inner.shutdown(code)
    }
}
impl ferroui_controls::application_lifetimes::IClassicDesktopStyleApplicationLifetime
    for DesktopTestLifetime
{
    fn try_shutdown(&self, code: i32) -> bool {
        self.inner.try_shutdown(code)
    }
    fn args(&self) -> Option<Vec<String>> {
        self.inner.args()
    }
    fn shutdown_mode(&self) -> ShutdownMode {
        self.inner.shutdown_mode()
    }
    fn set_shutdown_mode(&self, value: ShutdownMode) {
        self.inner.set_shutdown_mode(value)
    }
    fn main_window(&self) -> Option<Ref<Window>> {
        self.inner.main_window()
    }
    fn set_main_window(&self, value: Option<Ref<Window>>) {
        self.inner.set_main_window(value)
    }
    fn windows(&self) -> Vec<Ref<Window>> {
        self.inner.windows()
    }
    fn shutdown_requested(
        &self,
        handler: Rc<dyn Fn(&ferroui_controls::application_lifetimes::ShutdownRequestedEventArgs)>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.inner.shutdown_requested(move |e| handler(e))
    }
}

#[test]
fn nullable_template_assignability_keeps_object_identity_and_calls_a_failing_converter_once() {
    run(|| {
        use ferroui_base::data::core::{ValueType, ValueTypes};
        use ferroui_controls::templates::IDataTemplate;
        let template = ferroui_markup_xaml::templates::DataTemplate::new();
        let object: BoxedValue = template.clone();
        let converted = ValueTypes::try_convert(
            Some(&object),
            ValueType::of::<Option<Rc<dyn IDataTemplate>>>(),
        )
        .unwrap()
        .unwrap();
        let converted = converted
            .downcast_ref::<Option<Rc<dyn IDataTemplate>>>()
            .unwrap()
            .as_ref()
            .unwrap();
        assert!(Rc::ptr_eq(converted, &(template as Rc<dyn IDataTemplate>)));
        let wrong: BoxedValue = Rc::new(42i32);
        assert!(ValueTypes::try_convert(
            Some(&wrong),
            ValueType::of::<Option<Rc<dyn IDataTemplate>>>()
        )
        .is_none());
        #[derive(Clone, PartialEq)]
        struct Source;
        #[derive(Clone, PartialEq)]
        struct Target;
        let count = Rc::new(std::cell::Cell::new(0));
        let called = count.clone();
        ValueTypes::register_conversion::<Source, Target>(move |_| {
            called.set(called.get() + 1);
            None
        });
        let source: BoxedValue = Rc::new(Source);
        assert!(ValueTypes::try_convert(Some(&source), ValueType::of::<Target>()).is_none());
        assert_eq!(count.get(), 1);
    });
}

#[test]
fn source_theme_hook_captures_its_manager_and_observes_the_same_live_preset_list() {
    run(|| {
        use ferrodock_ferro::themes::IDockThemeManager;
        use ferrodock_model::collections::ReadOnlyListHandle;
        use ferroui_system::collections::object_model::ObservableCollection;
        struct Manager {
            names: ReadOnlyListHandle<String>,
            variants: RefCell<Vec<i32>>,
            presets: RefCell<Vec<i32>>,
        }
        impl IDockThemeManager for Manager {
            fn switch(&self, index: i32) {
                self.variants.borrow_mut().push(index)
            }
            fn preset_names(&self) -> ReadOnlyListHandle<String> {
                self.names.clone()
            }
            fn current_preset_index(&self) -> i32 {
                1
            }
            fn switch_preset(&self, index: i32) {
                self.presets.borrow_mut().push(index)
            }
        }
        let original = App::theme_manager();
        let names = ObservableCollection::from_items(vec!["One".into(), "Two".into()]);
        let list = ReadOnlyListHandle::from_observable(names.clone());
        let manager = Rc::new(Manager {
            names: list.clone(),
            variants: RefCell::new(Vec::new()),
            presets: RefCell::new(Vec::new()),
        });
        App::set_theme_manager(Some(manager.clone()));
        let view = crate::views::MainView::new();
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        drain(&window);
        let preset =
            ferroui_base::controls::NameScopeExtensions::get::<ComboBox>(&view, "PresetComboBox");
        assert_eq!(preset.selected_index(), 1);
        assert!(
            manager.presets.borrow().is_empty(),
            "the source subscribes after initial selection"
        );
        let source = preset.items_source().unwrap();
        assert_eq!(source.list().identity(), Some(list.identity()));
        assert!(source.is_notifying());
        names.add("Three".into());
        drain(&window);
        assert_eq!(preset.items_view().count(), 3);
        preset.set_selected_index(2);
        drain(&window);
        assert_eq!(&*manager.presets.borrow(), &[2]);
        App::set_theme_manager(None);
        let button =
            ferroui_base::controls::NameScopeExtensions::get::<Button>(&view, "ThemeButton");
        button.command().unwrap().execute(None);
        drain(&window);
        assert_eq!(
            &*manager.variants.borrow(),
            &[1],
            "the existing source closure retains its original manager"
        );
        preset.set_selected_index(-1);
        drain(&window);
        assert_eq!(&*manager.presets.borrow(), &[2]);
        let disabled = crate::views::MainView::new();
        assert!(ferroui_base::controls::NameScopeExtensions::get::<Button>(
            &disabled,
            "ThemeButton"
        )
        .command()
        .is_none());
        assert!(
            ferroui_base::controls::NameScopeExtensions::get::<ComboBox>(
                &disabled,
                "PresetComboBox"
            )
            .items_source()
            .is_none()
        );
        window.close();
        App::set_theme_manager(original);
    });
}
