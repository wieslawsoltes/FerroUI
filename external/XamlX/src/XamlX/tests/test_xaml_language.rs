//! Port of the upstream test language (`TestXamlLanguage.cs`, `CompilerTestBase.cs`) and of the
//! test classes used by the ported tests, declared in the fake type system.

use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::{IXamlAstNode, XamlDocument};
use crate::compiler::{XamlCompiler, XamlImperativeCompiler};
use crate::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use crate::emit::{IXamlEmitResult, XamlLanguageEmitMappings};
use crate::exceptions::XamlResult;
use crate::parsers::XDocumentXamlParser;
use crate::testing::{FakeAssembly, FakeCustomAttribute, FakeTypeSystem};
use crate::transform::{
    TransformerConfiguration, XamlDiagnosticsHandler, XamlLanguageTypeMappings, XamlValueConverter,
};
use crate::type_system::{IXamlType, XamlValue};

/// The emitter backend handle used by tests that only transform.
pub struct NoEmitter;

pub struct NoEmitResult;

impl IXamlEmitResult for NoEmitResult {
    fn return_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn valid(&self) -> bool {
        true
    }
}

pub const NS: &str = "XamlParserTests";

pub struct TestHost {
    pub ts: Rc<FakeTypeSystem>,
    pub asm: Rc<FakeAssembly>,
    pub configuration: Rc<TransformerConfiguration>,
    pub diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>>,
}

pub struct TestHostOptions {
    /// Report every error as fatal, i.e. fail on the first error instead of collecting them.
    pub errors_are_fatal: bool,
    pub custom_value_converter: Option<XamlValueConverter>,
}

impl TestHost {
    /// The upstream `CompilerTestBase` configuration: diagnostics are collected.
    pub fn new() -> Self {
        Self::with_options(TestHostOptions {
            errors_are_fatal: false,
            custom_value_converter: None,
        })
    }

    pub fn with_options(options: TestHostOptions) -> Self {
        let ts = FakeTypeSystem::new();
        let asm = ts.define_assembly(NS);
        define_test_language(&ts, &asm);
        define_test_classes(&ts, &asm);

        let t = |name: &str| ts.get(&format!("{NS}.{name}"));
        let mut mappings = XamlLanguageTypeMappings::new(&*ts).expect("default mappings");
        mappings
            .xmlns_attributes
            .push(t("XmlnsDefinitionAttribute"));
        mappings.content_attributes.push(t("ContentAttribute"));
        mappings
            .whitespace_significant_collection_attributes
            .push(t("WhitespaceSignificantCollectionAttribute"));
        mappings
            .trim_surrounding_whitespace_attributes
            .push(t("TrimSurroundingWhitespaceAttribute"));
        mappings
            .usable_during_initialization_attributes
            .push(t("UsableDuringInitializationAttribute"));
        mappings
            .deferred_content_property_attributes
            .push(t("DeferredContentAttribute"));
        mappings.root_object_provider = Some(t("ITestRootObjectProvider"));
        mappings.uri_context_provider = Some(t("ITestUriContext"));
        mappings.provide_value_target = Some(t("ITestProvideValueTarget"));
        mappings.i_add_child = Some(t("IAddChild"));
        mappings.i_add_child_of_t = Some(t("IAddChild`1"));

        let diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
        let collected = diagnostics.clone();
        let errors_are_fatal = options.errors_are_fatal;
        let handler = XamlDiagnosticsHandler {
            handle_diagnostic: Some(Box::new(move |diagnostic| {
                collected.borrow_mut().push(diagnostic.clone());
                if errors_are_fatal && diagnostic.severity >= XamlDiagnosticSeverity::Error {
                    XamlDiagnosticSeverity::Fatal
                } else {
                    diagnostic.severity
                }
            })),
            ..XamlDiagnosticsHandler::default()
        };

        let configuration = TransformerConfiguration::new(
            ts.as_type_system(),
            ts.as_type_system().find_assembly(NS),
            mappings,
            None,
            options.custom_value_converter,
            None,
            Some(handler),
        )
        .expect("configuration");

        Self {
            ts,
            asm,
            configuration: Rc::new(configuration),
            diagnostics,
        }
    }

