//! Tests of the pipeline assembly and of the group transformers, over the test fixture.

use std::cell::RefCell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstExtensions, XamlAstNamePropertyReference,
    XamlAstNewClrObjectNode, XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode,
    XamlAstXamlPropertyValueNode, XamlDeferredContentNode, XamlDocument,
    XamlLoadMethodDelegateNode, XamlManipulationGroupNode, XamlPropertyAssignmentNode,
    XamlStaticOrTargetedReturnMethodCallNode,
};
use xamlx::diagnostics::XamlDiagnosticSeverity;
use xamlx::exceptions::XamlResult;
use xamlx::type_system::{IXamlMethod, IXamlType};

use super::group_transformers::{NewServiceProviderNode, COMPILED_RESOURCES_TYPE_NAME};
use super::transformers::*;
use super::{
    FerroXamlIlCompiler, IXamlDocumentResource, IXamlDocumentTypeBuilderProvider,
    XamlDocumentResource, XamlDocumentUsage,
};
use crate::testing::objects::*;
use crate::testing::TestFramework;

fn compiler(fw: &TestFramework) -> FerroXamlIlCompiler {
    FerroXamlIlCompiler::new(fw.configuration.clone()).expect("pipeline")
}

fn errors(fw: &TestFramework) -> Vec<String> {
    fw.reported_diagnostics()
        .iter()
        .filter(|d| d.severity >= XamlDiagnosticSeverity::Error)
        .map(|d| d.title.clone())
        .collect()
}

fn text_of(value: &Rc<dyn IXamlAstValueNode>) -> String {
    value.cast::<XamlAstTextNode>().expect("text").text()
}

#[test]
fn pipeline_has_the_upstream_order() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    assert_eq!(
        compiler.transformer_names(),
        [
            "XNameTransformer",
            "IgnoredDirectivesTransformer",
            "FerroXamlIlDesignPropertiesTransformer",
            "FerroBindingExtensionTransformer",
            "KnownDirectivesTransformer",
            "XamlIntrinsicsTransformer",
            "XArgumentsTransformer",
            "TypeReferenceResolver",
            "XDataTypeTransformer",
            "MarkupExtensionTransformer",
            "TextNodeMerger",
            "FerroXamlIlResolveClassesPropertiesTransformer",
            "FerroXamlIlTransformRoutedEvent",
            "FerroXamlIlTransformInstanceAttachedProperties",
            "FerroXamlIlTransformSyntheticCompiledBindingMembers",
            "PropertyReferenceResolver",
            "FerroXamlIlFerroPropertyResolver",
            "FerroXamlIlReorderClassesPropertiesTransformer",
            "FerroXamlIlClassesTransformer",
            "FerroXamlIlControlThemeTransformer",
            "FerroXamlIlSelectorTransformer",
            "FerroXamlIlQueryTransformer",
            "FerroXamlIlDuplicateSettersChecker",
            "FerroXamlIlControlTemplateTargetTypeMetadataTransformer",
            "FerroXamlIlBindingPathParser",
            "FerroXamlIlSetterTargetTypeMetadataTransformer",
            "FerroXamlIlSetterTransformer",
            "FerroXamlIlStyleValidatorTransformer",
            "FerroXamlIlConstructorServiceProviderTransformer",
            "FerroXamlIlTransitionsTypeMetadataTransformer",
            "FerroXamlIlResolveByNameMarkupExtensionReplacer",
            "FerroXamlIlThemeVariantProviderTransformer",
            "FerroXamlIlDataTemplateWarningsTransformer",
            "ContentConvertTransformer",
            "RemoveWhitespaceBetweenPropertyValuesTransformer",
            "ResolveContentPropertyTransformer",
            "ResolvePropertyValueAddersTransformer",
            "ApplyWhitespaceNormalization",
            "ObsoleteWarningsTransformer",
            "StaticIntrinsicsPostProcessTransformer",
            "FerroXamlIlOptionMarkupExtensionTransformer",
            "ConvertPropertyValuesToAssignmentsTransformer",
            "ConstructableObjectTransformer",
            "AddNameScopeRegistration",
            "FerroXamlIlControlTemplatePartsChecker",
            "FerroXamlIlDataContextTypeTransformer",
            "FerroXamlIlBindingPathTransformer",
            "FerroXamlResourceTransformer",
            "FerroXamlIlCompiledBindingsMetadataRemover",
            "NewObjectTransformer",
            "DeferredContentTransformer",
            "TopDownInitializationTransformer",
            "FerroXamlIlControlTemplatePriorityTransformer",
            "FerroXamlIlMetadataRemover",
            "FerroXamlIlEnsureResourceDictionaryCapacityTransformer",
            "FerroXamlIlRootObjectScope",
            "FerroXamlIlAddSourceInfoTransformer",
        ]
    );
    let simplifiers: Vec<String> = compiler
        .simplification_transformers
        .iter()
        .map(|t| t.transformer_name())
        .collect();
    assert_eq!(simplifiers, ["FlattenAstTransformer"]);
    let group: Vec<String> = compiler
        .group_transformers
        .iter()
        .map(|t| t.transformer_name())
        .collect();
    assert_eq!(
        group,
        ["XamlMergeResourceGroupTransformer", "FerroXamlIncludeTransformer"]
    );
    assert_eq!(FerroXamlIlCompiler::POPULATE_NAME, "__FerroXamlIlPopulate");
    assert_eq!(FerroXamlIlCompiler::BUILD_NAME, "__FerroXamlIlBuild");
}

