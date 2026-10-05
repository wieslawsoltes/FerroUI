//! Fixture additions and helpers for the tests of the object, property, resource and pipeline
//! transformers.
//!
//! [`extend_objects_fixture`] declares, with the names and signatures of the real framework
//! types, what those transformers look for and the base fixture lacks: `INamed`, the `Design`
//! class, `ResolveByNameAttribute` and a `Label` using it, the adders of the resource
//! dictionary interfaces, `ResourceDictionary.ThemeDictionaries`/`MergedDictionaries`, the
//! `Loaded` properties of the include classes, and the `OnPlatform`/`OnFormFactor` option
//! markup extensions.

use std::collections::HashMap;
use std::rc::Rc;

use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstVisitor, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlDocument, XamlPropertyAssignmentNode, XamlRootObjectNode,
};
use xamlx::diagnostics::XamlDiagnostic;
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::parsers::XDocumentXamlParser;
use xamlx::testing::{FakeCustomAttribute, FakeType};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlType, XamlValue};
use xamlx::XamlNamespaces;

use super::{create_test_framework, TestFramework};
use crate::compiler_extensions::transformers::FERRO_XML_NAMESPACE;
use crate::compiler_extensions::FerroXamlIlCompiler;

/// The xmlns declarations test documents start with: the framework namespace as default,
/// `x` for the XAML language and `d` for design-time markup.
pub const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:d='http://schemas.microsoft.com/expression/blend/2008'";

fn has_property(type_: &FakeType, name: &str) -> bool {
    type_.properties().iter().any(|p| p.name() == name)
}

fn has_method(type_: &FakeType, name: &str) -> bool {
    type_.methods().iter().any(|m| m.name() == name)
}

fn generic(fw: &TestFramework, definition: &str, arguments: &[Rc<dyn IXamlType>]) -> Rc<dyn IXamlType> {
    match fw.t(definition).make_generic_type(arguments) {
        Ok(type_) => type_,
        Err(e) => panic!("Unable to construct {definition}: {e}"),
    }
}

