use crate::service_overlay_tests::{descendants, Environment, ASSEMBLY};
use ferrodock_controls_recycling::to_recycling_value;
use ferrodock_ferro::controls::overlays::{
    ConfirmationDialogControl, ConfirmationOverlayControl, DialogOverlayControl, DialogShellControl,
};
use ferrodock_model::services::*;
use ferrodock_model_reactiveui_services::overlays::services::*;
use ferroui_base::input::{PhysicalKey, RawInputModifiers};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::{Border, Button, Control, Grid, TextBlock, Window};
use ferroui_headless::{HeadlessUnitTestSession, HeadlessWindowExtensions};
use std::rc::Rc;
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferrodock_ferro_themes_fluent::register_types();
                ferroui_markup_xaml::register_types();
                crate::compiled_markup::register();
                let _environment = Environment::new();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn load() -> (Ref<Grid>, Ref<Window>) {
    load_path("ferres://ferrodock_markup_tests/ServiceOverlayConsumer.xaml")
}
fn load_path(uri: &str) -> (Ref<Grid>, Ref<Window>) {
    let value = crate::compiled_xaml::try_load(None, uri).unwrap().unwrap();
    let grid = value.downcast_ref::<Ref<Grid>>().unwrap().clone();
    let window = Window::new();
    window.set_width(800.0);
    window.set_height(600.0);
    window.set_content(Some(Control::boxed(grid.clone())));
    window.show();
    update(&window);
    (grid, window)
}
fn update(window: &Window) {
    Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
}
fn press(window: &Window, button: &Button) {
    assert!(button.focus());
    window.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::empty());
    window.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::empty());
    update(window);
}
#[test]
fn compiled_request_template_displays_original_native_content_and_close_button_completes_task() {
    run(|| {
        let (grid, window) = load();
        let overlay = grid
            .children()
            .get(1)
            .cast::<DialogOverlayControl>()
            .unwrap();
        let service = DockDialogService::new(None, None);
        overlay.set_dialog_service(Some(service.clone()));
        let content = Border::new();
        content.set_width(100.0);
        content.set_height(40.0);
        let task = service.show_async_boxed(
            DialogContent::new(to_recycling_value(Control::boxed(content.clone()))),
            Some("Original title".into()),
        );
        update(&window);
        let shell = descendants(&overlay)
            .iter()
            .find_map(|c| c.cast::<DialogShellControl>())
            .unwrap();
        assert_eq!(shell.title().as_deref(), Some("Original title"));
        assert_eq!(
            Control::from_boxed(&shell.content().unwrap()),
            Some(content.clone().upcast())
        );
        let tree = descendants(&shell);
        let presenter = tree
            .iter()
            .find_map(|c| {
                c.cast::<ContentPresenter>()
                    .filter(|p| p.child().is_some_and(|child| child.ptr_eq(&content)))
            })
            .unwrap();
        assert_eq!(presenter.child(), Some(content.clone().upcast::<Control>()));
        let title = tree
            .iter()
            .find_map(|c| {
                c.cast::<TextBlock>()
                    .filter(|t| t.text().as_deref() == Some("Original title"))
            })
            .unwrap();
        assert_eq!(title.text().as_deref(), Some("Original title"));
        let card = shell.visual_children().get(0).cast::<Border>().unwrap();
        assert_eq!(card.min_width(), 320.0);
        assert_eq!(card.max_width(), 560.0);
        assert_eq!(card.padding(), Thickness::uniform(16.0));
        let close = tree.iter().find_map(|c| c.cast::<Button>()).unwrap();
        assert!(close.is_visible());
        assert!(Rc::ptr_eq(
            &shell.close_command().unwrap(),
            &close.command().unwrap()
        ));
        shell.set_is_close_visible(false);
        assert!(!close.is_visible());
        shell.set_is_close_visible(true);
        assert!(close.is_visible());
        shell.set_title(Some("Changed title".into()));
        assert_eq!(title.text().as_deref(), Some("Changed title"));
        assert!(!task.is_completed());
        press(&window, &close);
        assert!(!service.has_dialogs());
        assert!(!overlay.is_overlay_visible());
        assert!(task.is_completed_successfully());
        assert!(task.result().unwrap().is_none());
        window.close();
    });
}
#[test]
fn compiled_confirmation_template_buttons_complete_true_and_false_through_headless_keyboard() {
    run(|| {
        let (grid, window) = load();
        let overlay = grid
            .children()
            .get(2)
            .cast::<ConfirmationOverlayControl>()
            .unwrap();
        let service = DockConfirmationService::new(None, None);
        overlay.set_confirmation_service(Some(service.clone()));
        for confirm in [true, false] {
            let task = service.confirm_async(
                "Title".into(),
                "Message".into(),
                "Do it".into(),
                "Keep it".into(),
            );
            update(&window);
            let shell = descendants(&overlay)
                .iter()
                .find_map(|c| c.cast::<ConfirmationDialogControl>())
                .unwrap();
            assert_eq!(shell.title().as_deref(), Some("Title"));
            assert_eq!(shell.message().as_deref(), Some("Message"));
            assert_eq!(shell.confirm_text().as_deref(), Some("Do it"));
            assert_eq!(shell.cancel_text().as_deref(), Some("Keep it"));
            let tree = descendants(&shell);
            let buttons: Vec<_> = tree.iter().filter_map(|c| c.cast::<Button>()).collect();
            assert_eq!(buttons.len(), 2);
            assert_eq!(
                buttons[0]
                    .content()
                    .unwrap()
                    .downcast_ref::<String>()
                    .map(String::as_str),
                Some("Keep it")
            );
            assert_eq!(
                buttons[1]
                    .content()
                    .unwrap()
                    .downcast_ref::<String>()
                    .map(String::as_str),
                Some("Do it")
            );
            assert!(Rc::ptr_eq(
                &shell.cancel_command().unwrap(),
                &buttons[0].command().unwrap()
            ));
            assert!(Rc::ptr_eq(
                &shell.confirm_command().unwrap(),
                &buttons[1].command().unwrap()
            ));
            assert!(tree
                .iter()
                .filter_map(|c| c.cast::<TextBlock>())
                .any(|t| t.text().as_deref() == Some("Message")));
            let card = shell.visual_children().get(0).cast::<Border>().unwrap();
            assert_eq!(card.padding(), Thickness::uniform(20.0));
            assert_eq!(card.max_width(), 440.0);
            assert!(!task.is_completed());
            press(&window, &buttons[usize::from(confirm)]);
            assert!(task.is_completed_successfully());
            assert_eq!(task.result().unwrap(), confirm);
            assert!(!service.has_confirmations());
            assert!(!overlay.is_overlay_visible());
        }
        window.close();
    });
}
#[test]
fn original_default_layers_are_fresh_per_host_and_bind_all_six_original_services() {
    run(|| {
        use ferrodock_ferro::controls::overlays::{BusyOverlayControl, OverlayHost};
        use ferrodock_model_reactiveui_services::overlays::hosting::HostOverlayServicesAdapter;
        let (grid, window) =
            load_path("ferres://ferrodock_markup_tests/DefaultOverlayConsumer.xaml");
        let first = grid.children().get(0).cast::<OverlayHost>().unwrap();
        let second = grid.children().get(1).cast::<OverlayHost>().unwrap();
        let global_busy = DockGlobalBusyService::new(None, None);
        let global_dialog = DockGlobalDialogService::new(None, None);
        let global_confirmation = DockGlobalConfirmationService::new(None, None);
        let aggregate = || -> Rc<dyn IHostOverlayServices> {
            HostOverlayServicesAdapter::new(
                DockBusyService::new(Some(global_busy.clone()), None),
                DockDialogService::new(Some(global_dialog.clone()), None),
                DockConfirmationService::new(Some(global_confirmation.clone()), None),
                global_busy.clone(),
                global_dialog.clone(),
                global_confirmation.clone(),
            )
        };
        let first_services = aggregate();
        let second_services = aggregate();
        first.set_data_context(Some(Rc::new(first_services.clone())));
        second.set_data_context(Some(Rc::new(second_services.clone())));
        update(&window);
        let first_layers = first.overlay_layers();
        let second_layers = second.overlay_layers();
        assert_eq!(first_layers.count(), 3);
        assert_eq!(second_layers.count(), 3);
        assert!(!first_layers.ptr_eq(&second_layers));
        for index in 0..3 {
            assert_ne!(
                first_layers.get(index).unwrap().overlay(),
                second_layers.get(index).unwrap().overlay()
            );
        }
        // The source host can retain its old private pipeline after an equivalent
        // public collection replacement. Inspect the actual displayed controls.
        let busy = first.child().unwrap().cast::<BusyOverlayControl>().unwrap();
        let other_busy = second
            .child()
            .unwrap()
            .cast::<BusyOverlayControl>()
            .unwrap();
        let confirmation = Control::from_boxed(&busy.content().unwrap())
            .unwrap()
            .cast::<ConfirmationOverlayControl>()
            .unwrap();
        let dialog = Control::from_boxed(&confirmation.content().unwrap())
            .unwrap()
            .cast::<DialogOverlayControl>()
            .unwrap();
        assert!(Rc::ptr_eq(
            &busy.busy_service().unwrap(),
            &first_services.busy()
        ));
        assert!(Rc::ptr_eq(
            &busy.global_busy_service().unwrap(),
            &first_services.global_busy_service()
        ));
        assert!(Rc::ptr_eq(
            &dialog.dialog_service().unwrap(),
            &first_services.dialogs()
        ));
        assert!(Rc::ptr_eq(
            &dialog.global_dialog_service().unwrap(),
            &first_services.global_dialog_service()
        ));
        assert!(Rc::ptr_eq(
            &confirmation.confirmation_service().unwrap(),
            &first_services.confirmations()
        ));
        assert!(Rc::ptr_eq(
            &confirmation.global_confirmation_service().unwrap(),
            &first_services.global_confirmation_service()
        ));
        assert!(Rc::ptr_eq(
            &other_busy.busy_service().unwrap(),
            &second_services.busy()
        ));
        assert!(busy.theme().is_some());
        assert!(dialog.theme().is_some());
        assert!(confirmation.theme().is_some());
        let scope = first_services.busy().begin("local work".into());
        update(&window);
        assert!(busy.is_busy());
        assert!(busy.show_progress());
        assert!(other_busy.is_busy());
        assert!(!other_busy.show_progress());
        assert_eq!(busy.busy_message().as_deref(), Some("local work"));
        assert!(!busy.is_content_enabled());
        assert!(!other_busy.is_content_enabled());
        scope.dispose();
        update(&window);
        assert!(!busy.is_busy());
        assert!(!other_busy.is_busy());
        let task = first_services
            .confirmations()
            .confirm_async_default("Default title".into(), "Default message".into());
        update(&window);
        let shell = descendants(&first)
            .iter()
            .find_map(|c| c.cast::<ConfirmationDialogControl>())
            .unwrap();
        let buttons: Vec<_> = descendants(&shell)
            .iter()
            .filter_map(|c| c.cast::<Button>())
            .collect();
        press(&window, &buttons[1]);
        assert!(task.result().unwrap());
        first.set_data_context(Some(Rc::new(second_services.clone())));
        update(&window);
        assert!(Rc::ptr_eq(
            &busy.busy_service().unwrap(),
            &second_services.busy()
        ));
        assert!(Rc::ptr_eq(
            &dialog.dialog_service().unwrap(),
            &second_services.dialogs()
        ));
        assert!(Rc::ptr_eq(
            &confirmation.confirmation_service().unwrap(),
            &second_services.confirmations()
        ));
        window.close();
    });
}

