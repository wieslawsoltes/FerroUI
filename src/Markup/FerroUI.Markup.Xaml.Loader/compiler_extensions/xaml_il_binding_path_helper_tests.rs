//! AST-level ports of the compile-time side of the upstream compiled binding tests
//! (`MarkupExtensions/CompiledBindingExtensionTests.cs`): path resolution against the data
//! context type, the resolved path elements and the errors.

use std::rc::Rc;

use xamlx::ast::{IXamlAstValueNode, XamlAstExtensions, XamlAstNodeExtensions};
use xamlx::type_system::IXamlType;

use crate::compiler_extensions::*;
use crate::testing::bindings::*;

const DC: &str = "x:DataType='local:TestDataContext'";
const TEST_DATA_CONTEXT: &str = "FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.TestDataContext";

/// The resolved paths of a window whose data type is `TestDataContext`; no error may be
/// reported.
fn paths(content: &str) -> Vec<String> {
    paths_in(DC, content)
}

fn paths_in(attributes: &str, content: &str) -> Vec<String> {
    let (_, paths, errors) = resolve(attributes, content);
    assert_eq!(errors, vec![], "{content}");
    paths
}

/// The single resolved path of a `TextBlock.Tag` binding.
fn path(binding: &str) -> String {
    let paths = paths(&format!("<TextBlock Tag='{{CompiledBinding {binding}}}'/>"));
    assert_eq!(paths.len(), 1, "{paths:?}");
    paths[0].clone()
}

/// The single error of a `TextBlock.Tag` binding: its code and its title without the line
/// information.
fn error(binding: &str) -> (String, String) {
    error_in(DC, &format!("<TextBlock Tag='{{CompiledBinding {binding}}}'/>"))
}

fn error_in(attributes: &str, content: &str) -> (String, String) {
    let (_, paths, errors) = resolve(attributes, content);
    assert_eq!(paths, Vec::<String>::new());
    assert_eq!(errors.len(), 1, "{errors:?}");
    let (code, title) = errors[0].clone();
    let title = match title.find(" Line ") {
        Some(index) => title[..index].to_string(),
        None => title,
    };
    (code, title)
}

fn transform_error(message: &str) -> (String, String) {
    ("FRN2000".to_string(), message.to_string())
}

fn bindings_error(message: &str) -> (String, String) {
    ("FRN2100".to_string(), message.to_string())
}

// Properties

#[test]
fn resolves_clr_property_based_on_data_context_type() {
    assert_eq!(
        paths("<TextBlock Text='{CompiledBinding StringProperty}' Name='textBlock' />"),
        ["Property(TestDataContext.StringProperty:System.String)"]
    );
}

#[test]
fn resolves_clr_property_through_interface_inheritance() {
    assert_eq!(
        paths_in(
            "x:DataType='local:IHasPropertyDerived'",
            "<TextBlock Text='{CompiledBinding StringProperty}'/>"
        ),
        ["Property(IHasProperty.StringProperty:System.String)"]
    );
}

#[test]
fn resolves_path_passed_by_property() {
    assert_eq!(
        paths("<TextBlock Text='{CompiledBinding Path=StringProperty}'/>"),
        ["Property(TestDataContext.StringProperty:System.String)"]
    );
}

#[test]
fn resolves_static_clr_property() {
    assert_eq!(
        path("StaticProperty"),
        "Property(TestDataContext.StaticProperty:System.String)"
    );
}

#[test]
fn resolves_data_type_from_binding_property() {
    for data_type in ["local:TestDataContext", "{x:Type local:TestDataContext}"] {
        assert_eq!(
            paths_in(
                "",
                &format!(
                    "<TextBlock Text='{{CompiledBinding StringProperty, DataType={data_type}}}'/>"
                )
            ),
            ["Property(TestDataContext.StringProperty:System.String)"]
        );
    }
}

#[test]
fn resolves_nested_and_null_conditional_properties() {
    assert_eq!(
        path("NestedGenericString.Value"),
        "Property(TestDataContext.NestedGenericString:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.TestDataContext+NestedGeneric`1[System.String]) > Property(TestDataContext+NestedGeneric`1.Value:System.String)"
    );
    assert!(path("NestedGenericString?.Value").ends_with("Property(TestDataContext+NestedGeneric`1.Value:System.String)?"));
    // A property declared by a generic base class is found on the constructed type.
    assert!(path("GenericProperty.CurrentItem")
        .ends_with("Property(ListItemCollectionView`1.CurrentItem:System.Int32)"));
}

#[test]
fn resolves_nested_generic_data_types() {
    assert_eq!(
        paths_in(
            "x:DataType='{x:Type local:TestDataContext+NestedGeneric, x:TypeArguments=x:String}'",
            "<TextBlock Text='{CompiledBinding Value}'/>"
        ),
        ["Property(TestDataContext+NestedGeneric`1.Value:System.String)"]
    );
}

#[test]
fn a_registered_property_wins_over_the_clr_property() {
    assert_eq!(
        paths_in("", "<TextBlock Text='{CompiledBinding $self.Name}'/>"),
        ["Self(FerroUI.Controls.TextBlock) > FerroProperty(StyledElement.NameProperty:System.String)"]
    );
}

