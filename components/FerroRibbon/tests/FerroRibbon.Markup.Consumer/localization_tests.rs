//! Actual external derived root, compiled/runtime source-style markup and themes.
use crate::ribbon_consumer::ConsumerRibbon;
use ferroribbon_core::localization::{RibbonCulture, RibbonStrings};
use ferroui_base::{Rect, Size, data::core::ValueTypes, threading::Dispatcher};
use ferroui_controls::{Control, ToolTip, Window, automation::peers::ControlAutomationPeer};

#[test]
fn worker_translations_reach_compiled_and_runtime_ribbon_templates_after_ui_selection() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::worker_translations_reach_compiled_and_runtime_ribbon_templates_after_ui_selection"
    )) {
        return;
    }
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            RibbonStrings::set_current(Some(
                RibbonStrings::for_culture(Some(RibbonCulture::new("en".into(), "en".into())))
                    .unwrap(),
            ))
            .unwrap();
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("LocalizedRibbon.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/LocalizedRibbon.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ConsumerRibbon>()
                .unwrap();
            let window = Window::new();
            window.set_content(Some(Control::boxed(root.clone())));
            window.show();
            window.measure(Size::new(1000., 300.));
            window.arrange(Rect::new(0., 0., 1000., 300.));
            Dispatcher::ui_thread().run_jobs(None);
            let peer = ControlAutomationPeer::create_peer_for_element(
                &root.display_options_button().unwrap(),
            );
            assert_eq!(peer.get_name(), "Ribbon display options");
            std::thread::spawn(|| {
                ferroribbon_core::localization::RibbonStrings::register(
                    Some("x-ui-worker".into()),
                    Some(ferroribbon_core::model::RibbonItemsSource::from_values(
                        vec![
                            (
                                Some("RibbonDisplayOptions".into()),
                                Some("Worker ribbon options".into()),
                            ),
                            (Some("MoreOptions".into()), Some("Worker overflow".into())),
                            (
                                Some("DialogLauncher".into()),
                                Some("Worker settings: {0}".into()),
                            ),
                        ],
                    )),
                )
                .unwrap();
            })
            .join()
            .unwrap();
            assert_eq!(
                peer.get_name(),
                "Ribbon display options",
                "registering a different culture does not select it"
            );
            RibbonStrings::set_current(Some(
                RibbonStrings::for_culture(Some(RibbonCulture::new(
                    "x-ui-worker".into(),
                    "x".into(),
                )))
                .unwrap(),
            ))
            .unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(peer.get_name(), "Worker ribbon options");
            let tab = root.tabs().get(0).unwrap();
            assert_eq!(
                tab.overflow_button().label().as_deref(),
                Some("Worker overflow")
            );
            let launcher = tab.groups().get(0).unwrap().dialog_launcher().unwrap();
            assert_eq!(
                ControlAutomationPeer::create_peer_for_element(&launcher).get_name(),
                "Worker settings: Tools"
            );
            std::thread::spawn(|| {
                RibbonStrings::set_current(Some(
                    RibbonStrings::for_culture(Some(RibbonCulture::new("fr".into(), "fr".into())))
                        .unwrap(),
                ))
                .unwrap();
            })
            .join()
            .unwrap();
            assert_eq!(
                peer.get_name(),
                "Worker ribbon options",
                "background selection stays deferred"
            );
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(peer.get_name(), "Options d'affichage du ruban");
            std::thread::spawn(|| {
                RibbonStrings::set_current(Some(
                    RibbonStrings::for_culture(Some(RibbonCulture::new(
                        "x-ui-worker".into(),
                        "x".into(),
                    )))
                    .unwrap(),
                ))
                .unwrap();
                RibbonStrings::current()
                    .override_string(
                        Some("RibbonDisplayOptions".into()),
                        Some("Pinned options".into()),
                    )
                    .unwrap();
                RibbonStrings::register(
                    Some("x-ui-worker".into()),
                    Some(ferroribbon_core::model::RibbonItemsSource::from_values(
                        vec![
                            (
                                Some("RibbonDisplayOptions".into()),
                                Some("Registered options".into()),
                            ),
                            (Some("MoreOptions".into()), Some("Updated overflow".into())),
                        ],
                    )),
                )
                .unwrap();
            })
            .join()
            .unwrap();
            assert_eq!(peer.get_name(), "Options d'affichage du ruban");
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                peer.get_name(),
                "Pinned options",
                "worker registration preserves active instance overrides"
            );
            assert_eq!(
                tab.overflow_button().label().as_deref(),
                Some("Updated overflow")
            );
            window.set_content(None);
            window.close();
            Dispatcher::ui_thread().run_jobs(None);
        }
    });
}

