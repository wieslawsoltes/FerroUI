//! Tests of `ferro_xaml_il_language.rs`.

use std::any::Any;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, XamlAstClrTypeReference, XamlAstNode,
    XamlAstNodeExtensions, XamlAstTextNode, XamlLineInfo,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::testing::FakeCustomAttribute;
use xamlx::transform::{AstTransformationContext, XamlTransformHelpers};
use xamlx::type_system::{IXamlCustomAttribute, IXamlType, XamlValue, XamlVisibility};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl, xaml_query_interface};

use super::{FerroXamlIlLanguage, IOptionsMarkupExtensionNode};
use crate::compiler_extensions::ast_nodes::FerroXamlIlVectorLikeConstantAstNode;
use crate::compiler_extensions::transformers::{
    FerroXamlIlTargetTypeMetadataNode, ScopeTypes, FERRO_XML_NAMESPACE,
};
use crate::compiler_extensions::{
    IXamlIlFerroPropertyNode, XamlIlFerroClassProperty, XamlIlFerroPropertyFieldNode,
    XamlIlFerroPropertyNode,
};
use crate::testing::{create_test_framework, TestFramework};

fn text(fw: &TestFramework, value: &str) -> Rc<dyn IXamlAstValueNode> {
    XamlAstTextNode::with_type(
        &XamlLineInfo::new(2, 9),
        value,
        true,
        Some(fw.t("System.String")),
    )
}

fn type_reference(fw: &TestFramework, name: &str) -> Rc<dyn IXamlAstTypeReference> {
    XamlAstClrTypeReference::new(&XamlLineInfo::new(1, 1), fw.t(name), false)
}

/// Pushes a target type scope for `target` as a parent node of the context.
fn push_scope(
    fw: &TestFramework,
    context: &AstTransformationContext,
    target: &str,
    scope_type: ScopeTypes,
) {
    let scope = FerroXamlIlTargetTypeMetadataNode::new(
        text(fw, "scope"),
        type_reference(fw, target),
        scope_type,
    );
    context.push_parent(scope);
}

#[test]
fn configures_the_language_type_mappings() {
    let fw = create_test_framework();
    let (mappings, _) = FerroXamlIlLanguage::configure(&fw.as_type_system()).expect("language");
    let name = |t: &Option<Rc<dyn IXamlType>>| t.as_ref().map(|t| t.full_name());
    let names = |types: &[Rc<dyn IXamlType>]| -> Vec<String> {
        types.iter().map(|t| t.full_name()).collect()
    };

    assert_eq!(name(&mappings.service_provider).as_deref(), Some("System.IServiceProvider"));
    assert_eq!(
        name(&mappings.support_initialize).as_deref(),
        Some("System.ComponentModel.ISupportInitialize")
    );
    assert_eq!(
        names(&mappings.xmlns_attributes),
        ["FerroUI.Metadata.XmlnsDefinitionAttribute"]
    );
    assert_eq!(names(&mappings.content_attributes), ["FerroUI.Metadata.ContentAttribute"]);
    assert_eq!(
        names(&mappings.whitespace_significant_collection_attributes),
        ["FerroUI.Metadata.WhitespaceSignificantCollectionAttribute"]
    );
    assert_eq!(
        names(&mappings.trim_surrounding_whitespace_attributes),
        ["FerroUI.Metadata.TrimSurroundingWhitespaceAttribute"]
    );
    assert_eq!(
        names(&mappings.type_converter_attributes),
        ["System.ComponentModel.TypeConverterAttribute"]
    );
    assert_eq!(
        name(&mappings.provide_value_target).as_deref(),
        Some("FerroUI.Markup.Xaml.IProvideValueTarget")
    );
    assert_eq!(
        name(&mappings.root_object_provider).as_deref(),
        Some("FerroUI.Markup.Xaml.IRootObjectProvider")
    );
    assert_eq!(
        mappings.root_object_provider_intermediate_root_property_name.as_deref(),
        Some("IntermediateRootObject")
    );
    assert_eq!(
        name(&mappings.uri_context_provider).as_deref(),
        Some("FerroUI.Markup.Xaml.IUriContext")
    );
    assert_eq!(
        name(&mappings.parent_stack_provider).as_deref(),
        Some("FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlParentStackProvider")
    );
    assert_eq!(
        name(&mappings.xml_namespace_info_provider).as_deref(),
        Some("FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlXmlNamespaceInfoProvider")
    );
    assert_eq!(
        names(&mappings.deferred_content_property_attributes),
        ["FerroUI.Metadata.TemplateContentAttribute"]
    );
    assert_eq!(
        name(&mappings.deferred_content_executor_customization_default_type_parameter).as_deref(),
        Some("FerroUI.Controls.Control")
    );
    assert_eq!(
        mappings
            .deferred_content_executor_customization_type_parameter_deferred_content_attribute_property_names,
        ["TemplateResultType"]
    );
    assert_eq!(
        mappings
            .deferred_content_executor_customization
            .as_ref()
            .map(|m| m.name())
            .as_deref(),
        Some("DeferredTransformationFactoryV3")
    );
    assert_eq!(
        names(&mappings.usable_during_initialization_attributes),
        ["FerroUI.Metadata.UsableDuringInitializationAttribute"]
    );
    assert_eq!(
        mappings
            .inner_service_provider_factory_method
            .as_ref()
            .map(|m| m.name())
            .as_deref(),
        Some("CreateInnerServiceProviderV1")
    );
    assert_eq!(name(&mappings.i_add_child).as_deref(), Some("FerroUI.Metadata.IAddChild"));
    assert_eq!(
        name(&mappings.i_add_child_of_t).as_deref(),
        Some("FerroUI.Metadata.IAddChild`1")
    );
    assert!(mappings.custom_attribute_resolver.is_some());
}

