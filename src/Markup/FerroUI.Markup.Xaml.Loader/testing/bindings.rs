//! Fixture additions and helpers for the tests of the compiled binding transformers.
//!
//! [`create_bindings_test_framework`] adds to the fake framework (extended with
//! [`super::styles::extend_with_styles`]) what the binding transformers look up:
//!
//! * the members of the binding classes: `CompiledBinding` (`Path`, `Source`, `Mode`,
//!   `StringFormat`, ...), `CompiledBindingExtension` (derives from `CompiledBinding`,
//!   constructor taking a `CompiledBindingPath`, typed `ProvideValue`, `DataType`),
//!   `ReflectionBindingExtension`, `RelativeSource` and `RelativeSourceExtension`,
//!   `StaticResourceExtension(object)`, `MultiBinding.Bindings`;
//! * `INamed` on `StyledElement`, `StyledElement.Styles`, `IResourceDictionary.Add`,
//!   `Visual.IsVisible`, `InputElement.Focusable`, `ContentPresenter`, the
//!   `InheritDataTypeFromItems` attribute on `ItemsControl.ItemTemplate`;
//! * the base class library members the binding paths walk: `String.Length`, the indexer of
//!   `List<T>`, `ObservableCollection<T>`, `INotifyPropertyChanged`,
//!   `INotifyCollectionChanged`, `DefaultMemberAttribute`, `Decimal`;
//! * the view models and controls of the upstream compiled binding tests, with the same
//!   names and member signatures, in the assembly [`UNIT_TESTS_ASSEMBLY`] and the namespace
//!   [`LOCAL_NAMESPACE`] (`TestDataContext`, `MethodAsCommandDataContext`,
//!   `DataGridLikeControl`, `CustomDataTemplate`, ...).
//!
//! [`transform_bindings`] runs the default `xamlx` transformers with the binding transformers
//! (and the transformers of other areas they cooperate with: names, registered properties,
//! selectors, setters, control template scopes) inserted at their upstream positions.

use std::rc::Rc;

use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstVisitor, XamlAstCast, XamlAstExtensions,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlDocument, XamlRootObjectNode,
};
use xamlx::compiler::XamlCompiler;
use xamlx::diagnostics::XamlDiagnostic;
use xamlx::emit::XamlLanguageEmitMappings;
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::parsers::XDocumentXamlParser;
use xamlx::testing::{FakeCustomAttribute, FakeProperty, FakeType};
use xamlx::transform::IXamlAstTransformer;
use xamlx::type_system::{IXamlType, XamlValue};

use crate::compiler_extensions::transformers::{
    AddNameScopeRegistration, FerroBindingExtensionTransformer,
    FerroXamlIlBindingPathParser, FerroXamlIlBindingPathTransformer,
    FerroXamlIlCompiledBindingsMetadataRemover,
    FerroXamlIlControlTemplateTargetTypeMetadataTransformer, FerroXamlIlDataContextTypeTransformer,
    FerroXamlIlFerroPropertyResolver, FerroXamlIlSelectorTransformer,
    FerroXamlIlSetterTargetTypeMetadataTransformer, FerroXamlIlSetterTransformer,
    FerroXamlIlTransformSyntheticCompiledBindingMembers, XDataTypeTransformer, XNameTransformer,
    FERRO_XML_NAMESPACE,
};
use crate::compiler_extensions::{XamlIlBindingPathElementNode, XamlIlBindingPathNode};

use super::styles::extend_with_styles;
use super::{create_test_framework, NoEmitResult, NoEmitter, TestFramework};

/// The assembly of the view models and test controls.
pub const UNIT_TESTS_ASSEMBLY: &str = "FerroUI.Markup.Xaml.UnitTests";

/// The namespace of the view models and test controls.
pub const LOCAL_NAMESPACE: &str = "FerroUI.Markup.Xaml.UnitTests.MarkupExtensions";

/// The namespace declarations of a test document: the framework namespace as the default
/// one, the XAML language namespace as `x` and [`LOCAL_NAMESPACE`] as `local`.
pub fn xmlns() -> String {
    format!(
        "xmlns='{FERRO_XML_NAMESPACE}' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:local='clr-namespace:{LOCAL_NAMESPACE};assembly={UNIT_TESTS_ASSEMBLY}'"
    )
}

/// [`create_test_framework`] plus the types and members listed in the module documentation.
///
/// # Panics
/// Panics when the fixture cannot be extended (a test set-up error).
pub fn create_bindings_test_framework() -> TestFramework {
    let fw = create_test_framework();
    extend_with_styles(&fw);
    extend_with_bindings(&fw);
    fw
}

fn has_type(fw: &TestFramework, full_name: &str) -> bool {
    fw.as_type_system().find_type(full_name).is_some()
}

fn has_property(type_: &FakeType, name: &str) -> bool {
    type_.as_type().properties().iter().any(|p| p.name() == name)
}

fn has_method(type_: &FakeType, name: &str) -> bool {
    type_.as_type().methods().iter().any(|m| m.name() == name)
}