#[test]
fn configuration_properties_reach_the_transformers() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    // Upstream defaults: the resource transformer creates source info, the object one not.
    assert!(compiler.create_source_info());
    assert!(!compiler.is_design_mode());
    assert!(!compiler.default_compile_bindings());
    compiler.set_create_source_info(false);
    assert!(!compiler.create_source_info());
    compiler.set_is_design_mode(true);
    assert!(compiler.is_design_mode());
    compiler.set_default_compile_bindings(true);
    assert!(compiler.default_compile_bindings());
    assert!(Rc::ptr_eq(compiler.configuration(), &fw.configuration));
}

const DOCUMENT: &str = "
<ContentControl xmlns='https://github.com/ferroui'
                xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'
                xmlns:d='http://schemas.microsoft.com/expression/blend/2008'
                x:Class='Tests.MainView'
                d:DesignWidth='800' Design.Height='600'
                Tag='root'>
  <ContentControl.Resources>
    <SolidColorBrush x:Key='accent'/>
    <x:String x:Key='title'>Hello</x:String>
  </ContentControl.Resources>
  <Grid>
    <Border x:Name='frame' Classes='card wide' Classes.selected='True' Grid.Row='1' Padding='4'>
      <Button x:Name='ok' Click='OnClick'>OK</Button>
    </Border>
  </Grid>
</ContentControl>";

/// Not a port: the handler lookup of the language (`FerroXamlIlLanguage::custom_value_converter`)
/// accepts a method whose parameters are wider than the ones the delegate passes when no method
/// matches them exactly.
#[test]
fn a_handler_with_wider_parameters_is_the_delegate_of_an_event() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    define_main_view(&fw);
    let handler_of = |name: &str| {
        let xaml = format!(
            "<ContentControl xmlns='https://github.com/ferroui' \
             xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' x:Class='Tests.MainView'>\
             <Button Click='{name}'/></ContentControl>"
        );
        let mut document = compiler.parse(&xaml, None).expect("parsed");
        document.document = Some("MainView.xaml".to_string());
        compiler.transform(&mut document).expect("transformed");
        let root = document.root().expect("root");
        let click = &assignments(&root, "Click")[0];
        let handler = click.values.borrow()[0]
            .cast::<XamlLoadMethodDelegateNode>()
            .expect("handler");
        (handler.method.name(), handler.method.parameters()[1].full_name())
    };
    assert_eq!(handler_of("OnAnything"), ("OnAnything".to_string(), "System.Object".to_string()));
    // The method with exactly the parameters of the delegate is the one upstream finds.
    assert_eq!(
        handler_of("OnClick"),
        ("OnClick".to_string(), "FerroUI.Interactivity.RoutedEventArgs".to_string())
    );
    assert_eq!(errors(&fw), Vec::<String>::new());
}

