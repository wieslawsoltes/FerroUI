//! Actual original navigation views and source-class metadata through compiled bindings.
use crate::{
    view_models::{
        documents::DocumentViewModel,
        views::{DashboardViewModel, HomeViewModel},
        DockFactory,
    },
    views::views::{DashboardView, HomeView},
};
use ferrodock_model::{controls::*, core::*, BoxedValue as ModelValue};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    threading::{CancellationToken, Dispatcher},
    BoxedValue, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, Button, CheckBox, Control, StackPanel, TextBlock, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    use ferroui_fonts_inter::AppBuilderExtension;
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .with_inter_font()
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style());
            let app = Application::current().unwrap();
            app.styles()
                .add(ferrodock_ferro_themes_fluent::DockFluentTheme::new().as_style());
            app.data_templates()
                .add(crate::ViewLocator::new().as_data_template());
        })
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "original-sample-compiled-views",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f();
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
fn source_class_metadata_retains_exact_base_names_and_actual_model_identity() {
    run(|| {
        use ferrodock_model_reactiveui_ferro::reactive_metadata::{
            DockableBaseReference, DocumentReference,
        };
        let metadata = <DocumentViewModel as MarkupTyped>::MARKUP;
        assert_eq!(
            metadata.full_name(),
            "FerroDockReactiveUISample.ViewModels.Documents.DocumentViewModel"
        );
        let document = metadata.base_type().unwrap();
        assert_eq!(document.name, "Document");
        assert_eq!(document.base_type().unwrap().name, "DockableBase");
        assert_eq!(
            document.base_type().unwrap().base_type().unwrap().name,
            "ReactiveBase"
        );
        let model = DocumentViewModel::new();
        let value: BoxedValue = Rc::new(model.clone());
        let reference = ValueTypes::try_cast(&value, ValueType::of::<DocumentReference>()).unwrap();
        let reference = reference.downcast_ref::<DocumentReference>().unwrap();
        assert!(Rc::ptr_eq(
            &reference.model(),
            &(model.clone() as Rc<dyn IDockable>)
        ));
        let base = ValueTypes::try_cast(&value, ValueType::of::<DockableBaseReference>()).unwrap();
        let base = base.downcast_ref::<DockableBaseReference>().unwrap();
        assert!(Rc::ptr_eq(
            &base.model(),
            &(model.clone() as Rc<dyn IDockable>)
        ));
        let property = <DocumentReference as MarkupTyped>::MARKUP
            .find_property("MdiState")
            .unwrap();
        property.set.unwrap()(&[
            Some(Rc::new(reference.clone())),
            Some(Rc::new(MdiWindowState::Maximized)),
        ])
        .unwrap();
        assert_eq!(model.mdi_state(), MdiWindowState::Maximized);
        let tool: BoxedValue =
            Rc::new(crate::view_models::tools::Tool1ViewModel::new() as Rc<dyn IDockable>);
        assert!(ValueTypes::try_cast(&tool, ValueType::of::<DocumentReference>()).is_none());
        assert!(ValueTypes::try_cast(
            &tool,
            ValueType::of::<ferrodock_model_reactiveui_ferro::reactive_metadata::ToolReference>()
        )
        .is_some());
        let home = <HomeViewModel as MarkupTyped>::MARKUP;
        let mut parent = home.base_type();
        let mut names = Vec::new();
        while let Some(meta) = parent {
            names.push(meta.name);
            parent = meta.base_type();
        }
        assert_eq!(
            names,
            [
                "RootDock",
                "DockBase",
                "DockableBase",
                "ReactiveBase",
                "ReactiveObject"
            ]
        );
    });
}
#[test]
fn compiled_inherited_bindings_update_the_original_model_and_its_two_way_permissions() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://FerroDockReactiveUISample/tests/InheritedBindingConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let view = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        let model = DocumentViewModel::new();
        model.set_id("Document1".into());
        model.set_title("Original document".into());
        view.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
            model.clone(),
        )));
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        drain(&window);
        let text = |name: &str| {
            view.find_descendant_of_type_where::<TextBlock>(false, |text| {
                text.name().as_deref() == Some(name)
            })
            .unwrap()
        };
        assert_eq!(
            text("TitleText").text().as_deref(),
            Some("Original document")
        );
        assert_eq!(text("IdText").text().as_deref(), Some("Document1"));
        assert_eq!(text("StateText").text().as_deref(), Some("Normal"));
        model.set_title("Changed by the engine".into());
        model.set_mdi_state(MdiWindowState::Maximized);
        drain(&window);
        assert_eq!(
            text("TitleText").text().as_deref(),
            Some("Changed by the engine")
        );
        assert_eq!(text("StateText").text().as_deref(), Some("Maximized"));
        let toggle = view.find_descendant_of_type::<CheckBox>(false).unwrap();
        toggle.set_is_checked(Some(false));
        assert!(!model.can_close());
        model.set_can_close(true);
        drain(&window);
        assert_eq!(toggle.is_checked(), Some(true));
        window.close();
    });
}
#[test]
fn original_dashboard_view_binds_captured_root_context_and_executes_real_home_navigation() {
    run(|| {
        let factory = DockFactory::new(crate::models::DemoData::new());
        let root = factory.create_layout().unwrap();
        factory.init_layout(root.clone());
        let dashboard = root
            .visible_dockables()
            .unwrap()
            .get(0)
            .expect("source dashboard");
        assert_eq!(dashboard.runtime_type_name(), "DashboardViewModel");
        root.set_active_dockable(Some(dashboard.clone()));
        let view = DashboardView::new();
        view.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
            dashboard.clone(),
        )));
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        drain(&window);
        assert!(view
            .find_descendant_of_type_where::<TextBlock>(false, |text| text.text().as_deref()
                == Some("Dashboard"))
            .is_some());
        dashboard.set_title("Changed dashboard".into());
        drain(&window);
        assert!(view
            .find_descendant_of_type_where::<TextBlock>(false, |text| text.text().as_deref()
                == Some("Changed dashboard"))
            .is_some());
        let button = view.find_descendant_of_type::<Button>(false).unwrap();
        assert_eq!(
            button
                .command_parameter()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "Home"
        );
        let command = button.command().expect("source root Navigate binding");
        assert!(command.can_execute(button.command_parameter().as_ref()));
        command.execute(button.command_parameter().as_ref());
        assert_eq!(root.active_dockable().unwrap().id(), "Home");
        window.close();
    });
}
#[test]
fn original_home_view_uses_actual_custom_root_base_bindings_and_reacts_to_focus() {
    run(|| {
        let home = HomeViewModel::new();
        let doc = DocumentViewModel::new();
        doc.set_title("Nested content".into());
        home.set_active_dockable(Some(doc.clone()));
        let view = HomeView::new();
        view.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
            home.clone(),
        )));
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        drain(&window);
        let content = view
            .find_descendant_of_type::<ferroui_controls::ContentControl>(false)
            .unwrap();
        let active = content.content().unwrap();
        let active = ValueTypes::try_cast(&active, ValueType::of::<Rc<dyn IDockable>>()).unwrap();
        assert!(Rc::ptr_eq(
            active.downcast_ref::<Rc<dyn IDockable>>().unwrap(),
            &(doc.clone() as Rc<dyn IDockable>)
        ));
        home.set_focused_dockable(Some(doc));
        drain(&window);
        let grid = Control::from_boxed(&view.content().unwrap())
            .unwrap()
            .cast::<ferroui_controls::Grid>()
            .unwrap();
        let text = grid.children().get(1).cast::<TextBlock>().unwrap();
        assert_eq!(
            text.text().as_deref(),
            Some("FerroDockReactiveUISample.ViewModels.Documents.DocumentViewModel")
        );
        window.close();
    });
}
#[test]
fn raw_context_classes_keep_source_identity_and_native_runtime_names() {
    run(|| {
        use ferrodock_controls_recycling::{from_recycling_value, to_recycling_value};
        let context = crate::models::documents::DemoDocument::new();
        let raw: ModelValue = context.clone();
        let native = from_recycling_value(&raw);
        assert!(native.is::<crate::models::documents::DemoDocument>());
        assert_eq!(
            ValueTypes::to_display_string(Some(&native)),
            "FerroDockReactiveUISample.Models.Documents.DemoDocument"
        );
        assert!(Rc::ptr_eq(&raw, &to_recycling_value(native)));
        let model = DashboardViewModel::new();
        let source: ModelValue = Rc::new(model.clone() as Rc<dyn IDockable>);
        let native = from_recycling_value(&source);
        assert!(native.is::<Rc<dyn IDockable>>());
        assert!(Rc::ptr_eq(&source, &to_recycling_value(native)));
    });
}

