//! Independent compiled/runtime templates consumed by the public factory/model API.
use ferroribbon::{
    controls::Ribbon,
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::model::*;
use ferroui_base::{
    BoxedValue, Ref,
    controls::ResourceDictionary,
    data::core::{ValueType, ValueTypes},
    styling::ThemeVariant,
    threading::Dispatcher,
};
use ferroui_controls::{
    Control, StackPanel, TextBlock, Window, presenters::ContentPresenter, templates::IDataTemplate,
};
use std::rc::Rc;
fn cast<T: Clone + 'static>(value: &BoxedValue) -> T {
    ValueTypes::try_cast(value, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
fn assert_template_text(p: &ContentPresenter, expected: &str) {
    assert!(
        p.get_visual_descendants()
            .filter_map(|c| c.cast::<TextBlock>())
            .any(|t| t.text().as_deref() == Some(expected)),
        "expected rendered template {expected}"
    );
}
#[test]
fn independent_compiled_runtime_includes_preserve_custom_template_precedence_identity_and_updates()
{
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("FactoryCustomResources.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/FactoryCustomResources.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let resource = |key: &str| {
                panel
                    .resources()
                    .try_get_value(&key.into())
                    .unwrap()
                    .unwrap()
            };
            let factory = ValueTypes::as_object(&*resource("Factory"))
                .unwrap()
                .cast::<crate::factory_consumer::ConsumerElementFactory>()
                .unwrap();
            let model = cast::<Rc<dyn RibbonCustomItemModelImpl>>(&resource("Custom"));
            let lookup = ValueTypes::as_object(&*resource("LookupResources"))
                .unwrap()
                .cast::<ResourceDictionary>()
                .unwrap();
            let ribbon = Ribbon::new();
            ribbon.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(ThemeVariant::dark()),
            );
            assert_eq!(ribbon.actual_theme_variant(), Some(ThemeVariant::dark()));
            ribbon.resources().add_merged_dictionary(lookup.clone());
            factory.set_ribbon(Some(ribbon.clone()));
            let bindings = RibbonModelBindings::new();
            let element = factory
                .create_item(Some(model.clone()), bindings.clone())
                .unwrap();
            let presenter = element.cast::<ContentPresenter>().unwrap();
            panel.children().add(element);
            let w = Window::new();
            w.set_width(600.);
            w.set_height(280.);
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            w.update_layout();
            assert_template_text(&presenter, "included template");
            assert_eq!(factory.created_custom_count(), 1);
            let include = lookup.merged_dictionaries().get(1);
            let loaded = include
                .resource_dictionary()
                .expect("erased native include dictionary service");
            let first = cast::<Rc<dyn IDataTemplate>>(
                &loaded.try_get_value(&"Pick".into()).unwrap().unwrap(),
            );
            assert!(Rc::ptr_eq(&presenter.content_template().unwrap(), &first));
            let again = include.resource_dictionary().unwrap();
            // Native interface views can use distinct adapters for one object.
            // Dictionary equality compares underlying native object identity.
            assert!(*loaded == *again);
            again.set(
                "ConsumerSharedIdentity".into(),
                Some(Rc::new("same dictionary".to_string())),
            );
            assert_eq!(
                loaded
                    .try_get_value(&"ConsumerSharedIdentity".into())
                    .unwrap()
                    .unwrap()
                    .downcast_ref::<String>()
                    .map(String::as_str),
                Some("same dictionary")
            );
            again.remove(&"ConsumerSharedIdentity".into());
            model.set_template_key(Some("Themed".into()));
            w.update_layout();
            assert_template_text(&presenter, "dark template");
            let dark = lookup
                .theme_dictionary(&ThemeVariant::dark())
                .unwrap()
                .resource_dictionary()
                .unwrap();
            dark.set(
                "Themed".into(),
                loaded.try_get_value(&"Alternate".into()).unwrap(),
            );
            model.set_content(Some(RibbonObject::value("changed payload".to_string())));
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            w.update_layout();
            assert_template_text(&presenter, "replacement template");
            model.set_template_key(Some("Pick".into()));
            w.update_layout();
            assert_template_text(&presenter, "included template");
            assert!(Rc::ptr_eq(&presenter.content_template().unwrap(), &first));
            assert_eq!(
                factory.created_custom_count(),
                1,
                "template changes must reuse the presenter"
            );
            assert_eq!(
                panel
                    .children()
                    .get(0)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Pick")
            );
            bindings.dispose();
            model.set_template_key(Some("Themed".into()));
            w.update_layout();
            assert!(Rc::ptr_eq(&presenter.content_template().unwrap(), &first));
            w.close();
            w.set_content(None);
            factory.set_ribbon(None);
            // Native library class remains the same one used for direct construction.
            let _: Ref<RibbonElementFactory> = factory.upcast();
        }
    });
}
