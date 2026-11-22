//! Independent compiler/interpreter regression: metadata value constructors,
//! including a framework-independent value represented by a separate descriptor.
extern crate self as value_constructor_fixture;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition};
pub use values::CountedValue;

mod values {
    use std::cell::Cell;
    ferroui_base::ferro_thread_local! { static CALLS: Cell<u32> = const { Cell::new(0) }; }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CountedValue {
        pub rows: i32,
        pub columns: i32,
    }
    impl CountedValue {
        pub fn new(rows: i32, columns: i32) -> Self {
            CALLS.with(|calls| calls.set(calls.get() + 1));
            Self { rows, columns }
        }
        pub fn zero() -> Self {
            Self::new(0, 0)
        }
        pub fn reject(_label: String) -> Result<Self, String> {
            CALLS.with(|calls| calls.set(calls.get() + 1));
            Err("value constructor rejected its argument".into())
        }
        pub fn reset_calls() {
            CALLS.with(|calls| calls.set(0));
        }
        pub fn calls() -> u32 {
            CALLS.with(Cell::get)
        }
    }
}

mod unnamed_values {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct NoPublicValue;
    impl NoPublicValue {
        pub fn new() -> Self {
            Self
        }
    }
}
pub struct NoPublicValueMetadata;
ferroui_base::ferro_markup_type!(struct NoPublicValueMetadata as "NoPublicValue" {
    namespace: "ValueConstructors", this: unnamed_values::NoPublicValue, handles: [unnamed_values::NoPublicValue],
    constructors: [() => unnamed_values::NoPublicValue::new]
});