#[test]
fn main_window_metadata_binds_status_layout_and_the_actual_reset_command() {
    run(|| {
        let model = crate::view_models::MainWindowViewModel::new();
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://FerroDockReactiveUISample/tests/MainWindowBindingConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let view = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        view.set_data_context(Some(Rc::new(model.clone())));
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        drain(&window);
        let status = view
            .find_descendant_of_type_where::<TextBlock>(false, |text| {
                text.name().as_deref() == Some("StatusText")
            })
            .unwrap();
        assert_eq!(status.text(), Some(model.global_status()));
        model.set_global_status("native source metadata".into());
        drain(&window);
        assert_eq!(status.text().as_deref(), Some("native source metadata"));
        let host = view
            .find_descendant_of_type_where::<ferroui_controls::ContentControl>(false, |control| {
                control.name().as_deref() == Some("LayoutHost")
            })
            .unwrap();
        let root = model.layout().unwrap();
        let content = host.tag().unwrap();
        let actual = ValueTypes::try_cast(&content, ValueType::of::<Rc<dyn IRootDock>>()).unwrap();
        assert!(Rc::ptr_eq(
            actual.downcast_ref::<Rc<dyn IRootDock>>().unwrap(),
            &root
        ));
        let button = view.find_descendant_of_type::<Button>(false).unwrap();
        let command = button.command().unwrap();
        assert!(command.can_execute(None));
        command.execute(None);
        drain(&window);
        let next = model.layout().unwrap();
        assert!(!Rc::ptr_eq(&root, &next));
        let actual =
            ValueTypes::try_cast(&host.tag().unwrap(), ValueType::of::<Rc<dyn IRootDock>>())
                .unwrap();
        assert!(Rc::ptr_eq(
            actual.downcast_ref::<Rc<dyn IRootDock>>().unwrap(),
            &next
        ));
        window.close();
    });
}