#[test]
fn resolves_attached_properties() {
    assert_eq!(
        path("$self.(Grid.Row)"),
        "Self(FerroUI.Controls.TextBlock) > FerroProperty(Grid.RowProperty:System.Int32)"
    );
    assert_eq!(path("(Grid.Row)"), "FerroProperty(Grid.RowProperty:System.Int32)");
    assert_eq!(
        error("$self.(Grid.Missing)"),
        transform_error(
            "Unable to find MissingProperty field on type FerroUI.Controls.Grid,FerroUI.Controls"
        )
    );
}

#[test]
fn reports_unknown_members() {
    assert_eq!(
        error("InvalidPath"),
        transform_error(&format!(
            "Unable to resolve property or method of name 'InvalidPath' on type '{TEST_DATA_CONTEXT}'."
        ))
    );
}

#[test]
fn reports_a_binding_without_a_data_type() {
    assert_eq!(
        error_in("", "<TextBlock Tag='{CompiledBinding NoDataContext}'/>"),
        bindings_error(
            "Cannot parse a compiled binding without an explicit x:DataType directive to give a starting data type for bindings."
        )
    );
}

#[test]
fn reports_multiple_errors_on_data_context_and_binding_path_errors() {
    let (_, paths, errors) = resolve(
        "",
        "<ContentControl Content='{CompiledBinding NoDataContext}'
                         Tag='{CompiledBinding NonExistentProp, DataType=local:TestDataContext}'
                         Height='{CompiledBinding invalid.}' />",
    );
    assert_eq!(paths, Vec::<String>::new());
    let codes: Vec<&str> = errors.iter().map(|(code, _)| code.as_str()).collect();
    assert_eq!(codes, ["FRN2000", "FRN2100", "FRN2000"]);
    assert!(errors[0].1.starts_with("Failed to parse binding path 'invalid.': "));
    assert!(errors[1].1.starts_with("Cannot parse a compiled binding without"));
    assert!(errors[2]
        .1
        .starts_with("Unable to resolve property or method of name 'NonExistentProp'"));
}

// Empty paths

#[test]
fn supports_empty_and_dot_paths() {
    for binding in [
        "{CompiledBinding}",
        "{CompiledBinding .}",
        "{CompiledBinding Path=., StringFormat=bar}",
        "{CompiledBinding StringFormat=bar}",
    ] {
        assert_eq!(
            paths(&format!("<TextBlock Text='{binding}'/>")),
            Vec::<String>::new(),
            "{binding}"
        );
    }
}

// Transform nodes

#[test]
fn negation_is_a_transform_element() {
    let fw = create_bindings_test_framework();
    let root = transform_root(&fw, &window(DC, "<TextBlock Tag='{CompiledBinding !BoolProperty}'/>"));
    let nodes = find_binding_nodes::<XamlIlBindingPathNode>(&root);
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        describe_path(&nodes[0]),
        "Not > Property(TestDataContext.BoolProperty:System.Boolean)"
    );
    assert_eq!(nodes[0].transform_elements.borrow().len(), 1);
    assert_eq!(nodes[0].elements.borrow().len(), 1);
    // The result type is the type of the first transform element.
    assert_eq!(nodes[0].binding_result_type().full_name(), "System.Boolean");
    let as_value: Rc<dyn IXamlAstValueNode> = nodes[0].clone();
    assert_eq!(value_type_name(&as_value), "FerroUI.Data.CompiledBindingPath");
    // `node is IXamlIlBindingPathNode`
    let as_path_node = as_value
        .cast::<dyn IXamlIlBindingPathNode>()
        .expect("binding path node");
    assert_eq!(as_path_node.binding_result_type().full_name(), "System.Boolean");
}

// Streams

#[test]
fn resolves_stream_bindings() {
    assert_eq!(
        path("TaskProperty^"),
        "Property(TestDataContext.TaskProperty:System.Threading.Tasks.Task`1[System.String]) > StreamTask<System.String>"
    );
    assert_eq!(
        path("ObservableProperty^"),
        "Property(TestDataContext.ObservableProperty:System.IObservable`1[System.String]) > StreamObservable<System.String>"
    );
    assert_eq!(
        error("StringProperty^"),
        transform_error(
            "Compiled bindings do not support stream bindings for objects of type System.String."
        )
    );
}

// Indexers

#[test]
fn resolves_indexer_bindings() {
    assert_eq!(
        path("ListProperty[3]"),
        "Property(TestDataContext.ListProperty:System.Collections.Generic.List`1[System.String]) > Indexer(List`1.Item[3]:System.String;notifying=false)"
    );
    assert_eq!(
        path("ObservableCollectionProperty[3]"),
        "Property(TestDataContext.ObservableCollectionProperty:System.Collections.ObjectModel.ObservableCollection`1[System.String]) > Indexer(ObservableCollection`1.Item[3]:System.String;notifying=true)"
    );
    assert!(path("NonIntegerIndexerProperty[Test]")
        .ends_with("Indexer(TestDataContext+NonIntegerIndexer.Item[Test]:System.String;notifying=false)"));
    // The indexer of an interface is found on the interface it extends.
    assert!(path("NonIntegerIndexerInterfaceProperty[Test]")
        .ends_with("Indexer(INonIntegerIndexer.Item[Test]:System.String;notifying=false)"));
}

