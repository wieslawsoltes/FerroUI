//! Tests of the compiled binding transformers other than the path resolution itself (which is
//! covered by `xaml_il_binding_path_helper_tests.rs`): the `x:CompileBindings` scope, the
//! synthetic binding members, `x:DataType`, the data context type inference, the path parser,
//! the start type of the path transformer and the metadata remover.

use std::rc::Rc;

use ferroui_base::data::core::parsers::Node;
use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstClrProperty, XamlAstConstructableObjectNode,
    XamlAstExtensions, XamlAstNodeExtensions, XamlAstXmlDirective, XamlPropertyAssignmentNode,
};

use super::*;
use crate::compiler_extensions::XamlIlBindingPathNode;
use crate::testing::bindings::*;
use crate::testing::TestFramework;

const DC: &str = "x:DataType='local:TestDataContext'";
const LOCAL: &str = "FerroUI.Markup.Xaml.UnitTests.MarkupExtensions";

fn strip_line(title: &str) -> String {
    match title.find(" Line ") {
        Some(index) => title[..index].to_string(),
        None => title.to_string(),
    }
}

/// The single error of the document: its code and its title without the line information.
fn single_error(attributes: &str, content: &str) -> (String, String) {
    let (_, _, errors) = resolve(attributes, content);
    assert_eq!(errors.len(), 1, "{errors:?}");
    (errors[0].0.clone(), strip_line(&errors[0].1))
}

fn type_name_of(node: &Rc<dyn IXamlAstValueNode>) -> String {
    value_type_name(node)
}

/// The types of the compiled binding / reflection binding extension objects of the document.
fn binding_extension_types(root: &Rc<dyn IXamlAstNode>) -> Vec<String> {
    find_binding_nodes::<XamlAstConstructableObjectNode>(root)
        .iter()
        .filter_map(|n| n.type_().get_clr_type().ok())
        .map(|t| t.name())
        .filter(|name| name.ends_with("BindingExtension"))
        .collect()
}

// FerroBindingExtensionTransformer

#[test]
fn binding_is_compiled_when_requested() {
    let binding = "<TextBlock Text='{Binding StringProperty}'/>";
    for (attributes, default, expected) in [
        ("x:CompileBindings='True'", false, "CompiledBindingExtension"),
        ("x:CompileBindings='true'", false, "CompiledBindingExtension"),
        ("x:CompileBindings='False'", true, "ReflectionBindingExtension"),
        ("", false, "ReflectionBindingExtension"),
        ("", true, "CompiledBindingExtension"),
    ] {
        let fw = create_bindings_test_framework();
        let root = transform_root_with(
            &fw,
            &window(&format!("{DC} {attributes}"), binding),
            BindingsPipelineOptions {
                compile_bindings_by_default: default,
                ..BindingsPipelineOptions::default()
            },
        );
        assert_eq!(reported_errors(&fw), vec![], "{attributes}");
        assert_eq!(binding_extension_types(&root), [expected], "{attributes} {default}");
        // The directive is consumed.
        assert!(find_binding_nodes::<XamlAstXmlDirective>(&root)
            .iter()
            .all(|d| *d.name.borrow() != "CompileBindings"));
    }
}

#[test]
fn the_nearest_compile_bindings_scope_wins() {
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(
            &format!("{DC} x:CompileBindings='True'"),
            "<StackPanel>
               <TextBlock Text='{Binding StringProperty}'/>
               <Border x:CompileBindings='False'><TextBlock Text='{Binding StringProperty}'/></Border>
               <TextBlock><TextBlock.Text><Binding Path='StringProperty'/></TextBlock.Text></TextBlock>
             </StackPanel>",
        ),
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(
        binding_extension_types(&root),
        [
            "CompiledBindingExtension",
            "ReflectionBindingExtension",
            "CompiledBindingExtension"
        ]
    );
    // Without the remover the scope nodes are still in the tree.
    let scopes = find_binding_nodes::<FerroXamlIlCompileBindingsNode>(&root);
    assert_eq!(
        scopes.iter().map(|s| s.compile_bindings).collect::<Vec<_>>(),
        [true, false]
    );
    assert_eq!(type_name_of(&scopes[0].value()), "FerroUI.Controls.Window");
    assert_eq!(scopes[0].type_name(), "FerroXamlIlCompileBindingsNode");
}

#[test]
fn an_explicit_compiled_or_reflection_binding_ignores_the_scope() {
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(
            &format!("{DC} x:CompileBindings='False'"),
            "<StackPanel><TextBlock Text='{CompiledBinding StringProperty}'/><TextBlock Text='{ReflectionBinding StringProperty}'/></StackPanel>",
        ),
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(
        binding_extension_types(&root),
        ["CompiledBindingExtension", "ReflectionBindingExtension"]
    );
}

#[test]
fn throws_on_invalid_compile_bindings_directive() {
    assert_eq!(
        single_error(
            &format!("{DC} x:CompileBindings='notabool'"),
            "<TextBlock Text='{Binding StringProperty}'/>"
        ),
        (
            "FRN2100".to_string(),
            "The value of x:CompileBindings must be a literal boolean value.".to_string()
        )
    );
}