#[test]
fn maps_the_framework_xml_namespace() {
    let fw = create_test_framework();
    let namespaces = fw
        .configuration
        .xmlns_mappings
        .namespaces
        .get(FERRO_XML_NAMESPACE)
        .expect("the framework namespace is mapped");
    let has = |assembly: &str, namespace: &str| {
        namespaces
            .iter()
            .any(|(asm, ns)| asm.name() == assembly && ns == namespace)
    };
    assert!(has("FerroUI.Base", "FerroUI"));
    assert!(has("FerroUI.Base", "FerroUI.Styling"));
    assert!(has("FerroUI.Controls", "FerroUI.Controls"));
    assert!(has("FerroUI.Controls", "FerroUI.Controls.Primitives"));
    assert!(has("FerroUI.Markup", "FerroUI.Data"));
    assert!(has("FerroUI.Markup.Xaml", "FerroUI.Markup.Xaml.Templates"));
    assert_eq!(namespaces.len(), 15 + 11 + 2 + 3);
}

#[test]
fn attribute_resolver_supplies_type_converters() {
    let fw = create_test_framework();
    let configuration = &fw.configuration;
    let type_converter_attribute = fw.t("System.ComponentModel.TypeConverterAttribute");
    let converter_of = |type_: &Rc<dyn IXamlType>| -> Option<String> {
        let attributes =
            configuration.get_custom_attribute_for_type(&**type_, &*type_converter_attribute);
        let attribute = attributes.first()?;
        assert!(attribute.type_().equals(&*type_converter_attribute));
        assert!(attribute.properties().is_empty());
        // A constructed attribute is equal to nothing, not even itself.
        assert!(!attribute.equals(&**attribute));
        match attribute.parameters().as_slice() {
            [XamlValue::Type(converter)] => Some(converter.full_name()),
            other => panic!("unexpected parameters {other:?}"),
        }
    };

    let converters = "FerroUI.Markup.Xaml.Converters";
    for (type_name, converter) in [
        ("FerroUI.Media.IImage", format!("{converters}.BitmapTypeConverter")),
        ("FerroUI.Media.Imaging.Bitmap", format!("{converters}.BitmapTypeConverter")),
        ("FerroUI.Media.IImageBrushSource", format!("{converters}.BitmapTypeConverter")),
        ("FerroUI.Controls.WindowIcon", format!("{converters}.IconTypeConverter")),
        (
            "System.Globalization.CultureInfo",
            "System.ComponentModel.CultureInfoConverter".to_string(),
        ),
        ("System.Uri", format!("{converters}.FerroUriTypeConverter")),
        ("System.TimeSpan", format!("{converters}.TimeSpanTypeConverter")),
        ("FerroUI.Media.FontFamily", format!("{converters}.FontFamilyTypeConverter")),
    ] {
        assert_eq!(converter_of(&fw.t(type_name)), Some(converter), "{type_name}");
    }

    let points = fw
        .t("System.Collections.Generic.IList`1")
        .make_generic_type(&[fw.t("FerroUI.Point")])
        .expect("IList<Point>");
    assert_eq!(
        converter_of(&points),
        Some(format!("{converters}.PointsListTypeConverter"))
    );

    // FerroList<T> gets the generic list converter; types deriving from it do not.
    let list_of_double = fw
        .t("FerroUI.Collections.FerroList`1")
        .make_generic_type(&[fw.t("System.Double")])
        .expect("FerroList<double>");
    assert_eq!(
        converter_of(&list_of_double),
        Some("FerroUI.Collections.FerroListConverter`1[System.Double]".to_string())
    );
    assert_eq!(converter_of(&fw.t("FerroUI.Controls.Classes")), None);
    assert_eq!(converter_of(&fw.t("FerroUI.Controls.Control")), None);

    // Only the type converter attribute is synthesised, and never for properties.
    assert!(configuration
        .get_custom_attribute_for_type(
            &*fw.t("System.Uri"),
            &*fw.t("FerroUI.Metadata.ContentAttribute")
        )
        .is_empty());
    let text_property = fw
        .t("FerroUI.Controls.TextBlock")
        .properties()
        .into_iter()
        .find(|p| p.name() == "FontFamily")
        .expect("FontFamily");
    assert!(configuration
        .get_custom_attribute_for_property(&*text_property, &*type_converter_attribute)
        .is_empty());
}