#[test]
fn indexer_arguments_are_converted_to_the_parameter_types() {
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(DC, "<StackPanel><TextBlock Tag='{CompiledBinding ObservableCollectionProperty[3]}'/><TextBlock Tag='{CompiledBinding NonIntegerIndexerProperty[Test]}'/><TextBlock Tag='{CompiledBinding ListProperty[3]}'/></StackPanel>"),
    );
    let nodes = find_binding_nodes::<XamlIlBindingPathNode>(&root);
    let indexer = |index: usize| match nodes[index].elements.borrow().last().cloned() {
        Some(XamlIlBindingPathElementNode::ClrIndexer(indexer)) => indexer,
        _ => panic!("not an indexer"),
    };

    let observable = indexer(0);
    assert_eq!(observable.values.len(), 1);
    assert_eq!(value_type_name(&observable.values[0]), "System.Int32");
    assert_eq!(observable.indexer_key, "3");
    // A notifying collection indexed with one integer follows collection changes.
    assert!(matches!(
        observable.accessor_factory(&fw.configuration),
        Ok(XamlIlPropertyAccessorFactory::Indexer { .. })
    ));
    let info = observable.property_info(&fw.configuration).expect("property info");
    assert_eq!(
        info.name,
        "System.Collections.ObjectModel.ObservableCollection`1,System.Runtime.Item[3]"
    );
    assert_eq!(info.indexer_arguments.len(), 1);
    let setter = info.setter.as_ref().expect("setter");
    assert_eq!(setter.value_type.full_name(), "System.String");
    assert!(!setter.value_is_value_type);

    let string_key = indexer(1);
    assert_eq!(value_type_name(&string_key.values[0]), "System.String");
    assert!(matches!(
        string_key.accessor_factory(&fw.configuration),
        Ok(XamlIlPropertyAccessorFactory::Inpc)
    ));

    // A list that does not notify uses the plain accessor even with an integer index.
    assert!(matches!(
        indexer(2).accessor_factory(&fw.configuration),
        Ok(XamlIlPropertyAccessorFactory::Inpc)
    ));
}

#[test]
fn resolves_array_indexer_bindings() {
    assert_eq!(
        path("ArrayProperty[3]"),
        "Property(TestDataContext.ArrayProperty:System.String[]) > ArrayElement([3]:System.String)"
    );
    assert_eq!(
        error("ArrayProperty[x]"),
        transform_error("Unable to convert 'x' to an integer.")
    );
}

#[test]
fn reports_indexer_errors() {
    assert_eq!(
        error("StringProperty[0]"),
        transform_error("The type '$System.String' does not have an indexer.")
    );
    // The conversion of the argument fails.
    let (_, paths, errors) = resolve(DC, "<TextBlock Tag='{CompiledBinding ListProperty[x]}'/>");
    assert_eq!(paths, Vec::<String>::new());
    assert_eq!(errors.len(), 1);
}

// Casts

#[test]
fn supports_cast_to_type_in_expression() {
    let window_with_data_context = format!(
        "Ancestor(FerroUI.Controls.Window;0;dc={TEST_DATA_CONTEXT}) > FerroProperty(StyledElement.DataContextProperty:{TEST_DATA_CONTEXT})"
    );
    let content = |path: &str| {
        let paths = paths(&format!("<ContentControl Content='{{CompiledBinding {path}}}'/>"));
        assert_eq!(paths.len(), 1);
        paths[0].clone()
    };
    assert_eq!(
        content("$parent.((local:TestDataContext)DataContext)"),
        format!("{window_with_data_context} > TypeCast({TEST_DATA_CONTEXT})")
    );
    assert_eq!(
        content("$parent.((local:TestDataContext)DataContext).StringProperty"),
        format!("{window_with_data_context} > TypeCast({TEST_DATA_CONTEXT}) > Property(TestDataContext.StringProperty:System.String)")
    );
    assert_eq!(
        content("$parent.DataContext(local:TestDataContext).StringProperty"),
        format!("{window_with_data_context} > TypeCast({TEST_DATA_CONTEXT}) > Property(TestDataContext.StringProperty:System.String)")
    );
    assert_eq!(
        content("$parent.((local:OuterClass+NestedClass)DataContext).NestedProperty"),
        format!("{window_with_data_context} > TypeCast(FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.OuterClass+NestedClass) > Property(OuterClass+NestedClass.NestedProperty:System.String)")
    );
    assert_eq!(
        content("$parent.((local:IHasExplicitProperty)DataContext).ExplicitProperty"),
        format!("{window_with_data_context} > TypeCast(FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.IHasExplicitProperty) > Property(IHasExplicitProperty.ExplicitProperty:System.String)")
    );
    assert_eq!(
        content("((local:TestData)ObjectsArrayProperty[0]).StringProperty"),
        "Property(TestDataContext.ObjectsArrayProperty:System.Object[]) > ArrayElement([0]:System.Object) > TypeCast(FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.TestData) > Property(TestData.StringProperty:System.String)"
    );
}

#[test]
fn reports_a_cast_to_an_unknown_type() {
    let (code, title) = error("((local:Missing)StringProperty).Length");
    assert_eq!(code, "FRN2000");
    assert!(title.starts_with("Unable to resolve type Missing from namespace clr-namespace:"));
}

// Sources: $self, $parent, #name, RelativeSource

