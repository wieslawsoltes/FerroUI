//! The public ribbon types resolve from the default XML namespace, with no prefix, in the compiled
//! and in the run-time loader, and a name has one type.
use ferroribbon::compat::MenuFlyoutSeparator;
use ferroribbon::controls::{RibbonButton, RibbonScreenTip};
use ferroribbon::mvvm::RibbonElementFactory;
use ferroribbon::primitives::RibbonIconPresenter;
use ferroribbon_core::model::RibbonScreenTipModel;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{BoxedValue, PropertyValue, Ref};
use ferroui_controls::StackPanel;
use std::rc::Rc;

fn cast<T: PropertyValue>(value: &BoxedValue) -> T {
    ValueTypes::try_cast(value, ValueType::of::<T>())
        .unwrap()
        .downcast_ref::<T>()
        .unwrap()
        .clone()
}

fn load(runtime: bool) -> Ref<StackPanel> {
    let value = if runtime {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("DefaultNamespaceNames.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap()
    } else {
        crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/DefaultNamespaceNames.xaml",
        )
        .unwrap()
        .unwrap()
    };
    ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<StackPanel>()
        .unwrap()
}

#[test]
fn unprefixed_names_resolve_to_one_type_in_both_loaders() {
    crate::tests::run_standalone(|| {
        for runtime in [false, true] {
            let stack = load(runtime);
            let children = stack.children();
            // The control and the data model of a screen tip are two names.
            assert!(children.get(0).cast::<RibbonScreenTip>().is_some(), "{runtime}");
            assert!(children.get(1).cast::<RibbonButton>().is_some(), "{runtime}");
            assert!(children.get(2).cast::<RibbonIconPresenter>().is_some(), "{runtime}");
            assert!(children.get(3).cast::<MenuFlyoutSeparator>().is_some(), "{runtime}");
            let resource = |key: &str| {
                stack
                    .resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
                    .unwrap_or_else(|| panic!("the resource {key}"))
            };
            let model = cast::<Rc<RibbonScreenTipModel>>(&resource("Model"));
            assert_eq!(model.title().map(|title| title.to_utf8().unwrap()).as_deref(), Some("Model"));
            assert!(
                ValueTypes::as_object(&*resource("Factory"))
                    .and_then(|factory| factory.cast::<RibbonElementFactory>())
                    .is_some(),
                "{runtime}"
            );
        }
    });
}