#[test]
fn full_source_palette_and_exported_foreign_enum_keep_typed_values_aliases_and_resources() {
    run(|| {
        use ferrodock_model::core::DockOperation;
        use ferroui_base::controls::{ResourceDictionary, ResourceKey};
        use ferroui_base::data::core::ValueTypes;
        use ferroui_base::media::{Color, IBrush, SolidColorBrush, StreamGeometry};
        use ferroui_base::metadata::from_markup_value;
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/ThemeResourcesConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let dictionary = value.downcast_ref::<Ref<ResourceDictionary>>().unwrap();
        let owner = Grid::new();
        owner.set_resources(dictionary.clone());
        let get = |key: &str| {
            dictionary
                .try_get_resource(&ResourceKey::from(key), None)
                .flatten()
                .unwrap_or_else(|| panic!("resource {key}"))
        };
        let values = [
            ("None", DockOperation::None),
            ("Fill", DockOperation::Fill),
            ("Left", DockOperation::Left),
            ("Bottom", DockOperation::Bottom),
            ("Right", DockOperation::Right),
            ("Top", DockOperation::Top),
            ("Window", DockOperation::Window),
            ("OuterLeft", DockOperation::OuterLeft),
            ("OuterTop", DockOperation::OuterTop),
            ("OuterRight", DockOperation::OuterRight),
            ("OuterBottom", DockOperation::OuterBottom),
        ];
        for (name, expected) in values {
            assert_eq!(
                get(&format!("Operation{name}")).downcast_ref::<DockOperation>(),
                Some(&expected)
            );
        }
        assert_eq!(
            get("DockDocumentControlIndicatorDockOperation").downcast_ref::<DockOperation>(),
            Some(&DockOperation::Fill)
        );
        assert_eq!(
            get("DockDialogMinWidth").downcast_ref::<f64>(),
            Some(&360.0)
        );
        assert_eq!(
            get("DockDialogMaxWidth").downcast_ref::<f64>(),
            Some(&640.0)
        );
        assert_eq!(
            get("DockConfirmationDialogMaxWidth").downcast_ref::<f64>(),
            Some(&480.0)
        );
        assert_eq!(
            get("DockDialogPadding").downcast_ref::<Thickness>(),
            Some(&Thickness::uniform(20.0))
        );
        let accent_value = get("DockApplicationAccentBrushLow");
        let accent: Rc<dyn IBrush> = from_markup_value(&Some(accent_value.clone())).unwrap();
        assert_eq!(
            accent.as_solid_color_brush().unwrap().color(),
            Color::from_rgb(0, 122, 204)
        );
        let workbench: Rc<dyn IBrush> =
            from_markup_value(&Some(get("DockSurfaceWorkbenchBrush"))).unwrap();
        let workbench = workbench
            .as_object()
            .unwrap()
            .downcast_ref::<SolidColorBrush>()
            .unwrap()
            .to_ref();
        dictionary.set("RegionColor", Some(Rc::new(Color::from_rgb(1, 2, 3))));
        assert_eq!(workbench.color(), Color::from_rgb(1, 2, 3));
        dictionary.set("RegionColor", Some(Rc::new(Color::from_rgb(4, 5, 6))));
        assert_eq!(workbench.color(), Color::from_rgb(4, 5, 6));
        assert!(Rc::ptr_eq(
            &accent_value,
            &get("DockTabActiveBackgroundBrush")
        ));
        for key in [
            "DockIconAddDocumentGeometry",
            "DockIconCloseGeometry",
            "DockIconMinimizeGeometry",
            "DockToolIconCloseGeometry",
            "DockIconMaximizeGeometry",
            "DockIconRestoreGeometry",
            "DockIconPinGeometry",
            "DockIconPinnedGeometry",
            "DockIconMenuGeometry",
            "DockIconStatusDockGeometry",
            "DockIconStatusFloatGeometry",
        ] {
            assert!(
                ValueTypes::as_object(&*get(key))
                    .unwrap()
                    .cast::<StreamGeometry>()
                    .is_some(),
                "{key}"
            );
        }
        assert_eq!(
            get("DocumentTabStripItemCloseString")
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("_Close")
        );
    });
}