fn generic(
    fw: &TestFramework,
    definition: &str,
    arguments: &[Rc<dyn IXamlType>],
) -> Rc<dyn IXamlType> {
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

fn attr(fw: &TestFramework, full_name: &str) -> Rc<FakeCustomAttribute> {
    FakeCustomAttribute::new(fw.t(full_name), vec![])
}

fn default_member(fw: &TestFramework, name: &str) -> Rc<FakeCustomAttribute> {
    FakeCustomAttribute::new(
        fw.t("System.Reflection.DefaultMemberAttribute"),
        vec![XamlValue::String(name.to_string())],
    )
}

/// Adds an attribute to an already declared property.
fn add_property_attribute(type_: &FakeType, name: &str, attribute: Rc<FakeCustomAttribute>) {
    let properties = type_.as_type().properties();
    let Some(property) = properties.iter().find(|p| p.name() == name) else {
        panic!("{} has no property {name}", type_.as_type().full_name());
    };
    match property.as_any().downcast_ref::<FakeProperty>() {
        Some(property) => {
            property.add_attribute(attribute);
        }
        None => panic!("{name} is not a fake property"),
    }
}

/// A styled property: CLR property `name` plus static field `<name>Property` of type
/// `StyledProperty<value_type>`.
fn styled(fw: &TestFramework, owner: &FakeType, name: &str, value_type: Rc<dyn IXamlType>) {
    if has_property(owner, name) {
        return;
    }
    owner.add_property(name, value_type.clone());
    owner.add_field(
        &format!("{name}Property"),
        generic(fw, "FerroUI.StyledProperty`1", &[value_type]),
        true,
        None,
    );
}

/// Adds the binding related members to an existing fake framework. Members that are already
/// declared are left alone.
pub fn extend_with_bindings(fw: &TestFramework) {
    if has_type(fw, &format!("{LOCAL_NAMESPACE}.TestDataContext")) {
        return;
    }

    let core = fw.type_system.core();
    let void = fw.t("System.Void");
    let object = fw.t("System.Object");
    let string = fw.t("System.String");
    let boolean = fw.t("System.Boolean");
    let int32 = fw.t("System.Int32");
    let type_ = fw.t("System.Type");
    let service_provider = fw.t("System.IServiceProvider");

    // The base class library.
    if !has_type(fw, "System.Reflection.DefaultMemberAttribute") {
        core.define_class("System.Reflection", "DefaultMemberAttribute")
            .set_base_type(fw.t("System.Attribute"));
    }
    if !has_type(fw, "System.ComponentModel.INotifyPropertyChanged") {
        core.define_interface("System.ComponentModel", "INotifyPropertyChanged");
    }
    if !has_type(fw, "System.Collections.Specialized.INotifyCollectionChanged") {
        core.define_interface("System.Collections.Specialized", "INotifyCollectionChanged");
    }
    if !has_type(fw, "System.Decimal") {
        core.define_struct("System", "Decimal");
    }
    let string_type = fw.fake_type("System.String");
    if !has_property(&string_type, "Length") {
        string_type.add_property_with("Length", int32.clone(), true, false, false);
    }
    let list = fw.fake_type("System.Collections.Generic.List`1");
    if !has_property(&list, "Item") {
        list.add_indexer("Item", list.generic_parameter(0), vec![int32.clone()], true);
        list.add_attribute(default_member(fw, "Item"));
    }
    if !has_type(fw, "System.Collections.ObjectModel.ObservableCollection`1") {
        let collection = core.define_generic_class(
            "System.Collections.ObjectModel",
            "ObservableCollection`1",
            &["T"],
        );
        collection.add_constructor(vec![]);
        let item = collection.generic_parameter(0);
        collection.add_interface(generic(
            fw,
            "System.Collections.Generic.IList`1",
            std::slice::from_ref(&item),
        ));
        collection.add_interface(fw.t("System.Collections.IList"));
        collection.add_interface(fw.t("System.Collections.Specialized.INotifyCollectionChanged"));
        collection.add_interface(fw.t("System.ComponentModel.INotifyPropertyChanged"));
        collection.add_indexer("Item", item.clone(), vec![int32.clone()], true);
        collection.add_method("Add", void.clone(), vec![item], false);
        collection.add_attribute(default_member(fw, "Item"));
    }

    // The framework: names, visuals, presenters.
    if !has_type(fw, "FerroUI.INamed") {
        fw.base
            .define_interface("FerroUI", "INamed")
            .add_property_with("Name", string.clone(), true, false, false);
    }
    let styled_element = fw.fake_type("FerroUI.StyledElement");
    styled_element.add_interface(fw.t("FerroUI.INamed"));
    if !has_property(&styled_element, "Styles") {
        styled_element.add_property_with("Styles", fw.t("FerroUI.Styling.Styles"), true, false, false);
    }
    let resource_dictionary_interface = fw.fake_type("FerroUI.Controls.IResourceDictionary");
    if !has_method(&resource_dictionary_interface, "Add") {
        // `IResourceDictionary : IDictionary<object, object?>`
        resource_dictionary_interface.add_method(
            "Add",
            void.clone(),
            vec![object.clone(), object.clone()],
            false,
        );
    }
    styled(fw, &fw.fake_type("FerroUI.Visual"), "IsVisible", boolean.clone());
    styled(
        fw,
        &fw.fake_type("FerroUI.Input.InputElement"),
        "Focusable",
        boolean.clone(),
    );
    if !has_type(fw, "FerroUI.Controls.Presenters.ContentPresenter") {
        let presenter = fw
            .controls
            .define_class("FerroUI.Controls.Presenters", "ContentPresenter");
        presenter.set_base_type(fw.t("FerroUI.Controls.Control"));
        presenter.add_constructor(vec![]);
        styled(fw, &presenter, "Content", object.clone());
    }
    let items_control = fw.fake_type("FerroUI.Controls.ItemsControl");
    let inherit_from_items = fw.t("FerroUI.Metadata.InheritDataTypeFromItemsAttribute");
    let item_template_is_marked = items_control
        .as_type()
        .properties()
        .iter()
        .find(|p| p.name() == "ItemTemplate")
        .is_some_and(|p| {
            p.custom_attributes()
                .iter()
                .any(|a| a.type_().equals(&*inherit_from_items))
        });
    if !item_template_is_marked {
        add_property_attribute(
            &items_control,
            "ItemTemplate",
            FakeCustomAttribute::new(
                inherit_from_items.clone(),
                vec![XamlValue::String("ItemsSource".to_string())],
            ),
        );
    }

    // The binding classes.
    if !has_type(fw, "FerroUI.Data.BindingMode") {
        fw.base.define_enum(
            "FerroUI.Data",
            "BindingMode",
            &[
                ("Default", 0),
                ("OneWay", 1),
                ("TwoWay", 2),
                ("OneTime", 3),
                ("OneWayToSource", 4),
            ],
        );
    }
    if !has_type(fw, "FerroUI.Data.RelativeSourceMode") {
        fw.base.define_enum(
            "FerroUI.Data",
            "RelativeSourceMode",
            &[
                ("DataContext", 0),
                ("TemplatedParent", 1),
                ("Self", 2),
                ("FindAncestor", 3),
            ],
        );
    }
    if !has_type(fw, "FerroUI.Data.TreeType") {
        fw.base
            .define_enum("FerroUI.Data", "TreeType", &[("Visual", 0), ("Logical", 1)]);
    }
    let binding_mode = fw.t("FerroUI.Data.BindingMode");
    let relative_source_mode = fw.t("FerroUI.Data.RelativeSourceMode");
    let tree_type = fw.t("FerroUI.Data.TreeType");
    let compiled_binding_path = fw.t("FerroUI.Data.CompiledBindingPath");
    let binding_base = fw.t("FerroUI.Data.BindingBase");

    let compiled_binding = fw.fake_type("FerroUI.Data.CompiledBinding");
    if !has_property(&compiled_binding, "Path") {
        compiled_binding.add_constructor(vec![compiled_binding_path.clone()]);
        compiled_binding.add_property("Path", compiled_binding_path.clone());
        compiled_binding.add_property("Source", object.clone());
        compiled_binding.add_property("Mode", binding_mode.clone());
        compiled_binding.add_property("StringFormat", string.clone());
        compiled_binding.add_property("FallbackValue", object.clone());
        compiled_binding.add_property("ConverterParameter", object.clone());
    }

    let compiled_binding_extension =
        fw.fake_type("FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindingExtension");
    if !has_property(&compiled_binding_extension, "DataType") {
        compiled_binding_extension.set_base_type(compiled_binding.as_type());
        compiled_binding_extension.add_constructor(vec![compiled_binding_path]);
        compiled_binding_extension.add_method(
            "ProvideValue",
            compiled_binding.as_type(),
            vec![service_provider.clone()],
            false,
        );
        compiled_binding_extension.add_property("DataType", type_.clone());
    }

    let reflection_binding_extension =
        fw.fake_type("FerroUI.Markup.Xaml.MarkupExtensions.ReflectionBindingExtension");
    if !has_property(&reflection_binding_extension, "Path") {
        reflection_binding_extension.add_constructor(vec![string.clone()]);
        reflection_binding_extension.add_property("Path", string.clone());
        reflection_binding_extension.add_method(
            "ProvideValue",
            binding_base.clone(),
            vec![service_provider.clone()],
            false,
        );
    }

    let relative_source = fw.fake_type("FerroUI.Data.RelativeSource");
    if !has_property(&relative_source, "Mode") {
        relative_source.add_constructor(vec![relative_source_mode.clone()]);
        relative_source.add_property("Mode", relative_source_mode.clone());
        relative_source.add_property("AncestorType", type_.clone());
        relative_source.add_property("AncestorLevel", int32.clone());
        relative_source.add_property("Tree", tree_type.clone());
    }
    if !has_type(
        fw,
        "FerroUI.Markup.Xaml.MarkupExtensions.RelativeSourceExtension",
    ) {
        let extension = fw.markup_xaml.define_class(
            "FerroUI.Markup.Xaml.MarkupExtensions",
            "RelativeSourceExtension",
        );
        extension.add_constructor(vec![]);
        extension.add_constructor(vec![relative_source_mode.clone()]);
        extension.add_method(
            "ProvideValue",
            relative_source.as_type(),
            vec![service_provider.clone()],
            false,
        );
        extension.add_property("Mode", relative_source_mode);
        extension.add_property("AncestorType", type_.clone());
        extension.add_property("AncestorLevel", int32.clone());
        extension.add_property("Tree", tree_type);
    }

    let static_resource =
        fw.fake_type("FerroUI.Markup.Xaml.MarkupExtensions.StaticResourceExtension");
    if !has_property(&static_resource, "ResourceKey") {
        static_resource.add_constructor(vec![object.clone()]);
        static_resource.add_property("ResourceKey", object.clone());
    }

    let multi_binding = fw.fake_type("FerroUI.Data.MultiBinding");
    if !has_property(&multi_binding, "Bindings") {
        multi_binding
            .add_property_with(
                "Bindings",
                generic(
                    fw,
                    "System.Collections.Generic.IList`1",
                    std::slice::from_ref(&binding_base),
                ),
                true,
                false,
                false,
            )
            .add_attribute(attr(fw, "FerroUI.Metadata.ContentAttribute"));
    }

    define_view_models(fw);
}

/// The view models and controls of the upstream compiled binding tests.
fn define_view_models(fw: &TestFramework) {
    let asm = fw.type_system.define_assembly(UNIT_TESTS_ASSEMBLY);
    let ns = LOCAL_NAMESPACE;
    let void = fw.t("System.Void");
    let object = fw.t("System.Object");
    let string = fw.t("System.String");
    let boolean = fw.t("System.Boolean");
    let int32 = fw.t("System.Int32");
    let type_ = fw.t("System.Type");
    let inpc = fw.t("System.ComponentModel.INotifyPropertyChanged");

    let class = |name: &str, base: Option<Rc<dyn IXamlType>>| {
        let type_ = asm.define_class(ns, name);
        if let Some(base) = base {
            type_.set_base_type(base);
        }
        type_.add_constructor(vec![]);
        type_
    };

    // Interfaces
    let non_integer_indexer = asm.define_interface(ns, "INonIntegerIndexer");
    non_integer_indexer.add_indexer("Item", string.clone(), vec![string.clone()], true);
    non_integer_indexer.add_attribute(default_member(fw, "Item"));
    let non_integer_indexer_derived = asm.define_interface(ns, "INonIntegerIndexerDerived");
    non_integer_indexer_derived.add_interface(non_integer_indexer.as_type());
    let has_property_interface = asm.define_interface(ns, "IHasProperty");
    has_property_interface.add_property("StringProperty", string.clone());
    let has_property_derived = asm.define_interface(ns, "IHasPropertyDerived");
    has_property_derived.add_interface(has_property_interface.as_type());
    let has_explicit_property = asm.define_interface(ns, "IHasExplicitProperty");
    has_explicit_property.add_property_with("ExplicitProperty", string.clone(), true, false, false);

    // Plain data
    let test_data = class("TestData", None);
    test_data.add_property("StringProperty", string.clone());
    class("OuterClass", None);
    class("OuterClass+NestedClass", None).add_property("NestedProperty", string.clone());
    let base_class = class("TestDataContextBaseClass", None);

    let items_collection = class("TestItemsCollectionDataContext", Some(base_class.as_type()));
    items_collection.add_property_with(
        "Items",
        generic(
            fw,
            "System.Collections.ObjectModel.ObservableCollection`1",
            &[test_data.as_type()],
        ),
        true,
        false,
        false,
    );

    // TestDataContext and its nested types
    let indexer = class("TestDataContext+NonIntegerIndexer", None);
    indexer.add_interface(inpc.clone());
    indexer.add_interface(non_integer_indexer_derived.as_type());
    indexer.add_indexer("Item", string.clone(), vec![string.clone()], true);
    indexer.add_attribute(default_member(fw, "Item"));

    let nested_generic = asm.define_generic_class(ns, "TestDataContext+NestedGeneric`1", &["T"]);
    nested_generic.add_constructor(vec![]);
    nested_generic.add_property("Value", nested_generic.generic_parameter(0));

    let collection_view = asm.define_generic_class(ns, "ListItemCollectionView`1", &["T"]);
    collection_view.add_constructor(vec![]);
    collection_view.set_base_type(generic(
        fw,
        "System.Collections.Generic.List`1",
        &[collection_view.generic_parameter(0)],
    ));
    collection_view.add_property("CurrentItem", collection_view.generic_parameter(0));

    let data_context = class("TestDataContext", Some(base_class.as_type()));
    data_context.add_interface(has_property_derived.as_type());
    data_context.add_interface(has_explicit_property.as_type());
    data_context.add_property("BoolProperty", boolean.clone());
    data_context.add_property("StringProperty", string.clone());
    data_context.add_property(
        "TaskProperty",
        generic(
            fw,
            "System.Threading.Tasks.Task`1",
            std::slice::from_ref(&string),
        ),
    );
    data_context.add_property(
        "ObservableProperty",
        generic(fw, "System.IObservable`1", std::slice::from_ref(&string)),
    );
    data_context.add_property(
        "ObservableCollectionProperty",
        generic(
            fw,
            "System.Collections.ObjectModel.ObservableCollection`1",
            std::slice::from_ref(&string),
        ),
    );
    data_context.add_property("ArrayProperty", array(&string));
    data_context.add_property("ObjectsArrayProperty", array(&object));
    data_context.add_property(
        "ListProperty",
        generic(
            fw,
            "System.Collections.Generic.List`1",
            std::slice::from_ref(&string),
        ),
    );
    data_context.add_property("NonIntegerIndexerProperty", indexer.as_type());
    data_context.add_property_with(
        "NonIntegerIndexerInterfaceProperty",
        non_integer_indexer_derived.as_type(),
        true,
        false,
        false,
    );
    data_context.add_property(
        "NestedGenericString",
        generic(
            fw,
            &format!("{ns}.TestDataContext+NestedGeneric`1"),
            std::slice::from_ref(&string),
        ),
    );
    data_context.add_property_with("ExplicitProperty", string.clone(), true, false, false);
    data_context.add_property_with("StaticProperty", string.clone(), true, false, true);
    data_context.add_property_with(
        "GenericProperty",
        generic(
            fw,
            &format!("{ns}.ListItemCollectionView`1"),
            std::slice::from_ref(&int32),
        ),
        true,
        false,
        false,
    );
    data_context.add_property("DecimalValue", fw.t("System.Decimal"));

    // Methods
    let method_data_context = class("MethodDataContext", None);
    method_data_context.add_method("Action", void.clone(), vec![], false);
    method_data_context.add_method("Func", object.clone(), vec![], false);
    method_data_context.add_method("Func2", object.clone(), vec![object.clone()], false);
    method_data_context.add_method(
        "CustomDelegateTypeVoid",
        void.clone(),
        vec![object.clone()],
        false,
    );
    method_data_context.add_method(
        "CustomDelegateTypeInt",
        object.clone(),
        vec![object.clone()],
        false,
    );
    method_data_context.add_method(
        "ManyParameters",
        void.clone(),
        (0..17).map(|_| int32.clone()).collect(),
        false,
    );

    let command_base = class("MethodAsCommandDataContextBase", None);
    command_base.add_method("VirtualObjectMethod", void.clone(), vec![object.clone()], false);
    command_base.add_method("VirtualInt32Method", void.clone(), vec![int32.clone()], false);
    command_base.add_method("VirtualStringMethod", void.clone(), vec![string.clone()], false);
    command_base.add_method("MethodWithNewSlot", void.clone(), vec![int32.clone()], false);

    let command = class("MethodAsCommandDataContext", Some(command_base.as_type()));
    command.add_interface(inpc);
    command.add_method("Method", void.clone(), vec![], false);
    command.add_method("ObjectMethod", void.clone(), vec![object.clone()], false);
    command.add_method("Int32Method", void.clone(), vec![int32.clone()], false);
    command.add_method("StringMethod", void.clone(), vec![string.clone()], false);
    for name in ["MethodWithOverloads", "MethodWithOverloads2"] {
        command.add_method(name, void.clone(), vec![], false);
        command.add_method(name, void.clone(), vec![int32.clone()], false);
        command.add_method(name, void.clone(), vec![string.clone()], false);
    }
    command.add_method("MethodWithOverloads", void.clone(), vec![object.clone()], false);
    command.add_method("MethodWithOverloads3", void.clone(), vec![], false);
    for name in ["MethodWithOverloads3", "MethodWithOverloads4"] {
        command.add_method(name, void.clone(), vec![int32.clone(), int32.clone()], false);
        command.add_method(name, void.clone(), vec![string.clone(), string.clone()], false);
    }
    command.add_method("VirtualObjectMethod", void.clone(), vec![object.clone()], false);
    command.add_method("VirtualInt32Method", void.clone(), vec![int32.clone()], false);
    command.add_method("VirtualStringMethod", void.clone(), vec![string.clone()], false);
    command.add_method("MethodWithNewSlot", void.clone(), vec![int32.clone()], false);
    command.add_property_with("Value", string.clone(), true, false, false);
    command.add_property("Parameter", object.clone());
    command.add_method("Do", void.clone(), vec![object.clone()], false);
    command
        .add_method("CanDo", boolean.clone(), vec![object.clone()], false)
        .add_attribute(FakeCustomAttribute::new(
            fw.t("FerroUI.Metadata.DependsOnAttribute"),
            vec![XamlValue::String("Parameter".to_string())],
        ));

    // Templates and controls
    let custom_data_template = class("CustomDataTemplate", None);
    custom_data_template.add_interface(fw.t("FerroUI.Controls.Templates.IDataTemplate"));
    custom_data_template
        .add_property("FancyDataType", type_.clone())
        .add_attribute(attr(fw, "FerroUI.Metadata.DataTypeAttribute"));
    custom_data_template
        .add_property("Content", object.clone())
        .add_attribute(attr(fw, "FerroUI.Metadata.ContentAttribute"))
        .add_attribute(attr(fw, "FerroUI.Metadata.TemplateContentAttribute"));
    class("CustomDataTemplateInherit", Some(custom_data_template.as_type()));

    let binding_base = fw.t("FerroUI.Data.BindingBase");
    let control = fw.t("FerroUI.Controls.Control");
    let assign_binding_control = class("AssignBindingControl", Some(control.clone()));
    assign_binding_control
        .add_property("X", binding_base.clone())
        .add_attribute(attr(fw, "FerroUI.Data.AssignBindingAttribute"));

    let data_grid = class("DataGridLikeControl", Some(control));
    let data_grid_column = class("DataGridLikeColumn", None);
    let enumerable = fw.t("System.Collections.IEnumerable");
    data_grid.add_property("Items", enumerable.clone());
    data_grid.add_field(
        "ItemsProperty",
        generic(
            fw,
            "FerroUI.DirectProperty`2",
            &[data_grid.as_type(), enumerable],
        ),
        true,
        None,
    );
    data_grid
        .add_property_with(
            "Columns",
            generic(
                fw,
                "FerroUI.Collections.FerroList`1",
                &[data_grid_column.as_type()],
            ),
            true,
            false,
            false,
        )
        .add_attribute(attr(fw, "FerroUI.Metadata.ContentAttribute"));
    let inherit_from_items = || {
        FakeCustomAttribute::with_properties(
            fw.t("FerroUI.Metadata.InheritDataTypeFromItemsAttribute"),
            vec![XamlValue::String("Items".to_string())],
            vec![("AncestorType", XamlValue::Type(data_grid.as_type()))],
        )
    };
    data_grid_column
        .add_property("Binding", binding_base)
        .add_attribute(attr(fw, "FerroUI.Data.AssignBindingAttribute"))
        .add_attribute(inherit_from_items());
    data_grid_column
        .add_property("Template", fw.t("FerroUI.Controls.Templates.IDataTemplate"))
        .add_attribute(inherit_from_items());
    class("DataGridLikeControlInheritor", Some(data_grid.as_type()));
}

/// What [`transform_bindings`] runs.
#[derive(Clone, Copy, Debug, Default)]
pub struct BindingsPipelineOptions {
    /// `FerroBindingExtensionTransformer::compile_bindings_by_default`.
    pub compile_bindings_by_default: bool,
    /// Also run `FerroXamlIlCompiledBindingsMetadataRemover` at the end.
    pub remove_metadata: bool,
    /// Stop before `FerroXamlIlDataContextTypeTransformer` and
    /// `FerroXamlIlBindingPathTransformer` (to look at the parsed paths).
    pub stop_before_data_context: bool,
}

/// The index of the transformer with the given type name.
fn index_of(transformers: &[Box<dyn IXamlAstTransformer>], name: &str) -> usize {
    match transformers.iter().position(|t| t.transformer_name() == name) {
        Some(index) => index,
        None => panic!("The pipeline has no {name}"),
    }
}

fn insert_all(
    transformers: &mut Vec<Box<dyn IXamlAstTransformer>>,
    index: usize,
    inserted: Vec<Box<dyn IXamlAstTransformer>>,
) {
    for (offset, transformer) in inserted.into_iter().enumerate() {
        transformers.insert(index + offset, transformer);
    }
}

/// The transformers of the pipeline, in order: the `xamlx` defaults with the framework's
/// transformers inserted where the upstream compiler inserts them.
pub fn bindings_pipeline(options: BindingsPipelineOptions) -> Vec<Box<dyn IXamlAstTransformer>> {
    // Only used to obtain the default transformer list.
    let fw = create_test_framework();
    let compiler: XamlCompiler<NoEmitter, NoEmitResult> = XamlCompiler::new(
        fw.configuration.clone(),
        Rc::new(XamlLanguageEmitMappings::new()),
        true,
    );
    let mut transformers = compiler.transformers;

    // Before everything else
    let binding_transformer = FerroBindingExtensionTransformer::new();
    binding_transformer
        .compile_bindings_by_default
        .set(options.compile_bindings_by_default);
    transformers.insert(0, Box::new(XNameTransformer));
    transformers.insert(1, Box::new(binding_transformer));

    // Targeted
    let index = index_of(&transformers, "PropertyReferenceResolver");
    insert_all(
        &mut transformers,
        index,
        vec![Box::new(FerroXamlIlTransformSyntheticCompiledBindingMembers)],
    );

    let index = index_of(&transformers, "PropertyReferenceResolver") + 1;
    insert_all(
        &mut transformers,
        index,
        vec![Box::new(FerroXamlIlFerroPropertyResolver)],
    );

    let index = index_of(&transformers, "ContentConvertTransformer");
    insert_all(
        &mut transformers,
        index,
        vec![
            Box::new(FerroXamlIlSelectorTransformer),
            Box::new(FerroXamlIlControlTemplateTargetTypeMetadataTransformer),
            Box::new(FerroXamlIlBindingPathParser),
            Box::new(FerroXamlIlSetterTargetTypeMetadataTransformer),
            Box::new(FerroXamlIlSetterTransformer),
        ],
    );

    let index = index_of(&transformers, "TypeReferenceResolver") + 1;
    insert_all(&mut transformers, index, vec![Box::new(XDataTypeTransformer)]);

    // After everything else
    transformers.push(Box::new(AddNameScopeRegistration));
    if !options.stop_before_data_context {
        transformers.push(Box::new(FerroXamlIlDataContextTypeTransformer));
        transformers.push(Box::new(FerroXamlIlBindingPathTransformer));
        if options.remove_metadata {
            transformers.push(Box::new(FerroXamlIlCompiledBindingsMetadataRemover));
        }
    }

    transformers
}

/// Parses `xaml` and runs [`bindings_pipeline`] over it the way `XamlCompiler::transform` does.
/// Transformation errors are reported as diagnostics (`fw.reported_diagnostics()`), parse
/// errors are returned.
pub fn transform_bindings(
    fw: &TestFramework,
    xaml: &str,
    options: BindingsPipelineOptions,
) -> XamlResult<XamlDocument> {
    let mut doc = XDocumentXamlParser::parse(xaml, None)?;
    let context = xamlx::transform::AstTransformationContext::new(
        fw.configuration.clone(),
        Some(&doc),
    );

    let mut root: Rc<dyn IXamlAstNode> = doc.root()?;
    let root_object = root.cast::<XamlAstObjectNode>().ok_or_else(|| {
        XamlError::invalid_cast("The root of the document is not an object node")
    })?;
    context.set_root_object(XamlRootObjectNode::new(&root_object));
    for transformer in bindings_pipeline(options) {
        context.visit_children(&*context.root_object()?, &*transformer)?;
        root = context.visit(&root, &*transformer)?;
    }

    doc.set_root(root);
    Ok(doc)
}

/// The root of the document transformed with the default options.
///
/// # Panics
/// Panics when the document cannot be parsed or a fatal error is raised.
pub fn transform_root(fw: &TestFramework, xaml: &str) -> Rc<dyn IXamlAstNode> {
    transform_root_with(fw, xaml, BindingsPipelineOptions::default())
}

/// The root of the document transformed with `options`.
///
/// # Panics
/// Panics when the document cannot be parsed or a fatal error is raised.
pub fn transform_root_with(
    fw: &TestFramework,
    xaml: &str,
    options: BindingsPipelineOptions,
) -> Rc<dyn IXamlAstNode> {
    match transform_bindings(fw, xaml, options).and_then(|doc| doc.root()) {
        Ok(root) => root,
        Err(e) => panic!("Unable to transform the document: {e}"),
    }
}

/// The errors reported so far, as `(code, title)` pairs.
pub fn reported_errors(fw: &TestFramework) -> Vec<(String, String)> {
    fw.reported_diagnostics()
        .into_iter()
        .map(|d: XamlDiagnostic| (d.code, d.title))
        .collect()
}

struct NodeCollector<K: ?Sized> {
    found: Vec<Rc<K>>,
}

impl<K: ?Sized + XamlAstCast> IXamlAstVisitor for NodeCollector<K> {
    fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(found) = node.cast::<K>() {
            self.found.push(found);
        }
        Ok(node)
    }
    fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
    fn pop(&mut self) {}
}

