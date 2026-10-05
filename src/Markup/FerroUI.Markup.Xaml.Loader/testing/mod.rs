//! The framework as an in-memory type system, for unit tests of the compiler extensions.
//!
//! [`create_test_framework`] declares, on a `xamlx::testing::FakeTypeSystem`, the assemblies
//! `FerroUI.Base`, `FerroUI.Controls`, `FerroUI.Markup` and `FerroUI.Markup.Xaml` with every
//! type and member the well-known types and the language definition look up (same namespaces
//! and signatures as the real framework types), the `XmlnsDefinition` attributes that map
//! [`FERRO_XML_NAMESPACE`] to the framework namespaces, and a small set of sample controls
//! (`Control`, `Panel`, `Decorator`, `Border`, `TextBlock`, `ContentControl`, `Button`, `Grid`,
//! `Style`, `Setter`, `ResourceDictionary`, `ControlTemplate`, ...).
//!
//! A registered property `Foo` of type `T` is declared the way the transformers see one: a CLR
//! property `Foo` plus a public static field `FooProperty` of type `StyledProperty<T>` (or
//! `AttachedProperty<T>` with static `GetFoo`/`SetFoo` accessors, or `DirectProperty<TOwner, T>`).

use std::cell::RefCell;
use std::rc::Rc;

use xamlx::ast::XamlDocument;
use xamlx::compiler::XamlCompiler;
use xamlx::diagnostics::XamlDiagnostic;
use xamlx::emit::{IXamlEmitResult, XamlLanguageEmitMappings};
use xamlx::exceptions::XamlResult;
use xamlx::parsers::XDocumentXamlParser;
use xamlx::testing::{FakeAssembly, FakeCustomAttribute, FakeProperty, FakeType, FakeTypeSystem};
use xamlx::transform::{AstTransformationContext, TransformerConfiguration, XamlDiagnosticsHandler};
use xamlx::type_system::{IXamlType, IXamlTypeSystem, XamlValue};
use xamlx::XamlNamespaces;

use crate::compiler_extensions::transformers::{
    FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions, FERRO_XML_NAMESPACE,
};
use crate::compiler_extensions::{
    FerroXamlDiagnosticCodes, FerroXamlIlCompilerConfiguration, FerroXamlIlLanguage,
    FerroXamlIlLanguageEmitMappings,
};

/// The fake framework and a compiler configuration over it.
pub struct TestFramework {
    pub type_system: Rc<FakeTypeSystem>,
    /// `FerroUI.Base`.
    pub base: Rc<FakeAssembly>,
    /// `FerroUI.Controls`.
    pub controls: Rc<FakeAssembly>,
    /// `FerroUI.Markup`.
    pub markup: Rc<FakeAssembly>,
    /// `FerroUI.Markup.Xaml`.
    pub markup_xaml: Rc<FakeAssembly>,
    /// The transformer configuration built from `FerroXamlIlLanguage::configure`, with
    /// `FerroXamlIlLanguage::custom_value_converter` and the framework's diagnostic codes.
    pub configuration: Rc<TransformerConfiguration>,
    /// The emit side of the language.
    pub emit_mappings: FerroXamlIlLanguageEmitMappings,
    /// The well-known types registered in `configuration`.
    pub types: Rc<FerroXamlIlWellKnownTypes>,
    /// Every diagnostic reported through `configuration`, in order. Errors are not fatal.
    pub diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>>,
}

impl TestFramework {
    /// The type with the given full name.
    ///
    /// # Panics
    /// Panics when the type is not declared.
    pub fn t(&self, full_name: &str) -> Rc<dyn IXamlType> {
        self.type_system.get(full_name)
    }

    /// The declaration of a non-generic type, to add members or attributes to it in a test.
    ///
    /// # Panics
    /// Panics when the type is not declared.
    pub fn fake_type(&self, full_name: &str) -> Rc<FakeType> {
        self.type_system.get_fake_type(full_name)
    }

    pub fn as_type_system(&self) -> Rc<dyn IXamlTypeSystem> {
        self.type_system.as_type_system()
    }

    /// A transformation context for a document whose default XML namespace is the framework's
    /// and whose `x` prefix is the XAML language namespace.
    pub fn create_context(&self) -> AstTransformationContext {
        let mut document = XamlDocument::new();
        document.document = Some("test.xaml".to_string());
        document
            .namespace_aliases
            .insert(String::new(), FERRO_XML_NAMESPACE.to_string());
        document
            .namespace_aliases
            .insert("x".to_string(), XamlNamespaces::XAML2006.to_string());
        AstTransformationContext::new(self.configuration.clone(), Some(&document))
    }

    /// The diagnostics reported so far.
    pub fn reported_diagnostics(&self) -> Vec<XamlDiagnostic> {
        self.diagnostics.borrow().clone()
    }

    /// Parses `xaml` and runs the default `xamlx` transformers over it with this configuration
    /// (none of the framework's own transformers).
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
}

/// The emitter back end handle of a compiler that only transforms.
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

/// Declares the fake framework and builds the compiler configuration over it.
///
/// # Panics
/// Panics when the fixture is inconsistent with the well-known types or the language
/// definition (a test set-up error).
pub fn create_test_framework() -> TestFramework {
    let ts = FakeTypeSystem::new();
    define_core_types(&ts);
    let base = ts.define_assembly("FerroUI.Base");
    define_base(&ts, &base);
    let controls = ts.define_assembly("FerroUI.Controls");
    define_controls(&ts, &controls);
    let markup = ts.define_assembly("FerroUI.Markup");
    define_markup(&ts, &markup);
    let markup_xaml = ts.define_assembly("FerroUI.Markup.Xaml");
    define_markup_xaml(&ts, &markup_xaml);
    define_xmlns_definitions(&ts, &base, &controls, &markup, &markup_xaml);

    let type_system = ts.as_type_system();
    let (language, emit_mappings) = match FerroXamlIlLanguage::configure(&type_system) {
        Ok(result) => result,
        Err(e) => panic!("The fake framework doesn't satisfy the language definition: {e}"),
    };

    let diagnostics: Rc<RefCell<Vec<XamlDiagnostic>>> = Rc::new(RefCell::new(Vec::new()));
    let collected = diagnostics.clone();
    let handler = XamlDiagnosticsHandler {
        code_mappings: Box::new(FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro),
        handle_diagnostic: Some(Box::new(move |diagnostic| {
            collected.borrow_mut().push(diagnostic.clone());
            diagnostic.severity
        })),
        ..XamlDiagnosticsHandler::default()
    };

    let configuration = match FerroXamlIlCompilerConfiguration::new(
        type_system.clone(),
        type_system.find_assembly("FerroUI.Controls"),
        language,
        None,
        Some(FerroXamlIlLanguage::value_converter()),
        None,
        Some(handler),
    ) {
        Ok(configuration) => configuration.as_transformer_configuration().clone(),
        Err(e) => panic!("The fake framework doesn't satisfy the well-known types: {e}"),
    };
    let types = configuration.get_ferro_types();

    TestFramework {
        type_system: ts,
        base,
        controls,
        markup,
        markup_xaml,
        configuration,
        emit_mappings,
        types,
        diagnostics,
    }
}