#[test]
fn original_option_arrays_preserve_null_flags_identity_and_non_notifying_element_writes() {
    run(|| {
        use crate::views::DockableOptionsView;
        use ferroui_base::metadata::MarkupInvokeError;
        use ferroui_controls::ItemsSource;
        let metadata = DockableOptionsView::TYPE.markup().unwrap();
        let get = |name| -> ItemsSource {
            let value = metadata.find_static_property(name).unwrap().get.unwrap()(&[])
                .unwrap_or_else(|error: MarkupInvokeError| panic!("{error}"))
                .unwrap();
            value.downcast_ref::<ItemsSource>().unwrap().clone()
        };
        let pinned = get("PinnedDockDisplayModeOptions");
        assert_eq!(pinned.count(), 3);
        assert!(pinned.get_at(0).is_none());
        assert_eq!(
            pinned
                .get_at(1)
                .unwrap()
                .downcast_ref::<PinnedDockDisplayMode>(),
            Some(&PinnedDockDisplayMode::Overlay)
        );
        assert_eq!(
            pinned
                .get_at(2)
                .unwrap()
                .downcast_ref::<PinnedDockDisplayMode>(),
            Some(&PinnedDockDisplayMode::Inline)
        );
        assert!(pinned == get("PinnedDockDisplayModeOptions"));
        assert!(!pinned.list().is_notifying());
        let states = get("DockingWindowStateOptions");
        let bits: Vec<_> = (0..states.count())
            .map(|i| {
                states
                    .get_at(i)
                    .unwrap()
                    .downcast_ref::<DockingWindowState>()
                    .unwrap()
                    .bits()
            })
            .collect();
        assert_eq!(bits, [1, 2, 4, 9, 10, 12, 17, 18, 20, 25, 28]);
        assert!(states == get("DockingWindowStateOptions"));
        let array = DockableOptionsView::docking_window_state_options();
        assert!(Rc::ptr_eq(
            &array,
            &DockableOptionsView::docking_window_state_options()
        ));
        array.borrow_mut()[0] = DockingWindowState::NONE;
        assert_eq!(
            states
                .get_at(0)
                .unwrap()
                .downcast_ref::<DockingWindowState>(),
            Some(&DockingWindowState::NONE)
        );
        array.borrow_mut()[0] = DockingWindowState::DOCKED;
    });
}