fn transform_document(fw: &TestFramework, compiler: &FerroXamlIlCompiler) -> XamlDocument {
    define_main_view(fw);
    let mut document = compiler.parse(DOCUMENT, None).expect("parsed");
    document.document = Some("MainView.xaml".to_string());
    compiler.transform(&mut document).expect("transformed");
    document
}

#[test]
fn full_pipeline_transforms_a_document() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let document = transform_document(&fw, &compiler);
    assert_eq!(errors(&fw), Vec::<String>::new());
    let root = document.root().expect("root");

    // The root is the x:Class type, split into creation and population.
    let transformed = compiler.get_transformed_root(&document).expect("root parts");
    assert_eq!(transformed.root_type.full_name(), "Tests.MainView");
    assert!(transformed.root_instance.is::<XamlAstNewClrObjectNode>());
    let manipulation = transformed
        .manipulation
        .expect("manipulation")
        .cast::<XamlManipulationGroupNode>()
        .expect("group");
    assert!(manipulation
        .children
        .borrow()
        .last()
        .expect("children")
        .is::<HandleRootObjectScopeNode>());

    // x:Name: Name assignments followed by name scope registrations.
    let registrations: Vec<(String, String)> = collect::<FerroNameScopeRegistrationXamlIlNode>(&root)
        .iter()
        .map(|r| {
            (
                text_of(&r.name()),
                r.target_type.as_ref().map(|t| t.name()).unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        registrations,
        [
            ("frame".to_string(), "Border".to_string()),
            ("ok".to_string(), "Button".to_string())
        ]
    );
    assert_eq!(assignments(&root, "Name").len(), 2);

    // Classes and single classes, in that order.
    let order: Vec<String> = collect::<XamlPropertyAssignmentNode>(&root)
        .iter()
        .map(|a| a.property.name())
        .filter(|n| n.to_lowercase().contains("class"))
        .collect();
    assert_eq!(order, ["Classes", "Classes", "class:selected"]);
    let selected = &assignments(&root, "class:selected")[0];
    assert_eq!(selected.possible_setters.borrow()[0].type_name(), "ClassValueSetter");

    // The attached property and a parsed value.
    let row = &assignments(&root, "Row")[0];
    assert_eq!(row.property.declaring_type().name(), "Grid");
    assert_eq!(assignments(&root, "Padding").len(), 1);

    // The routed event attribute.
    let click = &assignments(&root, "Click")[0];
    assert_eq!(
        click.possible_setters.borrow()[0].type_name(),
        "XamlDirectCallAddHandler"
    );
    let handler = click.values.borrow()[0]
        .cast::<XamlLoadMethodDelegateNode>()
        .expect("handler");
    assert_eq!(handler.method.name(), "OnClick");

    // Resources: one deferred, one added directly, with a capacity hint.
    let resources = assignments(&root, "Resources");
    assert_eq!(resources.len(), 2);
    assert!(resources[0].values.borrow()[1].is::<XamlDeferredContentNode>());
    assert!(!resources[1].values.borrow()[1].is::<XamlDeferredContentNode>());
    for resource in &resources {
        assert_eq!(resource.possible_setters.borrow()[0].type_name(), "AdderSetter");
        assert!(resource.possible_setters.borrow()[0]
            .as_any()
            .is::<ResourceAdderSetter>());
    }
    let capacity = collect::<EnsureCapacityNode>(&root);
    assert_eq!(capacity.len(), 1);
    assert_eq!(capacity[0].capacity, 2);

    // Design properties are dropped outside of design mode; no metadata nodes are left.
    assert!(assignments(&root, "Width").is_empty());
    assert!(assignments(&root, "Height").is_empty());
    assert!(collect::<FerroXamlIlTargetTypeMetadataNode>(&root).is_empty());
    assert!(collect::<NestedScopeMetadataNode>(&root).is_empty());
    assert!(collect::<XamlAstObjectNode>(&root).is_empty());
    assert!(collect::<XamlAstXamlPropertyValueNode>(&root).is_empty());
    // Source info for objects is off by default.
    assert!(collect::<XamlSourceInfoValueManipulation>(&root).is_empty());
}

#[test]
fn full_pipeline_in_design_mode_with_source_info() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    compiler.set_is_design_mode(true);
    compiler.set_create_source_info(true);
    let document = transform_document(&fw, &compiler);
    assert_eq!(errors(&fw), Vec::<String>::new());
    let root = document.root().expect("root");
    for name in ["Width", "Height"] {
        let assignment = &assignments(&root, name)[0];
        assert_eq!(assignment.property.declaring_type().name(), "Design");
    }
    let infos = collect::<XamlSourceInfoValueManipulation>(&root);
    assert!(infos.len() >= 5, "{}", dump_tree(&root));
    assert!(infos.iter().all(|i| i.document.as_deref() == Some("MainView.xaml")));
}