/// Declaration helpers bound to a type system and the assembly being declared.
struct Decl<'a> {
    ts: &'a Rc<FakeTypeSystem>,
    asm: &'a Rc<FakeAssembly>,
}

impl Decl<'_> {
    fn t(&self, full_name: &str) -> Rc<dyn IXamlType> {
        self.ts.get(full_name)
    }

    /// The declaration of an already declared type.
    fn fake(&self, full_name: &str) -> Rc<FakeType> {
        self.ts.get_fake_type(full_name)
    }

    fn generic(&self, definition: &str, arguments: &[Rc<dyn IXamlType>]) -> Rc<dyn IXamlType> {
        match self.t(definition).make_generic_type(arguments) {
            Ok(type_) => type_,
            Err(e) => panic!("Unable to construct {definition}: {e}"),
        }
    }

    fn class(&self, namespace: &str, name: &str, base: Option<&str>) -> Rc<FakeType> {
        let type_ = self.asm.define_class(namespace, name);
        if let Some(base) = base {
            type_.set_base_type(self.t(base));
        }
        type_
    }

    /// A class with a public parameterless constructor.
    fn constructable(&self, namespace: &str, name: &str, base: Option<&str>) -> Rc<FakeType> {
        let type_ = self.class(namespace, name, base);
        type_.add_constructor(vec![]);
        type_
    }

    fn generic_class(
        &self,
        namespace: &str,
        name: &str,
        parameters: &[&str],
        base: Option<Rc<dyn IXamlType>>,
    ) -> Rc<FakeType> {
        let type_ = self.asm.define_generic_class(namespace, name, parameters);
        if let Some(base) = base {
            type_.set_base_type(base);
        }
        type_
    }

    fn interface(&self, namespace: &str, name: &str) -> Rc<FakeType> {
        self.asm.define_interface(namespace, name)
    }

    fn structure(&self, namespace: &str, name: &str) -> Rc<FakeType> {
        self.asm.define_struct(namespace, name)
    }

    fn enumeration(&self, namespace: &str, name: &str, members: &[(&str, i32)]) -> Rc<FakeType> {
        self.asm.define_enum(namespace, name, members)
    }

    fn attribute(&self, namespace: &str, name: &str) -> Rc<FakeType> {
        self.class(namespace, name, Some("System.Attribute"))
    }

    fn attr(&self, full_name: &str) -> Rc<FakeCustomAttribute> {
        FakeCustomAttribute::new(self.t(full_name), vec![])
    }

    /// A styled property: CLR property `name` plus static field `<name>Property` of type
    /// `StyledProperty<value_type>`.
    fn styled(&self, owner: &FakeType, name: &str, value_type: Rc<dyn IXamlType>) -> Rc<FakeProperty> {
        let property = owner.add_property(name, value_type.clone());
        owner.add_field(
            &format!("{name}Property"),
            self.generic("FerroUI.StyledProperty`1", &[value_type]),
            true,
            None,
        );
        property
    }

    /// A direct property: CLR property `name` plus static field `<name>Property` of type
    /// `DirectProperty<owner, value_type>`.
    fn direct(&self, owner: &FakeType, name: &str, value_type: Rc<dyn IXamlType>) -> Rc<FakeProperty> {
        let property = owner.add_property(name, value_type.clone());
        owner.add_field(
            &format!("{name}Property"),
            self.generic("FerroUI.DirectProperty`2", &[owner.as_type(), value_type]),
            true,
            None,
        );
        property
    }

    /// An attached property: static `Get<name>`/`Set<name>` accessors plus static field
    /// `<name>Property` of type `AttachedProperty<value_type>`.
    fn attached(&self, owner: &FakeType, name: &str, host: &str, value_type: Rc<dyn IXamlType>) {
        let host = self.t(host);
        owner.add_method(
            &format!("Get{name}"),
            value_type.clone(),
            vec![host.clone()],
            true,
        );
        owner.add_method(
            &format!("Set{name}"),
            self.t("System.Void"),
            vec![host, value_type.clone()],
            true,
        );
        owner.add_field(
            &format!("{name}Property"),
            self.generic("FerroUI.AttachedProperty`1", &[value_type]),
            true,
            None,
        );
    }
}

/// Types of the base class library the framework refers to and the fake core assembly lacks.
fn define_core_types(ts: &Rc<FakeTypeSystem>) {
    let core = ts.core();
    let d = Decl { ts, asm: &core };
    let void = d.t("System.Void");
    let object = d.t("System.Object");
    let string = d.t("System.String");

    let time_span = d.structure("System", "TimeSpan");
    time_span.add_method("FromTicks", time_span.as_type(), vec![d.t("System.Int64")], true);

    let uri_kind = d.enumeration(
        "System",
        "UriKind",
        &[("RelativeOrAbsolute", 0), ("Absolute", 1), ("Relative", 2)],
    );
    match d.t("System.Uri").as_any().downcast_ref::<FakeType>() {
        Some(uri) => {
            uri.add_constructor(vec![string.clone(), uri_kind.as_type()]);
        }
        None => panic!("System.Uri is not a fake type"),
    }

    let event_handler = d.generic_class(
        "System",
        "EventHandler`1",
        &["TEventArgs"],
        Some(d.t("System.MulticastDelegate")),
    );
    event_handler.add_constructor(vec![object.clone(), d.t("System.IntPtr")]);
    event_handler.add_method(
        "Invoke",
        void.clone(),
        vec![object.clone(), event_handler.generic_parameter(0)],
        false,
    );

    d.generic_class("System.Threading.Tasks", "Task`1", &["TResult"], None);
    core.define_generic_interface(
        "System.Collections.Generic",
        "IDictionary`2",
        &["TKey", "TValue"],
    );
    d.generic_class("System", "WeakReference`1", &["T"], None);
    core.define_generic_interface("System", "IObservable`1", &["T"]);

    let command = d.interface("System.Windows.Input", "ICommand");
    command.add_method("Execute", void, vec![object.clone()], false);
    command.add_method("CanExecute", d.t("System.Boolean"), vec![object], false);

    d.constructable(
        "System.ComponentModel",
        "CultureInfoConverter",
        Some("System.ComponentModel.TypeConverter"),
    );
}