pub struct CountedValueMetadata;
ferroui_base::ferro_markup_type!(struct CountedValueMetadata as "CountedValue" {
    namespace: "ValueConstructors", this: CountedValue, handles: [CountedValue],
    constructors: [() => CountedValue::zero, (i32, i32) => CountedValue::new, try (String) => CountedValue::reject],
    properties: [Rows: i32 {get: |value: &CountedValue| value.rows}, Columns: i32 {get: |value: &CountedValue| value.columns}]
});

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ValueConstructors",
    crate_name: "value_constructor_fixture",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: "urn:value-constructors",
        namespace: "ValueConstructors",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {
    classes: [], types: [crate::CountedValueMetadata, crate::NoPublicValueMetadata], contracts: [], generics: [], generic_contracts: [],
}
// Verified with this fixture under -D warnings: the emitter clones every retained
// typed value and does not carry Rust Copy-trait metadata. Keep the one generated
// lint allowance local; handwritten fixture code retains strict lint checking.
#[allow(clippy::clone_on_copy)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_with_source_info, compiled_xaml};
pub fn register_types() {
    ferroui_markup_xaml::register_types();
    MarkupType::register_all(&[CountedValueMetadata::MARKUP, NoPublicValueMetadata::MARKUP]);
    MarkupAssembly::register(&ASSEMBLY);
    ferroui_base::data::core::ValueTypes::register_erased_value::<CountedValue>();
    register_rust_paths();
    ferroui_base::metadata::register_type_rust_paths(&[(
        || std::any::TypeId::of::<CountedValue>(),
        "value_constructor_fixture::CountedValue",
    )]);
    compiled_markup::register();
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{Thickness, controls::ResourceDictionary, data::core::ValueTypes};
    use ferroui_controls::ContentControl;
    use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
    use ferroui_markup_xaml_loader::{FerroRuntimeXamlLoader, rust_emitter::compile_documents};

    fn services() -> Option<std::rc::Rc<dyn ferroui_base::metadata::IServiceProvider>> {
        Some(ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers::create_root_service_provider_v3(None))
    }
    #[test]
    fn value_roots_are_typed_and_constructed_exactly_once_by_both_backends() {
        register_types();
        CountedValue::reset_calls();
        let value: CountedValue = compiled_xaml::build_value_xaml(services()).unwrap();
        assert_eq!((value.rows, value.columns), (2, 3));
        assert_eq!(CountedValue::calls(), 1);
        CountedValue::reset_calls();
        let value = FerroRuntimeXamlLoader::parse(include_str!("Value.xaml"), None).unwrap();
        assert_eq!(
            value.downcast_ref::<CountedValue>().unwrap(),
            &CountedValue {
                rows: 2,
                columns: 3
            }
        );
        assert_eq!(CountedValue::calls(), 1);
        CountedValue::reset_calls();
        let value: CountedValue = compiled_xaml::build_default_xaml(services()).unwrap();
        assert_eq!((value.rows, value.columns), (0, 0));
        assert_eq!(CountedValue::calls(), 1);
    }
    #[test]
    fn native_value_root_and_resource_values_match_both_backends() {
        register_types();
        let value: Thickness = compiled_xaml::build_thickness_xaml(services()).unwrap();
        assert_eq!(value, Thickness::new(1.25, 2.5, 3.75, 4.0));
        for compiled in [false, true] {
            CountedValue::reset_calls();
            let dict = if compiled {
                compiled_xaml::build_resources_xaml(services()).unwrap()
            } else {
                let root =
                    FerroRuntimeXamlLoader::parse(include_str!("Resources.xaml"), None).unwrap();
                ValueTypes::as_object(&*root)
                    .unwrap()
                    .cast::<ResourceDictionary>()
                    .unwrap()
            };
            let value = dict
                .try_get_resource(&"value".into(), None)
                .flatten()
                .unwrap();
            assert_eq!(
                value.downcast_ref::<CountedValue>().unwrap(),
                &CountedValue {
                    rows: 4,
                    columns: 5
                }
            );
            assert_eq!(CountedValue::calls(), 1);
            CountedValue::reset_calls();
            let control = if compiled {
                compiled_xaml::build_property_xaml(services()).unwrap()
            } else {
                let root =
                    FerroRuntimeXamlLoader::parse(include_str!("Property.xaml"), None).unwrap();
                ValueTypes::as_object(&*root)
                    .unwrap()
                    .cast::<ContentControl>()
                    .unwrap()
            };
            let value = control.content().unwrap();
            assert_eq!(
                value.downcast_ref::<CountedValue>().unwrap(),
                &CountedValue {
                    rows: 6,
                    columns: 7
                }
            );
            assert_eq!(CountedValue::calls(), 1);
        }
    }
    #[test]
    fn fallible_value_constructor_preserves_error_and_invocation_count() {
        register_types();
        CountedValue::reset_calls();
        let error = compiled_xaml::build_rejected_xaml(services()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("value constructor rejected its argument"),
            "{error}"
        );
        assert_eq!(CountedValue::calls(), 1);
        CountedValue::reset_calls();
        let error = FerroRuntimeXamlLoader::parse(include_str!("Rejected.xaml"), None).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("value constructor rejected its argument"),
            "{error}"
        );
        assert_eq!(CountedValue::calls(), 1);
    }
    #[test]
    fn value_root_with_source_info_is_constructed_once() {
        register_types();
        CountedValue::reset_calls();
        let value: CountedValue = compiled_with_source_info::build_value_xaml(services()).unwrap();
        assert_eq!((value.rows, value.columns), (2, 3));
        assert_eq!(CountedValue::calls(), 1);
        CountedValue::reset_calls();
        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        configuration.set_create_source_info(true);
        let value = FerroRuntimeXamlLoader::load_document(
            ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(include_str!("Value.xaml")),
            Some(configuration),
        );
        let value = value.unwrap();
        assert_eq!(
            value.downcast_ref::<CountedValue>().unwrap(),
            &CountedValue {
                rows: 2,
                columns: 3
            }
        );
        assert_eq!(CountedValue::calls(), 1);
    }
    #[test]
    fn runtime_emitter_uses_public_value_paths_and_single_constructor_expressions() {
        register_types();
        let docs = compile_documents(
            &[
                ("Value.xaml", include_str!("Value.xaml")),
                ("Thickness.xaml", include_str!("Thickness.xaml")),
            ],
            None,
            &RuntimeXamlLoaderConfiguration::new(),
            &[],
        );
        for (doc, value_type) in docs.iter().zip([
            "::value_constructor_fixture::CountedValue",
            "::ferroui_base::Thickness",
        ]) {
            let source = doc.source.as_ref().unwrap();
            assert!(
                source.contains(&format!("Result<{value_type},")),
                "{source}"
            );
            assert_eq!(source.matches("::__markup_new_").count(), 1, "{source}");
            assert!(!source.contains("::values::CountedValue"), "{source}");
            assert!(!source.contains("::thickness::Thickness"), "{source}");
        }
    }
    #[test]
    fn runtime_emitter_requires_a_recorded_public_value_path() {
        register_types();
        let xaml = "<NoPublicValue xmlns='urn:value-constructors'/>";
        let root = FerroRuntimeXamlLoader::parse(xaml, None).unwrap();
        assert!(
            root.downcast_ref::<unnamed_values::NoPublicValue>()
                .is_some()
        );
        let docs = compile_documents(
            &[("Unnamed.xaml", xaml)],
            None,
            &RuntimeXamlLoaderConfiguration::new(),
            &[],
        );
        let error = docs[0].source.as_ref().unwrap_err();
        assert!(
            error.contains("no public Rust path is recorded for the root value"),
            "{error}"
        );
    }
    #[test]
    fn invalid_value_constructor_arguments_are_errors_in_both_backends() {
        register_types();
        for arguments in [
            "<x:Int32>1</x:Int32>",
            "<x:Int32>1</x:Int32><x:Int32>2</x:Int32><x:Int32>3</x:Int32>",
            "<x:String>bad</x:String><x:Int32>2</x:Int32>",
        ] {
            let xaml = format!(
                "<CountedValue xmlns='urn:value-constructors' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><x:Arguments>{arguments}</x:Arguments></CountedValue>"
            );
            CountedValue::reset_calls();
            assert!(FerroRuntimeXamlLoader::parse(&xaml, None).is_err());
            assert_eq!(CountedValue::calls(), 0);
            let docs = compile_documents(
                &[("Invalid.xaml", &xaml)],
                None,
                &RuntimeXamlLoaderConfiguration::new(),
                &[],
            );
            assert!(docs[0].source.is_err());
            assert_eq!(CountedValue::calls(), 0);
        }
    }
}