#[test]
fn source_ide_presets_load_all_resources_and_preserve_distinct_editor_colors() {
    run(|| {
        use ferroui_base::controls::{ResourceDictionary, ResourceKey};
        use ferroui_base::media::Color;
        // Default aliases require the source accent dictionary as their owner,
        // just as in DockFluentTheme; apply each preset after that dictionary.
        for (preset, color) in [
            ("Default", None),
            ("RiderDark", Some(0xff1f2124)),
            ("RiderLight", Some(0xffffffff)),
            ("VsCodeDark", Some(0xff1e1e1e)),
            ("VsCodeLight", Some(0xffffffff)),
        ] {
            let value = crate::compiled_xaml::try_load(
                None,
                &format!("ferres://ferrodock_markup_tests/Preset{preset}Consumer.xaml"),
            )
            .unwrap()
            .unwrap();
            let dictionary = value.downcast_ref::<Ref<ResourceDictionary>>().unwrap();
            let owner = Grid::new();
            owner.set_resources(dictionary.clone());
            let preset_dictionary = dictionary
                .merged_dictionaries()
                .get(1)
                .as_object()
                .unwrap()
                .downcast_ref::<ResourceDictionary>()
                .unwrap()
                .to_ref();
            for key in preset_dictionary.keys() {
                assert!(preset_dictionary.get(&key).is_some(), "{preset}: {key:?}");
            }
            let editor = owner
                .try_find_resource(&ResourceKey::from("DockSurfaceEditorBrush"), None)
                .flatten()
                .unwrap();
            let editor: Rc<dyn ferroui_base::media::IBrush> =
                ferroui_base::metadata::from_markup_value(&Some(editor)).unwrap();
            let editor = editor.as_solid_color_brush().unwrap();
            if let Some(color) = color {
                assert_eq!(editor.color(), Color::from_uint32(color), "{preset}");
            }
        }
    });
}