fn define_base(ts: &Rc<FakeTypeSystem>, asm: &Rc<FakeAssembly>) {
    let d = Decl { ts, asm };
    let void = d.t("System.Void");
    let object = d.t("System.Object");
    let string = d.t("System.String");
    let boolean = d.t("System.Boolean");
    let int32 = d.t("System.Int32");
    let double = d.t("System.Double");
    let uint32 = d.t("System.UInt32");
    let disposable = d.t("System.IDisposable");
    let type_ = d.t("System.Type");

    // FerroUI.Metadata
    for name in [
        "XmlnsDefinitionAttribute",
        "ContentAttribute",
        "WhitespaceSignificantCollectionAttribute",
        "TrimSurroundingWhitespaceAttribute",
        "TemplateContentAttribute",
        "UsableDuringInitializationAttribute",
        "DependsOnAttribute",
        "DataTypeAttribute",
        "InheritDataTypeFromItemsAttribute",
        "InheritDataTypeFromAttribute",
        "MarkupExtensionOptionAttribute",
        "MarkupExtensionDefaultOptionAttribute",
        "ControlTemplateScopeAttribute",
        "FerroListAttribute",
    ] {
        d.attribute("FerroUI.Metadata", name);
    }
    d.enumeration(
        "FerroUI.Metadata",
        "InheritDataTypeFromScopeKind",
        &[("Style", 1), ("ControlTemplate", 2)],
    );
    let add_child = d.interface("FerroUI.Metadata", "IAddChild");
    add_child.add_method("AddChild", void.clone(), vec![object.clone()], false);
    let add_child_of_t =
        asm.define_generic_interface("FerroUI.Metadata", "IAddChild`1", &["T"]);
    add_child_of_t.add_method(
        "AddChild",
        void.clone(),
        vec![add_child_of_t.generic_parameter(0)],
        false,
    );

    // FerroUI.Data
    let binding_priority = d.enumeration(
        "FerroUI.Data",
        "BindingPriority",
        &[
            ("Animation", -1),
            ("LocalValue", 0),
            ("StyleTrigger", 1),
            ("Template", 2),
            ("Style", 3),
            ("Inherited", 4),
            ("Unset", i32::MAX),
        ],
    );
    d.attribute("FerroUI.Data", "AssignBindingAttribute");
    d.class("FerroUI.Data", "BindingBase", None);
    let binding_base = d.t("FerroUI.Data.BindingBase");
    d.class("FerroUI.Data", "BindingExpressionBase", None)
        .add_interface(disposable.clone());
    d.constructable("FerroUI.Data", "MultiBinding", Some("FerroUI.Data.BindingBase"));
    d.constructable("FerroUI.Data", "CompiledBinding", Some("FerroUI.Data.BindingBase"));
    d.constructable("FerroUI.Data", "CompiledBindingPathBuilder", None);
    d.class("FerroUI.Data", "CompiledBindingPath", None);
    d.constructable("FerroUI.Data", "RelativeSource", None);
    d.interface("FerroUI.Data.Core", "IPropertyInfo");
    d.class("FerroUI.Data.Core", "ClrPropertyInfo", None)
        .add_interface(d.t("FerroUI.Data.Core.IPropertyInfo"));
    asm.define_generic_interface(
        "FerroUI.Data.Core",
        "IPropertyInfo`2",
        &["TOwner", "TValue"],
    )
    .add_interface(d.t("FerroUI.Data.Core.IPropertyInfo"));
    d.generic_class(
        "FerroUI.Data.Core",
        "ClrPropertyInfo`2",
        &["TOwner", "TValue"],
        Some(d.t("FerroUI.Data.Core.ClrPropertyInfo")),
    );
    d.interface("FerroUI.Data.Core.Plugins", "IPropertyAccessor");

    // FerroUI: the property system
    d.class("FerroUI", "UnsetValueType", None);
    let ferro_property = d.class("FerroUI", "FerroProperty", None);
    ferro_property.add_field("UnsetValue", object.clone(), true, None);
    ferro_property.add_property_with("Name", string.clone(), true, false, false);
    let ferro_property_t = d.generic_class(
        "FerroUI",
        "FerroProperty`1",
        &["TValue"],
        Some(ferro_property.as_type()),
    );
    let styled_property_t = d.generic_class("FerroUI", "StyledProperty`1", &["TValue"], None);
    styled_property_t.set_base_type(
        d.generic("FerroUI.FerroProperty`1", &[styled_property_t.generic_parameter(0)]),
    );
    let attached_property_t = d.generic_class("FerroUI", "AttachedProperty`1", &["TValue"], None);
    attached_property_t.set_base_type(
        d.generic("FerroUI.StyledProperty`1", &[attached_property_t.generic_parameter(0)]),
    );
    let direct_property_base =
        d.generic_class("FerroUI", "DirectPropertyBase`1", &["TValue"], None);
    direct_property_base.set_base_type(
        d.generic("FerroUI.FerroProperty`1", &[direct_property_base.generic_parameter(0)]),
    );
    let direct_property = d.generic_class("FerroUI", "DirectProperty`2", &["TOwner", "TValue"], None);
    direct_property.set_base_type(
        d.generic("FerroUI.DirectPropertyBase`1", &[direct_property.generic_parameter(1)]),
    );
    let _ = ferro_property_t;

    let ferro_object = d.constructable("FerroUI", "FerroObject", None);
    ferro_object.add_method(
        "GetValue",
        object.clone(),
        vec![ferro_property.as_type()],
        false,
    );
    ferro_object.add_method(
        "SetValue",
        disposable.clone(),
        vec![
            ferro_property.as_type(),
            object.clone(),
            binding_priority.as_type(),
        ],
        false,
    );
    {
        let styled_definition = d.t("FerroUI.StyledProperty`1");
        let priority = binding_priority.as_type();
        let result = disposable.clone();
        ferro_object.add_generic_method("SetValue", &["T"], false, |g| {
            let property = match styled_definition.make_generic_type(&[g[0].clone()]) {
                Ok(property) => property,
                Err(e) => panic!("Unable to construct StyledProperty<T>: {e}"),
            };
            (result, vec![property, g[0].clone(), priority])
        });
    }
    ferro_object.add_method(
        "Bind",
        d.t("FerroUI.Data.BindingExpressionBase"),
        vec![ferro_property.as_type(), binding_base.clone()],
        false,
    );
    d.class("FerroUI", "FerroObjectExtensions", None);

    // FerroUI.Collections
    let ferro_list = d.generic_class("FerroUI.Collections", "FerroList`1", &["T"], None);
    ferro_list.add_constructor(vec![]);
    ferro_list.add_constructor(vec![int32.clone()]);
    ferro_list.add_interface(d.generic(
        "System.Collections.Generic.IList`1",
        &[ferro_list.generic_parameter(0)],
    ));
    ferro_list.add_interface(d.generic(
        "System.Collections.Generic.IReadOnlyList`1",
        &[ferro_list.generic_parameter(0)],
    ));
    ferro_list.add_interface(d.t("System.Collections.IList"));
    ferro_list.add_property("Capacity", int32.clone());
    ferro_list.add_property_with("Count", int32.clone(), true, false, false);
    ferro_list.add_method(
        "Add",
        void.clone(),
        vec![ferro_list.generic_parameter(0)],
        false,
    );
    d.generic_class(
        "FerroUI.Collections",
        "FerroListConverter`1",
        &["T"],
        Some(d.t("System.ComponentModel.TypeConverter")),
    )
    .add_constructor(vec![]);

    // FerroUI: value types
    for (name, count) in [
        ("Thickness", 4usize),
        ("Point", 2),
        ("Vector", 2),
        ("Size", 2),
        ("Matrix", 6),
        ("CornerRadius", 4),
    ] {
        let value = d.structure("FerroUI", name);
        value.add_constructor((0..count).map(|_| double.clone()).collect());
        value.add_method("Parse", value.as_type(), vec![string.clone()], true);
    }
    match d.t("FerroUI.Thickness").as_any().downcast_ref::<FakeType>() {
        Some(thickness) => {
            thickness.add_constructor(vec![double.clone()]);
            thickness.add_constructor(vec![double.clone(), double.clone()]);
        }
        None => panic!("FerroUI.Thickness is not a fake type"),
    }
    let relative_unit = d.enumeration("FerroUI", "RelativeUnit", &[("Relative", 0), ("Absolute", 1)]);
    let relative_point = d.structure("FerroUI", "RelativePoint");
    relative_point.add_constructor(vec![double.clone(), double.clone(), relative_unit.as_type()]);
    relative_point.add_constructor(vec![d.t("FerroUI.Point"), relative_unit.as_type()]);

    // FerroUI.Controls (the part that lives in the base assembly)
    let name_scope_interface = d.interface("FerroUI.Controls", "INameScope");
    name_scope_interface.add_method(
        "Register",
        void.clone(),
        vec![string.clone(), object.clone()],
        false,
    );
    name_scope_interface.add_method("Find", object.clone(), vec![string.clone()], false);
    name_scope_interface.add_method("Complete", void.clone(), vec![], false);
    name_scope_interface.add_property_with("IsCompleted", boolean.clone(), true, false, false);
    d.interface("FerroUI.Controls", "IDeferredContent");
    d.interface("FerroUI.Controls", "IThemeVariantProvider");
    d.interface("FerroUI.Controls", "IResourceDictionary");
    let classes = d.constructable("FerroUI.Controls", "Classes", None);
    classes.set_base_type(d.generic("FerroUI.Collections.FerroList`1", &[string.clone()]));

    // FerroUI.Animation
    d.class("FerroUI.Animation", "Animatable", Some("FerroUI.FerroObject"));
    d.interface("FerroUI.Animation", "ITransition");
    let transitions = d.constructable("FerroUI.Animation", "Transitions", None);
    transitions.set_base_type(d.generic(
        "FerroUI.Collections.FerroList`1",
        &[d.t("FerroUI.Animation.ITransition")],
    ));

    // FerroUI.Styling
    let theme_variant = d.class("FerroUI.Styling", "ThemeVariant", None);
    theme_variant.add_constructor(vec![object.clone(), theme_variant.as_type()]);
    for name in ["Default", "Light", "Dark"] {
        theme_variant.add_property_with(name, theme_variant.as_type(), true, false, true);
    }
    theme_variant.add_property_with("Key", object.clone(), true, false, false);
    d.interface("FerroUI.Styling", "IStyle");
    d.class("FerroUI.Styling", "Selector", None);
    d.class("FerroUI.Styling", "Selectors", None);
    d.class("FerroUI.Styling", "StyleQueries", None);
    d.class("FerroUI.Styling", "SetterBase", None);

    // StyledElement and the visual base classes
    let styled_element = d.constructable("FerroUI", "StyledElement", Some("FerroUI.Animation.Animatable"));
    styled_element.add_interface(d.t("System.ComponentModel.ISupportInitialize"));
    d.direct(&styled_element, "Name", string.clone());
    d.styled(&styled_element, "DataContext", object.clone());
    styled_element.add_property_with("Classes", classes.as_type(), true, false, false);
    d.class("FerroUI", "StyledElementExtensions", None);
    let styled_element_extensions = d.t("FerroUI.StyledElementExtensions");
    match styled_element_extensions.as_any().downcast_ref::<FakeType>() {
        Some(extensions) => {
            extensions.add_method(
                "BindClass",
                disposable.clone(),
                vec![
                    styled_element.as_type(),
                    string.clone(),
                    binding_base.clone(),
                    object.clone(),
                ],
                true,
            );
            extensions.add_method(
                "GetClassProperty",
                ferro_property.as_type(),
                vec![string.clone()],
                true,
            );
        }
        None => panic!("FerroUI.StyledElementExtensions is not a fake type"),
    }
    let name_scope = d.constructable("FerroUI.Controls", "NameScope", None);
    name_scope.add_interface(name_scope_interface.as_type());
    name_scope.add_method(
        "SetNameScope",
        void.clone(),
        vec![styled_element.as_type(), name_scope_interface.as_type()],
        true,
    );
    name_scope.add_method(
        "GetNameScope",
        name_scope_interface.as_type(),
        vec![styled_element.as_type()],
        true,
    );

    let resource_dictionary = d.constructable("FerroUI.Controls", "ResourceDictionary", None);
    resource_dictionary.add_interface(d.t("FerroUI.Controls.IResourceDictionary"));
    resource_dictionary.add_interface(d.t("FerroUI.Controls.IThemeVariantProvider"));
    resource_dictionary.add_property_with("Count", int32.clone(), true, false, false);
    resource_dictionary.add_method(
        "Add",
        void.clone(),
        vec![object.clone(), object.clone()],
        false,
    );
    resource_dictionary.add_method(
        "AddDeferred",
        void.clone(),
        vec![object.clone(), d.t("FerroUI.Controls.IDeferredContent")],
        false,
    );
    resource_dictionary.add_method(
        "AddNotSharedDeferred",
        void.clone(),
        vec![object.clone(), d.t("FerroUI.Controls.IDeferredContent")],
        false,
    );
    resource_dictionary.add_method("EnsureCapacity", void.clone(), vec![int32.clone()], false);
    styled_element.add_property("Resources", d.t("FerroUI.Controls.IResourceDictionary"));

    d.constructable("FerroUI", "Visual", Some("FerroUI.StyledElement"));
    d.constructable("FerroUI.Layout", "Layoutable", Some("FerroUI.Visual"));
    d.styled(
        &d.fake("FerroUI.Layout.Layoutable"),
        "Margin",
        d.t("FerroUI.Thickness"),
    );
    d.styled(&d.fake("FerroUI.Layout.Layoutable"), "Width", double.clone());
    d.styled(&d.fake("FerroUI.Layout.Layoutable"), "Height", double.clone());

    // FerroUI.Interactivity
    let routing_strategies = d.enumeration(
        "FerroUI.Interactivity",
        "RoutingStrategies",
        &[("Direct", 1), ("Tunnel", 2), ("Bubble", 4)],
    );
    routing_strategies.add_attribute(FakeCustomAttribute::new(d.t("System.FlagsAttribute"), vec![]));
    let routed_event = d.class("FerroUI.Interactivity", "RoutedEvent", None);
    let routed_event_t = d.generic_class(
        "FerroUI.Interactivity",
        "RoutedEvent`1",
        &["TEventArgs"],
        Some(routed_event.as_type()),
    );
    let _ = routed_event_t;
    d.constructable("FerroUI.Interactivity", "RoutedEventArgs", None);
    let interactive = d.constructable(
        "FerroUI.Interactivity",
        "Interactive",
        Some("FerroUI.Layout.Layoutable"),
    );
    interactive.add_method(
        "AddHandler",
        void.clone(),
        vec![
            routed_event.as_type(),
            d.t("System.Delegate"),
            routing_strategies.as_type(),
            boolean.clone(),
        ],
        false,
    );
    {
        let routed_event_definition = d.t("FerroUI.Interactivity.RoutedEvent`1");
        let event_handler_definition = d.t("System.EventHandler`1");
        let routes = routing_strategies.as_type();
        let handled_events_too = boolean.clone();
        let result = void.clone();
        interactive.add_generic_method("AddHandler", &["TEventArgs"], false, |g| {
            let argument = std::slice::from_ref(&g[0]);
            let (event, handler) = match (
                routed_event_definition.make_generic_type(argument),
                event_handler_definition.make_generic_type(argument),
            ) {
                (Ok(event), Ok(handler)) => (event, handler),
                _ => panic!("Unable to construct the AddHandler<TEventArgs> signature"),
            };
            (result, vec![event, handler, routes, handled_events_too])
        });
    }

    // FerroUI.Media
    let brush = d.interface("FerroUI.Media", "IBrush");
    d.interface("FerroUI.Media", "IImage");
    d.interface("FerroUI.Media", "IImageBrushSource");
    let bitmap = d.class("FerroUI.Media.Imaging", "Bitmap", None);
    bitmap.add_interface(d.t("FerroUI.Media.IImage"));
    bitmap.add_interface(d.t("FerroUI.Media.IImageBrushSource"));
    let color = d.structure("FerroUI.Media", "Color");
    color.add_method("FromUInt32", color.as_type(), vec![uint32.clone()], true);
    color.add_method("Parse", color.as_type(), vec![string.clone()], true);
    let solid_brush = d.class("FerroUI.Media.Immutable", "ImmutableSolidColorBrush", None);
    solid_brush.add_interface(brush.as_type());
    solid_brush.add_constructor(vec![uint32.clone()]);
    let brush_class = d.class("FerroUI.Media", "Brush", Some("FerroUI.Animation.Animatable"));
    brush_class.add_interface(brush.as_type());
    let solid_color_brush = d.constructable("FerroUI.Media", "SolidColorBrush", Some("FerroUI.Media.Brush"));
    d.styled(&solid_color_brush, "Color", color.as_type());
    let font_family = d.class("FerroUI.Media", "FontFamily", None);
    font_family.add_constructor(vec![string.clone()]);
    font_family.add_constructor(vec![d.t("System.Uri"), string.clone()]);
    let text_trimming = d.class("FerroUI.Media", "TextTrimming", None);
    for name in [
        "None",
        "CharacterEllipsis",
        "WordEllipsis",
        "PrefixCharacterEllipsis",
        "LeadingCharacterEllipsis",
        "PathSegmentEllipsis",
    ] {
        text_trimming.add_property_with(name, text_trimming.as_type(), true, false, true);
    }
    d.constructable("FerroUI.Media", "TextDecoration", Some("FerroUI.FerroObject"));
    let text_decoration_collection = d.constructable("FerroUI.Media", "TextDecorationCollection", None);
    text_decoration_collection.set_base_type(d.generic(
        "FerroUI.Collections.FerroList`1",
        &[d.t("FerroUI.Media.TextDecoration")],
    ));
    let text_decorations = d.class("FerroUI.Media", "TextDecorations", None);
    for name in ["Underline", "Strikethrough", "Overline", "Baseline"] {
        text_decorations.add_property_with(
            name,
            text_decoration_collection.as_type(),
            true,
            false,
            true,
        );
    }

    // FerroUI.Input
    let standard_cursor_type = d.enumeration(
        "FerroUI.Input",
        "StandardCursorType",
        &[
            ("Arrow", 0),
            ("Ibeam", 1),
            ("Wait", 2),
            ("Cross", 3),
            ("UpArrow", 4),
            ("SizeWestEast", 5),
            ("SizeNorthSouth", 6),
            ("SizeAll", 7),
            ("No", 8),
            ("Hand", 9),
            ("AppStarting", 10),
            ("Help", 11),
        ],
    );
    d.class("FerroUI.Input", "Cursor", None)
        .add_constructor(vec![standard_cursor_type.as_type()]);
    d.constructable("FerroUI.Input", "InputElement", Some("FerroUI.Interactivity.Interactive"));
    d.styled(
        &d.fake("FerroUI.Input.InputElement"),
        "Cursor",
        d.t("FerroUI.Input.Cursor"),
    );
    d.styled(&d.fake("FerroUI.Input.InputElement"), "IsEnabled", boolean.clone());

    // FerroUI.Styling: setters, styles, themes
    let setter = d.constructable("FerroUI.Styling", "Setter", Some("FerroUI.Styling.SetterBase"));
    setter.add_constructor(vec![ferro_property.as_type(), object.clone()]);
    setter.add_property("Property", ferro_property.as_type());
    setter
        .add_property("Value", object.clone())
        .add_attribute(d.attr("FerroUI.Metadata.ContentAttribute"))
        .add_attribute(d.attr("FerroUI.Data.AssignBindingAttribute"))
        .add_attribute(FakeCustomAttribute::new(
            d.t("FerroUI.Metadata.DependsOnAttribute"),
            vec![XamlValue::String("Property".to_string())],
        ));
    let style_base = d.class("FerroUI.Styling", "StyleBase", Some("FerroUI.FerroObject"));
    style_base.add_interface(d.t("FerroUI.Styling.IStyle"));
    style_base.add_interface(add_child.as_type());
    style_base.add_property_with(
        "Setters",
        d.generic(
            "System.Collections.Generic.IList`1",
            &[d.t("FerroUI.Styling.SetterBase")],
        ),
        true,
        false,
        false,
    );
    style_base.add_property_with(
        "Children",
        d.generic(
            "System.Collections.Generic.IList`1",
            &[d.t("FerroUI.Styling.IStyle")],
        ),
        true,
        false,
        false,
    );
    style_base.add_property("Resources", d.t("FerroUI.Controls.IResourceDictionary"));
    style_base.add_method("Add", void.clone(), vec![d.t("FerroUI.Styling.SetterBase")], false);
    style_base.add_method("Add", void.clone(), vec![d.t("FerroUI.Styling.IStyle")], false);
    let style = d.constructable("FerroUI.Styling", "Style", Some("FerroUI.Styling.StyleBase"));
    style.add_property("Selector", d.t("FerroUI.Styling.Selector"));
    let control_theme =
        d.constructable("FerroUI.Styling", "ControlTheme", Some("FerroUI.Styling.StyleBase"));
    control_theme.add_property("TargetType", type_.clone());
    control_theme.add_property("BasedOn", control_theme.as_type());
    let container_query =
        d.constructable("FerroUI.Styling", "ContainerQuery", Some("FerroUI.Styling.StyleBase"));
    container_query.add_property("Name", string.clone());
    let styles = d.constructable("FerroUI.Styling", "Styles", Some("FerroUI.FerroObject"));
    styles.add_interface(d.t("FerroUI.Styling.IStyle"));
    styles.add_interface(d.generic(
        "System.Collections.Generic.IList`1",
        &[d.t("FerroUI.Styling.IStyle")],
    ));
    styles.add_method("Add", void.clone(), vec![d.t("FerroUI.Styling.IStyle")], false);
    styles.add_property("Resources", d.t("FerroUI.Controls.IResourceDictionary"));

    // FerroUI.Utilities
    d.class("FerroUI.Utilities", "TypeUtilities", None);
}