#[test]
fn binds_to_self() {
    assert_eq!(
        paths("<TextBlock Text='{CompiledBinding $self}'/>"),
        ["Self(FerroUI.Controls.TextBlock)"]
    );
    // No data type is needed.
    assert_eq!(
        paths_in("", "<TextBlock Text='{CompiledBinding $self.Name}'/>"),
        ["Self(FerroUI.Controls.TextBlock) > FerroProperty(StyledElement.NameProperty:System.String)"]
    );
    for binding in [
        "{CompiledBinding RelativeSource={RelativeSource Self}}",
        "{CompiledBinding RelativeSource={RelativeSource Mode=Self}}",
    ] {
        assert_eq!(
            paths(&format!("<TextBlock Text='{binding}'/>")),
            ["Self(FerroUI.Controls.TextBlock)"]
        );
    }
}

#[test]
fn binds_to_self_in_style() {
    assert_eq!(
        paths_in(
            "",
            "<Window.Styles><Style Selector='Button'><Setter Property='IsVisible' Value='{CompiledBinding $self.IsEnabled}'/></Style></Window.Styles>"
        ),
        ["Self(FerroUI.Controls.Button) > FerroProperty(InputElement.IsEnabledProperty:System.Boolean)"]
    );
}

#[test]
fn binds_to_self_in_multi_binding() {
    assert_eq!(
        paths_in(
            "x:DataType='local:TestDataContext' x:CompileBindings='True'",
            "<TextBlock><TextBlock.Text><MultiBinding><Binding Path='$self.Name'/><Binding Path='StringProperty'/></MultiBinding></TextBlock.Text></TextBlock>"
        ),
        [
            "Self(FerroUI.Controls.TextBlock) > FerroProperty(StyledElement.NameProperty:System.String)",
            "Property(TestDataContext.StringProperty:System.String)"
        ]
    );
    assert_eq!(
        paths_in(
            "x:CompileBindings='True'",
            "<Window.Styles><Style Selector='TextBlock'><Setter Property='Text'><MultiBinding><Binding Path='$self.Text'/></MultiBinding></Setter></Style></Window.Styles>"
        ),
        ["Self(FerroUI.Controls.TextBlock) > FerroProperty(TextBlock.TextProperty:System.String)"]
    );
}

#[test]
fn supports_parent_in_path() {
    assert_eq!(
        paths_in("", "<ContentControl Content='{CompiledBinding $parent.Title}'/>"),
        ["Ancestor(FerroUI.Controls.Window;0;dc=-) > FerroProperty(Window.TitleProperty:System.String)"]
    );
    assert_eq!(
        paths_in(
            "",
            "<StackPanel><Border><TextBlock Text='{CompiledBinding $parent[Control;1].Name}'/></Border></StackPanel>"
        ),
        ["Ancestor(FerroUI.Controls.Control;1;dc=-) > FerroProperty(StyledElement.NameProperty:System.String)"]
    );
    // The type filter is kept even when no such ancestor is in the document.
    assert_eq!(
        paths("<StackPanel><Border><TextBlock Text='{CompiledBinding $parent[Grid].Name}'/></Border></StackPanel>"),
        ["Ancestor(FerroUI.Controls.Grid;0;dc=-) > FerroProperty(StyledElement.NameProperty:System.String)"]
    );
    assert_eq!(
        error_in(
            DC,
            "<StackPanel><Border><TextBlock Text='{CompiledBinding $parent[5].Name}'/></Border></StackPanel>"
        ),
        transform_error("Unable to resolve implicit ancestor type based on XAML tree.")
    );
}

#[test]
fn resolves_parent_data_context_type_based_on_context() {
    let expected = format!(
        "Ancestor(FerroUI.Controls.Panel;0;dc={TEST_DATA_CONTEXT}) > FerroProperty(StyledElement.DataContextProperty:{TEST_DATA_CONTEXT}) > Property(TestDataContext.StringProperty:System.String)"
    );
    for binding in ["$parent[Panel].DataContext.StringProperty", "$parent.DataContext.StringProperty"] {
        assert_eq!(
            paths(&format!("<Panel><TextBlock Text='{{CompiledBinding {binding}}}'/></Panel>")),
            [expected.clone()]
        );
    }

    // The data context of the ancestor itself is used, not the one of the binding.
    assert_eq!(
        paths("<Panel><Button DataContext='{CompiledBinding StringProperty}'><TextBlock Text='{CompiledBinding $parent.DataContext.Length}'/></Button></Panel>"),
        [
            "Property(TestDataContext.StringProperty:System.String)",
            "Ancestor(FerroUI.Controls.Button;0;dc=System.String) > FerroProperty(StyledElement.DataContextProperty:System.String) > Property(String.Length:System.Int32)"
        ]
    );
}

#[test]
fn resolves_element_name_bindings() {
    let named = format!("ElementName(text:FerroUI.Controls.TextBlock;dc={TEST_DATA_CONTEXT})");
    assert_eq!(
        paths(
            "<StackPanel>
               <TextBlock Text='{CompiledBinding StringProperty}' x:Name='text' />
               <TextBlock Text='{CompiledBinding #text.Text}' />
               <TextBlock Text='{CompiledBinding Text, ElementName=text}' />
               <TextBlock Text='{CompiledBinding ElementName=text}' />
             </StackPanel>"
        ),
        [
            "Property(TestDataContext.StringProperty:System.String)".to_string(),
            format!("{named} > FerroProperty(TextBlock.TextProperty:System.String)"),
            format!("{named} > FerroProperty(TextBlock.TextProperty:System.String)"),
            named,
        ]
    );
}