#[test]
fn parse_resolves_and_overrides_the_root_type() {
    let fw = create_objects_test_framework();
    define_main_view(&fw);
    let compiler = compiler(&fw);

    // Without x:Class the root element type is resolved.
    let document = compiler
        .parse(&format!("<Border {XMLNS} Tag='t' Grid.Row='1'/>"), None)
        .expect("parsed");
    let root = document.root().expect("root").cast::<XamlAstObjectNode>().expect("object");
    assert_eq!(
        root.type_.borrow().clone().get_clr_type().expect("clr").name(),
        "Border"
    );
    // The property references of the root's attributes follow the root type.
    let references = collect::<XamlAstNamePropertyReference>(&root.as_node());
    let tag = references.iter().find(|r| r.name() == "Tag").expect("Tag");
    assert!(tag.declaring_type.borrow().same_node(&root.type_.borrow().clone()));
    assert!(tag.target_type.borrow().same_node(&root.type_.borrow().clone()));
    let row = references.iter().find(|r| r.name() == "Row").expect("Row");
    assert!(!row.declaring_type.borrow().same_node(&root.type_.borrow().clone()));
    assert!(row.target_type.borrow().same_node(&root.type_.borrow().clone()));

    // x:Class wins over the element type.
    let document = compiler
        .parse(&format!("<ContentControl {XMLNS} x:Class='Tests.MainView'/>"), None)
        .expect("parsed");
    let root = document.root().expect("root").cast::<XamlAstObjectNode>().expect("object");
    assert_eq!(
        root.type_.borrow().clone().get_clr_type().expect("clr").full_name(),
        "Tests.MainView"
    );

    // An override must derive from the declared root type.
    let document = compiler
        .parse(
            &format!("<ContentControl {XMLNS}/>"),
            Some(fw.t("FerroUI.Controls.Button")),
        )
        .expect("parsed");
    let root = document.root().expect("root").cast::<XamlAstObjectNode>().expect("object");
    assert_eq!(
        root.type_.borrow().clone().get_clr_type().expect("clr").name(),
        "Button"
    );
    let error = compiler
        .parse(
            &format!("<ContentControl {XMLNS}/>"),
            Some(fw.t("FerroUI.Controls.Border")),
        )
        .err()
        .expect("error");
    assert!(
        error.message().starts_with(
            "Unable to substitute FerroUI.Controls:FerroUI.Controls.ContentControl with FerroUI.Controls:FerroUI.Controls.Border"
        ),
        "{}",
        error.message()
    );
    // An unknown x:Class type is a type system error.
    assert!(compiler
        .parse(&format!("<ContentControl {XMLNS} x:Class='No.Such'/>"), None)
        .is_err());
}

// Group transformers

struct TestTypeBuilderProvider {
    populate: Rc<dyn IXamlMethod>,
    build: Option<Rc<dyn IXamlMethod>>,
}