fn define_controls(ts: &Rc<FakeTypeSystem>, asm: &Rc<FakeAssembly>) {
    let d = Decl { ts, asm };
    let void = d.t("System.Void");
    let object = d.t("System.Object");
    let string = d.t("System.String");
    let int32 = d.t("System.Int32");
    let double = d.t("System.Double");
    let type_ = d.t("System.Type");
    let content = || d.attr("FerroUI.Metadata.ContentAttribute");

    // Templates
    d.interface("FerroUI.Controls", "ITemplate");
    let template_of_t =
        asm.define_generic_interface("FerroUI.Controls", "ITemplate`1", &["TControl"]);
    template_of_t.add_interface(d.t("FerroUI.Controls.ITemplate"));
    template_of_t.add_method("Build", template_of_t.generic_parameter(0), vec![], false);
    d.interface("FerroUI.Controls.Templates", "IDataTemplate");
    d.interface("FerroUI.Controls.Templates", "IControlTemplate");

    // Control
    let control = d.constructable("FerroUI.Controls", "Control", Some("FerroUI.Input.InputElement"));
    d.styled(&control, "Tag", object.clone());
    control.add_property_with(
        "DataTemplates",
        d.generic(
            "FerroUI.Collections.FerroList`1",
            &[d.t("FerroUI.Controls.Templates.IDataTemplate")],
        ),
        true,
        false,
        false,
    );
    let controls_collection = d.constructable("FerroUI.Controls", "Controls", None);
    controls_collection
        .set_base_type(d.generic("FerroUI.Collections.FerroList`1", &[control.as_type()]));

    // Value types
    let grid_unit_type = d.enumeration(
        "FerroUI.Controls",
        "GridUnitType",
        &[("Auto", 0), ("Pixel", 1), ("Star", 2)],
    );
    let grid_length = d.structure("FerroUI.Controls", "GridLength");
    grid_length.add_constructor(vec![double.clone()]);
    grid_length.add_constructor(vec![double.clone(), grid_unit_type.as_type()]);
    grid_length.add_method("Parse", grid_length.as_type(), vec![string.clone()], true);
    let window_transparency_level = d.structure("FerroUI.Controls", "WindowTransparencyLevel");
    for name in ["None", "Transparent", "Blur", "AcrylicBlur", "Mica"] {
        window_transparency_level.add_property_with(
            name,
            window_transparency_level.as_type(),
            true,
            false,
            true,
        );
    }
    d.class("FerroUI.Controls", "WindowIcon", None);

    // Grid definitions
    d.class("FerroUI.Controls", "DefinitionBase", Some("FerroUI.FerroObject"));
    let definition_list = d.generic_class("FerroUI.Controls", "DefinitionList`1", &["T"], None);
    definition_list.set_base_type(d.generic(
        "FerroUI.Collections.FerroList`1",
        &[definition_list.generic_parameter(0)],
    ));
    definition_list.add_attribute(FakeCustomAttribute::with_properties(
        d.t("FerroUI.Metadata.FerroListAttribute"),
        vec![],
        vec![(
            "Separators",
            XamlValue::Array(vec![
                XamlValue::String(",".to_string()),
                XamlValue::String(" ".to_string()),
            ]),
        )],
    ));
    for (definition, list, length) in [
        ("ColumnDefinition", "ColumnDefinitions", "Width"),
        ("RowDefinition", "RowDefinitions", "Height"),
    ] {
        let definition_type =
            d.constructable("FerroUI.Controls", definition, Some("FerroUI.Controls.DefinitionBase"));
        definition_type.add_constructor(vec![double.clone(), grid_unit_type.as_type()]);
        definition_type.add_constructor(vec![grid_length.as_type()]);
        d.styled(&definition_type, length, grid_length.as_type());
        let list_type = d.constructable("FerroUI.Controls", list, None);
        list_type.set_base_type(
            d.generic("FerroUI.Controls.DefinitionList`1", &[definition_type.as_type()]),
        );
    }

    // Panels
    let panel = d.constructable("FerroUI.Controls", "Panel", Some("FerroUI.Controls.Control"));
    panel
        .add_property_with("Children", controls_collection.as_type(), true, false, false)
        .add_attribute(content());
    d.styled(&panel, "Background", d.t("FerroUI.Media.IBrush"));
    d.constructable("FerroUI.Controls", "StackPanel", Some("FerroUI.Controls.Panel"));
    let grid = d.constructable("FerroUI.Controls", "Grid", Some("FerroUI.Controls.Panel"));
    grid.add_property("ColumnDefinitions", d.t("FerroUI.Controls.ColumnDefinitions"));
    grid.add_property("RowDefinitions", d.t("FerroUI.Controls.RowDefinitions"));
    d.attached(&grid, "Row", "FerroUI.Controls.Control", int32.clone());
    d.attached(&grid, "Column", "FerroUI.Controls.Control", int32.clone());

    // Decorators
    let decorator = d.constructable("FerroUI.Controls", "Decorator", Some("FerroUI.Controls.Control"));
    d.styled(&decorator, "Child", control.as_type())
        .add_attribute(content());
    d.styled(&decorator, "Padding", d.t("FerroUI.Thickness"));
    let border = d.constructable("FerroUI.Controls", "Border", Some("FerroUI.Controls.Decorator"));
    d.styled(&border, "Background", d.t("FerroUI.Media.IBrush"));
    d.styled(&border, "BorderBrush", d.t("FerroUI.Media.IBrush"));
    d.styled(&border, "BorderThickness", d.t("FerroUI.Thickness"));
    d.styled(&border, "CornerRadius", d.t("FerroUI.CornerRadius"));

    // Text
    let text_block = d.constructable("FerroUI.Controls", "TextBlock", Some("FerroUI.Controls.Control"));
    d.styled(&text_block, "Text", string.clone())
        .add_attribute(content());
    d.styled(&text_block, "Foreground", d.t("FerroUI.Media.IBrush"));
    d.styled(&text_block, "FontFamily", d.t("FerroUI.Media.FontFamily"));
    d.styled(&text_block, "FontSize", double.clone());
    d.styled(&text_block, "TextTrimming", d.t("FerroUI.Media.TextTrimming"));
    d.styled(
        &text_block,
        "TextDecorations",
        d.t("FerroUI.Media.TextDecorationCollection"),
    );

    // Templated and content controls
    let templated_control = d.constructable(
        "FerroUI.Controls.Primitives",
        "TemplatedControl",
        Some("FerroUI.Controls.Control"),
    );
    d.styled(&templated_control, "Background", d.t("FerroUI.Media.IBrush"));
    d.styled(&templated_control, "Foreground", d.t("FerroUI.Media.IBrush"));
    d.styled(&templated_control, "Padding", d.t("FerroUI.Thickness"));
    d.styled(
        &templated_control,
        "Template",
        d.t("FerroUI.Controls.Templates.IControlTemplate"),
    );
    let content_control = d.constructable(
        "FerroUI.Controls",
        "ContentControl",
        Some("FerroUI.Controls.Primitives.TemplatedControl"),
    );
    d.styled(&content_control, "Content", object.clone())
        .add_attribute(content())
        .add_attribute(FakeCustomAttribute::new(
            d.t("FerroUI.Metadata.DependsOnAttribute"),
            vec![XamlValue::String("ContentTemplate".to_string())],
        ));
    d.styled(
        &content_control,
        "ContentTemplate",
        d.t("FerroUI.Controls.Templates.IDataTemplate"),
    );
    let button = d.constructable("FerroUI.Controls", "Button", Some("FerroUI.Controls.ContentControl"));
    let routed_event_args = d.t("FerroUI.Interactivity.RoutedEventArgs");
    button.add_field(
        "ClickEvent",
        d.generic(
            "FerroUI.Interactivity.RoutedEvent`1",
            std::slice::from_ref(&routed_event_args),
        ),
        true,
        None,
    );
    button.add_event(
        "Click",
        d.generic("System.EventHandler`1", std::slice::from_ref(&routed_event_args)),
    );
    d.styled(&button, "Command", d.t("System.Windows.Input.ICommand"));
    d.styled(&button, "CommandParameter", object.clone());
    let items_control = d.constructable(
        "FerroUI.Controls",
        "ItemsControl",
        Some("FerroUI.Controls.Primitives.TemplatedControl"),
    );
    d.styled(&items_control, "ItemsSource", d.t("System.Collections.IEnumerable"));
    d.styled(
        &items_control,
        "ItemTemplate",
        d.t("FerroUI.Controls.Templates.IDataTemplate"),
    );
    let window = d.constructable("FerroUI.Controls", "Window", Some("FerroUI.Controls.ContentControl"));
    d.styled(&window, "Title", string.clone());
    d.styled(&window, "Icon", d.t("FerroUI.Controls.WindowIcon"));
    window.add_property(
        "TransparencyLevelHint",
        d.generic(
            "System.Collections.Generic.IReadOnlyList`1",
            &[window_transparency_level.as_type()],
        ),
    );
    let _ = (void, type_);
}

