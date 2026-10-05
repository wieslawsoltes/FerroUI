//! Ports of `MarkupExtensionTests.cs` and `ServiceProviderTests.cs`.

use std::cell::Cell;
use std::rc::Rc;

use ferroui_base::metadata::{IServiceProvider, MarkupValue};
use ferroui_base::utilities::Uri;

use crate::runtime::interpreter::services::{
    IProvideValueTarget, IRootObjectProvider, IUriContext, IXamlParentStackProvider, IXamlXmlNamespaceInfoProvider,
};
use crate::runtime::interpreter::XamlXmlNamespaceInfo;
use crate::runtime::type_system::ITypeDescriptorContext;

use super::classes::*;
use super::{boxed, get, HostOptions, TestHost, TestServiceProvider, X};

fn value_provider(value: Object) -> Option<Rc<dyn IServiceProvider>> {
    Some(TestServiceProvider::new().with(ExtensionValueHolder(value)))
}

fn me(attributes: &str) -> String {
    format!("<MarkupExtensionTestsClass xmlns='rt-test' xmlns:x='{X}' {attributes}/>")
}

// --- MarkupExtensionTests ----------------------------------------------------

#[test]
fn object_should_be_casted_to_string() {
    let host = TestHost::new();
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&me("StringProperty='{ObjectTestExtension Returned=test}'"));
    assert_eq!(res.string_property.borrow().as_deref(), Some("test"));
}

#[test]
fn object_should_be_casted_to_value_type() {
    let host = TestHost::new();
    let res = host
        .run_with::<Rc<MarkupExtensionTestsClass>>(&me("IntProperty='{ServiceProviderValue}'"), value_provider(boxed(123)));
    assert_eq!(*res.int_property.borrow(), 123);
}

#[test]
fn object_should_be_casted_to_nullable_value_type() {
    let host = TestHost::new();
    let res = host.run_with::<Rc<MarkupExtensionTestsClass>>(
        &me("NullableIntProperty='{ServiceProviderValue}'"),
        value_provider(boxed(123)),
    );
    assert_eq!(*res.nullable_int_property.borrow(), Some(123));
}

#[test]
fn extensions_should_be_able_to_populate_content_lists() {
    let host = TestHost::new();
    let res = host.run_with::<Rc<MarkupExtensionTestsClass>>(
        "
<MarkupExtensionTestsClass xmlns='rt-test'>
    <ServiceProviderValue/>
    <ServiceProviderValue/>
</MarkupExtensionTestsClass>",
        value_provider(boxed(123)),
    );
    assert_eq!(res.int_list.items(), [123, 123]);
}

#[test]
fn extensions_should_be_able_to_assign_lists() {
    let host = TestHost::new();
    let res = host.run_with::<Rc<MarkupExtensionTestsClass>>(
        &me("IntList2='{ServiceProviderIntList}'"),
        value_provider(boxed(123)),
    );
    assert_eq!(res.int_list2.borrow().items(), [123]);
}

#[test]
fn extensions_which_dont_return_collections_should_not_be_able_to_assign_lists() {
    let host = TestHost::new();
    let error = host.error(&me("IntList2='{ServiceProviderValue}'"), value_provider(boxed(123)));
    assert_eq!(error.type_name(), "InvalidCastException", "{}", error.message());
}

#[test]
fn extensions_should_not_be_able_to_assign_to_read_only_lists() {
    let host = TestHost::new();
    let error = host.error(&me("ReadOnlyIntList='{ServiceProviderIntList}'"), value_provider(boxed(123)));
    assert_eq!(error.type_name(), "XamlLoadException", "{}", error.message());
}

#[test]
fn extensions_should_be_able_to_populate_content_dictionaries() {
    let host = TestHost::new();
    let res = host.run_with::<Rc<MarkupExtensionContentDictionaryClass>>(
        &format!(
            "
<MarkupExtensionContentDictionaryClass xmlns='rt-test' xmlns:x='{X}'>
    <ServiceProviderValue x:Key='First'/>
    <ServiceProviderValue x:Key='Second'/>
</MarkupExtensionContentDictionaryClass>"
        ),
        value_provider(boxed(123)),
    );
    assert_eq!(*res.int_dic.0.borrow(), [("First".to_string(), 123), ("Second".to_string(), 123)]);
}