/// Adds the framework types and members described in the module documentation to the fake
/// framework. Calling it again is harmless.
pub fn extend_objects_fixture(fw: &TestFramework) {
    let ts = &fw.type_system;
    let exists = |name: &str| fw.as_type_system().find_type(name).is_some();
    let void = fw.t("System.Void");
    let object = fw.t("System.Object");
    let string = fw.t("System.String");
    let boolean = fw.t("System.Boolean");
    let double = fw.t("System.Double");
    let control = fw.t("FerroUI.Controls.Control");
    let attr = |name: &str| FakeCustomAttribute::new(fw.t(name), vec![]);

    // INamed
    if !exists("FerroUI.INamed") {
        let named = fw.base.define_interface("FerroUI", "INamed");
        named.add_property_with("Name", string.clone(), true, false, false);
        fw.fake_type("FerroUI.StyledElement").add_interface(named.as_type());
    }

    // Classes.Set(string, bool)
    let classes = fw.fake_type("FerroUI.Controls.Classes");
    if !has_method(&classes, "Set") {
        classes.add_method("Set", void.clone(), vec![string.clone(), boolean.clone()], false);
    }

    // Animatable.Transitions
    let animatable = fw.fake_type("FerroUI.Animation.Animatable");
    if !has_property(&animatable, "Transitions") {
        animatable.add_property("Transitions", fw.t("FerroUI.Animation.Transitions"));
    }

    // Design
    if !exists("FerroUI.Controls.Design") {
        let design = fw.controls.define_class("FerroUI.Controls", "Design");
        for (name, value_type) in [
            ("Width", double.clone()),
            ("Height", double.clone()),
            ("DataContext", object.clone()),
            ("PreviewWith", control.clone()),
        ] {
            design.add_method(&format!("Get{name}"), value_type.clone(), vec![control.clone()], true);
            design.add_method(
                &format!("Set{name}"),
                void.clone(),
                vec![control.clone(), value_type.clone()],
                true,
            );
            design.add_field(
                &format!("{name}Property"),
                generic(fw, "FerroUI.AttachedProperty`1", &[value_type]),
                true,
                None,
            );
        }
    }

    // ResolveByName
    if !exists("FerroUI.Controls.ResolveByNameAttribute") {
        let attribute = fw.base.define_class("FerroUI.Controls", "ResolveByNameAttribute");
        attribute.set_base_type(fw.t("System.Attribute"));
        let label = fw.controls.define_class("FerroUI.Controls", "Label");
        label.set_base_type(fw.t("FerroUI.Controls.ContentControl"));
        label.add_constructor(vec![]);
        label
            .add_property("Target", control.clone())
            .add_attribute(attr("FerroUI.Controls.ResolveByNameAttribute"));
        let extension =
            fw.fake_type("FerroUI.Markup.Xaml.MarkupExtensions.ResolveByNameExtension");
        if !extension.constructors().iter().any(|c| c.parameters().len() == 1) {
            extension.add_constructor(vec![string.clone()]);
        }
    }

    // Resource dictionaries
    let resource_dictionary_interface = fw.fake_type("FerroUI.Controls.IResourceDictionary");
    if !has_method(&resource_dictionary_interface, "Add") {
        resource_dictionary_interface.add_method(
            "Add",
            void.clone(),
            vec![object.clone(), object.clone()],
            false,
        );
    }
    let dictionary = ts.get_fake_type("System.Collections.Generic.IDictionary`2");
    if !has_method(&dictionary, "Add") {
        dictionary.add_method(
            "Add",
            void.clone(),
            vec![dictionary.generic_parameter(0), dictionary.generic_parameter(1)],
            false,
        );
    }
    let theme_variant_provider = fw.fake_type("FerroUI.Controls.IThemeVariantProvider");
    if !has_property(&theme_variant_provider, "Key") {
        theme_variant_provider.add_property("Key", fw.t("FerroUI.Styling.ThemeVariant"));
    }
    if !exists("FerroUI.Controls.IResourceProvider") {
        let provider = fw.base.define_interface("FerroUI.Controls", "IResourceProvider");
        resource_dictionary_interface.add_interface(provider.as_type());
        for include in ["ResourceInclude", "MergeResourceInclude"] {
            fw.fake_type(&format!("FerroUI.Markup.Xaml.Styling.{include}"))
                .add_interface(provider.as_type());
        }
    }
    let resource_dictionary = fw.fake_type("FerroUI.Controls.ResourceDictionary");
    if !has_property(&resource_dictionary, "ThemeDictionaries") {
        resource_dictionary.add_property_with(
            "ThemeDictionaries",
            generic(
                fw,
                "System.Collections.Generic.IDictionary`2",
                &[
                    fw.t("FerroUI.Styling.ThemeVariant"),
                    theme_variant_provider.as_type(),
                ],
            ),
            true,
            false,
            false,
        );
    }
    if !has_property(&resource_dictionary, "MergedDictionaries") {
        resource_dictionary.add_property_with(
            "MergedDictionaries",
            generic(
                fw,
                "System.Collections.Generic.IList`1",
                &[fw.t("FerroUI.Controls.IResourceProvider")],
            ),
            true,
            false,
            false,
        );
    }
    let list = ts.get_fake_type("System.Collections.Generic.IList`1");
    if !has_method(&list, "Add") {
        list.add_method("Add", void.clone(), vec![list.generic_parameter(0)], false);
    }

    // Includes
    for (include, loaded) in [
        ("StyleInclude", "FerroUI.Styling.IStyle"),
        ("ResourceInclude", "FerroUI.Controls.IResourceDictionary"),
    ] {
        let include = fw.fake_type(&format!("FerroUI.Markup.Xaml.Styling.{include}"));
        if !has_property(&include, "Loaded") {
            include.add_property_with("Loaded", fw.t(loaded), true, false, false);
        }
    }

    // Option markup extensions
    if !exists("FerroUI.Markup.Xaml.MarkupExtensions.OnPlatformExtension") {
        let option = |value: XamlValue| {
            FakeCustomAttribute::new(
                fw.t("FerroUI.Metadata.MarkupExtensionOptionAttribute"),
                vec![value],
            )
        };
        let on = fw.fake_type("FerroUI.Markup.Xaml.MarkupExtensions.On");
        if !has_property(&on, "Options") {
            on.add_property("Options", string.clone());
        }

        let on_platform = fw
            .markup_xaml
            .define_class("FerroUI.Markup.Xaml.MarkupExtensions", "OnPlatformExtension");
        on_platform.add_constructor(vec![]);
        on_platform.add_constructor(vec![object.clone()]);
        on_platform.add_method("ProvideValue", object.clone(), vec![], false);
        on_platform.add_method("ShouldProvideOption", boolean.clone(), vec![string.clone()], true);
        on_platform
            .add_property("Default", object.clone())
            .add_attribute(attr("FerroUI.Metadata.MarkupExtensionDefaultOptionAttribute"));
        for (name, value) in [("Windows", "WINDOWS"), ("macOS", "OSX"), ("Linux", "LINUX")] {
            on_platform
                .add_property(name, object.clone())
                .add_attribute(option(XamlValue::String(value.to_string())));
        }
        on_platform
            .add_property("Content", object.clone())
            .add_attribute(attr("FerroUI.Metadata.ContentAttribute"));

        let form_factor = fw.base.define_enum(
            "FerroUI.Platform",
            "FormFactorType",
            &[("Unknown", 0), ("Desktop", 1), ("Mobile", 2)],
        );
        let on_form_factor = fw
            .markup_xaml
            .define_class("FerroUI.Markup.Xaml.MarkupExtensions", "OnFormFactorExtension");
        on_form_factor.add_constructor(vec![]);
        on_form_factor.add_constructor(vec![object.clone()]);
        on_form_factor.add_method(
            "ProvideValue",
            object.clone(),
            vec![fw.t("System.IServiceProvider")],
            false,
        );
        on_form_factor.add_method(
            "ShouldProvideOption",
            boolean,
            vec![fw.t("System.IServiceProvider"), form_factor.as_type()],
            false,
        );
        on_form_factor
            .add_property("Default", object.clone())
            .add_attribute(attr("FerroUI.Metadata.MarkupExtensionDefaultOptionAttribute"));
        for (name, value) in [("Desktop", 1), ("Mobile", 2)] {
            on_form_factor
                .add_property(name, object.clone())
                .add_attribute(option(XamlValue::Int32(value)));
        }
        // An option whose value no ShouldProvideOption overload accepts.
        on_form_factor
            .add_property("Tv", object.clone())
            .add_attribute(option(XamlValue::String("Television".to_string())));
    }
}

