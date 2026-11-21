use crate::title_consumer::{ConsumerTitle, TitleView};
use ferroribbon::controls::Ribbon;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    interactivity::RoutedEventArgs,
    metadata::{MarkupType, MarkupTyped},
    threading::Dispatcher,
};
use ferroui_controls::{Button, Control, Window, templates::IControlTemplate};
use std::rc::Rc;
#[test]
fn independent_title_compiled_runtime_binding_retemplate_identity_and_cancellation() {
    super::tests::run(|| {
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[
            (
                "ferroribbon_markup_consumer::title_consumer",
                "FerroRibbon.Consumer",
            ),
            (
                "ferroribbon_markup_consumer::spinner_consumer",
                "FerroRibbon.Consumer",
            ),
        ]);
        ferroui_base::TypeInfo::register_all(&[ConsumerTitle::TYPE, TitleView::TYPE]);
        MarkupType::register_all(&[crate::spinner_consumer::NumberViewModel::MARKUP]);
        ValueTypes::register_reference::<crate::spinner_consumer::NumberViewModel>();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Title.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Title.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<TitleView>()
                .unwrap();
            let bar = view.children().get(1).cast::<ConsumerTitle>().unwrap();
            let ribbon = view.children().get(0).cast::<Ribbon>().unwrap();
            assert!(bar.ribbon().unwrap().ptr_eq(&ribbon));
            let qat = ribbon.quick_access_tool_bar().unwrap();
            let vm = ValueTypes::try_convert(
                view.data_context().as_ref(),
                ValueType::of::<Rc<crate::spinner_consumer::NumberViewModel>>(),
            )
            .flatten()
            .unwrap();
            let vm = vm
                .downcast_ref::<Rc<crate::spinner_consumer::NumberViewModel>>()
                .unwrap()
                .clone();
            let w = Window::new();
            w.set_content(Some(Control::boxed(view.clone())));
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            assert!(bar.templates() > 0);
            assert_eq!(bar.display_title().as_deref(), Some("Document"));
            assert_eq!(bar.subtitle().as_deref(), Some("Saved"));
            assert!(bar.get_visual_descendants().any(|v| v.ptr_eq(&qat)));
            assert_eq!(bar.search_width(), 4.);
            vm.set_value(120.);
            assert_eq!(bar.search_width(), 120.);
            let old_button = bar.app_button().unwrap();
            old_button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(view.calls(), 1);
            let template = view
                .resources()
                .try_get_resource(&"Replacement".into(), None)
                .flatten()
                .unwrap();
            let template = ValueTypes::try_convert(
                Some(&template),
                ValueType::of::<Rc<dyn IControlTemplate>>(),
            )
            .flatten()
            .unwrap();
            bar.set_template(Some(
                template
                    .downcast_ref::<Rc<dyn IControlTemplate>>()
                    .unwrap()
                    .clone(),
            ));
            bar.apply_template();
            assert_eq!(bar.templates(), 2);
            assert!(bar.get_visual_descendants().any(|v| v.ptr_eq(&qat)));
            old_button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(view.calls(), 1, "old template button must be detached");
            bar.app_button()
                .unwrap()
                .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(view.calls(), 2);
            let weak = view.downgrade();
            w.set_content(None);
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            w.close();
            drop((view, value));
            assert!(
                weak.upgrade().is_none(),
                "XAML handlers must not retain the view"
            );
        }
    });
}

