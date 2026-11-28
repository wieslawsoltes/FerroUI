#[test]
fn source_split_markup_content_two_way_binding_and_downstream_virtual_dispatch() {
    super::tests::run(|| {
        use ferroribbon::controls::{IRibbonItem, RibbonSplitButton};
        use ferroui_base::{
            StaticType, TypeInfo,
            data::core::{ValueType, ValueTypes},
        };
        use ferroui_controls::{Control, MenuFlyout, StackPanel, Window};
        use std::rc::Rc;
        let _theme = super::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::split_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[<crate::split_consumer::ConsumerSplit as StaticType>::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Split.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Split.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let source = panel
                .children()
                .get(0)
                .cast::<crate::split_consumer::ConsumerSplit>()
                .unwrap();
            let mirror = panel.children().get(1).cast::<RibbonSplitButton>().unwrap();
            assert_eq!(
                source
                    .flyout()
                    .unwrap()
                    .cast::<MenuFlyout>()
                    .unwrap()
                    .items()
                    .view()
                    .count(),
                1
            );
            source.set_is_checked(Some(true));
            assert_eq!(mirror.is_checked(), Some(true));
            mirror.set_is_checked(None);
            assert_eq!(source.is_checked(), None);
            let checked = ValueTypes::try_cast(
                &Control::boxed(source.clone()),
                ValueType::of::<Rc<dyn ferroribbon::controls::IRibbonCheckable>>(),
            )
            .unwrap();
            checked
                .downcast_ref::<Rc<dyn ferroribbon::controls::IRibbonCheckable>>()
                .unwrap()
                .set_is_checked(Some(false));
            let item = ValueTypes::try_cast(
                &Control::boxed(source.clone()),
                ValueType::of::<Rc<dyn IRibbonItem>>(),
            )
            .unwrap();
            assert!(item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap().invoke());
            assert_eq!(source.calls(), 1);
            assert_eq!(mirror.is_checked(), Some(true));
            let w = Window::new();
            w.set_content(Some(Control::boxed(panel)));
            w.show();
            w.measure(ferroui_base::Size::new(600., 300.));
            w.arrange(ferroui_base::Rect::new(0., 0., 600., 300.));
            assert!(
                source
                    .get_visual_descendants()
                    .any(|c| c.name().as_deref() == Some("PART_PrimaryButton"))
            );
            w.close();
            w.set_content(None);
        }
    });
}