/// The fake framework with [`extend_objects_fixture`] applied.
pub fn create_objects_test_framework() -> TestFramework {
    let fw = create_test_framework();
    extend_objects_fixture(&fw);
    fw
}

/// Parses a document the way the compiler does (design-time markup is kept).
///
/// # Panics
/// Panics when the text is not well-formed XAML (a test set-up error).
pub fn parse(xaml: &str) -> XamlDocument {
    let mut compatibility_mappings = HashMap::new();
    compatibility_mappings.insert(
        XamlNamespaces::BLEND2008.to_string(),
        XamlNamespaces::BLEND2008.to_string(),
    );
    let mut document = match XDocumentXamlParser::parse(xaml, Some(&compatibility_mappings)) {
        Ok(document) => document,
        Err(e) => panic!("Unable to parse the test document: {e}"),
    };
    document.document = Some("test.xaml".to_string());
    document
}

/// Runs `transformers` over the document like `XamlCompiler.Transform` does.
pub fn run_transformers(
    fw: &TestFramework,
    document: &mut XamlDocument,
    transformers: &[Box<dyn IXamlAstTransformer>],
) -> XamlResult<()> {
    let ctx = AstTransformationContext::new(fw.configuration.clone(), Some(document));
    let mut root = document.root()?;
    let root_object = root
        .cast::<XamlAstObjectNode>()
        .ok_or_else(|| XamlError::invalid_cast("The root is not an object node"))?;
    ctx.set_root_object(XamlRootObjectNode::new(&root_object));
    for transformer in transformers {
        ctx.visit_children(&*ctx.root_object()?, &**transformer)?;
        root = ctx.visit(&root, &**transformer)?;
    }
    document.set_root(root);
    Ok(())
}

/// The names of the transformers written by the other areas (styles, selectors, templates,
/// bindings); [`objects_pipeline`] leaves them out.
const OTHER_AREAS: &[&str] = &[
    "FerroBindingExtensionTransformer",
    "FerroXamlIlTransformSyntheticCompiledBindingMembers",
    "FerroXamlIlControlThemeTransformer",
    "FerroXamlIlSelectorTransformer",
    "FerroXamlIlQueryTransformer",
    "FerroXamlIlDuplicateSettersChecker",
    "FerroXamlIlControlTemplateTargetTypeMetadataTransformer",
    "FerroXamlIlBindingPathParser",
    "FerroXamlIlSetterTargetTypeMetadataTransformer",
    "FerroXamlIlSetterTransformer",
    "FerroXamlIlStyleValidatorTransformer",
    "FerroXamlIlDataTemplateWarningsTransformer",
    "XDataTypeTransformer",
    "FerroXamlIlControlTemplatePartsChecker",
    "FerroXamlIlDataContextTypeTransformer",
    "FerroXamlIlBindingPathTransformer",
    "FerroXamlIlControlTemplatePriorityTransformer",
];

/// The compiler pipeline reduced to the XamlX standard transformers and the object, property
/// and resource transformers, in their pipeline positions.
///
/// # Panics
/// Panics when the pipeline cannot be assembled (a test set-up error).
pub fn objects_pipeline(fw: &TestFramework) -> FerroXamlIlCompiler {
    let mut compiler = match FerroXamlIlCompiler::new(fw.configuration.clone()) {
        Ok(compiler) => compiler,
        Err(e) => panic!("Unable to assemble the pipeline: {e}"),
    };
    compiler
        .transformers
        .retain(|t| !OTHER_AREAS.contains(&t.transformer_name().as_str()));
    compiler
}

/// Parses and transforms `xaml` with [`objects_pipeline`] and returns the transformed root.
pub fn transform_objects(fw: &TestFramework, xaml: &str) -> XamlResult<Rc<dyn IXamlAstNode>> {
    transform_with(&objects_pipeline(fw), xaml)
}