#[test]
fn independent_compiled_runtime_title_consumes_public_native_window_attachment() {
    use ferroribbon::controls::RibbonTitleBarWindowState;
    use ferroui_controls::{
        chrome::{TitleBarDecorations, WindowDecorationProperties, WindowDrawnDecorations},
        platform::{IWindowImpl, PlatformRequestedDrawnDecoration},
        testing::{CompositorTestServices, MockWindowingPlatform, TestServices},
    };
    let services = CompositorTestServices::start(TestServices::styled_window());
    ferroribbon::register_types();
    ferroui_markup_xaml::register_types();
    ferroui_base::metadata::MarkupAssembly::register(&crate::CONSUMER_ASSEMBLY);
    ferroui_base::TypeInfo::register_namespaces(&[
        (
            "ferroribbon_markup_consumer::title_consumer",
            "FerroRibbon.Consumer",
        ),
        (
            "ferroribbon_markup_consumer::spinner_consumer",
            "FerroRibbon.Consumer",
        ),
    ]);
    ferroui_base::TypeInfo::register_all(&[ConsumerTitle::TYPE, TitleView::TYPE]);
    MarkupType::register_all(&[crate::spinner_consumer::NumberViewModel::MARKUP]);
    ValueTypes::register_reference::<crate::spinner_consumer::NumberViewModel>();
    crate::compiled_markup::register();
    for runtime in [false, true] {
        let value = if runtime {
            ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                include_str!("Title.xaml"),
                Some(&crate::CONSUMER_ASSEMBLY),
            )
            .unwrap()
        } else {
            crate::compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Title.xaml")
                .unwrap()
                .unwrap()
        };
        let view = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<TitleView>()
            .unwrap();
        let bar = view.children().get(1).cast::<ConsumerTitle>().unwrap();
        let platform = MockWindowingPlatform::create_window_mock();
        platform.needs_managed_decorations.set(true);
        platform
            .requested_drawn_decorations
            .set(PlatformRequestedDrawnDecoration::TITLE_BAR);
        services.setup(&platform);
        let window = Window::with_impl(platform.clone());
        window.set_width(1000.);
        window.set_height(200.);
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        platform.is_client_area_extended_to_decorations.set(true);
        platform
            .extend_client_area_to_decorations_changed()
            .unwrap()(true);
        services.run_jobs();
        window.update_layout();
        services.run_jobs();
        let flags = WindowDrawnDecorations::get_title_bar_decorations(&window);
        assert_eq!(
            bar.attach_to_window(&window),
            Ok(RibbonTitleBarWindowState::Ready)
        );
        assert!(bar.caption_buttons_inset().right > 0.);
        assert_eq!(
            WindowDrawnDecorations::get_title_bar_decorations(&window),
            flags & !TitleBarDecorations::TITLE
        );
        bar.app_button()
            .unwrap()
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(view.calls(), 1);
        bar.detach_from_window();
        assert_eq!(
            bar.window_attachment_state(),
            RibbonTitleBarWindowState::Detached
        );
        assert_eq!(
            WindowDrawnDecorations::get_title_bar_decorations(&window),
            flags
        );
        let called = Rc::new(std::cell::Cell::new(false));
        let seen = called.clone();
        let weak = bar.downgrade();
        let token = window.property_changed(move |event| {
            if event.property() == WindowDecorationProperties::element_role_property().as_property()
                && !seen.replace(true)
            {
                weak.upgrade().unwrap().detach_from_window();
            }
        });
        assert_eq!(
            bar.attach_to_window(&window),
            Ok(RibbonTitleBarWindowState::Detached)
        );
        token.dispose();
        assert!(called.get());
        assert_eq!(
            WindowDrawnDecorations::get_title_bar_decorations(&window),
            flags
        );
        assert_eq!(
            WindowDecorationProperties::get_element_role(&window),
            ferroui_base::input::WindowDecorationsElementRole::None
        );
        bar.attach_to_window(&window).unwrap();
        let called = Rc::new(std::cell::Cell::new(false));
        let seen = called.clone();
        let weak = bar.downgrade();
        let target = window.downgrade();
        let token = window.property_changed(move |event| {
            if event.property() == WindowDecorationProperties::element_role_property().as_property()
                && !seen.replace(true)
            {
                assert_eq!(
                    weak.upgrade()
                        .unwrap()
                        .attach_to_window(&target.upgrade().unwrap()),
                    Ok(RibbonTitleBarWindowState::Pending)
                );
            }
        });
        bar.detach_from_window();
        token.dispose();
        assert!(called.get());
        assert_eq!(
            bar.window_attachment_state(),
            RibbonTitleBarWindowState::Ready
        );
        bar.detach_from_window();
        assert_eq!(
            WindowDrawnDecorations::get_title_bar_decorations(&window),
            flags
        );
        let replacement = ConsumerTitle::new();
        view.children().add(replacement.clone().upcast::<Control>());
        services.run_jobs();
        window.update_layout();
        services.run_jobs();
        assert!(
            replacement.search_box().is_some(),
            "second instance resolves the implicit control theme"
        );
        assert!(
            !replacement
                .search_box()
                .unwrap()
                .ptr_eq(&bar.search_box().unwrap()),
            "templates create independent parts"
        );
        bar.attach_to_window(&window).unwrap();
        let inset = bar.caption_buttons_inset();
        replacement.attach_to_window(&window).unwrap();
        assert_eq!(
            bar.window_attachment_state(),
            RibbonTitleBarWindowState::Superseded
        );
        assert_eq!(bar.caption_buttons_inset(), inset);
        // Source OnApplyTemplate reclaims the associated window even after a
        // different titlebar became the active drag-region owner.
        bar.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
            |_, _| {
                let border = ferroui_controls::Border::new();
                border.set_background(Some(ferroui_base::media::Brushes::transparent()));
                border.upcast()
            },
        )));
        bar.apply_template();
        assert_eq!(
            bar.window_attachment_state(),
            RibbonTitleBarWindowState::Ready
        );
        assert_eq!(
            replacement.window_attachment_state(),
            RibbonTitleBarWindowState::Superseded
        );
        replacement.detach_from_window();
        assert!(window.extend_client_area_to_decorations_hint());
        bar.detach_from_window();
        assert!(!window.extend_client_area_to_decorations_hint());
        assert_eq!(
            WindowDrawnDecorations::get_title_bar_decorations(&window),
            flags
        );
        window.close();
    }
}