impl IXamlDocumentTypeBuilderProvider for TestTypeBuilderProvider {
    fn populate_method(&self) -> Rc<dyn IXamlMethod> {
        self.populate.clone()
    }
    fn build_method(&self) -> Option<Rc<dyn IXamlMethod>> {
        self.build.clone()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Parses and transforms a document and wraps it as a compilation resource whose generated
/// methods are declared on a fake output type.
fn resource(
    fw: &TestFramework,
    compiler: &FerroXamlIlCompiler,
    name: &str,
    xaml: &str,
    with_build_method: bool,
    class_type: Option<Rc<dyn IXamlType>>,
) -> Rc<dyn IXamlDocumentResource> {
    let mut document = compiler.parse(xaml, None).expect("parsed");
    document.document = Some(format!("{name}.xaml"));
    compiler.transform(&mut document).expect("transformed");
    let root_type = compiler
        .get_transformed_root(&document)
        .expect("root parts")
        .root_type;
    let output = fw.controls.define_class("CompiledTests", &format!("{name}_Output"));
    let sp = fw.t("System.IServiceProvider");
    let populate: Rc<dyn IXamlMethod> = output.add_method(
        FerroXamlIlCompiler::POPULATE_NAME,
        fw.t("System.Void"),
        vec![sp.clone(), root_type.clone()],
        true,
    );
    let build: Option<Rc<dyn IXamlMethod>> = with_build_method.then(|| {
        let build: Rc<dyn IXamlMethod> =
            output.add_method(FerroXamlIlCompiler::BUILD_NAME, root_type, vec![sp], true);
        build
    });
    let resource: Rc<dyn IXamlDocumentResource> = XamlDocumentResource::new(
        Rc::new(RefCell::new(document)),
        Some(format!("ferres://Tests/{name}.xaml")),
        None,
        class_type,
        true,
        Box::new(move || -> XamlResult<Rc<dyn IXamlDocumentTypeBuilderProvider>> {
            Ok(Rc::new(TestTypeBuilderProvider {
                populate: populate.clone(),
                build: build.clone(),
            }))
        }),
    );
    resource
}

fn root_of(resource: &Rc<dyn IXamlDocumentResource>) -> Rc<dyn IXamlAstNode> {
    resource.xaml_document().borrow().root().expect("root")
}

fn dictionary(body: &str) -> String {
    format!("<ResourceDictionary {XMLNS}>{body}</ResourceDictionary>")
}

#[test]
fn merge_resource_include_is_inlined() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(
            "<ResourceDictionary.MergedDictionaries><MergeResourceInclude Source='/B.xaml'/></ResourceDictionary.MergedDictionaries><x:String x:Key='a'>1</x:String>",
        ),
        true,
        None,
    );
    let b = resource(
        &fw,
        &compiler,
        "B",
        &dictionary("<x:String x:Key='b1'>1</x:String><x:String x:Key='b2'>2</x:String>"),
        true,
        None,
    );
    assert_eq!(errors(&fw), Vec::<String>::new());
    compiler.transform_group(&[a.clone(), b.clone()]).expect("group");
    assert_eq!(errors(&fw), Vec::<String>::new());

    let root = root_of(&a);
    assert!(assignments(&root, "MergedDictionaries").is_empty());
    let keys: Vec<String> = assignments(&root, "Content")
        .iter()
        .map(|c| text_of(&c.values.borrow()[0]))
        .collect();
    // As upstream: the merged group is put in front and then flattened, which appends its
    // members behind the own resources.
    assert_eq!(keys, ["a", "b1", "b2"]);
    let capacity = collect::<EnsureCapacityNode>(&root);
    assert_eq!(capacity.len(), 1);
    assert_eq!(capacity[0].capacity, 3);
    assert_eq!(b.usage(), XamlDocumentUsage::Merged);
    assert_eq!(a.usage(), XamlDocumentUsage::Unknown);
}