#[test]
fn downstream_compiled_and_runtime_roots_refresh_accessible_names_and_release_subscriptions() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::downstream_compiled_and_runtime_roots_refresh_accessible_names_and_release_subscriptions"
    )) {
        return;
    }
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        let event = RibbonStrings::current_changed();
        let baseline = event.handler_count();
        for runtime in [false, true] {
            RibbonStrings::set_current(Some(
                RibbonStrings::for_culture(Some(RibbonCulture::new("en".into(), "en".into())))
                    .unwrap(),
            ))
            .unwrap();
            let weak = {
                let value = if runtime {
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                        include_str!("LocalizedRibbon.xaml"),
                        Some(&crate::CONSUMER_ASSEMBLY),
                    )
                    .unwrap()
                } else {
                    crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferroribbon_markup_consumer/LocalizedRibbon.xaml",
                    )
                    .unwrap()
                    .unwrap()
                };
                let root = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<ConsumerRibbon>()
                    .unwrap();
                let events = event.clone();
                root.loaded(move |_, _| {
                    assert_eq!(events.handler_count(), baseline + 1);
                    RibbonStrings::set_current(Some(
                        RibbonStrings::for_culture(Some(RibbonCulture::new(
                            "fr".into(),
                            "fr".into(),
                        )))
                        .unwrap(),
                    ))
                    .unwrap();
                });
                let events = event.clone();
                root.unloaded(move |_, _| {
                    assert_eq!(events.handler_count(), baseline);
                    RibbonStrings::set_current(Some(
                        RibbonStrings::for_culture(Some(RibbonCulture::new(
                            "en".into(),
                            "en".into(),
                        )))
                        .unwrap(),
                    ))
                    .unwrap();
                });
                let window = Window::new();
                window.set_content(Some(Control::boxed(root.clone())));
                window.show();
                window.measure(Size::new(1000., 300.));
                window.arrange(Rect::new(0., 0., 1000., 300.));
                Dispatcher::ui_thread().run_jobs(None);
                assert!(root.templates() > 0);
                assert_eq!(event.handler_count(), baseline + 1);
                let options = root.display_options_button().unwrap();
                let peer = ControlAutomationPeer::create_peer_for_element(&options);
                assert_eq!(peer.get_name(), "Options d'affichage du ruban");
                for culture in ["fr", "de", "pl", "es"] {
                    let strings = RibbonStrings::for_culture(Some(RibbonCulture::new(
                        culture.into(),
                        culture.into(),
                    )))
                    .unwrap();
                    let expected = strings.ribbon_display_options().unwrap().to_utf8().unwrap();
                    let before = peer.get_name();
                    RibbonStrings::set_current(Some(strings)).unwrap();
                    assert_eq!(peer.get_name(), before, "refresh is deferred");
                    Dispatcher::ui_thread().run_jobs(None);
                    assert_eq!(peer.get_name(), expected);
                    assert_eq!(
                        ToolTip::get_tip(&options).unwrap().downcast_ref::<String>(),
                        Some(&expected)
                    );
                    let tab = root.tabs().get(0).unwrap();
                    assert_eq!(
                        tab.overflow_button().label(),
                        RibbonStrings::current()
                            .more_options()
                            .map(|v| v.to_utf8().unwrap())
                    );
                    assert_eq!(
                        tab.header().as_deref(),
                        Some("Home"),
                        "application headers are preserved"
                    );
                }
                window.set_content(None);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(event.handler_count(), baseline);
                assert_eq!(
                    peer.get_name(),
                    "Opciones de presentación de la cinta",
                    "a change from the application Unloaded handler cannot refresh the detached root"
                );
                window.close();
                root.downgrade()
            };
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                weak.upgrade().is_none(),
                "compiled/runtime root releases without a static-event cycle"
            );
        }
    });
}
