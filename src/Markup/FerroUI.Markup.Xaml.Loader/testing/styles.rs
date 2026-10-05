//! Fixture additions and helpers for the tests of the style, selector, setter and template
//! transformers.
//!
//! [`create_styles_test_framework`] adds to the fake framework what those transformers look up:
//! the `Selectors` and `StyleQueries` builder methods (declared in the upstream order, so that
//! the "first matching method" lookups are exercised), `StyleQuery`, `ContainerQuery.Query`,
//! `ResourceDictionary.MergedDictionaries`, the `ControlTemplateScope` attribute on
//! `IControlTemplate`, `Template`, `TemplatePartAttribute` with a `TextBox` that declares
//! template parts, and `ListBox`/`ListBoxItem`.

use std::rc::Rc;

use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstValueNode, IXamlAstVisitor, XamlAstCast,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlConstantNode,
    XamlRootObjectNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::parsers::XDocumentXamlParser;
use xamlx::testing::FakeCustomAttribute;
use xamlx::transform::transformers::{
    KnownDirectivesTransformer, MarkupExtensionTransformer, PropertyReferenceResolver,
    TextNodeMerger, TypeReferenceResolver, XArgumentsTransformer, XamlIntrinsicsTransformer,
};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlType, XamlValue};

use crate::compiler_extensions::transformers::{
    CombinatorSelectorType, XamlIlAndQueryNode, XamlIlAttachedPropertyEqualsSelector,
    XamlIlCombinatorSelector, XamlIlHeightQuery, XamlIlNestingSelector, XamlIlNotSelector,
    XamlIlNthChildSelector, XamlIlOrQueryNode, XamlIlOrSelectorNode, XamlIlPropertyEqualsSelector,
    XamlIlQueryNode, XamlIlSelectorNode, XamlIlStringSelector, XamlIlTypeSelector,
    XamlIlWidthQuery, FERRO_XML_NAMESPACE,
};

use super::{create_test_framework, TestFramework};

/// The namespace declarations of a test document: the framework namespace as the default one
/// and the XAML language namespace as `x`.
pub fn xmlns() -> String {
    format!("xmlns='{FERRO_XML_NAMESPACE}' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'")
}

/// [`create_test_framework`] plus the types and members listed in the module documentation.
///
/// # Panics
/// Panics when the fixture cannot be extended (a test set-up error).
pub fn create_styles_test_framework() -> TestFramework {
    let fw = create_test_framework();
    extend_with_styles(&fw);
    fw
}

fn has_method(type_: &Rc<dyn IXamlType>, name: &str) -> bool {
    type_.methods().iter().any(|m| m.name() == name)
}

fn has_type(fw: &TestFramework, full_name: &str) -> bool {
    fw.as_type_system().find_type(full_name).is_some()
}

fn generic(fw: &TestFramework, definition: &str, arguments: &[Rc<dyn IXamlType>]) -> Rc<dyn IXamlType> {
    match fw.t(definition).make_generic_type(arguments) {
        Ok(type_) => type_,
        Err(e) => panic!("Unable to construct {definition}: {e}"),
    }
}

fn array(type_: &Rc<dyn IXamlType>) -> Rc<dyn IXamlType> {
    match type_.make_array_type(1) {
        Ok(array) => array,
        Err(e) => panic!("Unable to construct an array type: {e}"),
    }
}

