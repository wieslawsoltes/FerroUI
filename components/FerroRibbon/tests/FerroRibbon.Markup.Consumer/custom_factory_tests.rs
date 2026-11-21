//! External consumer of source-style custom model markup and native virtuals.
use ferroribbon::{
    controls::{Ribbon, RibbonGroup},
    metadata::models::RibbonCustomItemModelMetadata,
    mvvm::{RibbonElementFactory, RibbonModelBindings},
};
use ferroribbon_core::model::*;
use ferroui_base::{
    BoxedValue,
    data::core::{ValueType, ValueTypes},
    metadata::MarkupTyped,
    threading::Dispatcher,
};
use ferroui_controls::{
    Button, Control, StackPanel, TextBlock, Window, presenters::ContentPresenter,
};
use ferroui_system::collections::object_model::ObservableCollection;
use std::rc::Rc;
fn cast<T: Clone + 'static>(v: &BoxedValue) -> T {
    ValueTypes::try_cast(v, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}
#[test]
fn independent_compiled_runtime_custom_models_template_bindings_subclass_dispatch_and_recreation() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("FactoryCustom.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/FactoryCustom.xaml",
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
                    .try_get_resource(&key.into(), None)
                    .flatten()
                    .unwrap()
            };
            let factory = ValueTypes::as_object(&*resource("Factory"))
                .unwrap()
                .cast::<crate::factory_consumer::ConsumerElementFactory>()
                .unwrap();
            let model = cast::<Rc<dyn RibbonCustomItemModelImpl>>(&resource("Data"));
            let direct = cast::<Rc<dyn RibbonCustomItemModelImpl>>(&resource("Direct"));
            let existing = Control::from_boxed(&resource("Existing")).unwrap();
            let weak_existing = existing.downgrade();
            let ribbon = Ribbon::new();
            ribbon.resources().add("Keyed", Some(resource("Keyed")));
            factory.set_ribbon(Some(ribbon));
            let group = panel.children().get(1).cast::<RibbonGroup>().unwrap();
            let bindings = RibbonModelBindings::new();
            let models: ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>> =
                ObservableCollection::new_managed();
            models.add(Some(model.clone()));
            models.add(Some(direct.clone()));
            let f = factory.clone().upcast::<RibbonElementFactory>();
            let mirror = bindings.items(
                models,
                Rc::new(group.items()),
                Rc::new(move |m, b| f.create_item(m, b)),
                true,
                None,
                None,
            );
            let w = Window::new();
            w.set_width(700.);
            w.set_height(500.);
            w.set_content(Some(Control::boxed(panel.clone())));
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(factory.created_custom_count(), 2);
            assert_eq!(
                panel
                    .children()
                    .get(0)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Keyed")
            );
            let old = group
                .items()
                .get(0)
                .unwrap()
                .cast::<ContentPresenter>()
                .unwrap();
            assert!(old.content_template().is_some());
            assert!(
                old.get_visual_descendants()
                    .filter_map(|v| v.cast::<TextBlock>())
                    .any(|t| t.text().as_deref() == Some("Template from markup")),
                "native compiled DataTemplate rendered"
            );
            assert!(group.items().get(1).unwrap().ptr_eq(&existing));
            assert_eq!(model.id().unwrap().to_utf8().unwrap(), "data");
            assert!(!model.can_add_to_quick_access());
            let new = Button::new();
            new.set_content(Some(Rc::new("Switched native content".to_owned())));
            // Use the public metadata setter at the native/Core object boundary.
            let property = RibbonCustomItemModelMetadata::MARKUP
                .properties
                .iter()
                .find(|p| p.name == "Content")
                .unwrap();
            let target: BoxedValue = Rc::new(model.clone());
            (property.set.unwrap())(&[Some(target), Some(Control::boxed(new.clone()))]).unwrap();
            assert_eq!(factory.created_custom_count(), 3);
            assert!(group.items().get(0).unwrap().ptr_eq(&new));
            assert!(old.get_logical_parent().is_none());
            assert!(ferroribbon::mvvm::RibbonElementFactory::get_model(&new).is_some());
            bindings.dispose();
            assert_eq!(group.items().count(), 0);
            drop(mirror);
            w.close();
            w.set_content(None);
            drop(panel);
            drop(value);
            drop(direct);
            drop(existing);
            // Release legitimate view/template/factory/model owners before
            // checking for an otherwise-unrooted model/content cycle.
            drop(old);
            drop(new);
            drop(model);
            drop(group);
            drop(factory);
            drop(bindings);
            assert!(
                weak_existing.upgrade().is_none(),
                "no model/content association cycle"
            );
        }
    });
}
#[test]
fn independent_custom_constructor_metadata_preserves_optional_arguments_payload_identity_and_defaults()
 {
    crate::tests::run(|| {
        let constructors = RibbonCustomItemModelMetadata::MARKUP.constructors;
        assert_eq!(constructors.len(), 3);
        let payload: BoxedValue = Rc::new("payload".to_owned());
        for (arity, ctor) in constructors.iter().enumerate() {
            let args = [
                Some(Rc::new("custom".to_owned()) as BoxedValue),
                Some(payload.clone()),
            ];
            let value = (ctor.invoke)(&args[..arity]).unwrap().unwrap();
            let m = cast::<Rc<dyn RibbonCustomItemModelImpl>>(&value);
            assert!(m.label().is_none());
            assert!(m.template_key().is_none());
            assert!(!m.can_add_to_quick_access());
            if arity == 0 {
                assert_eq!(m.id().unwrap().as_utf16().len(), 32);
            } else {
                assert_eq!(m.id().unwrap().to_utf8().unwrap(), "custom");
            }
            if arity == 2 {
                let original: Rc<dyn std::any::Any> = payload.clone();
                assert!(Rc::ptr_eq(&m.content().unwrap().payload(), &original));
            } else {
                assert!(m.content().is_none());
            }
        }
    });
}