#[test]
fn throws_on_invalid_binding_path_when_compiled_through_the_directive() {
    let (code, title) = single_error(
        &format!("{DC} x:CompileBindings='True'"),
        "<TextBlock Text='{Binding InvalidPath}'/>",
    );
    assert_eq!(code, "FRN2000");
    assert!(title.starts_with("Unable to resolve property or method of name 'InvalidPath'"));
}

#[test]
fn bindings_exceptions_are_tagged_transform_exceptions() {
    let line_info = xamlx::ast::XamlLineInfo::new(2, 5);
    let e = XamlBindingsTransformException::new("bad", &line_info, None);
    assert!(XamlBindingsTransformException::is(&e));
    assert!(!XamlDataContextException::is(&e));
    assert_eq!(e.type_name(), "XamlBindingsTransformException");
    assert_eq!((e.line_number(), e.line_position()), (Some(2), Some(5)));

    let inner = xamlx::exceptions::XamlError::invalid_operation("inner");
    let e = XamlDataContextException::new("bad", &line_info, Some(inner));
    assert!(XamlDataContextException::is(&e));
    assert_eq!(e.type_name(), "XamlDataContextException");
    assert_eq!(e.inner_exception().map(|i| i.message()), Some("inner".to_string()));
}

// FerroXamlIlTransformSyntheticCompiledBindingMembers

#[test]
fn element_name_and_relative_source_become_synthetic_members_of_compiled_bindings_only() {
    // On a compiled binding they are consumed by the path parser.
    let fw = create_bindings_test_framework();
    let root = transform_root_with(
        &fw,
        &window(
            DC,
            "<StackPanel><TextBlock x:Name='text'/><TextBlock Text='{CompiledBinding Text, ElementName=text}'/></StackPanel>",
        ),
        BindingsPipelineOptions {
            stop_before_data_context: true,
            ..BindingsPipelineOptions::default()
        },
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert!(find_binding_nodes::<FerroSyntheticCompiledBindingProperty>(&root).is_empty());
    let binding = find_binding_nodes::<XamlAstConstructableObjectNode>(&root)
        .into_iter()
        .find(|n| n.type_().get_clr_type().is_ok_and(|t| t.name() == "CompiledBindingExtension"))
        .expect("binding");
    assert!(binding.children.borrow().is_empty());

    // On any other type the name is an ordinary (here: unknown) property.
    let (_, _, errors) = resolve(DC, "<TextBlock Text='{ReflectionBinding Text, ElementName=text}'/>");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].1.starts_with(
        "Unable to resolve suitable regular or attached property ElementName on type"
    ));
}

#[test]
fn synthetic_property_node() {
    let line_info = xamlx::ast::XamlLineInfo::new(3, 4);
    let node = FerroSyntheticCompiledBindingProperty::new(
        &line_info,
        SyntheticCompiledBindingPropertyName::RelativeSource,
    );
    assert_eq!(node.name, SyntheticCompiledBindingPropertyName::RelativeSource);
    assert_eq!(node.type_name(), "FerroSyntheticCompiledBindingProperty");
    let as_node: Rc<dyn IXamlAstNode> = node;
    assert!(as_node.is::<dyn xamlx::ast::IXamlAstPropertyReference>());
    assert!(!as_node.is::<XamlAstClrProperty>());
    assert_eq!((as_node.line(), as_node.position()), (3, 4));
}

// XDataTypeTransformer

fn data_templates(templates: &str) -> String {
    format!("<Window.DataTemplates>{templates}</Window.DataTemplates>")
}

/// The assignments of the first object of the given type name, as `Property=ValueNodeType`.
fn assignments_of(root: &Rc<dyn IXamlAstNode>, type_name: &str) -> Vec<String> {
    let object = find_binding_nodes::<XamlAstConstructableObjectNode>(root)
        .into_iter()
        .find(|n| n.type_().get_clr_type().is_ok_and(|t| t.name() == type_name))
        .expect("object");
    let children = object.children.borrow().clone();
    children
        .iter()
        .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
        .map(|a| {
            let value = a.values.borrow().last().map(|v| v.type_name()).unwrap_or("-");
            format!("{}={value}", a.property.name())
        })
        .collect()
}

#[test]
fn x_data_type_becomes_the_data_type_property_of_a_template() {
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(
            "",
            &data_templates(
                "<DataTemplate x:DataType='local:TestDataContext'><TextBlock Text='{CompiledBinding StringProperty}'/></DataTemplate>",
            ),
        ),
    );
    assert_eq!(reported_errors(&fw), vec![]);
    // The directive was converted into an assignment of `DataTemplate.DataType`...
    assert!(assignments_of(&root, "DataTemplate").contains(&"DataType=XamlTypeExtensionNode".to_string()));
    assert!(find_binding_nodes::<XamlAstXmlDirective>(&root).is_empty());
    // ... from which the data context type is inferred.
    assert_eq!(
        resolved_paths(&root),
        ["Property(TestDataContext.StringProperty:System.String)"]
    );
}