/// Adds the style related members to an existing fake framework. Members that are already
/// declared are left alone.
pub fn extend_with_styles(fw: &TestFramework) {
    let object = fw.t("System.Object");
    let string = fw.t("System.String");
    let int32 = fw.t("System.Int32");
    let double = fw.t("System.Double");
    let type_ = fw.t("System.Type");
    let selector = fw.t("FerroUI.Styling.Selector");
    let ferro_property = fw.t("FerroUI.FerroProperty");
    let control = fw.t("FerroUI.Controls.Control");

    // Selectors, in the upstream declaration order.
    let selectors = fw.fake_type("FerroUI.Styling.Selectors");
    if !has_method(&selectors.as_type(), "OfType") {
        let s = || selector.clone();
        let func = generic(fw, "System.Func`2", &[s(), s()]);
        selectors.add_method("Child", s(), vec![s()], true);
        selectors.add_method("Class", s(), vec![s(), string.clone()], true);
        selectors.add_method("Descendant", s(), vec![s()], true);
        selectors.add_method("Is", s(), vec![s(), type_.clone()], true);
        {
            let (result, previous) = (s(), s());
            selectors.add_generic_method("Is", &["T"], true, |_| (result, vec![previous]));
        }
        selectors.add_method("Name", s(), vec![s(), string.clone()], true);
        selectors.add_method("Nesting", s(), vec![s()], true);
        selectors.add_method("Not", s(), vec![s(), func], true);
        selectors.add_method("Not", s(), vec![s(), s()], true);
        selectors.add_method("NthChild", s(), vec![s(), int32.clone(), int32.clone()], true);
        selectors.add_method("NthLastChild", s(), vec![s(), int32.clone(), int32.clone()], true);
        selectors.add_method("OfType", s(), vec![s(), type_.clone()], true);
        {
            let (result, previous) = (s(), s());
            selectors.add_generic_method("OfType", &["T"], true, |_| (result, vec![previous]));
        }
        selectors.add_method("Or", s(), vec![array(&selector)], true);
        selectors.add_method(
            "Or",
            s(),
            vec![generic(fw, "System.Collections.Generic.IReadOnlyList`1", &[s()])],
            true,
        );
        {
            let (result, previous, value) = (s(), s(), object.clone());
            let property_definition = fw.t("FerroUI.FerroProperty`1");
            selectors.add_generic_method("PropertyEquals", &["T"], true, |g| {
                let property = match property_definition.make_generic_type(&[g[0].clone()]) {
                    Ok(property) => property,
                    Err(e) => panic!("Unable to construct FerroProperty<T>: {e}"),
                };
                (result, vec![previous, property, value])
            });
        }
        selectors.add_method(
            "PropertyEquals",
            s(),
            vec![s(), ferro_property.clone(), object.clone()],
            true,
        );
        selectors.add_method("Template", s(), vec![s()], true);
    }

    // Container queries.
    if !has_type(fw, "FerroUI.Styling.StyleQuery") {
        fw.base.define_class("FerroUI.Styling", "StyleQuery");
    }
    let style_query = fw.t("FerroUI.Styling.StyleQuery");
    if !has_type(fw, "FerroUI.Styling.StyleQueryComparisonOperator") {
        fw.base.define_enum(
            "FerroUI.Styling",
            "StyleQueryComparisonOperator",
            &[
                ("None", 0),
                ("Equals", 1),
                ("LessThan", 2),
                ("GreaterThan", 3),
                ("LessThanOrEquals", 4),
                ("GreaterThanOrEquals", 5),
            ],
        );
    }
    let comparison_operator = fw.t("FerroUI.Styling.StyleQueryComparisonOperator");
    let container_query = fw.fake_type("FerroUI.Styling.ContainerQuery");
    if !container_query.as_type().properties().iter().any(|p| p.name() == "Query") {
        container_query.add_property("Query", style_query.clone());
    }
    let style_queries = fw.fake_type("FerroUI.Styling.StyleQueries");
    if !has_method(&style_queries.as_type(), "Width") {
        let q = || style_query.clone();
        let list = generic(fw, "System.Collections.Generic.IReadOnlyList`1", &[q()]);
        style_queries.add_method("Width", q(), vec![q(), comparison_operator.clone(), double.clone()], true);
        style_queries.add_method("Height", q(), vec![q(), comparison_operator, double], true);
        style_queries.add_method("Or", q(), vec![array(&style_query)], true);
        style_queries.add_method("Or", q(), vec![list.clone()], true);
        style_queries.add_method("And", q(), vec![array(&style_query)], true);
        style_queries.add_method("And", q(), vec![list], true);
    }

    // ResourceDictionary.MergedDictionaries
    if !has_type(fw, "FerroUI.Controls.IResourceProvider") {
        fw.base.define_interface("FerroUI.Controls", "IResourceProvider");
    }
    let resource_dictionary = fw.fake_type("FerroUI.Controls.ResourceDictionary");
    if !resource_dictionary
        .as_type()
        .properties()
        .iter()
        .any(|p| p.name() == "MergedDictionaries")
    {
        resource_dictionary.add_interface(fw.t("FerroUI.Controls.IResourceProvider"));
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

    // Control template scopes and templates.
    let scope_attribute = fw.t("FerroUI.Metadata.ControlTemplateScopeAttribute");
    let control_template_interface = fw.fake_type("FerroUI.Controls.Templates.IControlTemplate");
    if !control_template_interface
        .as_type()
        .custom_attributes()
        .iter()
        .any(|a| a.type_().equals(&*scope_attribute))
    {
        control_template_interface.add_attribute(FakeCustomAttribute::new(scope_attribute, vec![]));
    }
    let template_of_control = generic(fw, "FerroUI.Controls.ITemplate`1", &[control.clone()]);
    if !has_type(fw, "FerroUI.Markup.Xaml.Templates.Template") {
        let template = fw.markup_xaml.define_class("FerroUI.Markup.Xaml.Templates", "Template");
        template.add_constructor(vec![]);
        template.add_interface(template_of_control);
        template
            .add_property("Content", object.clone())
            .add_attribute(FakeCustomAttribute::new(
                fw.t("FerroUI.Metadata.ContentAttribute"),
                vec![],
            ))
            .add_attribute(FakeCustomAttribute::new(
                fw.t("FerroUI.Metadata.TemplateContentAttribute"),
                vec![],
            ));
    }

    // Template parts.
    if !has_type(fw, "FerroUI.Controls.Metadata.TemplatePartAttribute") {
        fw.base
            .define_class("FerroUI.Controls.Metadata", "TemplatePartAttribute")
            .set_base_type(fw.t("System.Attribute"));
    }
    if !has_type(fw, "FerroUI.Controls.TextBox") {
        let part_attribute = fw.t("FerroUI.Controls.Metadata.TemplatePartAttribute");
        let presenter = fw
            .controls
            .define_class("FerroUI.Controls.Presenters", "TextPresenter");
        presenter.set_base_type(control.clone());
        presenter.add_constructor(vec![]);
        let scroll_viewer = fw.controls.define_class("FerroUI.Controls", "ScrollViewer");
        scroll_viewer.set_base_type(fw.t("FerroUI.Controls.ContentControl"));
        scroll_viewer.add_constructor(vec![]);
        let text_box = fw.controls.define_class("FerroUI.Controls", "TextBox");
        text_box.set_base_type(fw.t("FerroUI.Controls.Primitives.TemplatedControl"));
        text_box.add_constructor(vec![]);
        // [TemplatePart("PART_TextPresenter", typeof(TextPresenter), IsRequired = true)]
        text_box.add_attribute(FakeCustomAttribute::with_properties(
            part_attribute.clone(),
            vec![
                XamlValue::String("PART_TextPresenter".to_string()),
                XamlValue::Type(presenter.as_type()),
            ],
            vec![("IsRequired", XamlValue::Boolean(true))],
        ));
        // [TemplatePart("PART_ScrollViewer", typeof(ScrollViewer))]
        text_box.add_attribute(FakeCustomAttribute::new(
            part_attribute.clone(),
            vec![
                XamlValue::String("PART_ScrollViewer".to_string()),
                XamlValue::Type(scroll_viewer.as_type()),
            ],
        ));
        // [TemplatePart(Name = "PART_ClearButton", Type = typeof(Button))]
        text_box.add_attribute(FakeCustomAttribute::with_properties(
            part_attribute,
            vec![],
            vec![
                ("Name", XamlValue::String("PART_ClearButton".to_string())),
                ("Type", XamlValue::Type(fw.t("FerroUI.Controls.Button"))),
            ],
        ));
    }

    // Items controls with a known container type.
    if !has_type(fw, "FerroUI.Controls.ListBox") {
        let list_box = fw.controls.define_class("FerroUI.Controls", "ListBox");
        list_box.set_base_type(fw.t("FerroUI.Controls.ItemsControl"));
        list_box.add_constructor(vec![]);
        let list_box_item = fw.controls.define_class("FerroUI.Controls", "ListBoxItem");
        list_box_item.set_base_type(fw.t("FerroUI.Controls.ContentControl"));
        list_box_item.add_constructor(vec![]);
    }
}

/// The `xamlx` transformers that run before the framework's style transformers: everything up
/// to and including the property reference resolver.
pub fn front_transformers() -> Vec<Box<dyn IXamlAstTransformer>> {
    vec![
        Box::new(KnownDirectivesTransformer),
        Box::new(XamlIntrinsicsTransformer),
        Box::new(XArgumentsTransformer),
        Box::new(TypeReferenceResolver),
        Box::new(MarkupExtensionTransformer),
        Box::new(TextNodeMerger),
        Box::new(PropertyReferenceResolver),
    ]
}

/// Parses `xaml` and runs [`front_transformers`] followed by `transformers` over it, the way
/// the compiler does (one full pass per transformer). Returns the transformed root.
pub fn run_transformers(
    fw: &TestFramework,
    xaml: &str,
    transformers: Vec<Box<dyn IXamlAstTransformer>>,
) -> XamlResult<Rc<dyn IXamlAstNode>> {
    let doc = XDocumentXamlParser::parse(xaml, None)?;
    let ctx = AstTransformationContext::new(fw.configuration.clone(), Some(&doc));
    let mut root = doc.root()?;
    let root_object = root
        .cast::<XamlAstObjectNode>()
        .ok_or_else(|| XamlError::invalid_cast("The root is not an object node"))?;
    ctx.set_root_object(XamlRootObjectNode::new(&root_object));
    for transformer in front_transformers().into_iter().chain(transformers) {
        ctx.visit_children(&*ctx.root_object()?, &*transformer)?;
        root = ctx.visit(&root, &*transformer)?;
    }
    Ok(root)
}

struct Collector<K: ?Sized + XamlAstCast> {
    found: Vec<Rc<K>>,
}

impl<K: ?Sized + XamlAstCast> IXamlAstVisitor for Collector<K> {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(found) = node.cast::<K>() {
            self.found.push(found);
        }
        Ok(node)
    }
    fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
    fn pop(&mut self) {}
}