#[test]
fn original_document_and_all_eight_tool_views_bind_actual_models_and_edit_engine_permissions() {
    run(|| {
        use crate::{
            view_models::tools::*,
            views::{documents::DocumentView, tools::*},
        };
        use ferroui_controls::{ComboBox, Grid};
        let cases: Vec<(Rc<dyn IDockable>, Ref<Control>, &str)> = vec![
            (
                DocumentViewModel::new(),
                DocumentView::new().upcast(),
                "Document",
            ),
            (Tool1ViewModel::new(), Tool1View::new().upcast(), "Tool1"),
            (Tool2ViewModel::new(), Tool2View::new().upcast(), "Tool2"),
            (Tool3ViewModel::new(), Tool3View::new().upcast(), "Tool3"),
            (Tool4ViewModel::new(), Tool4View::new().upcast(), "Tool4"),
            (Tool5ViewModel::new(), Tool5View::new().upcast(), "Tool5"),
            (Tool6ViewModel::new(), Tool6View::new().upcast(), "Tool6"),
            (Tool7ViewModel::new(), Tool7View::new().upcast(), "Tool7"),
            (Tool8ViewModel::new(), Tool8View::new().upcast(), "Tool8"),
        ];
        for (model, view, name) in cases {
            model.set_id(name.into());
            model.set_title(format!("{name} title"));
            view.set_data_context(Some(ferrodock_ferro::model_projection::to_dockable_value(
                model.clone(),
            )));
            let window = Window::new();
            window.set_content(Some(Control::boxed(view.clone())));
            window.show();
            drain(&window);
            assert!(view
                .find_descendant_of_type_where::<TextBlock>(false, |text| text.text().as_deref()
                    == Some(name))
                .is_some());
            model.set_title(format!("{name} changed"));
            drain(&window);
            assert!(view
                .find_descendant_of_type_where::<TextBlock>(false, |text| text.text().as_deref()
                    == Some(&format!("{name} changed")))
                .is_some());
            let options = view
                .find_descendant_of_type::<crate::views::DockableOptionsView>(false)
                .unwrap();
            let grid = Control::from_boxed(&options.content().unwrap())
                .unwrap()
                .cast::<Grid>()
                .unwrap();
            let row = grid.children().get(0).cast::<StackPanel>().unwrap();
            for (i, property) in [
                "CanClose",
                "CanFloat",
                "CanPin",
                "KeepPinnedDockableVisible",
                "CanDrag",
                "CanDrop",
                "IsModified",
            ]
            .iter()
            .enumerate()
            {
                let check = row.children().get(i).cast::<CheckBox>().unwrap();
                let meta =
                    ferroui_base::metadata::MarkupType::find("FerroDock.Model.Core", "IDockable")
                        .unwrap();
                let member = meta.find_property(property).unwrap();
                let value = Rc::new(model.clone()) as BoxedValue;
                member.set.unwrap()(&[Some(value.clone()), Some(Rc::new(true))]).unwrap();
                drain(&window);
                assert_eq!(check.is_checked(), Some(true), "{name}.{property}");
                check.set_is_checked(Some(false));
                let current = member.get.unwrap()(&[Some(value)]).unwrap().unwrap();
                assert_eq!(
                    current.downcast_ref::<bool>(),
                    Some(&false),
                    "{name}.{property}"
                );
            }
            let pinned = grid.children().get(1).cast::<ComboBox>().unwrap();
            assert_eq!(pinned.items_view().count(), 3);
            model.set_pinned_dock_display_mode_override(Some(PinnedDockDisplayMode::Inline));
            drain(&window);
            assert_eq!(pinned.selected_index(), 2);
            pinned.set_selected_index(1);
            assert_eq!(
                model.pinned_dock_display_mode_override(),
                Some(PinnedDockDisplayMode::Overlay)
            );
            pinned.set_selected_index(0);
            assert_eq!(model.pinned_dock_display_mode_override(), None);
            let state = grid.children().get(2).cast::<ComboBox>().unwrap();
            assert_eq!(state.items_view().count(), 11);
            model.set_docking_state(
                DockingWindowState::DOCUMENT
                    | DockingWindowState::FLOATING
                    | DockingWindowState::HIDDEN,
            );
            drain(&window);
            assert_eq!(state.selected_index(), 10);
            state.set_selected_index(3);
            assert_eq!(
                model.docking_state(),
                DockingWindowState::DOCKED | DockingWindowState::FLOATING
            );
            window.close();
        }
    });
}