#[test]
fn non_boxed_value_type_should_be_convertable_to_nullable_and_object() {
    let host = TestHost::new();
    let res = host.run_with::<Rc<MarkupExtensionTestsClass>>(
        &me("NullableIntProperty='{ServiceProviderIntValue}' ObjectProperty='{ServiceProviderIntValue}'"),
        value_provider(boxed(123)),
    );
    assert_eq!(*res.nullable_int_property.borrow(), Some(123));
    assert_eq!(get::<i32>(&res.object_property.borrow()), 123);
}

#[test]
fn unknown_reference_type_to_value_type_should_trigger_invalid_cast_exception() {
    let host = TestHost::new();
    let error = host.error(&me("IntProperty='{ServiceProviderValue}'"), value_provider(boxed("test".to_string())));
    assert_eq!(error.type_name(), "InvalidCastException", "{}", error.message());
}

#[test]
fn value_type_to_reference_type_should_trigger_compile_error() {
    let host = TestHost::new();
    let error = host.error(&me("StringProperty='{ServiceProviderIntValue}'"), value_provider(boxed(123)));
    assert_eq!(error.type_name(), "XamlLoadException", "{}", error.message());
}

#[test]
fn mismatched_value_type_to_value_type_should_trigger_compile_error() {
    let host = TestHost::new();
    let error = host.error(&me("DoubleProperty='{ServiceProviderIntValue}'"), value_provider(boxed(123)));
    assert_eq!(error.type_name(), "XamlLoadException", "{}", error.message());
}

#[test]
fn mismatched_reference_type_to_reference_type_should_trigger_invalid_cast_exception() {
    let host = TestHost::new();
    let value = Uri::absolute("http://test/").expect("a URI");
    let error = host.error(&me("StringProperty='{ServiceProviderValue}'"), value_provider(boxed(value)));
    assert_eq!(error.type_name(), "InvalidCastException", "{}", error.message());
}

#[test]
fn markup_extension_with_directive_should_compile() {
    // An undefined directive reaches the back end, which has no evaluator for it: this shows
    // that the directive inside the extension was parsed.
    let host = TestHost::new();
    let error = host.error(&me("StringProperty='{ObjectTestExtension x:Dir=RTL}'"), None);
    assert_eq!(error.type_name(), "XamlLoadException", "{}", error.message());
    assert!(error.message().contains("XamlAstXmlDirective"), "{}", error.message());
}

#[test]
fn same_name_extension_should_work_without_generics() {
    let host = TestHost::new();
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&me("ObjectProperty='{GenericTestExtension Returned=test}'"));
    assert_eq!(get::<String>(&res.object_property.borrow()), "test");
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&format!(
        "
<MarkupExtensionTestsClass xmlns='rt-test' xmlns:x='{X}'>
    <MarkupExtensionTestsClass.ObjectProperty>
        <GenericTestExtension Returned='test' />
    </MarkupExtensionTestsClass.ObjectProperty>
</MarkupExtensionTestsClass>"
    ));
    assert_eq!(get::<String>(&res.object_property.borrow()), "test");
}

#[test]
fn resolve_single_generic_type_argument() {
    let host = TestHost::new();
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&me(
        "IntProperty='{GenericTestExtension Returned=5, x:TypeArguments=x:Int32}'",
    ));
    assert_eq!(*res.int_property.borrow(), 5);
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&format!(
        "
<MarkupExtensionTestsClass xmlns='rt-test' xmlns:x='{X}'>
    <MarkupExtensionTestsClass.IntProperty>
        <GenericTestExtension Returned='5' x:TypeArguments='x:Int32' />
    </MarkupExtensionTestsClass.IntProperty>
</MarkupExtensionTestsClass>"
    ));
    assert_eq!(*res.int_property.borrow(), 5);
}