#[test]
fn x_data_type_is_left_alone_when_the_data_type_property_is_set() {
    // IgnoresDataTemplateTypeFromDataTypePropertyIfXDataTypeDefined
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(
            "",
            &format!(
                "{}<ContentControl x:DataType='local:TestDataContext' Content='{{CompiledBinding}}'/>",
                data_templates(
                    "<DataTemplate DataType='local:TestDataContextBaseClass' x:DataType='local:TestDataContext'><TextBlock Text='{CompiledBinding StringProperty}'/></DataTemplate>"
                )
            ),
        ),
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(
        assignments_of(&root, "DataTemplate")
            .iter()
            .filter(|a| a.starts_with("DataType="))
            .count(),
        1
    );
    // The directive wins over the property for the data context type.
    assert_eq!(
        resolved_paths(&root),
        ["Property(TestDataContext.StringProperty:System.String)"]
    );
}

#[test]
fn x_data_type_stays_a_directive_on_types_without_a_data_type_property() {
    let fw = create_bindings_test_framework();
    let root = transform_root_with(
        &fw,
        &window(DC, "<TextBlock/>"),
        BindingsPipelineOptions {
            stop_before_data_context: true,
            ..BindingsPipelineOptions::default()
        },
    );
    assert_eq!(reported_errors(&fw), vec![]);
    let directives = find_binding_nodes::<XamlAstXmlDirective>(&root);
    assert_eq!(directives.len(), 1);
    assert_eq!(*directives[0].name.borrow(), "DataType");
}

// FerroXamlIlDataContextTypeTransformer

/// The data context scopes of the document, in document order: `ScopedType:DataContextType`.
fn data_context_scopes(root: &Rc<dyn IXamlAstNode>) -> Vec<String> {
    find_binding_nodes::<FerroXamlIlDataContextTypeMetadataNode>(root)
        .iter()
        .map(|scope| {
            let scoped = scope
                .value()
                .type_()
                .get_clr_type()
                .map(|t| t.name())
                .unwrap_or_default();
            let data_type = if scope.is_uninferrable() {
                "?".to_string()
            } else {
                scope.data_context_type.full_name()
            };
            format!("{scoped}:{data_type}")
        })
        .collect()
}

fn scopes(attributes: &str, content: &str) -> (TestFramework, Vec<String>) {
    let fw = create_bindings_test_framework();
    let root = transform_root(&fw, &window(attributes, content));
    let scopes = data_context_scopes(&root);
    (fw, scopes)
}

#[test]
fn x_data_type_directive_creates_a_data_context_scope() {
    let (fw, found) = scopes(DC, "<StackPanel><Border x:DataType='x:String'/><TextBlock/></StackPanel>");
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(
        found,
        [format!("Window:{LOCAL}.TestDataContext"), "Border:System.String".to_string()]
    );

    // `x:Type` syntax; the directive is removed from the object.
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window("x:DataType='{x:Type local:TestDataContext}'", "<TextBlock/>"),
    );
    assert_eq!(data_context_scopes(&root), [format!("Window:{LOCAL}.TestDataContext")]);
    assert!(find_binding_nodes::<XamlAstXmlDirective>(&root).is_empty());
    let scope = find_binding_nodes::<FerroXamlIlDataContextTypeMetadataNode>(&root).remove(0);
    assert_eq!(scope.type_name(), "FerroXamlIlDataContextTypeMetadataNode");
    assert_eq!(
        scope.to_node_string(),
        format!("DataType = {LOCAL}.TestDataContext")
    );
    assert_eq!(type_name_of(&(scope.clone() as Rc<dyn IXamlAstValueNode>)), "FerroUI.Controls.Window");
}

#[test]
fn x_data_type_must_be_a_type_name() {
    assert_eq!(
        single_error(DC, "<Panel x:DataType='{x:Null}'/>"),
        (
            "FRN2101".to_string(),
            "x:DataType should be set to a type name.".to_string()
        )
    );
    let (code, title) = single_error("x:DataType='bogus'", "<TextBlock/>");
    assert_eq!(code, "FRN2000");
    assert!(title.starts_with("Unable to resolve type bogus from namespace"));
}