#[test]
fn source_locator_uses_exact_factories_and_preserves_the_original_missing_view_and_error_paths() {
    run(|| {
        use crate::{view_models::tools::*, ViewLocator};
        use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
        let locator = ViewLocator::new();
        assert!(!locator.match_(None));
        assert!(locator.build(&None).is_none());
        let other: BoxedValue = Rc::new(42i32);
        assert!(!locator.match_(Some(&other)));
        let cases: Vec<(Rc<dyn IDockable>, &str)> = vec![
            (DocumentViewModel::new(), "DocumentView"),
            (Tool1ViewModel::new(), "Tool1View"),
            (Tool2ViewModel::new(), "Tool2View"),
            (Tool3ViewModel::new(), "Tool3View"),
            (Tool4ViewModel::new(), "Tool4View"),
            (Tool5ViewModel::new(), "Tool5View"),
            (Tool6ViewModel::new(), "Tool6View"),
            (Tool7ViewModel::new(), "Tool7View"),
            (Tool8ViewModel::new(), "Tool8View"),
            (DashboardViewModel::new(), "DashboardView"),
            (HomeViewModel::new(), "HomeView"),
        ];
        for (model, name) in cases {
            let value = ferrodock_ferro::model_projection::to_dockable_value(model);
            assert!(locator.match_(Some(&value)));
            let first = locator.build(&Some(value.clone())).unwrap();
            let second = locator.build(&Some(value)).unwrap();
            assert_eq!(first.get_type().name(), name);
            assert!(!Ref::<Control>::ptr_eq(&first, &second));
            assert!(
                first.data_context().is_none(),
                "the source locator leaves context assignment to its presenter"
            );
        }
        let main: BoxedValue = Rc::new(crate::view_models::MainWindowViewModel::new());
        assert!(locator.match_(Some(&main)));
        let missing = locator
            .build(&Some(main))
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(
            missing.text().as_deref(),
            Some("Not Found: FerroDockReactiveUISample.Views.MainWindowView")
        );
        let unmapped = ferrodock_ferro::model_projection::to_dockable_value(
            ferrodock_model_reactiveui::controls::Document::new(),
        );
        assert!(locator.match_(Some(&unmapped)));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || locator.build(&Some(unmapped))
        ))
        .is_err());
    });
}

#[test]
fn actual_main_window_composes_the_original_sample_graph_and_rebinds_after_new_layout() {
    run(|| {
        use ferrodock_ferro::controls::DockControl;
        use ferroui_base::media::{imaging::BitmapInterpolationMode, RenderOptions};
        let model = crate::view_models::MainWindowViewModel::new();
        let window = crate::views::MainWindow::new();
        window.set_data_context(Some(Rc::new(model.clone())));
        window.show();
        drain(&window);
        assert_eq!(window.title().as_deref(), Some("FerroDock Demo"));
        assert_eq!(
            RenderOptions::get_bitmap_interpolation_mode(&window),
            BitmapInterpolationMode::HighQuality
        );
        let dock = window
            .find_descendant_of_type::<DockControl>(false)
            .unwrap();
        assert!(Rc::ptr_eq(
            &dock.layout().unwrap(),
            &(model.layout().unwrap() as Rc<dyn IDock>)
        ));
        assert!(window
            .find_descendant_of_type_where::<ferrodock_ferro::controls::RootDockControl>(
                false,
                |control| control
                    .data_context()
                    .and_then(|value| ValueTypes::try_cast(
                        &value,
                        ValueType::of::<Rc<dyn IDockable>>()
                    ))
                    .is_some_and(
                        |value| value.downcast_ref::<Rc<dyn IDockable>>().unwrap().id() == "Home"
                    )
            )
            .is_some());
        assert!(window
            .find_descendant_of_type::<crate::views::documents::DocumentView>(false)
            .is_some());
        let before = model.layout().unwrap();
        model.reset_layout();
        drain(&window);
        let after = model.layout().unwrap();
        assert!(!Rc::ptr_eq(&before, &after));
        assert!(Rc::ptr_eq(
            &dock.layout().unwrap(),
            &(after as Rc<dyn IDock>)
        ));
        assert!(window
            .find_descendant_of_type::<crate::views::documents::DocumentView>(false)
            .is_some());
        window.close();
    });
}