/// Every node of kind `K` below (and including) `root`, in document order.
///
/// # Panics
/// Panics when a node fails to visit its children (a test set-up error).
pub fn find_binding_nodes<K: ?Sized + XamlAstCast>(root: &Rc<dyn IXamlAstNode>) -> Vec<Rc<K>> {
    let mut collector = NodeCollector::<K> { found: Vec::new() };
    if let Err(e) = visit_node(root, &mut collector) {
        panic!("Unable to visit the tree: {e}");
    }
    collector.found
}

fn optional_type(type_: &Option<Rc<dyn IXamlType>>) -> String {
    match type_ {
        Some(type_) => type_.full_name(),
        None => "-".to_string(),
    }
}

fn parameter_list(parameters: &[Rc<dyn IXamlType>]) -> String {
    let names: Vec<String> = parameters.iter().map(|p| p.name()).collect();
    names.join(",")
}

fn null_conditional(accepts_null: bool) -> &'static str {
    if accepts_null {
        "?"
    } else {
        ""
    }
}

/// A one-line description of a resolved path element.
pub fn describe_element(element: &XamlIlBindingPathElementNode) -> String {
    match element {
        XamlIlBindingPathElementNode::Not(_) => "Not".to_string(),
        XamlIlBindingPathElementNode::StreamObservable(e) => {
            format!("StreamObservable<{}>", e.type_.full_name())
        }
        XamlIlBindingPathElementNode::StreamTask(e) => {
            format!("StreamTask<{}>", e.type_.full_name())
        }
        XamlIlBindingPathElementNode::SelfElement(e) => format!("Self({})", e.type_.full_name()),
        XamlIlBindingPathElementNode::FindAncestor(e) => format!(
            "Ancestor({};{};dc={})",
            e.type_.full_name(),
            e.level,
            optional_type(&e.data_context_type)
        ),
        XamlIlBindingPathElementNode::FindVisualAncestor(e) => {
            format!("VisualAncestor({};{})", e.type_.full_name(), e.level)
        }
        XamlIlBindingPathElementNode::ElementName(e) => format!(
            "ElementName({}:{};dc={})",
            e.name,
            e.type_.full_name(),
            optional_type(&e.data_context_type)
        ),
        XamlIlBindingPathElementNode::TemplatedParent(e) => {
            format!("TemplatedParent({})", e.type_.full_name())
        }
        XamlIlBindingPathElementNode::FerroProperty(e) => format!(
            "FerroProperty({}.{}:{}){}",
            e.field.declaring_type().name(),
            e.field.name(),
            e.type_.full_name(),
            null_conditional(e.accepts_null)
        ),
        XamlIlBindingPathElementNode::ClrProperty(e) => format!(
            "Property({}.{}:{}){}",
            e.property.declaring_type().name(),
            e.property.name(),
            e.type_().full_name(),
            null_conditional(e.accepts_null)
        ),
        XamlIlBindingPathElementNode::ClrMethod(e) => format!(
            "Method({}.{}({}):{}){}",
            e.method.declaring_type().name(),
            e.method.name(),
            parameter_list(&e.method.parameters()),
            e.type_.full_name(),
            null_conditional(e.accepts_null)
        ),
        XamlIlBindingPathElementNode::ClrMethodAsCommand(e) => format!(
            "Command({}.{}({});can={};depends=[{}])",
            e.execute_method.declaring_type().name(),
            e.execute_method.name(),
            parameter_list(&e.execute_method.parameters()),
            e.can_execute_method
                .as_ref()
                .map(|m| m.name())
                .unwrap_or_else(|| "-".to_string()),
            e.depends_on_properties.join(",")
        ),
        XamlIlBindingPathElementNode::ClrIndexer(e) => format!(
            "Indexer({}.{}[{}]:{};notifying={})",
            e.property.declaring_type().name(),
            e.property.name(),
            e.indexer_key,
            e.type_().full_name(),
            e.is_notifying_collection
        ),
        XamlIlBindingPathElementNode::ArrayIndexer(e) => {
            let indices: Vec<String> = e.values.iter().map(|v| v.to_string()).collect();
            format!("ArrayElement([{}]:{})", indices.join(","), e.type_().full_name())
        }
        XamlIlBindingPathElementNode::TypeCast(e) => format!("TypeCast({})", e.type_.full_name()),
    }
}

