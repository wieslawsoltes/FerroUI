//! A separate dependency consumer exercises compiled and runtime source-style menu XAML.
#[test]
fn source_style_menus_icons_attached_property_collections_and_compiled_binding() {
    super::tests::run(|| {
        use ferroui_base::{StaticType, TypeInfo};
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::menu_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[<crate::menu_consumer::ConsumerMenu as StaticType>::TYPE]);
        let _theme = super::tests::NativeThemeScope::new();
        use ferroribbon::{compat::*, controls::RibbonButton};
        use ferroui_base::data::core::ValueTypes;
        use ferroui_controls::{MenuFlyout, StackPanel};
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Menus.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Menus.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let button = panel.children().get(0).cast::<RibbonButton>().unwrap();
            let flyout = button.flyout().unwrap().cast::<MenuFlyout>().unwrap();
            let get = |index| {
                ferroui_controls::Control::from_boxed(&flyout.items().view().get_at(index).unwrap())
                    .unwrap()
            };
            let first = get(0).cast::<MenuFlyoutItem>().unwrap();
            assert_eq!(first.text().as_deref(), Some("_Literal-"));
            assert_eq!(
                first.keyboard_accelerator_text_override(),
                "Localized shortcut"
            );
            assert!(first.icon().is_some());
            first.invoke_menu_core();
            assert_eq!(
                first
                    .cast::<crate::menu_consumer::ConsumerMenu>()
                    .unwrap()
                    .calls(),
                1
            );
            let source = get(1).cast::<ToggleMenuFlyoutItem>().unwrap();
            let mirror = get(2).cast::<ToggleMenuFlyoutItem>().unwrap();
            assert!(mirror.is_checked());
            mirror.set_is_checked(false);
            assert!(!source.is_checked());
            source.set_is_checked(true);
            assert!(mirror.is_checked());
            let sub = get(3).cast::<MenuFlyoutSubItem>().unwrap();
            assert_eq!(sub.items().count(), 3);
            assert!(RadioMenuFlyoutItem::get_are_check_states_enabled(&sub));
            let original = sub.items().get(0).unwrap();
            let radio = original.cast::<RadioMenuFlyoutItem>().unwrap();
            assert!(radio.is_checked());
            assert_eq!(radio.group_name().as_deref(), Some("consumer"));
            let overflow = button.create_overflow_menu_items_core();
            let copy = overflow
                .iter()
                .next()
                .unwrap()
                .unwrap()
                .cast::<MenuFlyoutSubItem>()
                .unwrap();
            assert_eq!(copy.items().count(), 4);
            assert!(!copy.items().get(3).unwrap().ptr_eq(&sub));
        }
    });
}
#[test]
fn public_menu_static_overloads_are_invocable_from_dependency_metadata() {
    super::tests::run(|| {
        use ferroribbon::{
            compat::{ImageIcon, MenuFlyoutItem},
            controls::{RibbonButton, RibbonItemHelper, RibbonMenu},
            primitives::{RibbonDeferredImage, RibbonImageLoadError},
        };
        use ferroui_base::Ref;
        use ferroui_base::metadata::{MarkupTyped, from_markup_value, into_markup_value};
        use ferroui_controls::IconElement;
        for method in RibbonMenu::MARKUP.find_methods("Item") {
            let mut args = vec![into_markup_value(Some("label".to_owned())), None];
            while args.len() < method.parameters.len() {
                args.push(None);
            }
            let value = (method.invoke)(&args).unwrap();
            let item = from_markup_value::<Ref<MenuFlyoutItem>>(&value).unwrap();
            assert_eq!(item.text().as_deref(), Some("label"));
        }
        assert_eq!(RibbonMenu::MARKUP.find_methods("Item").count(), 3);
        for method in RibbonItemHelper::MARKUP.find_methods("CreateMenuItem") {
            let source = RibbonButton::new();
            source.set_is_enabled(false);
            let mut args = vec![
                into_markup_value(source),
                into_markup_value(Some("overflow".to_owned())),
                None,
                None,
            ];
            while args.len() < method.parameters.len() {
                args.push(into_markup_value(Some(true)));
            }
            let value = (method.invoke)(&args).unwrap();
            let item = from_markup_value::<Ref<ferroui_controls::Control>>(&value)
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap();
            assert!(!item.is_enabled());
            assert_eq!(item.text().as_deref(), Some("overflow"));
        }
        assert_eq!(
            RibbonItemHelper::MARKUP
                .find_methods("CreateMenuItem")
                .count(),
            2
        );
        let image = RibbonItemHelper::MARKUP
            .find_methods("CreateMenuIcon")
            .next()
            .unwrap();
        let value = (image.invoke)(&[into_markup_value(
            "ferres://FerroRibbon/Assets/missing-consumer-menu-icon.png".to_owned(),
        )])
        .unwrap();
        let icon = from_markup_value::<Option<Ref<IconElement>>>(&value)
            .unwrap()
            .unwrap()
            .cast::<ImageIcon>()
            .unwrap();
        let source = icon.source().unwrap();
        let failed = source
            .as_object()
            .unwrap()
            .downcast_ref::<RibbonDeferredImage>()
            .unwrap();
        assert!(matches!(
            failed.load_result().unwrap().unwrap_err().as_ref(),
            RibbonImageLoadError::Load(_)
        ));
    });
}