#[test]
fn resolve_double_generic_type_argument() {
    let host = TestHost::new();
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&me(
        "ObjectProperty='{GenericTestExtension Returned1=5, Returned2=0.4, x:TypeArguments=\"x:Int32,x:Single\"}'",
    ));
    assert_eq!(get::<(i32, f32)>(&res.object_property.borrow()), (5, 0.4));
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&format!(
        "
<MarkupExtensionTestsClass xmlns='rt-test' xmlns:x='{X}'>
    <MarkupExtensionTestsClass.ObjectProperty>
        <GenericTestExtension Returned1='5' Returned2='0.4' x:TypeArguments='x:Int32,x:Single' />
    </MarkupExtensionTestsClass.ObjectProperty>
</MarkupExtensionTestsClass>"
    ));
    assert_eq!(get::<(i32, f32)>(&res.object_property.borrow()), (5, 0.4));
}

#[test]
fn resolve_single_generic_type_argument_with_nested_extension() {
    let host = TestHost::new();
    let res = host.run::<Rc<MarkupExtensionTestsClass>>(&me(
        "StringProperty='{GenericTestExtension Returned={GenericTestExtension Returned=test, x:TypeArguments=x:String}, x:TypeArguments=x:Object}'",
    ));
    assert_eq!(res.string_property.borrow().as_deref(), Some("test"));
}

// --- ServiceProviderTests ----------------------------------------------------

fn callback_provider(
    callback: impl Fn(Rc<dyn IServiceProvider>) -> Object + 'static,
    parent_stack: Option<Rc<dyn IXamlParentStackProvider>>,
) -> Option<Rc<dyn IServiceProvider>> {
    let provider = TestServiceProvider::new().with(Callback(Rc::new(callback)));
    Some(match parent_stack {
        Some(parent_stack) => provider.with(parent_stack),
        None => provider,
    })
}

struct ListParentsProvider(Vec<MarkupValue>);

impl IXamlParentStackProvider for ListParentsProvider {
    fn parents(&self) -> Vec<MarkupValue> {
        self.0.clone()
    }
}

fn id_of(value: &MarkupValue) -> String {
    get::<Rc<ServiceProviderTestsClass>>(value).id.borrow().clone().unwrap_or_default()
}

#[test]
fn parent_stack_should_provide_info_about_parents() {
    for import_parents in [true, false] {
        let imported: Option<Rc<dyn IXamlParentStackProvider>> = import_parents.then(|| {
            Rc::new(ListParentsProvider(vec![boxed("Parent1".to_string()), boxed("Parent2".to_string())])) as _
        });
        let num = Rc::new(Cell::new(0));
        let counter = num.clone();
        let host = TestHost::new();
        host.build(
            "
<ServiceProviderTestsClass xmlns='rt-test' Id='root' Property='{Callback}'>
    <ServiceProviderTestsClass.Child>
        <ServiceProviderTestsClass Id='direct' Property='{Callback}'/>
    </ServiceProviderTestsClass.Child>
    <ServiceProviderTestsClass Id='content' Property='{Callback}'/>
</ServiceProviderTestsClass>",
            callback_provider(
                move |provider| {
                    let stack =
                        provider.get_service_of::<Rc<dyn IXamlParentStackProvider>>().expect("parent stack").parents();
                    let base_count = if import_parents { 2 } else { 0 };
                    if import_parents {
                        assert_eq!(get::<String>(&stack[stack.len() - 2]), "Parent1");
                        assert_eq!(get::<String>(&stack[stack.len() - 1]), "Parent2");
                    }
                    match counter.get() {
                        0 => {
                            assert_eq!(stack.len(), base_count + 1);
                            assert_eq!(id_of(&stack[0]), "root");
                        }
                        1 => {
                            assert_eq!(stack.len(), base_count + 2);
                            assert_eq!(id_of(&stack[0]), "direct");
                            assert_eq!(id_of(&stack[1]), "root");
                        }
                        2 => {
                            assert_eq!(stack.len(), base_count + 2);
                            assert_eq!(id_of(&stack[0]), "content");
                            assert_eq!(id_of(&stack[1]), "root");
                        }
                        _ => panic!("unexpected call"),
                    }
                    counter.set(counter.get() + 1);
                    boxed("Value".to_string())
                },
                imported,
            ),
        )
        .unwrap_or_else(|e| panic!("{}", e.message()));
        assert_eq!(num.get(), 3);
    }
}