#[test]
fn describes_the_runtime_context_members() {
    let fw = create_test_framework();
    let (mappings, emit) = FerroXamlIlLanguage::configure(&fw.as_type_system()).expect("language");
    let definition = emit
        .context_type_builder_callback(&mappings)
        .expect("context definition");

    let field = &definition.name_scope_field;
    assert_eq!(field.name, "FerroNameScope");
    assert_eq!(field.name, FerroXamlIlLanguage::CONTEXT_NAME_SCOPE_FIELD_NAME);
    assert_eq!(field.field_type.full_name(), "FerroUI.Controls.INameScope");
    assert_eq!(field.visibility, XamlVisibility::Public);
    assert!(!field.is_static);
    assert_eq!(field.service_provider_get_service_method.name(), "GetService");
    assert_eq!(
        field
            .service_provider_get_service_method
            .declaring_type()
            .full_name(),
        "System.IServiceProvider"
    );

    let provider = &definition.eager_parent_stack_provider;
    assert_eq!(
        provider.interface_type.full_name(),
        "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlEagerParentStackProvider"
    );
    assert_eq!(provider.direct_parents_stack_getter.name(), "get_DirectParentsStack");
    assert_eq!(
        provider.direct_parents_stack_getter.return_type().full_name(),
        "System.Collections.Generic.IReadOnlyList`1[System.Object]"
    );
    assert_eq!(provider.parent_provider_getter.name(), "get_ParentProvider");
    assert!(provider
        .parent_provider_getter
        .return_type()
        .equals(&*provider.interface_type));
    assert_eq!(
        provider.parent_stack_provider.full_name(),
        "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlParentStackProvider"
    );
    assert_eq!(provider.service_provider_get_service_method.name(), "GetService");
    let as_eager = &provider.as_eager_parent_stack_provider_method;
    assert_eq!(as_eager.name(), "AsEagerParentStackProvider");
    assert!(as_eager.is_static());
    assert!(as_eager.return_type().equals(&*provider.interface_type));
    assert!(as_eager.parameters()[0].equals(&*provider.parent_stack_provider));

    // The callback needs the parent stack provider mapping.
    let (mut incomplete, emit) =
        FerroXamlIlLanguage::configure(&fw.as_type_system()).expect("language");
    incomplete.parent_stack_provider = None;
    assert!(emit.context_type_builder_callback(&incomplete).is_err());
}

#[test]
fn custom_value_converter_only_converts_text() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let not_text: Rc<dyn IXamlAstValueNode> = FerroXamlIlVectorLikeConstantAstNode::new(
        &XamlLineInfo::new(1, 1),
        &fw.types,
        fw.types.point.clone(),
        fw.types.point_full_constructor.clone(),
        vec![1.0, 2.0],
    )
    .expect("node");
    let result =
        FerroXamlIlLanguage::custom_value_converter(&context, &not_text, None, &fw.types.thickness)
            .expect("no error");
    assert!(result.is_none());
}