fn define_markup(ts: &Rc<FakeTypeSystem>, asm: &Rc<FakeAssembly>) {
    let d = Decl { ts, asm };
    let binding = d.constructable("FerroUI.Data", "Binding", Some("FerroUI.Data.BindingBase"));
    binding.add_constructor(vec![d.t("System.String")]);
    binding.add_property("Path", d.t("System.String"));
}

fn define_markup_xaml(ts: &Rc<FakeTypeSystem>, asm: &Rc<FakeAssembly>) {
    let d = Decl { ts, asm };
    let void = d.t("System.Void");
    let object = d.t("System.Object");
    let string = d.t("System.String");
    let int32 = d.t("System.Int32");
    let type_ = d.t("System.Type");
    let service_provider = d.t("System.IServiceProvider");

    // Service interfaces
    let provide_value_target = d.interface("FerroUI.Markup.Xaml", "IProvideValueTarget");
    provide_value_target.add_property_with("TargetObject", object.clone(), true, false, false);
    provide_value_target.add_property_with("TargetProperty", object.clone(), true, false, false);
    let root_object_provider = d.interface("FerroUI.Markup.Xaml", "IRootObjectProvider");
    root_object_provider.add_property_with("RootObject", object.clone(), true, false, false);
    root_object_provider.add_property_with("IntermediateRootObject", object.clone(), true, false, false);
    d.interface("FerroUI.Markup.Xaml", "IUriContext")
        .add_property("BaseUri", d.t("System.Uri"));
    let markup_extension = d.class("FerroUI.Markup.Xaml", "MarkupExtension", None);
    markup_extension.add_method(
        "ProvideValue",
        object.clone(),
        vec![service_provider.clone()],
        false,
    );

    // Runtime helpers
    let parent_stack_provider =
        d.interface("FerroUI.Markup.Xaml.XamlIl.Runtime", "IFerroXamlIlParentStackProvider");
    parent_stack_provider.add_property_with(
        "Parents",
        d.generic(
            "System.Collections.Generic.IEnumerable`1",
            std::slice::from_ref(&object),
        ),
        true,
        false,
        false,
    );
    let eager_parent_stack_provider = d.interface(
        "FerroUI.Markup.Xaml.XamlIl.Runtime",
        "IFerroXamlIlEagerParentStackProvider",
    );
    eager_parent_stack_provider.add_interface(parent_stack_provider.as_type());
    eager_parent_stack_provider.add_property_with(
        "DirectParentsStack",
        d.generic(
            "System.Collections.Generic.IReadOnlyList`1",
            std::slice::from_ref(&object),
        ),
        true,
        false,
        false,
    );
    eager_parent_stack_provider.add_property_with(
        "ParentProvider",
        eager_parent_stack_provider.as_type(),
        true,
        false,
        false,
    );
    d.interface(
        "FerroUI.Markup.Xaml.XamlIl.Runtime",
        "IFerroXamlIlXmlNamespaceInfoProvider",
    );
    let runtime_helpers = d.class("FerroUI.Markup.Xaml.XamlIl.Runtime", "XamlIlRuntimeHelpers", None);
    {
        let deferred_content = d.t("FerroUI.Controls.IDeferredContent");
        let builder = d.t("System.IntPtr");
        let provider = service_provider.clone();
        runtime_helpers.add_generic_method("DeferredTransformationFactoryV3", &["T"], true, |_| {
            (deferred_content, vec![builder, provider])
        });
    }
    runtime_helpers.add_method(
        "CreateInnerServiceProviderV1",
        service_provider.clone(),
        vec![service_provider.clone()],
        true,
    );
    runtime_helpers.add_method(
        "AsEagerParentStackProvider",
        eager_parent_stack_provider.as_type(),
        vec![parent_stack_provider.as_type()],
        true,
    );

    // Markup extensions
    for name in [
        "CompiledBindingExtension",
        "ReflectionBindingExtension",
        "ResolveByNameExtension",
        "StaticResourceExtension",
        "DynamicResourceExtension",
    ] {
        let extension = d.constructable("FerroUI.Markup.Xaml.MarkupExtensions", name, None);
        extension.add_method(
            "ProvideValue",
            object.clone(),
            vec![service_provider.clone()],
            false,
        );
    }
    d.constructable("FerroUI.Markup.Xaml.MarkupExtensions", "On", None)
        .add_property("Content", object.clone())
        .add_attribute(d.attr("FerroUI.Metadata.ContentAttribute"));
    d.class(
        "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings",
        "PropertyInfoAccessorFactory",
        None,
    );

    // Templates
    let template_content_attribute = || d.attr("FerroUI.Metadata.TemplateContentAttribute");
    let control_template = d.constructable("FerroUI.Markup.Xaml.Templates", "ControlTemplate", None);
    control_template.add_interface(d.t("FerroUI.Controls.Templates.IControlTemplate"));
    control_template
        .add_property("Content", object.clone())
        .add_attribute(d.attr("FerroUI.Metadata.ContentAttribute"))
        .add_attribute(template_content_attribute());
    control_template.add_property("TargetType", type_.clone());
    let data_template = d.constructable("FerroUI.Markup.Xaml.Templates", "DataTemplate", None);
    data_template.add_interface(d.t("FerroUI.Controls.Templates.IDataTemplate"));
    data_template
        .add_property("Content", object.clone())
        .add_attribute(d.attr("FerroUI.Metadata.ContentAttribute"))
        .add_attribute(template_content_attribute());
    data_template
        .add_property("DataType", type_.clone())
        .add_attribute(d.attr("FerroUI.Metadata.DataTypeAttribute"));

    // Includes
    for (name, interface) in [
        ("StyleInclude", Some("FerroUI.Styling.IStyle")),
        ("ResourceInclude", None),
        ("MergeResourceInclude", None),
    ] {
        let include = d.class("FerroUI.Markup.Xaml.Styling", name, None);
        include.add_constructor(vec![d.t("System.Uri")]);
        include.add_constructor(vec![service_provider.clone()]);
        include.add_property("Source", d.t("System.Uri"));
        if let Some(interface) = interface {
            include.add_interface(d.t(interface));
        }
    }

    // Converters
    for name in [
        "BitmapTypeConverter",
        "PointsListTypeConverter",
        "IconTypeConverter",
        "FerroUriTypeConverter",
        "TimeSpanTypeConverter",
        "FontFamilyTypeConverter",
    ] {
        d.constructable(
            "FerroUI.Markup.Xaml.Converters",
            name,
            Some("System.ComponentModel.TypeConverter"),
        );
    }

    // Diagnostics
    let xaml_source_info = d.class("FerroUI.Markup.Xaml.Diagnostics", "XamlSourceInfo", None);
    xaml_source_info.add_constructor(vec![int32.clone(), int32.clone(), string.clone()]);
    xaml_source_info.add_method(
        "SetXamlSourceInfo",
        void.clone(),
        vec![object.clone(), xaml_source_info.as_type()],
        true,
    );
    xaml_source_info.add_method(
        "SetXamlSourceInfo",
        void,
        vec![
            d.t("FerroUI.Controls.IResourceDictionary"),
            object,
            xaml_source_info.as_type(),
        ],
        true,
    );
}