#[test]
fn merge_resource_include_merges_theme_dictionaries() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let theme = |key: &str| {
        format!(
            "<ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Dark'><x:String x:Key='{key}1'>1</x:String><x:String x:Key='{key}2'>2</x:String></ResourceDictionary></ResourceDictionary.ThemeDictionaries>"
        )
    };
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(&format!(
            "<ResourceDictionary.MergedDictionaries><MergeResourceInclude Source='ferres://Tests/B.xaml'/></ResourceDictionary.MergedDictionaries>{}",
            theme("a")
        )),
        true,
        None,
    );
    let b = resource(&fw, &compiler, "B", &dictionary(&theme("b")), true, None);
    compiler.transform_group(&[a.clone(), b]).expect("group");
    assert_eq!(errors(&fw), Vec::<String>::new());

    let root = root_of(&a);
    let themes = assignments(&root, "ThemeDictionaries");
    assert_eq!(themes.len(), 1, "{}", dump_tree(&root));
    let keys: Vec<String> = assignments(&themes[0].as_node(), "Content")
        .iter()
        .map(|c| text_of(&c.values.borrow()[0]))
        .collect();
    // The included document's only manipulation is its theme dictionary, which stays in
    // front; the own entries are appended to it.
    assert_eq!(keys, ["b1", "b2", "a1", "a2"]);
    let capacity = collect::<EnsureCapacityNode>(&themes[0].as_node());
    assert_eq!(capacity.len(), 1);
    assert_eq!(capacity[0].capacity, 4);
}

#[test]
fn merge_resource_include_errors() {
    // The target is not part of the compilation.
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(
            "<ResourceDictionary.MergedDictionaries><MergeResourceInclude Source='/Missing.xaml'/></ResourceDictionary.MergedDictionaries><x:String x:Key='own'>1</x:String>",
        ),
        true,
        None,
    );
    compiler.transform_group(&[a.clone()]).expect("group");
    assert_eq!(
        errors(&fw)
            .iter()
            .filter(|e| e.starts_with(
                "Node MergeResourceInclude is unable to resolve \"ferres://tests/Missing.xaml\" path."
            ))
            .count(),
        1,
        "{:?}",
        errors(&fw)
    );

    // Another dictionary after a MergeResourceInclude.
    let fw = create_objects_test_framework();
    let compiler = self::compiler(&fw);
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(
            "<ResourceDictionary.MergedDictionaries><MergeResourceInclude Source='/B.xaml'/><ResourceDictionary/></ResourceDictionary.MergedDictionaries>",
        ),
        true,
        None,
    );
    let b = resource(&fw, &compiler, "B", &dictionary(""), true, None);
    compiler.transform_group(&[a.clone(), b.clone()]).expect("group");
    assert!(
        errors(&fw).iter().any(|e| e.starts_with(
            "MergeResourceInclude should always be included last when mixing with other dictionaries"
        )),
        "{:?}",
        errors(&fw)
    );
    assert_eq!(b.usage(), XamlDocumentUsage::Unknown);

    // The target is not a resource dictionary.
    let fw = create_objects_test_framework();
    let compiler = self::compiler(&fw);
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(
            "<ResourceDictionary.MergedDictionaries><MergeResourceInclude Source='/B.xaml'/></ResourceDictionary.MergedDictionaries><x:String x:Key='own'>1</x:String>",
        ),
        true,
        None,
    );
    let b = resource(&fw, &compiler, "B", &format!("<Border {XMLNS}/>"), true, None);
    compiler.transform_group(&[a, b]).expect("group");
    assert!(
        errors(&fw)
            .iter()
            .any(|e| e.starts_with("MergeResourceInclude can only include another ResourceDictionary")),
        "{:?}",
        errors(&fw)
    );

    // A source that is not a constant resource URI is an error for MergeResourceInclude.
    let fw = create_objects_test_framework();
    let compiler = self::compiler(&fw);
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(
            "<ResourceDictionary.MergedDictionaries><MergeResourceInclude Source='https://example.org/B.xaml'/></ResourceDictionary.MergedDictionaries><x:String x:Key='own'>1</x:String>",
        ),
        true,
        None,
    );
    compiler.transform_group(&[a]).expect("group");
    let diagnostics = fw.reported_diagnostics();
    assert!(diagnostics.iter().any(|d| d.severity == XamlDiagnosticSeverity::Error
        && d.title.starts_with(
            "\"MergeResourceInclude.Source\" supports only \"ferres://\" absolute or relative uri."
        )));
}