#[test]
fn resolves_element_name_data_context_type_based_on_context() {
    let expected = format!(
        "ElementName(MyWindow:FerroUI.Controls.Window;dc={TEST_DATA_CONTEXT}) > FerroProperty(StyledElement.DataContextProperty:{TEST_DATA_CONTEXT}) > Property(TestDataContext.StringProperty:System.String)"
    );
    for binding in [
        "{CompiledBinding ElementName=MyWindow, Path=DataContext.StringProperty}",
        "{CompiledBinding #MyWindow.DataContext.StringProperty}",
    ] {
        assert_eq!(
            paths_in(
                "x:DataType='local:TestDataContext' x:Name='MyWindow'",
                &format!("<TextBlock Text='{binding}'/>")
            ),
            [expected.clone()]
        );
    }
}

#[test]
fn resolves_element_names_in_templates() {
    let template = |content: &str| {
        format!("<Window.Styles><Style Selector='Button'><Setter Property='Template'><ControlTemplate>{content}</ControlTemplate></Setter></Style></Window.Styles>")
    };
    // A name of the template's own scope.
    assert_eq!(
        paths_in(
            "",
            &template("<StackPanel><TextBlock x:Name='InnerTextBox' Text='abc'/><ContentPresenter Content='{CompiledBinding Text, ElementName=InnerTextBox}'/></StackPanel>")
        ),
        ["ElementName(InnerTextBox:FerroUI.Controls.TextBlock;dc=-) > FerroProperty(TextBlock.TextProperty:System.String)"]
    );
    // A name of the document's scope, used inside the template.
    assert_eq!(
        paths_in(
            "x:Name='w' Title='t'",
            &template("<ContentPresenter Content='{CompiledBinding #w.Title}'/>")
        ),
        ["ElementName(w:FerroUI.Controls.Window;dc=-) > FerroProperty(Window.TitleProperty:System.String)"]
    );
    // A name inside a template is not visible outside of it.
    let (code, title) = error_in(
        "",
        &format!(
            "{}<TextBlock Text='{{CompiledBinding #inner.Text}}'/>",
            template("<StackPanel><TextBlock x:Name='inner'/></StackPanel>")
        ),
    );
    assert_eq!(
        (code.as_str(), title.as_str()),
        (
            "FRN2000",
            "Unable to find element 'inner' in the current namescope. Unable to use a compiled binding with a name binding if the name cannot be found at compile time."
        )
    );
}

#[test]
fn resolves_relative_source_bindings() {
    for ancestor_type in ["Window", "{x:Type Window}"] {
        assert_eq!(
            paths_in(
                "x:DataType='local:TestDataContext' Title='foo'",
                &format!("<TextBlock Text='{{CompiledBinding Title, RelativeSource={{RelativeSource AncestorType={ancestor_type}}}}}'/>")
            ),
            ["VisualAncestor(FerroUI.Controls.Window;0) > FerroProperty(Window.TitleProperty:System.String)"]
        );
    }
    // `Mode=DataContext` leaves the data context as the source.
    assert_eq!(
        paths("<TextBlock Text='{CompiledBinding StringProperty, RelativeSource={RelativeSource DataContext}}'/>"),
        ["Property(TestDataContext.StringProperty:System.String)"]
    );
    // Element syntax.
    assert_eq!(
        paths_in(
            "x:DataType='local:TestDataContext' x:CompileBindings='True'",
            "<TextBlock><TextBlock.Text><Binding Path='Name'><Binding.RelativeSource><RelativeSource Mode='Self'/></Binding.RelativeSource></Binding></TextBlock.Text></TextBlock>"
        ),
        ["Self(FerroUI.Controls.TextBlock) > FerroProperty(StyledElement.NameProperty:System.String)"]
    );
}

#[test]
fn resolves_templated_parent_bindings() {
    assert_eq!(
        paths_in(
            "",
            "<Window.Styles><Style Selector='Button'><Setter Property='Template'><ControlTemplate><ContentPresenter Focusable='{CompiledBinding !Focusable, RelativeSource={RelativeSource TemplatedParent}}'/></ControlTemplate></Setter></Style></Window.Styles>"
        ),
        ["Not > TemplatedParent(FerroUI.Controls.Button) > FerroProperty(InputElement.FocusableProperty:System.Boolean)"]
    );
    assert_eq!(
        paths_in(
            "",
            "<Button><Button.Template><ControlTemplate><ContentPresenter Content='{CompiledBinding RelativeSource={RelativeSource TemplatedParent}, Path=Content}'/></ControlTemplate></Button.Template></Button>"
        ),
        ["TemplatedParent(FerroUI.Controls.Button) > FerroProperty(ContentControl.ContentProperty:System.Object)"]
    );
}

#[test]
fn a_logical_ancestor_relative_source_adds_no_element() {
    // As upstream: the path resolution has no case for the logical ancestor node, so the
    // path is resolved against the data context.
    assert_eq!(
        error_in(
            DC,
            "<StackPanel><Border><TextBlock Text='{CompiledBinding Name, RelativeSource={RelativeSource Mode=FindAncestor, Tree=Logical, AncestorType=StackPanel}}'/></Border></StackPanel>"
        ),
        transform_error(&format!(
            "Unable to resolve property or method of name 'Name' on type '{TEST_DATA_CONTEXT}'."
        ))
    );
}

// Methods and commands