#[test]
fn type_descriptor_context_is_the_context() {
    let called = Rc::new(Cell::new(false));
    let flag = called.clone();
    let host = TestHost::new();
    host.build(
        "<ServiceProviderTestsClass xmlns='rt-test' Property='{Callback}'/>",
        callback_provider(
            move |provider| {
                let context = provider.get_service_of::<Rc<dyn ITypeDescriptorContext>>().expect("the context");
                let as_provider: Rc<dyn IServiceProvider> = context;
                assert!(as_provider.get_service_of::<Rc<dyn IRootObjectProvider>>().is_some());
                flag.set(true);
                boxed("Value".to_string())
            },
            None,
        ),
    )
    .unwrap_or_else(|e| panic!("{}", e.message()));
    assert!(called.get());
}

#[test]
fn provide_value_target_provides_info_about_properties() {
    let num = Rc::new(Cell::new(0));
    let counter = num.clone();
    let host = TestHost::new();
    host.build(
        "
<ServiceProviderTestsClass xmlns='rt-test'
    Property='{Callback}'
    ServiceProviderTests.AttachedProperty='{Callback}'
/>",
        callback_provider(
            move |provider| {
                let target = provider.get_service_of::<Rc<dyn IProvideValueTarget>>().expect("provide value target");
                let object = get::<Rc<ServiceProviderTestsClass>>(&target.target_object());
                match counter.get() {
                    0 => assert_eq!(get::<String>(&target.target_property()), "Property"),
                    1 => {
                        assert_eq!(get::<String>(&object.property.borrow()), "1");
                        assert_eq!(get::<String>(&target.target_property()), "AttachedProperty");
                    }
                    _ => panic!("unexpected call"),
                }
                counter.set(counter.get() + 1);
                boxed(counter.get().to_string())
            },
            None,
        ),
    )
    .unwrap_or_else(|e| panic!("{}", e.message()));
    assert_eq!(num.get(), 2);
}

#[test]
fn provide_value_target_target_object_is_valid_after_nested_push_pop() {
    let num = Rc::new(Cell::new(0));
    let counter = num.clone();
    let host = TestHost::new();
    host.build(
        "
<ServiceProviderTestsClass xmlns='rt-test'
    Property='{Callback Nested={Callback}}'
    ServiceProviderTests.AttachedProperty='{Callback}'
/>",
        callback_provider(
            move |provider| {
                let target = provider.get_service_of::<Rc<dyn IProvideValueTarget>>().expect("provide value target");
                let property = get::<String>(&target.target_property());
                match counter.get() {
                    0 => {
                        let _ = get::<Rc<CallbackExtension>>(&target.target_object());
                        assert_eq!(property, "Nested");
                    }
                    1 => {
                        let _ = get::<Rc<ServiceProviderTestsClass>>(&target.target_object());
                        assert_eq!(property, "Property");
                    }
                    2 => {
                        let _ = get::<Rc<ServiceProviderTestsClass>>(&target.target_object());
                        assert_eq!(property, "AttachedProperty");
                    }
                    _ => panic!("unexpected call"),
                }
                counter.set(counter.get() + 1);
                boxed(counter.get().to_string())
            },
            None,
        ),
    )
    .unwrap_or_else(|e| panic!("{}", e.message()));
    assert_eq!(num.get(), 3);
}

