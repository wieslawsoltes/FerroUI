//! Native equivalents of unchanged upstream XamlNamespaceTests.cs at 9755e281.
//! Copyright Wiesław Šoltés, MIT.
use ferroribbon::{controls::*, primitives::RibbonIconPresenter};
use ferroui_base::{BoxedValue, Ref, controls::NameScope, data::core::ValueTypes};
use ferroui_controls::{Control, StackPanel, TextBlock, UserControl, Window};
use std::rc::Rc;

fn documents(name: &str, text: &str) -> [BoxedValue; 2] {
    [
        crate::compiled_xaml::try_load(
            None,
            &format!("ferres://ferroribbon_markup_consumer/{name}.xaml"),
        )
        .unwrap()
        .unwrap(),
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            text,
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap(),
    ]
}

fn mount(value: BoxedValue) -> (Ref<UserControl>, Ref<Window>) {
    let view = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<UserControl>()
        .unwrap();
    let window = Window::new();
    window.set_width(1200.);
    window.set_height(500.);
    window.set_content(Some(Rc::new(view.clone())));
    window.show();
    window.update_layout();
    (view, window)
}

fn named(view: &UserControl, name: &str) -> Ref<Control> {
    NameScope::get_name_scope(view)
        .unwrap()
        .find(name)
        .unwrap()
        .cast()
        .unwrap()
}

fn content(view: &UserControl) -> Ref<Control> {
    ValueTypes::as_object(&*view.content().unwrap())
        .unwrap()
        .cast()
        .unwrap()
}

#[test]
fn declared_xml_namespace_uri_resolves_types() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for value in documents("ExplicitXmlnsView", include_str!("ExplicitXmlnsView.xaml")) {
            let (view, window) = mount(value);
            let ribbon = named(&view, "RibbonElement").cast::<Ribbon>().unwrap();
            assert!(
                ribbon
                    .find_item(Some("cut".into()))
                    .unwrap()
                    .cast::<RibbonButton>()
                    .is_some()
            );
            assert_eq!(
                RibbonXmlns::URI,
                "https://github.com/wieslawsoltes/RibbonSpace"
            );
            window.close();
            window.set_content(None);
        }
    });
}

#[test]
fn recommended_rs_prefix_resolves_controls_and_primitives_with_native_declaration() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for value in documents("ImplicitXmlnsView", include_str!("ImplicitXmlnsView.xaml")) {
            let (view, window) = mount(value);
            let panel = content(&view).cast::<StackPanel>().unwrap();
            let ribbon = named(&view, "RibbonElement").cast::<Ribbon>().unwrap();
            assert!(panel.children().get(0).ptr_eq(&ribbon));
            assert!(
                ribbon
                    .find_item(Some("bold".into()))
                    .unwrap()
                    .cast::<RibbonToggleButton>()
                    .is_some()
            );
            assert!(
                panel
                    .children()
                    .get(1)
                    .cast::<RibbonIconPresenter>()
                    .is_some()
            );
            window.close();
            window.set_content(None);
        }
    });
}

#[test]
fn unprefixed_types_property_elements_templates_and_attached_properties_resolve() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for value in documents("GlobalXmlnsView", include_str!("GlobalXmlnsView.xaml")) {
            let (view, window) = mount(value);
            let panel = content(&view).cast::<StackPanel>().unwrap();
            let title = named(&view, "TitleBarElement")
                .cast::<RibbonTitleBar>()
                .unwrap();
            let ribbon = named(&view, "RibbonElement").cast::<Ribbon>().unwrap();
            assert!(panel.children().get(1).ptr_eq(&ribbon));
            let find = ribbon
                .find_item(Some("find".into()))
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            assert_eq!(find.min_width(), 80.);
            assert_eq!(RibbonKeyTip::get_key_tip(&find).as_deref(), Some("FD"));
            assert_eq!(title.title().as_deref(), Some("Document"));
            let ids = ribbon.get_quick_access_item_ids();
            assert_eq!(ids.count(), 1);
            assert_eq!(ids.get(0), Some("save".into()));
            let toolbar = panel.children().get(2).cast::<RibbonToolBar>().unwrap();
            assert_eq!(
                RibbonContextualToolBar::get_context(&toolbar.items().get(0).unwrap()).as_deref(),
                Some("pen")
            );
            assert!(
                panel
                    .children()
                    .get(3)
                    .cast::<RibbonIconPresenter>()
                    .is_some()
            );
            window.close();
            window.set_content(None);
        }
    });
}

#[test]
fn public_namespace_constants_work_in_rust_compiled_and_runtime_markup() {
    crate::tests::run(|| {
        let prefix = ferroribbon::ASSEMBLY
            .xmlns_prefixes
            .iter()
            .find(|entry| entry.xml_namespace == RibbonXmlns::URI)
            .unwrap();
        assert_eq!(prefix.prefix, RibbonXmlns::PREFIX);
        for value in documents(
            "NamespaceConstants",
            include_str!("NamespaceConstants.xaml"),
        ) {
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            for (index, expected) in [
                RibbonXmlns::URI,
                RibbonXmlns::PREFIX,
                RibbonXmlns::GLOBAL_URI,
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(
                    panel
                        .children()
                        .get(index)
                        .cast::<TextBlock>()
                        .unwrap()
                        .text()
                        .as_deref(),
                    Some(expected)
                );
            }
        }
    });
}

#[test]
fn original_global_uri_is_an_explicit_alias_for_controls_and_primitives() {
    crate::tests::run(|| {
        for value in documents(
            "LegacyGlobalNamespace",
            include_str!("LegacyGlobalNamespace.xaml"),
        ) {
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let ribbon = panel.children().get(0).cast::<Ribbon>().unwrap();
            assert!(ribbon.find_item(Some("legacy".into())).is_some());
            assert!(
                panel
                    .children()
                    .get(1)
                    .cast::<RibbonIconPresenter>()
                    .is_some()
            );
        }
    });
}