/// The `XmlnsDefinition` attributes of the framework assemblies.
fn define_xmlns_definitions(
    ts: &Rc<FakeTypeSystem>,
    base: &Rc<FakeAssembly>,
    controls: &Rc<FakeAssembly>,
    markup: &Rc<FakeAssembly>,
    markup_xaml: &Rc<FakeAssembly>,
) {
    let attribute_type = ts.get("FerroUI.Metadata.XmlnsDefinitionAttribute");
    let define = |asm: &Rc<FakeAssembly>, namespaces: &[&str]| {
        for namespace in namespaces {
            asm.add_attribute(FakeCustomAttribute::new(
                attribute_type.clone(),
                vec![
                    XamlValue::String(FERRO_XML_NAMESPACE.to_string()),
                    XamlValue::String(namespace.to_string()),
                ],
            ));
        }
    };

    define(
        base,
        &[
            "FerroUI",
            "FerroUI.Animation",
            "FerroUI.Animation.Easings",
            "FerroUI.Controls",
            "FerroUI.Data",
            "FerroUI.Data.Converters",
            "FerroUI.Input",
            "FerroUI.Input.GestureRecognizers",
            "FerroUI.Input.TextInput",
            "FerroUI.Layout",
            "FerroUI.LogicalTree",
            "FerroUI.Media",
            "FerroUI.Media.Imaging",
            "FerroUI.Media.Transformation",
            "FerroUI.Styling",
        ],
    );
    define(
        controls,
        &[
            "FerroUI",
            "FerroUI.Automation",
            "FerroUI.Controls",
            "FerroUI.Controls.Embedding",
            "FerroUI.Controls.Presenters",
            "FerroUI.Controls.Primitives",
            "FerroUI.Controls.Shapes",
            "FerroUI.Controls.Templates",
            "FerroUI.Controls.Notifications",
            "FerroUI.Controls.Chrome",
            "FerroUI.Controls.Documents",
        ],
    );
    define(markup, &["FerroUI.Data", "FerroUI.Markup.Data"]);
    define(
        markup_xaml,
        &[
            "FerroUI.Markup.Xaml.MarkupExtensions",
            "FerroUI.Markup.Xaml.Styling",
            "FerroUI.Markup.Xaml.Templates",
        ],
    );
}

pub mod styles;
pub mod objects;
pub mod bindings;