#[test]
fn supports_method_binding_as_delegate() {
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(
            "x:DataType='local:MethodDataContext'",
            "<StackPanel>
               <ContentControl Content='{CompiledBinding Action}' />
               <ContentControl Content='{CompiledBinding Func}' />
               <ContentControl Content='{CompiledBinding Func2}' />
               <ContentControl Content='{CompiledBinding CustomDelegateTypeVoid}' />
               <ContentControl Content='{CompiledBinding CustomDelegateTypeInt}' />
             </StackPanel>",
        ),
    );
    assert_eq!(reported_errors(&fw), vec![]);
    let nodes = find_binding_nodes::<XamlIlBindingPathNode>(&root);
    assert_eq!(
        nodes.iter().map(|n| describe_path(n)).collect::<Vec<_>>(),
        [
            "Method(MethodDataContext.Action():System.Delegate)",
            "Method(MethodDataContext.Func():System.Delegate)",
            "Method(MethodDataContext.Func2(Object):System.Delegate)",
            "Method(MethodDataContext.CustomDelegateTypeVoid(Object):System.Delegate)",
            "Method(MethodDataContext.CustomDelegateTypeInt(Object):System.Delegate)",
        ]
    );

    let delegate_type = |index: usize| match nodes[index].elements.borrow().last().cloned() {
        Some(XamlIlBindingPathElementNode::ClrMethod(method)) => {
            match method.specific_delegate_type(&fw.configuration) {
                Ok(XamlIlMethodDelegateType::Existing(type_)) => type_.full_name(),
                _ => panic!("no delegate type"),
            }
        }
        _ => panic!("not a method"),
    };
    assert_eq!(delegate_type(0), "System.Action");
    assert_eq!(delegate_type(1), "System.Func`1[System.Object]");
    assert_eq!(delegate_type(2), "System.Func`2[System.Object,System.Object]");
    assert_eq!(delegate_type(3), "System.Action`1[System.Object]");
    assert_eq!(delegate_type(4), "System.Func`2[System.Object,System.Object]");
}

#[test]
fn a_method_with_more_than_sixteen_parameters_needs_a_custom_delegate_type() {
    let fw = create_bindings_test_framework();
    let method = fw
        .t("FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.MethodDataContext")
        .methods()
        .into_iter()
        .find(|m| m.name() == "ManyParameters")
        .expect("method");
    let element = XamlIlClrMethodPathElementNode {
        method,
        type_: fw.configuration.well_known_types().delegate.clone(),
        accepts_null: false,
    };
    match element.specific_delegate_type(&fw.configuration) {
        Ok(XamlIlMethodDelegateType::Custom {
            name,
            return_type,
            parameters,
        }) => {
            assert!(!name.is_empty());
            assert_eq!(return_type.full_name(), "System.Void");
            assert_eq!(parameters.len(), 17);
        }
        _ => panic!("expected a custom delegate type"),
    }
}

#[test]
fn binding_a_method_to_a_command() {
    let (fw, paths, errors) = resolve(
        "x:DataType='local:MethodAsCommandDataContext'",
        "<StackPanel>
           <Button Command='{CompiledBinding Method}'/>
           <Button Command='{CompiledBinding ObjectMethod}'/>
           <Button Command='{CompiledBinding Int32Method}'/>
           <Button Command='{CompiledBinding StringMethod}'/>
           <Button Command='{CompiledBinding VirtualObjectMethod}'/>
           <Button Command='{CompiledBinding VirtualInt32Method}'/>
           <Button Command='{CompiledBinding MethodWithNewSlot}'/>
           <Button Command='{CompiledBinding MethodWithOverloads}'/>
           <Button Command='{CompiledBinding MethodWithOverloads3}'/>
           <Button Command='{CompiledBinding Do}'/>
           <TextBlock Text='{CompiledBinding Method}'/>
         </StackPanel>",
    );
    let _ = fw;
    assert_eq!(errors, vec![]);
    assert_eq!(
        paths,
        [
            "Command(MethodAsCommandDataContext.Method();can=-;depends=[])",
            "Command(MethodAsCommandDataContext.ObjectMethod(Object);can=-;depends=[])",
            "Command(MethodAsCommandDataContext.Int32Method(Int32);can=-;depends=[])",
            "Command(MethodAsCommandDataContext.StringMethod(String);can=-;depends=[])",
            // Overrides and hiding methods: the most derived declaration wins.
            "Command(MethodAsCommandDataContext.VirtualObjectMethod(Object);can=-;depends=[])",
            "Command(MethodAsCommandDataContext.VirtualInt32Method(Int32);can=-;depends=[])",
            "Command(MethodAsCommandDataContext.MethodWithNewSlot(Int32);can=-;depends=[])",
            // The overload taking System.Object is preferred.
            "Command(MethodAsCommandDataContext.MethodWithOverloads(Object);can=-;depends=[])",
            // Without one-parameter overloads the parameterless one is used.
            "Command(MethodAsCommandDataContext.MethodWithOverloads3();can=-;depends=[])",
            // `CanDo(object)` and its `DependsOn` attribute are picked up.
            "Command(MethodAsCommandDataContext.Do(Object);can=CanDo;depends=[Parameter])",
            // Not a command property: the method is bound as a delegate.
            "Method(MethodAsCommandDataContext.Method():System.Delegate)",
        ]
    );
}

