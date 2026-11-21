use crate::toolbar_consumer::ConsumerToolBar;
use ferroribbon::controls::{
    RibbonButton, RibbonContextualToolBar, RibbonMenuBar, RibbonMenuBarItem, RibbonToolBar,
};
use ferroui_base::{TypeInfo, controls::NameScope, data::core::ValueTypes};
use ferroui_controls::{Control, TextBlock, Window};
#[test]
fn independent_source_toolbar_markup_compiles_and_loads_runtime_with_subclass_collections_attached_context_and_templates()
 {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::toolbar_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[ConsumerToolBar::TYPE]);
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Toolbars.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Toolbars.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<Control>()
                .unwrap();
            let scope = NameScope::get_name_scope(&root).unwrap();
            let w = Window::new();
            w.set_width(600.);
            w.set_height(400.);
            w.set_content(Some(Control::boxed(root)));
            w.show();
            w.update_layout();
            let t = scope.find_as::<ConsumerToolBar>("Tools").unwrap();
            assert!(t.applied() > 0);
            assert_eq!(t.columns(), -2);
            assert_eq!(t.items().count(), 3);
            assert!(t.overflow_items().count() > 0);
            let held = t.items();
            assert!(held == t.items());
            assert!(held.get(0).unwrap().ptr_eq(&scope.find("Draw").unwrap()));
            let count = scope.find_as::<TextBlock>("Count").unwrap();
            assert_eq!(count.text().as_deref(), Some("3"));
            t.items().add(Some(RibbonButton::new().upcast()));
            w.update_layout();
            assert_eq!(count.text().as_deref(), Some("4"));
            let contexts = scope
                .find_as::<RibbonContextualToolBar>("Contexts")
                .unwrap();
            let text = scope.find_as::<RibbonToolBar>("TextTools").unwrap();
            let draw = scope.find_as::<RibbonToolBar>("DrawTools").unwrap();
            assert!(text.is_visible());
            assert!(!draw.is_visible());
            contexts.set_active_context(Some("draw".into()));
            assert!(!text.is_visible());
            assert!(draw.is_visible());
            let bar = scope.find_as::<RibbonMenuBar>("Menus").unwrap();
            let file = scope.find_as::<RibbonMenuBarItem>("File").unwrap();
            assert_eq!(file.items().count(), 1);
            assert!(
                file.items()
                    .get(0)
                    .unwrap()
                    .ptr_eq(&scope.find("Open").unwrap())
            );
            assert!(bar.open_menu(Some("FILE")));
            assert!(bar.is_menu_open());
            let custom = scope.find_as::<RibbonToolBar>("Custom").unwrap();
            assert!(
                custom
                    .get_template_descendants()
                    .iter()
                    .filter_map(|c| c.cast::<Control>())
                    .any(|c| c.name().as_deref() == Some("PART_ItemsHost"))
            );
            w.close();
        }
    });
}