/// Parses and transforms `xaml` with the given compiler and returns the transformed root.
pub fn transform_with(compiler: &FerroXamlIlCompiler, xaml: &str) -> XamlResult<Rc<dyn IXamlAstNode>> {
    let mut document = parse(xaml);
    compiler.transform(&mut document)?;
    document.root()
}

struct Collector<'a> {
    nodes: Vec<Rc<dyn IXamlAstNode>>,
    predicate: &'a dyn Fn(&Rc<dyn IXamlAstNode>) -> bool,
}

impl IXamlAstVisitor for Collector<'_> {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if (self.predicate)(&node) {
            self.nodes.push(node.clone());
        }
        Ok(node)
    }
    fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
    fn pop(&mut self) {}
}

/// Every node of type `T` below (and including) `root`, in document order.
///
/// # Panics
/// Panics when a node fails to visit its children (it cannot with a read-only visitor).
pub fn collect<T: IXamlAstNode>(root: &Rc<dyn IXamlAstNode>) -> Vec<Rc<T>> {
    let predicate = |node: &Rc<dyn IXamlAstNode>| node.is::<T>();
    let mut collector = Collector {
        nodes: Vec::new(),
        predicate: &predicate,
    };
    if let Err(e) = visit_node(root, &mut collector) {
        panic!("Unable to walk the tree: {e}");
    }
    collector
        .nodes
        .iter()
        .filter_map(|n| n.cast::<T>())
        .collect()
}

/// The property assignments of the property named `name` below `root`, in document order.
pub fn assignments(root: &Rc<dyn IXamlAstNode>, name: &str) -> Vec<Rc<XamlPropertyAssignmentNode>> {
    collect::<XamlPropertyAssignmentNode>(root)
        .into_iter()
        .filter(|a| a.property.name() == name)
        .collect()
}

/// The titles of the reported diagnostics.
pub fn diagnostic_titles(fw: &TestFramework) -> Vec<String> {
    fw.reported_diagnostics()
        .iter()
        .map(|d: &XamlDiagnostic| d.title.clone())
        .collect()
}

/// `FERRO_XML_NAMESPACE` must match the namespace [`XMLNS`] declares.
pub fn assert_xmlns_matches() {
    assert!(XMLNS.contains(FERRO_XML_NAMESPACE));
}

struct Dumper {
    depth: usize,
    out: String,
}

impl IXamlAstVisitor for Dumper {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let mut line = format!("{}{}", "  ".repeat(self.depth), node.type_name());
        if let Some(assignment) = node.cast::<XamlPropertyAssignmentNode>() {
            let setters: Vec<&'static str> = assignment
                .possible_setters
                .borrow()
                .iter()
                .map(|s| s.type_name())
                .collect();
            line.push_str(&format!(
                " {} [{}]",
                assignment.property.name(),
                setters.join(", ")
            ));
        } else if let Some(text) = node.cast::<xamlx::ast::XamlAstTextNode>() {
            line.push_str(&format!(" {:?}", text.text()));
        } else if let Some(type_reference) = node.cast::<xamlx::ast::XamlAstClrTypeReference>() {
            line.push_str(&format!(" {}", type_reference.type_.name()));
        }
        self.out.push_str(&line);
        self.out.push('\n');
        Ok(node)
    }
    fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {
        self.depth += 1;
    }
    fn pop(&mut self) {
        self.depth -= 1;
    }
}

/// An indented outline of the tree below `root` (node type names; property names and setter
/// kinds for assignments, the text of text nodes), for assertions and failure messages.
///
/// # Panics
/// Panics when a node fails to visit its children (it cannot with a read-only visitor).
pub fn dump_tree(root: &Rc<dyn IXamlAstNode>) -> String {
    let mut dumper = Dumper {
        depth: 0,
        out: String::new(),
    };
    if let Err(e) = visit_node(root, &mut dumper) {
        panic!("Unable to walk the tree: {e}");
    }
    dumper.out
}

/// Declares `Tests.MainView : ContentControl` (in the controls assembly) with the event
/// handler `void OnClick(object, RoutedEventArgs)`, for documents with `x:Class`. Calling it
/// again is harmless.
pub fn define_main_view(fw: &TestFramework) {
    if fw.as_type_system().find_type("Tests.MainView").is_some() {
        return;
    }
    let view = fw.controls.define_class("Tests", "MainView");
    view.set_base_type(fw.t("FerroUI.Controls.ContentControl"));
    view.add_constructor(vec![]);
    view.add_method(
        "OnClick",
        fw.t("System.Void"),
        vec![
            fw.t("System.Object"),
            fw.t("FerroUI.Interactivity.RoutedEventArgs"),
        ],
        false,
    );
}