const INCLUDE: &str =
    "<ResourceDictionary.MergedDictionaries><ResourceInclude Source='/B.xaml'/></ResourceDictionary.MergedDictionaries>";

#[test]
fn include_is_linked_to_the_build_method_of_the_target() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let a = resource(&fw, &compiler, "A", &dictionary(INCLUDE), true, None);
    let b = resource(
        &fw,
        &compiler,
        "B",
        &dictionary("<x:String x:Key='b'>1</x:String>"),
        true,
        None,
    );
    compiler.transform_group(&[a.clone(), b.clone()]).expect("group");
    assert_eq!(errors(&fw), Vec::<String>::new());

    let root = root_of(&a);
    let merged = &assignments(&root, "MergedDictionaries")[0];
    let value = merged.values.borrow()[0].clone();
    let wrapper = value.as_value_with_manipulation_node().expect("wrapper");
    let call = wrapper
        .value()
        .cast::<XamlStaticOrTargetedReturnMethodCallNode>()
        .expect("build call");
    assert_eq!(call.base.method().name(), FerroXamlIlCompiler::BUILD_NAME);
    assert_eq!(call.base.method().declaring_type().name(), "B_Output");
    let arguments = call.base.arguments.borrow().clone();
    assert_eq!(arguments.len(), 1);
    let provider = arguments[0].cast::<NewServiceProviderNode>().expect("service provider");
    let provider_node: Rc<dyn IXamlAstNode> = provider.clone();
    assert!(provider_node
        .as_needs_parent_stack()
        .expect("needs parent stack")
        .needs_parent_stack());
    assert_eq!(
        arguments[0].type_().get_clr_type().expect("typed").name(),
        "IServiceProvider"
    );
    assert_eq!(
        NewServiceProviderNode::CREATE_ROOT_SERVICE_PROVIDER_METHOD_NAME,
        "CreateRootServiceProviderV3"
    );
    // The include object is gone and the target counts as used.
    assert!(collect::<XamlAstNewClrObjectNode>(&root)
        .iter()
        .all(|o| o.type_.borrow().clone().get_clr_type().expect("clr").name() != "ResourceInclude"));
    assert_eq!(b.usage(), XamlDocumentUsage::Used);
}

#[test]
fn include_falls_back_to_the_class_of_the_target() {
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let themed = fw.controls.define_class("Tests", "ThemeResources");
    themed.set_base_type(fw.t("FerroUI.Controls.ResourceDictionary"));
    themed.add_constructor(vec![]);
    let a = resource(&fw, &compiler, "A", &dictionary(INCLUDE), true, None);
    let b = resource(
        &fw,
        &compiler,
        "B",
        &dictionary(""),
        false,
        Some(themed.as_type()),
    );
    compiler.transform_group(&[a.clone(), b]).expect("group");
    assert_eq!(errors(&fw), Vec::<String>::new());
    let root = root_of(&a);
    let merged = &assignments(&root, "MergedDictionaries")[0];
    let created = collect::<XamlAstNewClrObjectNode>(&merged.as_node());
    assert_eq!(
        created[0].type_.borrow().clone().get_clr_type().expect("clr").name(),
        "ThemeResources"
    );
}

