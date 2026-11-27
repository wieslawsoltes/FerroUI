use crate::dialog_lifecycle_consumer::DialogEventsHost;
use ferroribbon::controls::RibbonDialogOutcome;
use ferroui_base::{TypeInfo, data::core::ValueTypes, threading::Dispatcher};
use ferroui_controls::Window;
#[test]
fn independent_native_subclass_lifecycle_xaml_deferrals_and_future_queue_match_both_loaders() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::dialog_lifecycle_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[DialogEventsHost::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("DialogLifecycle.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/DialogLifecycle.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<DialogEventsHost>()
                .unwrap();
            let window = Window::new();
            window.set_width(800.);
            window.set_height(650.);
            window.show();
            window.update_layout();
            let future = host.show_dialog(window.clone().upcast()).unwrap();
            host.set_future(future);
            assert_eq!(host.counts(), (1, 0, 0));
            assert!(host.is_dialog_showing());
            host.primary_button().unwrap().on_click();
            assert_eq!(host.counts(), (1, 1, 0));
            assert!(host.is_dialog_showing());
            assert_eq!(host.future_result(), None);
            host.set_mode(2);
            host.primary_button().unwrap().on_click();
            assert_eq!(host.counts(), (1, 2, 0));
            assert!(host.is_dialog_closing());
            host.primary_button().unwrap().on_click();
            assert_eq!(
                host.counts(),
                (1, 2, 0),
                "closing guard suppresses a second button request"
            );
            host.complete();
            assert_eq!(host.counts(), (1, 2, 1));
            assert!(!host.is_dialog_showing());
            assert!(!host.is_dialog_closing());
            assert_eq!(host.future_result(), None);
            assert_eq!(host.result(), Some(RibbonDialogOutcome::Primary));
            assert_eq!(
                host.pending_during_closed(),
                (true, true),
                "dispatcher pumping inside Closed must not complete the outcome early"
            );
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(host.future_result(), Some(RibbonDialogOutcome::Primary));
            window.close();
        }
    });
}