/// Every node of kind `K` in the tree below (and including) `root`, in document order.
pub fn find_nodes<K: ?Sized + XamlAstCast>(root: &Rc<dyn IXamlAstNode>) -> Vec<Rc<K>> {
    let mut collector = Collector::<K> { found: Vec::new() };
    if let Err(e) = visit_node(root, &mut collector) {
        panic!("The collecting visitor failed: {e}");
    }
    collector.found
}

fn describe_value(value: &Rc<dyn IXamlAstValueNode>) -> String {
    if let Some(constant) = value.cast::<XamlConstantNode>() {
        return format!("{:?}", constant.constant);
    }
    if let Some(text) = value.cast::<XamlAstTextNode>() {
        return format!("{:?}", text.text());
    }
    value.type_name().to_string()
}

/// A compact description of a selector chain, e.g. `OfType(Button).Class(foo)`; the initial
/// node is the empty string.
pub fn describe_selector(selector: &Rc<dyn XamlIlSelectorNode>) -> String {
    let any = selector.as_any();
    let step = if let Some(n) = any.downcast_ref::<XamlIlTypeSelector>() {
        format!(
            "{}({})",
            if n.concrete { "OfType" } else { "Is" },
            n.target_type.name()
        )
    } else if let Some(n) = any.downcast_ref::<XamlIlStringSelector>() {
        format!("{}({})", n.selector_type.name(), n.string())
    } else if let Some(n) = any.downcast_ref::<XamlIlCombinatorSelector>() {
        match n.selector_type {
            CombinatorSelectorType::Child => "Child()".to_string(),
            CombinatorSelectorType::Descendant => "Descendant()".to_string(),
            CombinatorSelectorType::Template => "Template()".to_string(),
        }
    } else if let Some(n) = any.downcast_ref::<XamlIlNotSelector>() {
        format!("Not({})", describe_selector(&n.argument))
    } else if let Some(n) = any.downcast_ref::<XamlIlNthChildSelector>() {
        format!("{}({},{})", n.selector_type.name(), n.step, n.offset)
    } else if let Some(n) = any.downcast_ref::<XamlIlPropertyEqualsSelector>() {
        format!(
            "PropertyEquals({}={})",
            n.property().name(),
            describe_value(&n.value())
        )
    } else if let Some(n) = any.downcast_ref::<XamlIlAttachedPropertyEqualsSelector>() {
        format!(
            "PropertyEquals({}.{}={})",
            n.property_filed().declaring_type().name(),
            n.property_filed().name(),
            describe_value(&n.value())
        )
    } else if let Some(n) = any.downcast_ref::<XamlIlOrSelectorNode>() {
        let alternatives: Vec<String> = n.selectors().iter().map(describe_selector).collect();
        format!("Or[{}]", alternatives.join(", "))
    } else if any.is::<XamlIlNestingSelector>() {
        "Nesting()".to_string()
    } else {
        String::new()
    };
    match selector.previous().map(|p| describe_selector(&p)) {
        Some(previous) if !previous.is_empty() => format!("{previous}.{step}"),
        _ => step,
    }
}

/// A compact description of a query, e.g. `Width(3,400)`; the initial node is the empty string.
pub fn describe_query(query: &Rc<dyn XamlIlQueryNode>) -> String {
    let any = query.as_any();
    let step = if let Some(n) = any.downcast_ref::<XamlIlWidthQuery>() {
        format!("Width({},{})", n.operator_value(), n.value)
    } else if let Some(n) = any.downcast_ref::<XamlIlHeightQuery>() {
        format!("Height({},{})", n.operator_value(), n.value)
    } else if let Some(n) = any.downcast_ref::<XamlIlOrQueryNode>() {
        let members: Vec<String> = n.queries().iter().map(describe_query).collect();
        format!("Or[{}]", members.join(", "))
    } else if let Some(n) = any.downcast_ref::<XamlIlAndQueryNode>() {
        let members: Vec<String> = n.queries().iter().map(describe_query).collect();
        format!("And[{}]", members.join(", "))
    } else {
        String::new()
    };
    match query.previous().map(|p| describe_query(&p)) {
        Some(previous) if !previous.is_empty() => format!("{previous}.{step}"),
        _ => step,
    }
}