#[test]
fn custom_value_converter_applies_the_parse_intrinsics() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let result = FerroXamlIlLanguage::custom_value_converter(
        &context,
        &text(&fw, "1,2,3,4"),
        None,
        &fw.types.thickness,
    )
    .expect("no error")
    .expect("converted");
    assert!(result.is::<FerroXamlIlVectorLikeConstantAstNode>());

    // It is what the configuration uses for typed assignments.
    let converted = XamlTransformHelpers::try_get_correctly_typed_value(
        &context,
        &text(&fw, "4"),
        &fw.types.thickness,
    )
    .expect("no error")
    .expect("converted");
    let values = converted
        .cast::<FerroXamlIlVectorLikeConstantAstNode>()
        .expect("vector-like node")
        .values()
        .to_vec();
    assert_eq!(values, [4.0; 4]);

    // Nothing applies: the caller goes on with the built-in conversions.
    let unconverted = FerroXamlIlLanguage::custom_value_converter(
        &context,
        &text(&fw, "42"),
        None,
        &fw.t("System.Int32"),
    )
    .expect("no error");
    assert!(unconverted.is_none());
    assert!(fw.reported_diagnostics().is_empty());
}

#[test]
fn custom_value_converter_resolves_registered_properties_in_the_target_type_scope() {
    let fw = create_test_framework();
    let ferro_property = fw.types.ferro_property.clone();

    // CLR property of the scope's target type.
    let context = fw.create_context();
    push_scope(&fw, &context, "FerroUI.Controls.Border", ScopeTypes::Style);
    let result = FerroXamlIlLanguage::custom_value_converter(
        &context,
        &text(&fw, "Background"),
        None,
        &ferro_property,
    )
    .expect("no error")
    .expect("converted");
    let node = result.cast::<XamlIlFerroPropertyNode>().expect("property node");
    assert_eq!(node.property.name(), "Background");
    assert_eq!(node.ferro_property_type().full_name(), "FerroUI.Media.IBrush");
    assert_eq!(
        node.resolve_ferro_property_field().expect("field").name(),
        "BackgroundProperty"
    );

    // Attached property of another type.
    let result = FerroXamlIlLanguage::custom_value_converter(
        &context,
        &text(&fw, "Grid.Row"),
        None,
        &ferro_property,
    )
    .expect("no error")
    .expect("converted");
    let node = result
        .cast::<XamlIlFerroPropertyFieldNode>()
        .expect("field node");
    assert_eq!(node.field().name(), "RowProperty");
    assert_eq!(node.ferro_property_type().full_name(), "System.Int32");

    // Style class.
    let result = FerroXamlIlLanguage::custom_value_converter(
        &context,
        &text(&fw, "(Classes.selected)"),
        None,
        &ferro_property,
    )
    .expect("no error")
    .expect("converted");
    let node = result.cast::<XamlIlFerroClassProperty>().expect("class node");
    assert_eq!(node.class_name(), "selected");
}

#[test]
fn custom_value_converter_honours_the_requested_scope_kind() {
    let fw = create_test_framework();
    let ferro_property = fw.types.ferro_property.clone();
    let attribute = |kind: i32| -> Vec<Rc<dyn IXamlCustomAttribute>> {
        vec![FakeCustomAttribute::new(
            fw.types.inherit_data_type_from_attribute.clone(),
            vec![XamlValue::Int32(kind)],
        )]
    };
    let convert = |context: &AstTransformationContext,
                   attributes: Option<&[Rc<dyn IXamlCustomAttribute>]>|
     -> XamlResult<String> {
        let result = FerroXamlIlLanguage::custom_value_converter(
            context,
            &text(&fw, "Background"),
            attributes,
            &ferro_property,
        )?
        .expect("converted");
        let node = result.cast::<XamlIlFerroPropertyNode>().expect("property node");
        Ok(node.property.declaring_type().name())
    };

    // Outer control template scope (TemplatedControl), inner style scope (Border).
    let context = fw.create_context();
    push_scope(
        &fw,
        &context,
        "FerroUI.Controls.Primitives.TemplatedControl",
        ScopeTypes::ControlTemplate,
    );
    push_scope(&fw, &context, "FerroUI.Controls.Border", ScopeTypes::Style);

    // Without the attribute the innermost scope wins.
    assert_eq!(convert(&context, None).expect("converted"), "Border");
    assert_eq!(convert(&context, Some(&attribute(1))).expect("converted"), "Border");
    assert_eq!(
        convert(&context, Some(&attribute(2))).expect("converted"),
        "TemplatedControl"
    );
    // An unknown kind means "any scope".
    assert_eq!(convert(&context, Some(&attribute(7))).expect("converted"), "Border");

    // No scope of the requested kind.
    let style_only = fw.create_context();
    push_scope(&fw, &style_only, "FerroUI.Controls.Border", ScopeTypes::Style);
    let error = convert(&style_only, Some(&attribute(2))).expect_err("no control template scope");
    assert!(matches!(error, XamlError::Load(_)));
    assert_eq!(
        error.message(),
        "Unable to find the ControlTemplate scope for FerroProperty lookup Line 2, position 9."
    );

    // No scope at all.
    let empty = fw.create_context();
    let error = convert(&empty, None).expect_err("no scope");
    assert_eq!(
        error.message(),
        "Unable to find the parent scope for FerroProperty lookup Line 2, position 9."
    );
    let error = convert(&empty, Some(&attribute(1))).expect_err("no scope");
    assert_eq!(
        error.message(),
        "Unable to find the Style scope for FerroProperty lookup Line 2, position 9."
    );
}