#[test]
fn include_of_another_assembly_uses_its_compiled_resources_type() {
    // The host part of a URI is lower-cased, so the assembly is looked up by its lower-case
    // name.
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let other = fw.type_system.define_assembly("other.library");
    let (namespace, name) = COMPILED_RESOURCES_TYPE_NAME.split_once('.').expect("name");
    let resources = other.define_class(namespace, name);
    resources.add_method(
        "Build:/Themes/Dark.xaml",
        fw.t("FerroUI.Controls.ResourceDictionary"),
        vec![fw.t("System.IServiceProvider")],
        true,
    );
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(
            "<ResourceDictionary.MergedDictionaries><ResourceInclude Source='ferres://other.library/Themes/Dark.xaml'/><ResourceInclude Source='ferres://other.library/Themes/Light.xaml'/><ResourceInclude Source='ferres://missing.library/Themes/Light.xaml'/></ResourceDictionary.MergedDictionaries>",
        ),
        true,
        None,
    );
    compiler.transform_group(&[a.clone()]).expect("group");
    let root = root_of(&a);
    let calls = collect::<XamlStaticOrTargetedReturnMethodCallNode>(&root);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].base.method().name(), "Build:/Themes/Dark.xaml");
    // Each diagnostic is reported twice: the group transformers visit the children of the
    // root twice (as upstream).
    let mut reported = errors(&fw);
    reported.dedup();
    reported.sort();
    reported.dedup();
    assert_eq!(
        reported,
        [
            "Assembly \"missing.library\" was not found from the \"ferres://missing.library/Themes/Light.xaml\" source. Line 1, position 363.".to_string(),
            "Unable to resolve XAML resource \"ferres://other.library/Themes/Light.xaml\" in the \"other.library\" assembly. Make sure this file exists and is public. Line 1, position 295.".to_string(),
        ]
    );
}

#[test]
fn include_diagnostics() {
    // The target has neither a build method nor a class.
    let fw = create_objects_test_framework();
    let compiler = compiler(&fw);
    let a = resource(&fw, &compiler, "A", &dictionary(INCLUDE), true, None);
    let b = resource(&fw, &compiler, "B", &dictionary(""), false, None);
    compiler.transform_group(&[a, b]).expect("group");
    assert!(
        errors(&fw).iter().any(|e| e.starts_with(
            "Unable to resolve XAML resource \"ferres://tests/B.xaml\" in the current assembly."
        )),
        "{:?}",
        errors(&fw)
    );

    // The target has the wrong type.
    let fw = create_objects_test_framework();
    let compiler = self::compiler(&fw);
    let a = resource(&fw, &compiler, "A", &dictionary(INCLUDE), true, None);
    let b = resource(&fw, &compiler, "B", &format!("<Border {XMLNS}/>"), true, None);
    compiler.transform_group(&[a, b]).expect("group");
    assert!(
        errors(&fw).iter().any(|e| e.starts_with(
            "Resource \"ferres://tests/B.xaml\" is defined as \"FerroUI.Controls.Border\" type in the \"tests\" assembly, but expected \"FerroUI.Controls.IResourceDictionary\"."
        )),
        "{:?}",
        errors(&fw)
    );

    // A source outside of the resource scheme is only a warning: it is resolved at run time.
    let fw = create_objects_test_framework();
    let compiler = self::compiler(&fw);
    let a = resource(
        &fw,
        &compiler,
        "A",
        &dictionary(
            "<ResourceDictionary.MergedDictionaries><ResourceInclude Source='https://example.org/B.xaml'/></ResourceDictionary.MergedDictionaries>",
        ),
        true,
        None,
    );
    compiler.transform_group(&[a.clone()]).expect("group");
    assert_eq!(errors(&fw), Vec::<String>::new());
    let mut warnings: Vec<String> = fw
        .reported_diagnostics()
        .iter()
        .filter(|d| d.severity == XamlDiagnosticSeverity::Warning)
        .map(|d| d.title.clone())
        .collect();
    warnings.dedup();
    assert_eq!(
        warnings,
        ["\"ResourceInclude.Source\" supports only \"ferres://\" absolute or relative uri. This ResourceInclude will be resolved in runtime instead."]
    );
    // The include object is kept.
    assert!(collect::<XamlAstNewClrObjectNode>(&root_of(&a))
        .iter()
        .any(|o| o.type_.borrow().clone().get_clr_type().expect("clr").name() == "ResourceInclude"));
}