#[test]
fn binding_a_method_to_a_command_fails_without_a_usable_overload() {
    let type_name = "FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.MethodAsCommandDataContext";
    assert_eq!(
        error_in(
            "x:DataType='local:MethodAsCommandDataContext'",
            "<Button Command='{CompiledBinding MethodWithOverloads2}' CommandParameter='foo'/>"
        ),
        transform_error(&format!(
            "Unable to resolve method of name 'MethodWithOverloads2' on type '{type_name}'. Found 2 overloads accepting one parameter: 'System.Int32', 'System.String'. Expected either a single overload with one parameter, or an overload accepting System.Object."
        ))
    );
    assert_eq!(
        error_in(
            "x:DataType='local:MethodAsCommandDataContext'",
            "<Button Command='{CompiledBinding MethodWithOverloads4}' CommandParameter='foo'/>"
        ),
        transform_error(&format!(
            "Unable to resolve method of name 'MethodWithOverloads4' on type '{type_name}'. Found 2 overloads accepting more than one parameter. Expected a method with zero or one parameter."
        ))
    );
}

#[test]
fn binding_a_method_to_a_command_in_a_style() {
    // The type of the property a setter sets decides: `Command` is a command, `Tag` is not.
    assert_eq!(
        paths_in(
            "x:DataType='local:MethodAsCommandDataContext'",
            "<Window.Styles><Style Selector='Button'><Setter Property='Command' Value='{CompiledBinding Method}'/><Setter Property='Tag' Value='{CompiledBinding Method}'/></Style></Window.Styles>"
        ),
        [
            "Command(MethodAsCommandDataContext.Method();can=-;depends=[])",
            "Method(MethodAsCommandDataContext.Method():System.Delegate)"
        ]
    );
}

#[test]
fn command_elements_describe_their_trampolines() {
    let fw = create_bindings_test_framework();
    let root = transform_root(
        &fw,
        &window(
            "x:DataType='local:MethodAsCommandDataContext'",
            "<StackPanel><Button Command='{CompiledBinding Do}'/><Button Command='{CompiledBinding Int32Method}'/><Button Command='{CompiledBinding Method}'/></StackPanel>",
        ),
    );
    let nodes = find_binding_nodes::<XamlIlBindingPathNode>(&root);
    let command = |index: usize| match nodes[index].elements.borrow().last().cloned() {
        Some(XamlIlBindingPathElementNode::ClrMethodAsCommand(command)) => command,
        _ => panic!("not a command"),
    };
    let owner = "FerroUI.Markup.Xaml.UnitTests:FerroUI.Markup.Xaml.UnitTests.MarkupExtensions.MethodAsCommandDataContext";

    let do_command = command(0);
    assert_eq!(do_command.type_.full_name(), "System.Windows.Input.ICommand");
    let execute = do_command.execute_trampoline(&fw.configuration);
    assert_eq!(execute.name, format!("{owner}+Do_1!CommandExecuteTrampoline"));
    assert!(!execute.this_is_value_type);
    let parameter = execute.parameter.as_ref().expect("parameter");
    assert!(!parameter.unbox_any);
    let can_execute = do_command
        .can_execute_trampoline(&fw.configuration)
        .expect("can execute");
    assert_eq!(can_execute.name, format!("{owner}+CanDo!CommandCanExecuteTrampoline"));
    // One trampoline per method.
    assert!(Rc::ptr_eq(&execute, &do_command.execute_trampoline(&fw.configuration)));
    assert!(Rc::ptr_eq(
        &can_execute,
        &do_command.can_execute_trampoline(&fw.configuration).expect("can execute")
    ));

    let int32_command = command(1);
    let execute = int32_command.execute_trampoline(&fw.configuration);
    assert_eq!(execute.name, format!("{owner}+Int32Method_1!CommandExecuteTrampoline"));
    let parameter = execute.parameter.as_ref().expect("parameter");
    assert!(parameter.unbox_any);
    assert_eq!(parameter.parameter_type.full_name(), "System.Int32");
    assert!(int32_command.can_execute_trampoline(&fw.configuration).is_none());

    let execute = command(2).execute_trampoline(&fw.configuration);
    assert_eq!(execute.name, format!("{owner}+Method_0!CommandExecuteTrampoline"));
    assert!(execute.parameter.is_none());
}

// Typed emission and property infos

fn single_clr_property(
    fw: &crate::testing::TestFramework,
    attributes: &str,
    content: &str,
) -> (Rc<XamlIlBindingPathNode>, Option<Rc<XamlIlClrPropertyPathElementNode>>) {
    let root = transform_root(fw, &window(attributes, content));
    let nodes = find_binding_nodes::<XamlIlBindingPathNode>(&root);
    assert_eq!(nodes.len(), 1);
    let node = nodes[0].clone();
    let last = match node.elements.borrow().last().cloned() {
        Some(XamlIlBindingPathElementNode::ClrProperty(property)) => Some(property),
        _ => None,
    };
    (node, last)
}