#[test]
fn inner_provider_interception_works() {
    let called = Rc::new(Cell::new(false));
    let flag = called.clone();
    let host = TestHost::with_options(HostOptions { inner_provider_factory: true, ..HostOptions::default() });
    host.build(
        "<ServiceProviderTestsClass xmlns='rt-test' Property='{Callback}'/>",
        callback_provider(
            move |provider| {
                let root_provider = provider.get_service_of::<Rc<dyn IRootObjectProvider>>().expect("root provider");
                let inner = provider.get_service_of::<Rc<InnerProvider>>().expect("the inner provider");
                let _ = get::<Rc<ServiceProviderTestsClass>>(&inner.original_root_object());
                assert_eq!(get::<String>(&root_provider.root_object()), "Definitely not the root object");
                flag.set(true);
                boxed("Value".to_string())
            },
            None,
        ),
    )
    .unwrap_or_else(|e| panic!("{}", e.message()));
    assert!(called.get());
}

fn namespace_info(xaml: &str, expected: &[(&str, &str, Option<&str>)]) {
    let called = Rc::new(Cell::new(false));
    let flag = called.clone();
    let expected: Vec<(String, XamlXmlNamespaceInfo)> = expected
        .iter()
        .map(|(prefix, namespace, assembly)| {
            let info = XamlXmlNamespaceInfo {
                clr_namespace: namespace.to_string(),
                clr_assembly_name: assembly.map(str::to_string),
            };
            (prefix.to_string(), info)
        })
        .collect();
    let host = TestHost::new();
    host.build(
        xaml,
        callback_provider(
            move |provider| {
                let namespaces = provider
                    .get_service_of::<Rc<dyn IXamlXmlNamespaceInfoProvider>>()
                    .expect("namespace info")
                    .xml_namespaces();
                assert_eq!(namespaces.len(), expected.len());
                for (prefix, info) in &expected {
                    assert_eq!(namespaces.get(prefix).map(Vec::as_slice), Some(std::slice::from_ref(info)), "{prefix}");
                }
                flag.set(true);
                boxed("Value".to_string())
            },
            None,
        ),
    )
    .unwrap_or_else(|e| panic!("{}", e.message()));
    assert!(called.get());
}

#[test]
fn namespace_info_should_be_preserved() {
    namespace_info(
        "
<ServiceProviderTestsClass
    xmlns='rt-test'
    xmlns:clr1='clr-namespace:System.Collections.Generic;assembly=netstandard'
    xmlns:clr2='clr-namespace:Dummy;assembly=RtXamlParserTests'
    Property='{Callback}'/>",
        &[
            ("", "RtXamlParserTests", Some("RtXamlParserTests")),
            ("clr1", "System.Collections.Generic", Some("netstandard")),
            ("clr2", "Dummy", Some("RtXamlParserTests")),
        ],
    );
}

#[test]
fn namespace_info_should_be_preserved_with_using_syntax() {
    namespace_info(
        "
<ServiceProviderTestsClass
    xmlns='using:RtXamlParserTests'
    xmlns:clr1='using:System.Collections.Generic'
    xmlns:clr2='using:Dummy'
    Property='{Callback}'/>",
        &[("", "RtXamlParserTests", None), ("clr1", "System.Collections.Generic", None), ("clr2", "Dummy", None)],
    );
}

#[test]
fn uri_context_is_usable() {
    let called = Rc::new(Cell::new(false));
    let flag = called.clone();
    let host = TestHost::new();
    host.build(
        "<ServiceProviderTestsClass xmlns='rt-test' Property='{Callback}'/>",
        callback_provider(
            move |provider| {
                let base_uri = provider.get_service_of::<Rc<dyn IUriContext>>().expect("URI context").base_uri();
                assert_eq!(base_uri.map(|uri| uri.to_string()).as_deref(), Some("http://example.com/"));
                flag.set(true);
                boxed("Value".to_string())
            },
            None,
        ),
    )
    .unwrap_or_else(|e| panic!("{}", e.message()));
    assert!(called.get());
}

#[test]
fn unknown_services_should_return_null() {
    let host = TestHost::new();
    let res = host.run::<Rc<ServiceProviderTestsClass>>(
        "<ServiceProviderTestsClass xmlns='rt-test' Property='{UnknownServiceUsage Return=123}'/>",
    );
    assert_eq!(get::<String>(&res.property.borrow()), "123");
}