    pub fn t(&self, name: &str) -> Rc<dyn IXamlType> {
        self.ts.get(&format!("{NS}.{name}"))
    }

    /// The upstream `CompilerTestBase.Transform`: parse and run the default transformers.
    pub fn transform(&self, xaml: &str) -> XamlResult<XamlDocument> {
        let mut parsed = XDocumentXamlParser::parse(xaml, None)?;
        let compiler: XamlCompiler<NoEmitter, NoEmitResult> = XamlCompiler::new(
            self.configuration.clone(),
            Rc::new(XamlLanguageEmitMappings::new()),
            true,
        );
        compiler.transform(&mut parsed)?;
        Ok(parsed)
    }

    /// Parse and run the transformers of the imperative compiler (what upstream runs before emitting IL).
    pub fn transform_imperative(&self, xaml: &str) -> XamlResult<XamlDocument> {
        let mut parsed = XDocumentXamlParser::parse(xaml, None)?;
        let compiler: XamlImperativeCompiler<NoEmitter, NoEmitResult> = XamlImperativeCompiler::new(
            self.configuration.clone(),
            Rc::new(XamlLanguageEmitMappings::new()),
            true,
        );
        compiler.transform(&mut parsed)?;
        Ok(parsed)
    }

    pub fn transform_root(&self, xaml: &str) -> Rc<dyn IXamlAstNode> {
        let doc = self.transform(xaml).expect("transform");
        self.assert_no_errors();
        doc.root().expect("root")
    }

    pub fn errors(&self) -> Vec<XamlDiagnostic> {
        self.diagnostics
            .borrow()
            .iter()
            .filter(|d| d.severity >= XamlDiagnosticSeverity::Error)
            .cloned()
            .collect()
    }

    pub fn assert_no_errors(&self) {
        let errors: Vec<String> = self.errors().iter().map(|d| d.title.clone()).collect();
        assert!(errors.is_empty(), "Unexpected errors: {errors:#?}");
    }
}

fn define_test_language(ts: &Rc<FakeTypeSystem>, asm: &Rc<FakeAssembly>) {
    let attribute = ts.get("System.Attribute");
    let object = ts.get("System.Object");
    let void = ts.get("System.Void");
    for name in [
        "ContentAttribute",
        "WhitespaceSignificantCollectionAttribute",
        "TrimSurroundingWhitespaceAttribute",
        "XmlnsDefinitionAttribute",
        "UsableDuringInitializationAttribute",
        "DeferredContentAttribute",
    ] {
        asm.define_class(NS, name).set_base_type(attribute.clone());
    }

    let add_child = asm.define_interface(NS, "IAddChild");
    add_child.add_method("AddChild", void.clone(), vec![object.clone()], false);
    let add_child_of_t = asm.define_generic_interface(NS, "IAddChild`1", &["T"]);
    add_child_of_t.add_interface(add_child);
    add_child_of_t.add_method(
        "AddChild",
        void,
        vec![add_child_of_t.generic_parameter(0)],
        false,
    );

    asm.define_interface(NS, "ITestRootObjectProvider")
        .add_property_with("RootObject", object.clone(), true, false, false);
    let target = asm.define_interface(NS, "ITestProvideValueTarget");
    target.add_property_with("TargetObject", object.clone(), true, false, false);
    target.add_property_with("TargetProperty", object, true, false, false);
    asm.define_interface(NS, "ITestUriContext")
        .add_property("BaseUri", ts.get("System.Uri"));

    asm.add_attribute(FakeCustomAttribute::new(
        ts.get(&format!("{NS}.XmlnsDefinitionAttribute")),
        vec![
            XamlValue::String("test".to_string()),
            XamlValue::String(NS.to_string()),
        ],
    ));
}