/// A stand-in for the options markup extension node of the option transformer.
struct TestOptionsNode {
    base: XamlAstNode,
    convertible: bool,
}

xaml_line_info_impl!(TestOptionsNode, base);

impl IXamlAstNode for TestOptionsNode {
    xaml_ast_node_members!("TestOptionsNode", value);

    fn query_interface(self: Rc<Self>, slot: &mut dyn Any) -> bool {
        xaml_query_interface!(self, slot, dyn IOptionsMarkupExtensionNode)
    }
}

impl IXamlAstValueNode for TestOptionsNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        XamlAstClrTypeReference::new(self, xamlx::type_system::XamlPseudoType::unknown(), false)
    }
}

impl IOptionsMarkupExtensionNode for TestOptionsNode {
    fn convert_to_return_type(
        &self,
        _context: &AstTransformationContext,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        if !self.convertible {
            return Ok(None);
        }
        let converted: Rc<dyn IXamlAstValueNode> =
            XamlAstTextNode::with_type(self, "converted", true, Some(type_.clone()));
        Ok(Some(converted))
    }
}

#[test]
fn custom_value_converter_lets_an_options_node_convert_itself() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let options = |convertible: bool| -> Rc<dyn IXamlAstValueNode> {
        Rc::new(TestOptionsNode {
            base: XamlAstNode::new(&XamlLineInfo::new(1, 1)),
            convertible,
        })
    };

    let result = FerroXamlIlLanguage::custom_value_converter(
        &context,
        &options(true),
        None,
        &fw.types.thickness,
    )
    .expect("no error")
    .expect("converted");
    assert_eq!(result.cast::<XamlAstTextNode>().expect("text").text(), "converted");

    // A node that cannot convert its branches is not text either: nothing is converted.
    let result = FerroXamlIlLanguage::custom_value_converter(
        &context,
        &options(false),
        None,
        &fw.types.thickness,
    )
    .expect("no error");
    assert!(result.is_none());
}

#[test]
fn transforms_a_document_with_the_default_transformers() {
    let fw = create_test_framework();
    let document = fw
        .transform(
            r#"<Border xmlns="https://github.com/ferroui" Background="Red" BorderThickness="1,2" CornerRadius="4">
    <TextBlock Text="Hello" TextTrimming="WordEllipsis" Margin="8" />
</Border>"#,
        )
        .expect("transformed");
    let errors: Vec<_> = fw
        .reported_diagnostics()
        .into_iter()
        .filter(|d| d.severity >= xamlx::diagnostics::XamlDiagnosticSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");

    // Collect the type names of every node in the transformed tree.
    struct Collector(Vec<&'static str>);
    impl xamlx::ast::IXamlAstVisitor for Collector {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
            self.0.push(node.type_name());
            Ok(node)
        }
        fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
        fn pop(&mut self) {}
    }
    let mut collector = Collector(Vec::new());
    let root = document.root().expect("root");
    xamlx::ast::visit_node(&root, &mut collector).expect("visited");
    let count = |name: &str| collector.0.iter().filter(|n| **n == name).count();
    // Background="Red" became an immutable brush, the thickness-like values became constants
    // and the text trimming became a static getter call.
    assert_eq!(count("XamlAstNewClrObjectNode"), 1, "{:?}", collector.0);
    assert_eq!(count("FerroXamlIlVectorLikeConstantAstNode"), 3, "{:?}", collector.0);
    assert_eq!(count("XamlStaticOrTargetedReturnMethodCallNode"), 1, "{:?}", collector.0);
}