/// The transform elements followed by the elements of a resolved path, each described with
/// [`describe_element`] and joined with `" > "`.
pub fn describe_path(path: &XamlIlBindingPathNode) -> String {
    let mut parts: Vec<String> = path
        .transform_elements
        .borrow()
        .iter()
        .map(describe_element)
        .collect();
    parts.extend(path.elements.borrow().iter().map(describe_element));
    parts.join(" > ")
}

/// The descriptions ([`describe_path`]) of every resolved binding path of the document, in
/// document order.
pub fn resolved_paths(root: &Rc<dyn IXamlAstNode>) -> Vec<String> {
    find_binding_nodes::<XamlIlBindingPathNode>(root)
        .iter()
        .map(|path| describe_path(path))
        .collect()
}

/// The full name of the CLR type of a value node.
///
/// # Panics
/// Panics when the node's type is not resolved.
pub fn value_type_name(node: &Rc<dyn xamlx::ast::IXamlAstValueNode>) -> String {
    match node.type_().get_clr_type() {
        Ok(type_) => type_.full_name(),
        Err(e) => panic!("The node has no CLR type: {e}"),
    }
}

/// A test document: a `Window` with the namespaces of [`xmlns`], the given attributes and
/// content.
pub fn window(attributes: &str, content: &str) -> String {
    format!("<Window {} {attributes}>{content}</Window>", xmlns())
}

/// Transforms [`window`]`(attributes, content)` on a fresh framework and returns the
/// framework, the descriptions of the resolved binding paths and the reported errors.
///
/// # Panics
/// Panics when the document cannot be parsed or a fatal error is raised.
pub fn resolve(attributes: &str, content: &str) -> (TestFramework, Vec<String>, Vec<(String, String)>) {
    let fw = create_bindings_test_framework();
    let root = transform_root(&fw, &window(attributes, content));
    let paths = resolved_paths(&root);
    let errors = reported_errors(&fw);
    (fw, paths, errors)
}