fn define_test_classes(ts: &Rc<FakeTypeSystem>, asm: &Rc<FakeAssembly>) {
    let t = |name: &str| ts.get(&format!("{NS}.{name}"));
    let attr = |name: &str| FakeCustomAttribute::new(t(name), vec![]);
    let object = ts.get("System.Object");
    let string = ts.get("System.String");
    let boolean = ts.get("System.Boolean");
    let int32 = ts.get("System.Int32");
    let void = ts.get("System.Void");
    let list_of = |item: Rc<dyn IXamlType>| {
        ts.get("System.Collections.Generic.List`1")
            .make_generic_type(&[item])
            .expect("List<T>")
    };

    // WhitespaceTests.cs
    let control = asm.define_class(NS, "Control");
    control.add_constructor(vec![]);
    control.add_property("StrProp", string.clone());
    control.add_property("BoolProp", boolean.clone());
    control.add_event("Click", ts.get("System.Action"));

    let content_control = asm.define_class(NS, "ContentControl");
    content_control.set_base_type(control.clone());
    content_control.add_constructor(vec![]);
    content_control
        .add_property("Content", object.clone())
        .add_attribute(attr("ContentAttribute"));

    let mixed = asm.define_class(NS, "MixedContentControl");
    mixed.add_constructor(vec![]);
    mixed
        .add_property_with("Content", list_of(object.clone()), true, false, false)
        .add_attribute(attr("ContentAttribute"));

    let mixed_enumerable = asm.define_class(NS, "MixedEnumerableContentControl");
    mixed_enumerable.add_constructor(vec![]);
    mixed_enumerable
        .add_property("Items", ts.get("System.Collections.IEnumerable"))
        .add_attribute(attr("ContentAttribute"));

    let opt_in_collection = asm.define_class(NS, "WhitespaceOptInCollection");
    opt_in_collection.set_base_type(list_of(object.clone()));
    opt_in_collection.add_constructor(vec![]);
    opt_in_collection.add_attribute(attr("WhitespaceSignificantCollectionAttribute"));
    let opt_in = asm.define_class(NS, "WhitespaceOptInControl");
    opt_in.add_constructor(vec![]);
    opt_in
        .add_property_with("Content", opt_in_collection.clone(), true, false, false)
        .add_attribute(attr("ContentAttribute"));

    let inline = asm.define_class(NS, "Inline");
    inline.add_constructor(vec![]);
    let run = asm.define_class(NS, "Run");
    run.set_base_type(inline.clone());
    run.add_constructor(vec![]);
    run.add_property("Text", string.clone());
    let inline_collection = asm.define_class(NS, "InlineCollection");
    inline_collection.set_base_type(list_of(inline.clone()));
    inline_collection.add_constructor(vec![]);
    inline_collection.add_attribute(attr("WhitespaceSignificantCollectionAttribute"));
    inline_collection.add_method("Add", void.clone(), vec![string.clone()], false);
    let control_with_inlines = asm.define_class(NS, "ControlWithInlines");
    control_with_inlines.add_constructor(vec![]);
    control_with_inlines
        .add_property("Inlines", inline_collection.clone())
        .add_attribute(attr("ContentAttribute"));
    let inline_with_inlines = asm.define_class(NS, "InlineWithInlines");
    inline_with_inlines.set_base_type(inline);
    inline_with_inlines.add_constructor(vec![]);
    inline_with_inlines
        .add_property("Inlines", inline_collection)
        .add_attribute(attr("ContentAttribute"));

    let trim_control = asm.define_class(NS, "TrimControl");
    trim_control.add_constructor(vec![]);
    trim_control.add_attribute(attr("TrimSurroundingWhitespaceAttribute"));

    // ContentAttributeTests.cs
    let simple_content = asm.define_class(NS, "SimpleClassWithContentAttribute");
    simple_content.add_constructor(vec![]);
    simple_content
        .add_property("Text", string.clone())
        .add_attribute(attr("ContentAttribute"));
    let sub_content = asm.define_class(NS, "SubClassWithContentAttributeOverride");
    sub_content.set_base_type(simple_content);
    sub_content.add_constructor(vec![]);
    sub_content
        .add_property("OtherText", string.clone())
        .add_attribute(attr("ContentAttribute"));
    let two_content = asm.define_class(NS, "SimpleClassWithTwoContentAttributes");
    two_content.add_constructor(vec![]);
    two_content
        .add_property("Text", string.clone())
        .add_attribute(attr("ContentAttribute"));
    two_content
        .add_property("OtherText", string.clone())
        .add_attribute(attr("ContentAttribute"));

    // StandardWarningsTests.cs
    let obsolete = |message: &str, is_error: Option<bool>| {
        let mut parameters = vec![XamlValue::String(message.to_string())];
        if let Some(is_error) = is_error {
            parameters.push(XamlValue::Boolean(is_error));
        }
        FakeCustomAttribute::new(ts.get("System.ObsoleteAttribute"), parameters)
    };
    let obsolete_class = asm.define_class(NS, "ObsoleteClass");
    obsolete_class.add_constructor(vec![]);
    obsolete_class.add_attribute(obsolete("ObsoleteClass is obsolete", None));
    obsolete_class
        .add_property("ObjectProperty", object.clone())
        .add_attribute(obsolete("ObjectProperty is obsolete", None));
    obsolete_class
        .add_property_with("StaticProp", object.clone(), true, false, true)
        .add_attribute(obsolete("StaticProp is obsolete", Some(true)));

    let experimental = |message: Option<&str>| {
        let properties = match message {
            Some(m) => vec![("Message", XamlValue::String(m.to_string()))],
            None => vec![],
        };
        FakeCustomAttribute::with_properties(
            ts.get("System.Diagnostics.CodeAnalysis.ExperimentalAttribute"),
            vec![XamlValue::String("FOO123".to_string())],
            properties,
        )
    };
    let experimental_class = asm.define_class(NS, "ExperimentalClass");
    experimental_class.add_constructor(vec![]);
    experimental_class.add_attribute(experimental(None));
    experimental_class
        .add_property("ObjectProperty", object.clone())
        .add_attribute(experimental(None));
    experimental_class
        .add_property_with("StaticProp", object.clone(), true, false, true)
        .add_attribute(experimental(None));
    let experimental_message_class = asm.define_class(NS, "ExperimentalWithMessageClass");
    experimental_message_class.add_constructor(vec![]);
    experimental_message_class
        .add_attribute(experimental(Some("ExperimentalClass is experimental")));
    experimental_message_class
        .add_property("ObjectProperty", object.clone())
        .add_attribute(experimental(Some("ObjectProperty is experimental")));
    experimental_message_class
        .add_property_with("StaticProp", object.clone(), true, false, true)
        .add_attribute(experimental(Some("StaticProp is experimental")));

    // Enums
    let test_enum = asm.define_enum(NS, "TestEnum", &[("A", 1), ("B", 2)]);
    let flags_enum = asm.define_enum(NS, "FlagsEnum", &[("One", 1), ("Two", 2), ("Four", 4)]);
    flags_enum.add_attribute(FakeCustomAttribute::new(
        ts.get("System.FlagsAttribute"),
        vec![],
    ));

    // A value type with Parse methods
    let parseable = asm.define_struct(NS, "ParseableStruct");
    parseable.add_method("Parse", parseable.clone(), vec![string.clone()], true);
    let culture_parseable = asm.define_struct(NS, "CultureParseableStruct");
    culture_parseable.add_method(
        "Parse",
        culture_parseable.clone(),
        vec![string.clone(), ts.get("System.IFormatProvider")],
        true,
    );

    // A class converted through a type converter
    let converter = asm.define_class(NS, "ConvertedClassConverter");
    converter.set_base_type(ts.get("System.ComponentModel.TypeConverter"));
    converter.add_constructor(vec![]);
    let converted = asm.define_class(NS, "ConvertedClass");
    converted.add_attribute(FakeCustomAttribute::new(
        ts.get("System.ComponentModel.TypeConverterAttribute"),
        vec![XamlValue::Type(converter.clone())],
    ));

    // IntrinsicsTests.cs / ConvertersTests.cs
    let intrinsics = asm.define_class(NS, "IntrinsicsTestsClass");
    intrinsics.add_constructor(vec![]);
    intrinsics.add_property("ObjectProperty", object.clone());
    intrinsics.add_property("BoolProperty", boolean.clone());
    intrinsics.add_property("IntProperty", int32.clone());
    intrinsics.add_property(
        "NullableIntProperty",
        ts.get("System.Nullable`1")
            .make_generic_type(&[int32.clone()])
            .expect("Nullable<int>"),
    );
    intrinsics.add_property("DoubleProperty", ts.get("System.Double"));
    intrinsics.add_property("TypeProperty", ts.get("System.Type"));
    intrinsics.add_property("EnumProperty", test_enum.clone());
    intrinsics.add_property("FlagsProperty", flags_enum);
    intrinsics.add_property("ParseableProperty", parseable);
    intrinsics.add_property("CultureParseableProperty", culture_parseable);
    intrinsics.add_property("ConvertedProperty", converted);
    intrinsics.add_property_with("StaticProp", string.clone(), true, false, true);
    intrinsics.add_field("StaticField", string.clone(), true, None);
    intrinsics.add_field(
        "ConstField",
        int32.clone(),
        true,
        Some(XamlValue::Int32(42)),
    );
    intrinsics.add_method("OnClick", void.clone(), vec![], false);
    intrinsics.add_event("Click", ts.get("System.Action"));

    // Attached properties
    let attached = asm.define_class(NS, "AttachedProps");
    attached.add_method(
        "SetFoo",
        void.clone(),
        vec![control.clone(), string.clone()],
        true,
    );
    attached.add_method("GetFoo", string.clone(), vec![control.clone()], true);

    // Markup extensions
    let service_provider = ts.get("System.IServiceProvider");
    let extension = asm.define_class(NS, "TestExtension");
    extension.add_constructor(vec![]);
    extension.add_constructor(vec![object.clone()]);
    extension.add_property("Prop", object.clone());
    extension.add_method(
        "ProvideValue",
        object.clone(),
        vec![service_provider.clone()],
        false,
    );
    let typed_extension = asm.define_class(NS, "TypedExtension");
    typed_extension.add_constructor(vec![]);
    typed_extension.add_method("ProvideValue", int32.clone(), vec![], false);
    typed_extension.add_method(
        "ProvideValue",
        object.clone(),
        vec![service_provider],
        false,
    );
    let broken_extension = asm.define_class(NS, "BrokenExtension");
    broken_extension.add_constructor(vec![]);

    // Dictionaries and lists
    let dictionary_host = asm.define_class(NS, "DictionaryHost");
    dictionary_host.add_constructor(vec![]);
    dictionary_host
        .add_property_with(
            "Items",
            ts.get("System.Collections.Generic.Dictionary`2")
                .make_generic_type(&[string.clone(), object.clone()])
                .expect("Dictionary<string, object>"),
            true,
            false,
            false,
        )
        .add_attribute(attr("ContentAttribute"));
    let list_host = asm.define_class(NS, "ListHost");
    list_host.set_base_type(list_of(control.clone()));
    list_host.add_constructor(vec![]);
    let add_child_host = asm.define_class(NS, "AddChildHost");
    add_child_host.add_constructor(vec![]);
    add_child_host.add_interface(
        t("IAddChild`1")
            .make_generic_type(&[control.clone()])
            .expect("IAddChild<Control>"),
    );

    // Constructor arguments and generics
    let ctor_class = asm.define_class(NS, "CtorClass");
    ctor_class.add_constructor(vec![int32.clone(), string.clone()]);
    let generic = asm.define_generic_class(NS, "GenericClass`1", &["T"]);
    generic.add_constructor(vec![]);
    generic.add_property("Value", generic.generic_parameter(0));

    // Top-down initialization and deferred content
    let top_down_child = asm.define_class(NS, "TopDownChild");
    top_down_child.add_constructor(vec![]);
    top_down_child.add_property("StrProp", string.clone());
    top_down_child.add_attribute(FakeCustomAttribute::new(
        t("UsableDuringInitializationAttribute"),
        vec![XamlValue::Boolean(true)],
    ));
    let top_down_parent = asm.define_class(NS, "TopDownParent");
    top_down_parent.add_constructor(vec![]);
    top_down_parent.add_property("Child", top_down_child);
    let deferred_host = asm.define_class(NS, "DeferredHost");
    deferred_host.add_constructor(vec![]);
    deferred_host
        .add_property("Template", object)
        .add_attribute(attr("DeferredContentAttribute"));
}