#[test]
fn typed_emission_is_enabled_for_a_single_instance_property_of_a_class() {
    let fw = create_bindings_test_framework();

    let (node, property) =
        single_clr_property(&fw, DC, "<TextBlock Text='{CompiledBinding StringProperty}'/>");
    let property = property.expect("property");
    assert!(!property.emit_typed.get());
    node.try_enable_typed_emission();
    assert!(property.emit_typed.get());
    let typed = property.typed_property_info(&fw.configuration).expect("typed info");
    assert_eq!(typed.source_type.full_name(), TEST_DATA_CONTEXT);
    assert_eq!(typed.value_type.full_name(), "System.String");
    assert_eq!(
        typed.property_info_type.full_name(),
        format!("FerroUI.Data.Core.IPropertyInfo`2[{TEST_DATA_CONTEXT},System.String]")
    );
    assert_eq!(
        typed.clr_property_info_type.full_name(),
        format!("FerroUI.Data.Core.ClrPropertyInfo`2[{TEST_DATA_CONTEXT},System.String]")
    );
    assert_eq!(
        typed.name,
        format!("{TEST_DATA_CONTEXT},FerroUI.Markup.Xaml.UnitTests.StringProperty!Typed")
    );
    assert!(typed.getter.is_some() && typed.setter.is_some());
    assert!(matches!(property.accessor_factory(), XamlIlPropertyAccessorFactory::Inpc));

    // Not for a nested path, a negated path, a static property.
    for content in [
        "<TextBlock Text='{CompiledBinding NestedGenericString.Value}'/>",
        "<TextBlock Tag='{CompiledBinding !BoolProperty}'/>",
        "<TextBlock Text='{CompiledBinding StaticProperty}'/>",
    ] {
        let (node, property) = single_clr_property(&fw, DC, content);
        node.try_enable_typed_emission();
        assert!(!property.expect("property").emit_typed.get(), "{content}");
    }
}

#[test]
fn property_infos_are_described_once_per_property() {
    let fw = create_bindings_test_framework();
    let (_, string_property) =
        single_clr_property(&fw, DC, "<TextBlock Text='{CompiledBinding StringProperty}'/>");
    let string_property = string_property.expect("property");
    let (_, again) =
        single_clr_property(&fw, DC, "<TextBlock Tag='{CompiledBinding StringProperty}'/>");
    let again = again.expect("property");

    let info = string_property.property_info(&fw.configuration).expect("info");
    assert!(Rc::ptr_eq(
        &info,
        &again.property_info(&fw.configuration).expect("info")
    ));
    assert_eq!(
        info.name,
        format!("{TEST_DATA_CONTEXT},FerroUI.Markup.Xaml.UnitTests.StringProperty")
    );
    assert_eq!(info.property_name(), "StringProperty");
    assert_eq!(info.property_type().full_name(), "System.String");
    assert!(info.indexer_arguments.is_empty());
    let getter = info.getter.as_ref().expect("getter");
    assert!(getter.pass_this && !getter.this_is_value_type);
    assert!(matches!(getter.result, XamlIlClrPropertyGetterResult::Reference));
    let setter = info.setter.as_ref().expect("setter");
    assert!(setter.pass_this && !setter.value_is_value_type);

    // Booleans are returned through the cached boxes, other value types are boxed.
    let (_, bool_property) =
        single_clr_property(&fw, DC, "<TextBlock Tag='{CompiledBinding BoolProperty}'/>");
    let info = bool_property.expect("property").property_info(&fw.configuration).expect("info");
    assert!(matches!(
        info.getter.as_ref().expect("getter").result,
        XamlIlClrPropertyGetterResult::CachedBoxedBoolean
    ));
    assert!(info.setter.as_ref().expect("setter").value_is_value_type);
    let (_, decimal_property) =
        single_clr_property(&fw, DC, "<TextBlock Tag='{CompiledBinding DecimalValue}'/>");
    let info = decimal_property.expect("property").property_info(&fw.configuration).expect("info");
    match &info.getter.as_ref().expect("getter").result {
        XamlIlClrPropertyGetterResult::Boxed(type_) => assert_eq!(type_.full_name(), "System.Decimal"),
        _ => panic!("expected a boxed value"),
    }

    // A static, read-only property: no receiver, no setter.
    let (_, static_property) =
        single_clr_property(&fw, DC, "<TextBlock Tag='{CompiledBinding StaticProperty}'/>");
    let info = static_property.expect("property").property_info(&fw.configuration).expect("info");
    assert!(!info.getter.as_ref().expect("getter").pass_this);
    assert!(info.setter.is_none());
}

// update_compiled_binding_extension

#[test]
fn the_binding_result_type_types_the_data_context_below() {
    assert_eq!(
        paths("<TextBlock DataContext='{CompiledBinding StringProperty}' Text='{CompiledBinding Length}'/>"),
        [
            "Property(TestDataContext.StringProperty:System.String)",
            "Property(String.Length:System.Int32)"
        ]
    );
    // An empty path below a data context binding binds to that data context.
    assert_eq!(
        paths("<TextBlock DataContext='{CompiledBinding StringProperty}' Text='{CompiledBinding}'/>"),
        ["Property(TestDataContext.StringProperty:System.String)"]
    );
}

#[test]
fn an_empty_resolved_path_has_the_unknown_result_type() {
    let fw = create_bindings_test_framework();
    let types = fw.types.clone();
    let line_info = xamlx::ast::XamlLineInfo::new(1, 1);
    let node = XamlIlBindingPathNode::new(
        &line_info,
        types.compiled_binding_path.clone(),
        Vec::new(),
        Vec::new(),
    );
    let unknown: Rc<dyn IXamlType> = node.binding_result_type();
    assert!(xamlx::type_system::XamlPseudoType::is_unknown(&*unknown));
    let as_value: Rc<dyn IXamlAstValueNode> = node;
    assert!(as_value
        .type_()
        .get_clr_type()
        .expect("type")
        .equals(&*types.compiled_binding_path));
}
