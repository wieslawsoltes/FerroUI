use crate::application_menu_consumer::{ApplicationMenuView, ConsumerApplicationMenuItem};
use ferroribbon::controls::RibbonApplicationMenuItem;
use ferroui_base::data::core::ValueTypes;
use ferroui_controls::TextBlock;
#[test]
fn independent_application_menu_models_compile_load_bind_and_preserve_native_identity() {
    super::tests::run(|| {
        ferroui_base::TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::application_menu_consumer",
            "FerroRibbon.Consumer",
        )]);
        ferroui_base::TypeInfo::register_all(&[
            ApplicationMenuView::TYPE,
            ConsumerApplicationMenuItem::TYPE,
        ]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ApplicationMenuItems.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ApplicationMenuItems.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ApplicationMenuView>()
                .unwrap();
            let item = view.menu_item().unwrap();
            assert_eq!(item.label().as_deref(), Some("Open"));
            assert_eq!(
                item.cast::<ConsumerApplicationMenuItem>()
                    .unwrap()
                    .constructions(),
                1
            );
            assert!(item.items().ptr_eq(&item.items()));
            assert_eq!(item.items().count(), 2);
            assert_eq!(
                item.items().get(0).unwrap().label().as_deref(),
                Some("Drawing")
            );
            assert!(!item.items().get(1).unwrap().is_enabled());
            let count = view.children().get(0).cast::<TextBlock>().unwrap();
            let label = view.children().get(1).cast::<TextBlock>().unwrap();
            assert_eq!(count.text().as_deref(), Some("2"));
            assert_eq!(label.text().as_deref(), Some("Drawing"));
            view.set_header(Some("Save".into()));
            assert_eq!(item.label().as_deref(), Some("Save"));
            item.items().add(Some(RibbonApplicationMenuItem::new()));
            assert_eq!(count.text().as_deref(), Some("3"));
            item.items()
                .get(0)
                .unwrap()
                .set_label(Some("Changed".into()));
            assert_eq!(label.text().as_deref(), Some("Changed"));
            assert!(view.recent_item().unwrap().is_pinned());
            assert_eq!(view.calls(), 0);
            let first = item.items().get(0).unwrap();
            item.items().clear();
            assert_eq!(count.text().as_deref(), Some("0"));
            assert!(
                label.text().unwrap_or_default().is_empty(),
                "an absent indexed item clears the binding"
            );
            item.items().add(Some(first));
            assert_eq!(label.text().as_deref(), Some("Changed"));
            let weak_view = view.downgrade();
            drop(value);
            drop(view);
            assert!(
                weak_view.upgrade().is_none(),
                "retained nonvisual item and bindings do not keep their owner alive"
            );

            // Invocation is tested when the native menu owns these models; the
            // nonvisual public item deliberately has no extra Invoke API.
        }
    });
}

#[test]
fn independent_application_menu_compiled_runtime_native_commands_events_footer_and_reparented_bindings()
 {
    super::tests::run(|| {
        use crate::application_menu_consumer::ConsumerApplicationMenu;
        use ferroribbon::controls::Ribbon;
        use ferroui_controls::{Border, Button, Control, Window, primitives::Popup};
        let _theme = super::tests::NativeThemeScope::new();
        ferroui_base::TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::application_menu_consumer",
            "FerroRibbon.Consumer",
        )]);
        ferroui_base::TypeInfo::register_all(&[
            ApplicationMenuView::TYPE,
            ConsumerApplicationMenu::TYPE,
        ]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ApplicationMenu.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ApplicationMenu.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ApplicationMenuView>()
                .unwrap();
            let menu = view
                .children()
                .get(1)
                .cast::<ConsumerApplicationMenu>()
                .unwrap();
            assert_eq!(menu.constructions(), 1);
            let ribbon = view.children().get(0).cast::<Ribbon>().unwrap();
            assert!(menu.ribbon().unwrap().ptr_eq(&ribbon));
            assert_eq!(menu.recent_header().as_deref(), Some("Documents"));
            assert_eq!(
                view.children()
                    .get(2)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("1")
            );
            let parent = menu.items().get(0).unwrap();
            view.set_header(Some("Save As".into()));
            assert_eq!(parent.label().as_deref(), Some("Save As"));
            let leaf = parent.items().get(0).unwrap();
            let footer = menu.footer_items().get(0).unwrap();
            assert!(footer.is::<Button>());
            view.children().remove(menu.clone().upcast::<Control>());
            let window = Window::new();
            let anchor = Border::new();
            window.set_content(Some(Control::boxed(anchor.clone())));
            window.show();
            let popup = Popup::new();
            popup.set_placement_target(anchor.clone().upcast::<Control>());
            popup.set_parent(anchor);
            popup.set_child(menu.clone().upcast::<Control>());
            popup.set_is_open(true);
            window.update_layout();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(popup.is_inside_popup(&menu));
            view.set_header(Some("Reparented".into()));
            assert_eq!(parent.label().as_deref(), Some("Reparented"));
            menu.invoke(leaf);
            assert!(!popup.is_open());
            assert_eq!(view.command_runs(), 1);
            assert_eq!(view.calls(), 2);
            menu.invoke_recent(menu.recent_items().get(0).unwrap());
            assert_eq!(view.calls(), 4);
            assert!(menu.footer_items().get(0).unwrap().ptr_eq(&footer));
            window.close();
            popup.set_child(None);
        }
    });
}