#[test]
fn infers_data_context_type_from_data_context_binding() {
    // InfersCompiledBindingDataContextFromDataContextBinding
    let (fw, found) = scopes(
        DC,
        "<TextBlock DataContext='{CompiledBinding StringProperty}' Text='{CompiledBinding}'/>",
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(
        found,
        [format!("Window:{LOCAL}.TestDataContext"), "TextBlock:System.String".to_string()]
    );

    // Without a data type in scope the data context binding cannot be resolved. As upstream,
    // the binding's own `DataType` does not help here: the data context inference only
    // looks at a `DataType` that is still text, and by now the value was converted to a type.
    for binding in [
        "{CompiledBinding StringProperty}",
        "{CompiledBinding StringProperty, DataType=local:TestDataContext}",
    ] {
        assert_eq!(
            single_error("", &format!("<TextBlock DataContext='{binding}'/>")),
            (
                "FRN2101".to_string(),
                "Cannot parse a compiled binding without an explicit x:DataType directive to give a starting data type for bindings.".to_string()
            ),
            "{binding}"
        );
    }
}

#[test]
fn a_reflection_data_context_binding_makes_the_data_context_uninferrable() {
    // ThrowsOnUninferrableDataTypeFromNonCompiledDataContextBindingWithCompiledBindingPath
    let (fw, found) = scopes(
        DC,
        "<ContentControl DataContext='{Binding}'><TextBlock Text='{CompiledBinding StringProperty}'/></ContentControl>",
    );
    assert_eq!(
        found,
        [format!("Window:{LOCAL}.TestDataContext"), "ContentControl:?".to_string()]
    );
    let errors = reported_errors(&fw);
    assert_eq!(errors.len(), 1);
    assert!(errors[0]
        .1
        .starts_with("Unable to resolve property or method of name 'StringProperty' on type"));
}

#[test]
fn uninferrable_node_is_a_data_context_metadata_node() {
    let fw = create_bindings_test_framework();
    let root = transform_root(&fw, &window(DC, &data_templates("<DataTemplate><TextBlock/></DataTemplate>")));
    let scopes = find_binding_nodes::<FerroXamlIlDataContextTypeMetadataNode>(&root);
    assert_eq!(scopes.len(), 2);
    let template_scope = scopes[1].clone();
    assert!(template_scope.is_uninferrable());
    assert_eq!(
        template_scope.type_name(),
        "FerroXamlIlUninferrableDataContextMetadataNode"
    );
    assert_eq!(template_scope.to_node_string(), "DataType = Unknown");
    assert!(xamlx::type_system::XamlPseudoType::is_unknown(
        &*template_scope.data_context_type
    ));
    let as_node: Rc<dyn IXamlAstNode> = template_scope;
    assert!(FerroXamlIlUninferrableDataContextMetadataNode::is(&as_node));
    let window_scope: Rc<dyn IXamlAstNode> = scopes[0].clone();
    assert!(!FerroXamlIlUninferrableDataContextMetadataNode::is(&window_scope));
}

#[test]
fn infers_data_template_type_from_data_type_property() {
    // InfersDataTemplateTypeFromDataTypeProperty
    let (fw, paths, errors) = resolve(
        DC,
        &format!(
            "{}<ContentControl Content='{{CompiledBinding StringProperty}}'/>",
            data_templates(
                "<DataTemplate DataType='{x:Type x:String}'><TextBlock Text='{CompiledBinding Length}'/></DataTemplate>"
            )
        ),
    );
    let _ = fw;
    assert_eq!(errors, vec![]);
    assert_eq!(
        paths,
        [
            "Property(String.Length:System.Int32)",
            "Property(TestDataContext.StringProperty:System.String)"
        ]
    );
}

#[test]
fn infers_custom_data_template_based_on_attribute() {
    // InfersCustomDataTemplateBasedOnAttribute, ...FromBaseClass
    for template in ["CustomDataTemplate", "CustomDataTemplateInherit"] {
        let (fw, found) = scopes(
            "",
            &data_templates(&format!(
                "<local:{template} FancyDataType='local:TestDataContext'><TextBlock Text='{{CompiledBinding StringProperty}}'/></local:{template}>"
            )),
        );
        assert_eq!(reported_errors(&fw), vec![]);
        assert_eq!(found, [format!("{template}:{LOCAL}.TestDataContext")]);
    }
}

#[test]
fn throws_on_uninferrable_loose_data_template_without_data_type() {
    // ThrowsOnUninferrableLooseDataTemplateNoDataTypeWithCompiledBindingPath
    let (code, title) = single_error(
        DC,
        &data_templates("<DataTemplate><TextBlock Text='{CompiledBinding StringProperty}'/></DataTemplate>"),
    );
    assert_eq!(code, "FRN2000");
    assert!(title.starts_with("Unable to resolve property or method of name 'StringProperty' on type"));
    assert!(!title.contains("TestDataContext"));
}

fn items_control(attributes: &str, template_content: &str) -> String {
    format!(
        "<ItemsControl {attributes}><ItemsControl.ItemTemplate><DataTemplate>{template_content}</DataTemplate></ItemsControl.ItemTemplate></ItemsControl>"
    )
}

#[test]
fn infers_data_template_type_from_parent_collection_items_type() {
    // InfersDataTemplateTypeFromParentCollectionItemsType, ResolvesPathPassedByPropertyWithInnerItemTemplate
    for items_source in ["{CompiledBinding ListProperty}", "{CompiledBinding Path=ListProperty}"] {
        let fw = create_bindings_test_framework();
        let root = transform_root(
            &fw,
            &window(
                DC,
                &items_control(
                    &format!("ItemsSource='{items_source}'"),
                    "<TextBlock Text='{CompiledBinding Length}'/>",
                ),
            ),
        );
        assert_eq!(reported_errors(&fw), vec![]);
        assert_eq!(
            data_context_scopes(&root),
            [
                format!("Window:{LOCAL}.TestDataContext"),
                "DataTemplate:System.String".to_string()
            ]
        );
        assert_eq!(
            resolved_paths(&root),
            [
                "Property(TestDataContext.ListProperty:System.Collections.Generic.List`1[System.String])",
                "Property(String.Length:System.Int32)"
            ]
        );
    }
}

#[test]
fn throws_on_uninferrable_data_template_in_items_control_without_items_binding() {
    // ThrowsOnUninferrableDataTemplateInItemsControlWithoutItemsBinding
    let (fw, found) = scopes(
        DC,
        &items_control("", "<TextBlock Text='{CompiledBinding Property}'/>"),
    );
    assert_eq!(
        found,
        [format!("Window:{LOCAL}.TestDataContext"), "DataTemplate:?".to_string()]
    );
    let errors = reported_errors(&fw);
    assert_eq!(errors.len(), 1);
    assert!(errors[0]
        .1
        .starts_with("Unable to resolve property or method of name 'Property' on type"));

    // A reflection binding of the items does not help either.
    let (_, found) = scopes(
        DC,
        &items_control("ItemsSource='{ReflectionBinding ListProperty}'", "<TextBlock/>"),
    );
    assert_eq!(found[1], "DataTemplate:?");
}

#[test]
fn infers_data_type_from_parent_data_grid_items_type() {
    // InfersDataTypeFromParentDataGridItemsTypeInCaseOfControlInheritance
    let (fw, paths, errors) = resolve(
        "x:DataType='local:TestItemsCollectionDataContext'",
        "<local:DataGridLikeControlInheritor Items='{CompiledBinding Items}'>
           <local:DataGridLikeControlInheritor.Columns>
             <local:DataGridLikeColumn Binding='{CompiledBinding StringProperty}'/>
           </local:DataGridLikeControlInheritor.Columns>
         </local:DataGridLikeControlInheritor>",
    );
    let _ = fw;
    assert_eq!(errors, vec![]);
    assert_eq!(
        paths,
        [
            format!("Property(TestItemsCollectionDataContext.Items:System.Collections.ObjectModel.ObservableCollection`1[{LOCAL}.TestData])"),
            "Property(TestData.StringProperty:System.String)".to_string()
        ]
    );

    // InfersDataTemplateTypeFromParentDataGridItemsType
    let (fw, found) = scopes(
        DC,
        "<local:DataGridLikeControl Items='{CompiledBinding ListProperty}'>
           <local:DataGridLikeControl.Columns>
             <local:DataGridLikeColumn Binding='{CompiledBinding Length}'>
               <local:DataGridLikeColumn.Template>
                 <DataTemplate><TextBlock Text='{CompiledBinding Length}'/></DataTemplate>
               </local:DataGridLikeColumn.Template>
             </local:DataGridLikeColumn>
           </local:DataGridLikeControl.Columns>
         </local:DataGridLikeControl>",
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(
        found,
        [
            format!("Window:{LOCAL}.TestDataContext"),
            "CompiledBindingExtension:System.String".to_string(),
            "DataTemplate:System.String".to_string()
        ]
    );
}

#[test]
fn explicit_data_type_still_works_on_data_grid_like_controls() {
    // ExplicitDataTypeStillWorksOnDataGridLikeControls
    let (fw, paths, errors) = resolve(
        DC,
        "<local:DataGridLikeControl>
           <local:DataGridLikeControl.Columns>
             <local:DataGridLikeColumn Binding='{CompiledBinding Length}' x:DataType='x:String'>
               <local:DataGridLikeColumn.Template>
                 <DataTemplate x:DataType='x:String'><TextBlock Text='{CompiledBinding Length}'/></DataTemplate>
               </local:DataGridLikeColumn.Template>
             </local:DataGridLikeColumn>
           </local:DataGridLikeControl.Columns>
         </local:DataGridLikeControl>",
    );
    let _ = fw;
    assert_eq!(errors, vec![]);
    assert_eq!(
        paths,
        ["Property(String.Length:System.Int32)", "Property(String.Length:System.Int32)"]
    );
}

#[test]
fn resolves_data_type_for_assign_binding() {
    // ResolvesDataTypeForAssignBinding, ..._FromBindingProperty
    for document in [
        format!("<local:AssignBindingControl {} x:DataType='local:TestDataContext' X='{{CompiledBinding StringProperty}}'/>", xmlns()),
        format!("<local:AssignBindingControl {} X='{{CompiledBinding StringProperty, DataType=local:TestDataContext}}'/>", xmlns()),
    ] {
        let fw = create_bindings_test_framework();
        let root = transform_root(&fw, &document);
        assert_eq!(reported_errors(&fw), vec![]);
        assert_eq!(
            resolved_paths(&root),
            ["Property(TestDataContext.StringProperty:System.String)"]
        );
    }
}

// FerroXamlIlBindingPathParser

fn describe_parsed(node: &BindingExpressionNode) -> String {
    match node {
        BindingExpressionNode::Grammar(Node::PropertyName {
            accepts_null,
            property_name,
        }) => format!("{property_name}{}", if *accepts_null { "?" } else { "" }),
        BindingExpressionNode::Grammar(Node::Name { name }) => format!("#{name}"),
        BindingExpressionNode::Grammar(Node::Not) => "!".to_string(),
        BindingExpressionNode::Grammar(Node::SelfNode) => "$self".to_string(),
        BindingExpressionNode::Grammar(Node::EmptyExpression) => ".".to_string(),
        BindingExpressionNode::Grammar(other) => format!("{other:?}"),
        BindingExpressionNode::VisualAncestor(n) => {
            format!("VisualAncestor({};{})", n.type_.name(), n.level)
        }
        BindingExpressionNode::LogicalAncestor(n) => {
            format!("LogicalAncestor({};{})", n.type_.name(), n.level)
        }
        BindingExpressionNode::TemplatedParent(n) => format!("TemplatedParent({})", n.type_.name()),
    }
}

/// The parsed paths of the document (before they are resolved), each as its nodes joined
/// with a space.
fn parsed_paths(attributes: &str, content: &str) -> (TestFramework, Rc<dyn IXamlAstNode>, Vec<String>) {
    let fw = create_bindings_test_framework();
    let root = transform_root_with(
        &fw,
        &window(attributes, content),
        BindingsPipelineOptions {
            stop_before_data_context: true,
            ..BindingsPipelineOptions::default()
        },
    );
    let parsed = find_binding_nodes::<ParsedBindingPathNode>(&root)
        .iter()
        .map(|node| {
            node.path()
                .iter()
                .map(describe_parsed)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    (fw, root, parsed)
}

fn parsed(binding: &str) -> Vec<String> {
    let (fw, _, parsed) = parsed_paths(DC, &format!("<StackPanel><TextBlock Text='{binding}'/></StackPanel>"));
    assert_eq!(reported_errors(&fw), vec![], "{binding}");
    parsed
}

#[test]
fn parses_the_path_argument_and_the_path_property() {
    assert_eq!(parsed("{CompiledBinding StringProperty}"), ["StringProperty"]);
    assert_eq!(parsed("{CompiledBinding Path=StringProperty}"), ["StringProperty"]);
    assert_eq!(parsed("{CompiledBinding !A.B?.C}"), ["! A B C?"]);

    // The parsed node replaces the text and is typed as a compiled binding path.
    let (fw, root, _) = parsed_paths(DC, "<TextBlock Text='{CompiledBinding StringProperty}'/>");
    assert_eq!(reported_errors(&fw), vec![]);
    let node = find_binding_nodes::<ParsedBindingPathNode>(&root).remove(0);
    assert_eq!(node.type_name(), "ParsedBindingPathNode");
    assert_eq!(
        type_name_of(&(node as Rc<dyn IXamlAstValueNode>)),
        "FerroUI.Data.CompiledBindingPath"
    );
    let binding = find_binding_nodes::<XamlAstConstructableObjectNode>(&root)
        .into_iter()
        .find(|n| n.type_().get_clr_type().is_ok_and(|t| t.name() == "CompiledBindingExtension"))
        .expect("binding");
    // The constructor taking the path was selected.
    assert_eq!(binding.arguments.borrow().len(), 1);
    assert_eq!(binding.constructor.parameters().len(), 1);
}

#[test]
fn an_empty_path_is_removed() {
    for binding in ["{CompiledBinding .}", "{CompiledBinding Path=.}", "{CompiledBinding}"] {
        let (fw, root, parsed) = parsed_paths(DC, &format!("<TextBlock Text='{binding}'/>"));
        assert_eq!(reported_errors(&fw), vec![], "{binding}");
        assert_eq!(parsed, Vec::<String>::new(), "{binding}");
        let binding_node = find_binding_nodes::<XamlAstConstructableObjectNode>(&root)
            .into_iter()
            .find(|n| n.type_().get_clr_type().is_ok_and(|t| t.name() == "CompiledBindingExtension"))
            .expect("binding");
        // The parameterless constructor is used and no `Path` assignment is left.
        assert!(binding_node.arguments.borrow().is_empty(), "{binding}");
        assert!(
            find_binding_nodes::<XamlPropertyAssignmentNode>(&root)
                .iter()
                .all(|a| a.property.name() != "Path"),
            "{binding}"
        );
    }
}

#[test]
fn long_form_sources_are_inserted_after_the_transform_nodes() {
    assert_eq!(parsed("{CompiledBinding Text, ElementName=text}"), ["#text Text"]);
    assert_eq!(parsed("{CompiledBinding !Text, ElementName=text}"), ["! #text Text"]);
    assert_eq!(parsed("{CompiledBinding ElementName=text, Path=Text}"), ["#text Text"]);
    assert_eq!(parsed("{CompiledBinding ElementName=text}"), ["#text"]);
    assert_eq!(parsed("{CompiledBinding RelativeSource={RelativeSource Self}}"), ["$self"]);
    assert_eq!(
        parsed("{CompiledBinding Name, RelativeSource={RelativeSource Mode=Self}}"),
        ["$self Name"]
    );
    // Without arguments and without a mode the mode is FindAncestor.
    assert_eq!(
        parsed("{CompiledBinding Title, RelativeSource={RelativeSource AncestorType=Window}}"),
        ["VisualAncestor(Window;0) Title"]
    );
    assert_eq!(
        parsed("{CompiledBinding Title, RelativeSource={RelativeSource AncestorType={x:Type Window}, Tree=Visual}}"),
        ["VisualAncestor(Window;0) Title"]
    );
    assert_eq!(
        parsed("{CompiledBinding Name, RelativeSource={RelativeSource Mode=FindAncestor, Tree=Logical, AncestorType=StackPanel}}"),
        ["LogicalAncestor(StackPanel;0) Name"]
    );
    // The ancestor type of a logical ancestor defaults to the nearest styled element.
    assert_eq!(
        parsed("{CompiledBinding Name, RelativeSource={RelativeSource Mode=FindAncestor, Tree=Logical}}"),
        ["LogicalAncestor(TextBlock;0) Name"]
    );
    // `Mode=DataContext` adds nothing.
    assert_eq!(
        parsed("{CompiledBinding StringProperty, RelativeSource={RelativeSource DataContext}}"),
        ["StringProperty"]
    );
}

#[test]
fn templated_parent_relative_source_needs_a_control_template() {
    let (fw, _, parsed) = parsed_paths(
        "",
        "<Button><Button.Template><ControlTemplate><ContentPresenter Content='{CompiledBinding Content, RelativeSource={RelativeSource TemplatedParent}}'/></ControlTemplate></Button.Template></Button>",
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(parsed, ["TemplatedParent(Button) Content"]);

    assert_eq!(
        single_error(DC, "<TextBlock Text='{CompiledBinding Title, RelativeSource={RelativeSource TemplatedParent}}'/>"),
        (
            "FRN2100".to_string(),
            "A binding with a TemplatedParent RelativeSource has to be in a ControlTemplate.".to_string()
        )
    );
}

#[test]
fn reports_invalid_long_form_sources() {
    let bindings_error = |message: &str| ("FRN2100".to_string(), message.to_string());
    let only_one = "Only one of ElementName, Source, or RelativeSource specified as a binding source. Only one property is allowed.";
    for (binding, expected) in [
        ("{CompiledBinding Title, ElementName=a, RelativeSource={RelativeSource Self}}", only_one),
        ("{CompiledBinding Title, ElementName=a, Source=b}", only_one),
        (
            "{CompiledBinding Title, RelativeSource={RelativeSource Mode=FindAncestor}}",
            "AncestorType must be set for RelativeSourceMode.FindAncestor when searching the visual tree.",
        ),
        (
            "{CompiledBinding Title, RelativeSource={RelativeSource Mode=Bogus}}",
            "Unknown RelativeSource mode 'Bogus'.",
        ),
        (
            "{CompiledBinding Title, RelativeSource={RelativeSource AncestorType=Window, Tree=Sideways}}",
            "Unknown tree type 'Sideways'.",
        ),
        (
            "{CompiledBinding StringProperty, ElementName={x:Type Window}}",
            "Invalid ElementName 'XamlTypeExtensionNode'.",
        ),
        (
            "{CompiledBinding Name, RelativeSource={StaticResource x}}",
            "Expected an object of type 'FerroUI.Data.RelativeSource'. Found a object of type 'System.Runtime:System.Object'",
        ),
    ] {
        assert_eq!(
            single_error(DC, &format!("<TextBlock Text='{binding}'/>")),
            bindings_error(expected),
            "{binding}"
        );
    }

    // Element syntax with an object of another type.
    assert_eq!(
        single_error(
            &format!("{DC} x:CompileBindings='True'"),
            "<TextBlock><TextBlock.Text><Binding Path='Name'><Binding.RelativeSource><TextBlock/></Binding.RelativeSource></Binding></TextBlock.Text></TextBlock>"
        ),
        bindings_error(
            "Expected an object of type 'FerroUI.Data.RelativeSource'. Found a object of type 'FerroUI.Controls:FerroUI.Controls.TextBlock'"
        )
    );
}

#[test]
fn reports_unparsable_paths() {
    let (code, title) = single_error(DC, "<TextBlock Text='{CompiledBinding invalid.}'/>");
    assert_eq!(code, "FRN2000");
    assert_eq!(
        title,
        "Failed to parse binding path 'invalid.': Unexpected end of expression."
    );
}

#[test]
fn binding_expression_nodes() {
    assert!(BindingExpressionNode::Grammar(Node::Not).is_transform_node());
    assert!(!BindingExpressionNode::Grammar(Node::SelfNode).is_transform_node());
    assert!(BindingExpressionNode::Grammar(Node::EmptyExpression).is_empty_expression_node());
    let fw = create_bindings_test_framework();
    let templated_parent = BindingExpressionNode::TemplatedParent(TemplatedParentBindingExpressionNode {
        type_: fw.t("FerroUI.Controls.Button"),
    });
    assert!(!templated_parent.is_transform_node());
    assert!(!templated_parent.is_empty_expression_node());
}

// FerroXamlIlBindingPathTransformer

#[test]
fn binds_to_source() {
    // Binds_To_Source: the type of the literal source is the start type.
    let (_, paths, errors) = resolve(DC, "<TextBlock Text='{CompiledBinding Length, Source=Test}'/>");
    assert_eq!(errors, vec![]);
    assert_eq!(paths, ["Property(String.Length:System.Int32)"]);
}

#[test]
fn binds_to_source_static_resource() {
    let binding = "<TextBlock Text='{Binding StringProperty, Source={StaticResource dataKey}}'/>";
    let attributes = "x:DataType='local:TestDataContext' x:CompileBindings='True'";
    // Binds_To_Source_StaticResource (+1, _In_ResourceDictionary, _In_ResourceDictionary1)
    for resources in [
        "<local:TestData x:Key='dataKey' StringProperty='Foobar'/>",
        "<local:TestDataContext x:Key='other'/><local:TestData x:Key='dataKey' StringProperty='Foobar'/>",
        "<ResourceDictionary><local:TestData x:Key='dataKey' StringProperty='Foobar'/></ResourceDictionary>",
        "<ResourceDictionary><local:TestDataContext x:Key='other'/><local:TestData x:Key='dataKey' StringProperty='Foobar'/></ResourceDictionary>",
    ] {
        let (_, paths, errors) = resolve(
            attributes,
            &format!("<Window.Resources>{resources}</Window.Resources>{binding}"),
        );
        assert_eq!(errors, vec![], "{resources}");
        assert_eq!(paths, ["Property(TestData.StringProperty:System.String)"], "{resources}");
    }

    // A resource that is not found in the document has the type the extension provides.
    let (code, title) = single_error(attributes, binding);
    assert_eq!(code, "FRN2000");
    assert_eq!(
        title,
        "Unable to resolve property or method of name 'StringProperty' on type 'System.Object'."
    );
}

#[test]
fn binds_to_source_x_static() {
    // Binds_To_Source_xStatic
    let (_, paths, errors) = resolve(
        "x:CompileBindings='True'",
        "<ContentControl Content='{Binding Length, Source={x:Static local:TestDataContext.StaticProperty}}'/>",
    );
    assert_eq!(errors, vec![]);
    assert_eq!(paths, ["Property(String.Length:System.Int32)"]);
}

#[test]
fn the_data_type_of_the_binding_wins_over_the_scope() {
    let (_, paths, errors) = resolve(
        "x:DataType='local:TestData'",
        "<TextBlock Text='{CompiledBinding BoolProperty, DataType=local:TestDataContext}'/>",
    );
    assert_eq!(errors, vec![]);
    assert_eq!(paths, ["Property(TestDataContext.BoolProperty:System.Boolean)"]);
}

#[test]
fn the_resolved_path_replaces_the_parsed_one() {
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(DC, "<StackPanel><TextBlock Text='{CompiledBinding StringProperty}'/><TextBlock Text='{CompiledBinding Path=StringProperty}'/></StackPanel>"),
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert!(find_binding_nodes::<ParsedBindingPathNode>(&root).is_empty());
    let resolved = find_binding_nodes::<XamlIlBindingPathNode>(&root);
    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0].type_name(), "XamlIlBindingPathNode");

    let bindings: Vec<_> = find_binding_nodes::<XamlAstConstructableObjectNode>(&root)
        .into_iter()
        .filter(|n| n.type_().get_clr_type().is_ok_and(|t| t.name() == "CompiledBindingExtension"))
        .collect();
    // Constructor argument ...
    assert!(bindings[0].arguments.borrow()[0].is::<XamlIlBindingPathNode>());
    // ... and `Path` assignment.
    let path_assignment = bindings[1]
        .children
        .borrow()
        .iter()
        .find_map(|c| c.cast::<XamlPropertyAssignmentNode>())
        .expect("assignment");
    assert_eq!(path_assignment.property.name(), "Path");
    assert!(path_assignment.values.borrow()[0].is::<XamlIlBindingPathNode>());
}

// FerroXamlIlCompiledBindingsMetadataRemover

#[test]
fn the_metadata_remover_unwraps_all_binding_scopes() {
    let xaml = window(
        &format!("{DC} x:CompileBindings='True'"),
        "<StackPanel>
           <Border x:DataType='x:String' x:CompileBindings='False'><TextBlock Text='{CompiledBinding Length}'/></Border>
           <Button><Button.Template><ControlTemplate><ContentPresenter x:Name='p'/></ControlTemplate></Button.Template></Button>
         </StackPanel>",
    );

    let fw = create_bindings_test_framework();
    let root = transform_root(&fw, &xaml);
    assert_eq!(reported_errors(&fw), vec![]);
    assert_eq!(find_binding_nodes::<FerroXamlIlDataContextTypeMetadataNode>(&root).len(), 2);
    assert_eq!(find_binding_nodes::<FerroXamlIlCompileBindingsNode>(&root).len(), 2);
    assert_eq!(find_binding_nodes::<NestedScopeMetadataNode>(&root).len(), 1);
    // The compile-bindings scope was created first, around the object node; the data context
    // scope wraps the constructed object inside it.
    assert!(root.is::<FerroXamlIlCompileBindingsNode>());

    let fw = create_bindings_test_framework();
    let root = transform_root_with(
        &fw,
        &xaml,
        BindingsPipelineOptions {
            remove_metadata: true,
            ..BindingsPipelineOptions::default()
        },
    );
    assert_eq!(reported_errors(&fw), vec![]);
    assert!(find_binding_nodes::<FerroXamlIlDataContextTypeMetadataNode>(&root).is_empty());
    assert!(find_binding_nodes::<FerroXamlIlCompileBindingsNode>(&root).is_empty());
    assert!(find_binding_nodes::<NestedScopeMetadataNode>(&root).is_empty());
    // The root is the window object again and the resolved paths are still there.
    assert!(root.is::<XamlAstConstructableObjectNode>());
    assert_eq!(resolved_paths(&root), ["Property(String.Length:System.Int32)"]);
}
