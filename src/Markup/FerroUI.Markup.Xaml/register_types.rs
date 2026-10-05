//! The type table of this crate: its namespaces, its types and what it
//! states about itself for markup.

use crate::converters::{
    BitmapTypeConverter, ColorToBrushConverter, FerroPropertyTypeConverter, FerroUriTypeConverter,
    FontFamilyTypeConverter, ITypeDescriptorContext, IconTypeConverter, PointsListTypeConverter,
    ServiceProviderTypeDescriptorContext, TimeSpanTypeConverter, TypeConverter,
};
use crate::diagnostics::XamlSourceInfo;
use crate::markup_extensions::compiled_bindings::PropertyInfoAccessorFactory;
use crate::markup_extensions::{
    CompiledBindingExtension, DynamicResourceExtension, On, OnFormFactorExtension, OnPlatformExtension,
    ReflectionBindingExtension, RelativeSourceExtension, ResolveByNameExtension, StaticResourceExtension,
};
use crate::styling::{MergeResourceInclude, ResourceInclude, StyleInclude};
use crate::templates::{
    ControlTemplate, DataTemplate, FocusAdornerTemplate, ItemsPanelTemplate, Template, TemplateContent,
    TreeDataTemplate, WindowDrawnDecorationsTemplate,
};
use crate::xaml_il::runtime::{
    DeferredContent, DeferredContentBuilder, FerroXamlIlXmlNamespaceInfo, IFerroXamlIlControlTemplateProvider,
    IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider, IFerroXamlIlXmlNamespaceInfoProvider,
    XamlIlRuntimeHelpers,
};
use crate::xamlx_runtime::{IXamlParentStackProviderV1, IXamlXmlNamespaceInfoProviderV1, XamlXmlNamespaceInfoV1};
use crate::{FerroXamlLoader, IProvideValueTarget, IRootObjectProvider, IUriContext, IXamlTypeResolver, MarkupExtension};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::controls::{IDeferredContent, IResourceProvider, IThemeVariantProvider, ResourceHostRef};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::IPropertyInfo;
use ferroui_base::data::{BindingBase, CompiledBinding, CompiledBindingPath, ReflectionBinding, RelativeSource};
use ferroui_base::media::IBrush;
use ferroui_base::styling::{IStyle, ITemplate};
use crate::object_casts::rc_of;
use ferroui_base::metadata::{
    IAddChild, IServiceProvider, MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE,
};
use ferroui_base::utilities::{CultureInfo, Uri};
use ferroui_base::{Ref, StaticType, TypeInfo};
use ferroui_controls::chrome::IWindowDrawnDecorationsTemplate;
use ferroui_controls::templates::{
    IControlTemplate, IDataTemplate, IRecyclingDataTemplate, ITemplateOf, ITreeDataTemplate, ITypedDataTemplate,
};
use ferroui_controls::{Control, Panel};
use std::rc::Rc;

/// The dotted namespaces of the modules of this crate. A type belongs to the
/// namespace of the longest module path that is a prefix of the path of its
/// declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_markup_xaml", "FerroUI.Markup.Xaml"),
    ("ferroui_markup_xaml::converters", "FerroUI.Markup.Xaml.Converters"),
    ("ferroui_markup_xaml::data", "FerroUI.Markup.Xaml.MarkupExtensions"),
    ("ferroui_markup_xaml::diagnostics", "FerroUI.Markup.Xaml.Diagnostics"),
    ("ferroui_markup_xaml::markup_extensions", "FerroUI.Markup.Xaml.MarkupExtensions"),
    (
        "ferroui_markup_xaml::markup_extensions::compiled_bindings",
        "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings",
    ),
    ("ferroui_markup_xaml::parsers", "FerroUI.Markup.Xaml.Parsers"),
    ("ferroui_markup_xaml::styling", "FerroUI.Markup.Xaml.Styling"),
    ("ferroui_markup_xaml::templates", "FerroUI.Markup.Xaml.Templates"),
    ("ferroui_markup_xaml::xaml_il::runtime", "FerroUI.Markup.Xaml.XamlIl.Runtime"),
    ("ferroui_markup_xaml::xamlx_runtime", "XamlX.Runtime"),
];

/// What this crate states about itself for markup: its assembly name and
/// the namespaces the XML namespace of the framework maps to.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Markup.Xaml",
    crate_name: "ferroui_markup_xaml",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Markup.Xaml.MarkupExtensions" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Markup.Xaml.Styling" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.Markup.Xaml.Templates" },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The types of this crate that own registered properties.
const TYPES: &[&TypeInfo] = &[<XamlSourceInfo as StaticType>::TYPE];

macro_rules! markup_types {
    ($($type_:ty),* $(,)?) => {
        &[$(<$type_ as MarkupTyped>::MARKUP),*]
    };
}

/// Every type of this crate with markup metadata.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = markup_types![
    // FerroUI.Markup.Xaml
    FerroXamlLoader,
    dyn IProvideValueTarget,
    dyn IRootObjectProvider,
    dyn IUriContext,
    dyn IXamlTypeResolver,
    dyn MarkupExtension,
    // FerroUI.Markup.Xaml.Converters
    BitmapTypeConverter,
    ColorToBrushConverter,
    FerroPropertyTypeConverter,
    FerroUriTypeConverter,
    FontFamilyTypeConverter,
    IconTypeConverter,
    PointsListTypeConverter,
    TimeSpanTypeConverter,
    // System.ComponentModel
    dyn ITypeDescriptorContext,
    dyn TypeConverter,
    // FerroUI.Markup.Xaml.Diagnostics
    XamlSourceInfo,
    // FerroUI.Markup.Xaml.MarkupExtensions
    CompiledBindingExtension,
    DynamicResourceExtension,
    On,
    OnFormFactorExtension,
    OnPlatformExtension,
    ReflectionBindingExtension,
    RelativeSourceExtension,
    ResolveByNameExtension,
    StaticResourceExtension,
    // FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings
    PropertyInfoAccessorFactory,
    // FerroUI.Markup.Xaml.Styling
    MergeResourceInclude,
    ResourceInclude,
    StyleInclude,
    // FerroUI.Markup.Xaml.Templates
    ControlTemplate,
    DataTemplate,
    FocusAdornerTemplate,
    ItemsPanelTemplate,
    Template,
    TemplateContent,
    TreeDataTemplate,
    WindowDrawnDecorationsTemplate,
    // FerroUI.Markup.Xaml.XamlIl.Runtime
    DeferredContent,
    DeferredContentBuilder,
    FerroXamlIlXmlNamespaceInfo,
    dyn IFerroXamlIlControlTemplateProvider,
    dyn IFerroXamlIlEagerParentStackProvider,
    dyn IFerroXamlIlParentStackProvider,
    dyn IFerroXamlIlXmlNamespaceInfoProvider,
    XamlIlRuntimeHelpers,
    // XamlX.Runtime
    dyn IXamlParentStackProviderV1,
    dyn IXamlXmlNamespaceInfoProviderV1,
    XamlXmlNamespaceInfoV1,
];

/// Registers the namespaces and the types of this crate (and of the crates
/// it is built on) with the tables of known types
/// ([`TypeInfo::find`], [`MarkupType::find`]) and its assembly. Cheap and
/// idempotent; nothing is initialised until a type is used or looked into.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        TypeInfo::register_rust_paths(crate::rust_paths::CLASS_RUST_PATHS);
        MarkupType::register_rust_paths(crate::rust_paths::MARKUP_RUST_PATHS);
        MarkupType::register_all(MARKUP_TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        ValueTypes::register_global(register_value_types);
    });
}

/// The table of what each concrete type of this crate is assignable to:
/// `Concrete => Handle` (an unsizing or identity conversion of its handle)
/// or `Concrete => Handle, |value| ..` (the base class part). The value
/// registrations and the test that checks them are both made from it.
macro_rules! for_each_assignability {
    ($apply:ident) => {
        // Deferred content.
        $apply!(DeferredContent => Rc<dyn IDeferredContent>);

        // Bindings: the extensions are bindings, and are the binding class
        // they derive from.
        $apply!(DynamicResourceExtension => Rc<dyn BindingBase>);
        $apply!(CompiledBindingExtension => Rc<dyn BindingBase>);
        $apply!(CompiledBindingExtension => Rc<CompiledBinding>, |value| value.base().clone());
        $apply!(ReflectionBindingExtension => Rc<dyn BindingBase>);
        $apply!(ReflectionBindingExtension => Rc<ReflectionBinding>, |value| value.base().clone());

        // Templates.
        $apply!(ControlTemplate => Rc<dyn IControlTemplate>);
        $apply!(DataTemplate => Rc<dyn IDataTemplate>);
        $apply!(DataTemplate => Rc<dyn IRecyclingDataTemplate>);
        $apply!(DataTemplate => Rc<dyn ITypedDataTemplate>);
        $apply!(TreeDataTemplate => Rc<dyn IDataTemplate>);
        $apply!(TreeDataTemplate => Rc<dyn ITreeDataTemplate>);
        $apply!(TreeDataTemplate => Rc<dyn ITypedDataTemplate>);
        $apply!(Template => Rc<dyn ITemplateOf<Option<Ref<Control>>>>);
        $apply!(Template => Rc<dyn ITemplateOf<Ref<Control>>>);
        $apply!(Template => Rc<dyn ITemplate>);
        $apply!(FocusAdornerTemplate => Rc<dyn ITemplateOf<Ref<Control>>>);
        $apply!(FocusAdornerTemplate => Rc<Template>, |value| value.base().clone());
        $apply!(FocusAdornerTemplate => Rc<dyn ITemplateOf<Option<Ref<Control>>>>);
        $apply!(FocusAdornerTemplate => Rc<dyn ITemplate>);
        $apply!(ItemsPanelTemplate => Rc<dyn ITemplateOf<Option<Ref<Panel>>>>);
        $apply!(ItemsPanelTemplate => Rc<dyn ITemplate>);
        $apply!(WindowDrawnDecorationsTemplate => Rc<dyn IWindowDrawnDecorationsTemplate>);
        $apply!(WindowDrawnDecorationsTemplate => Rc<dyn ITemplate>);

        // Includes.
        $apply!(ResourceInclude => Rc<dyn IResourceProvider>);
        $apply!(ResourceInclude => Rc<dyn IThemeVariantProvider>);
        $apply!(MergeResourceInclude => Rc<ResourceInclude>, |value| value.base().clone());
        $apply!(MergeResourceInclude => Rc<dyn IResourceProvider>, |value| value.base().clone());
        $apply!(MergeResourceInclude => Rc<dyn IThemeVariantProvider>, |value| value.base().clone());
        $apply!(StyleInclude => Rc<dyn IStyle>);
        $apply!(StyleInclude => Rc<dyn IResourceProvider>);

        // Option markup extensions.
        $apply!(OnPlatformExtension => Rc<dyn IAddChild<Rc<On>>>);
        $apply!(OnFormFactorExtension => Rc<dyn IAddChild<Rc<On>>>);

        // Converters.
        $apply!(ColorToBrushConverter => Rc<dyn IValueConverter>);
        $apply!(BitmapTypeConverter => Rc<dyn TypeConverter>);
        $apply!(FerroPropertyTypeConverter => Rc<dyn TypeConverter>);
        $apply!(FerroUriTypeConverter => Rc<dyn TypeConverter>);
        $apply!(FontFamilyTypeConverter => Rc<dyn TypeConverter>);
        $apply!(IconTypeConverter => Rc<dyn TypeConverter>);
        $apply!(PointsListTypeConverter => Rc<dyn TypeConverter>);
        $apply!(TimeSpanTypeConverter => Rc<dyn TypeConverter>);
    };
}

/// What the untyped value conversions must know about the types of this
/// crate: each shared plain type is a reference type (the object, its handle
/// and its nullable handle are the same reference), and the ones that
/// implement a contract are assignable to the handle of the contract.
///
/// Registered once, for every thread, by [`register_types`].
fn register_value_types() {

    macro_rules! references {
        ($($type_:ty),* $(,)?) => {
            $(ValueTypes::register_reference::<$type_>();)*
        };
    }
    references![
        BitmapTypeConverter,
        ColorToBrushConverter,
        FerroPropertyTypeConverter,
        FerroUriTypeConverter,
        FontFamilyTypeConverter,
        IconTypeConverter,
        PointsListTypeConverter,
        TimeSpanTypeConverter,
        On,
        OnFormFactorExtension,
        OnPlatformExtension,
        RelativeSourceExtension,
        ResolveByNameExtension,
        StaticResourceExtension,
        MergeResourceInclude,
        ResourceInclude,
        StyleInclude,
        ControlTemplate,
        DataTemplate,
        FocusAdornerTemplate,
        ItemsPanelTemplate,
        Template,
        TreeDataTemplate,
        WindowDrawnDecorationsTemplate,
        DeferredContent,
        FerroXamlIlXmlNamespaceInfo,
        XamlXmlNamespaceInfoV1,
    ];
    // The binding extensions are not reference types: like every binding,
    // they are boxed as their handle.
    ValueTypes::register_nullable::<Rc<DynamicResourceExtension>>();
    ValueTypes::register_nullable::<Rc<CompiledBindingExtension>>();
    ValueTypes::register_nullable::<Rc<ReflectionBindingExtension>>();

    // Nullable forms of the values members of this crate take and return.
    ValueTypes::register_nullable::<Rc<dyn IServiceProvider>>();
    ValueTypes::register_nullable::<Rc<dyn ITypeDescriptorContext>>();
    ValueTypes::register_nullable::<Rc<dyn TypeConverter>>();
    ValueTypes::register_nullable::<Rc<dyn IFerroXamlIlEagerParentStackProvider>>();
    ValueTypes::register_nullable::<Uri>();
    ValueTypes::register_nullable::<CultureInfo>();
    ValueTypes::register_nullable::<XamlSourceInfo>();
    ValueTypes::register_nullable::<ValueType>();
    ValueTypes::register_nullable::<&'static TypeInfo>();
    ValueTypes::register_nullable::<Rc<dyn BindingBase>>();
    ValueTypes::register_nullable::<Rc<CompiledBinding>>();
    ValueTypes::register_nullable::<Rc<ReflectionBinding>>();
    ValueTypes::register_nullable::<Rc<RelativeSource>>();
    ValueTypes::register_nullable::<Rc<dyn IValueConverter>>();
    ValueTypes::register_nullable::<Rc<dyn IPropertyInfo>>();
    ValueTypes::register_nullable::<Rc<dyn IResourceProvider>>();
    ValueTypes::register_nullable::<Rc<dyn IThemeVariantProvider>>();
    ValueTypes::register_nullable::<Rc<dyn IStyle>>();
    ValueTypes::register_nullable::<Rc<dyn ITemplate>>();
    ValueTypes::register_nullable::<Rc<dyn IDeferredContent>>();
    ValueTypes::register_nullable::<ResourceHostRef>();
    ValueTypes::register_nullable::<CompiledBindingPath>();
    ValueTypes::register_nullable::<Rc<dyn IBrush>>();
    ValueTypes::register_nullable::<Rc<dyn IControlTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn IDataTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn IWindowDrawnDecorationsTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>();
    ValueTypes::register_nullable::<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>>();

    // A service provider is accepted where a type descriptor context is
    // expected: the context of markup is its service provider.
    ValueTypes::register_cast::<Rc<dyn IServiceProvider>, Rc<dyn ITypeDescriptorContext>>(|service_provider| {
        ServiceProviderTypeDescriptorContext::new(service_provider.clone())
    });
    ValueTypes::register_cast::<Rc<dyn IFerroXamlIlEagerParentStackProvider>, Rc<dyn IFerroXamlIlParentStackProvider>>(
        |provider| provider.clone(),
    );

    // Every concrete type is assignable to the types it derives from and to
    // the handles of the contracts it implements, whichever form its value
    // is boxed in: the handle (`Rc<T>`) or, for a reference type, the object
    // itself.
    macro_rules! assignable {
        ($concrete:ty => $handle:ty) => {
            assignable!($concrete => $handle, |value| {
                let handle: $handle = value.clone();
                handle
            });
        };
        ($concrete:ty => $handle:ty, $cast:expr) => {{
            let cast: fn(&Rc<$concrete>) -> $handle = $cast;
            ValueTypes::register_cast::<Rc<$concrete>, $handle>(cast);
            ValueTypes::register_boxed_cast::<$concrete, $handle>(move |boxed| rc_of::<$concrete>(boxed).map(|value| cast(&value)));
            ValueTypes::register_nullable::<$handle>();
        }};
    }
    for_each_assignability!(assignable);
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_mirrors_the_xmlns_definitions_of_the_original() {
        register_types();
        let assembly = MarkupAssembly::find("FerroUI.Markup.Xaml").unwrap();
        assert!(std::ptr::eq(assembly, &ASSEMBLY));
        let namespaces: Vec<_> = assembly.xmlns_definitions.iter().map(|d| d.namespace).collect();
        assert_eq!(
            namespaces,
            [
                "FerroUI.Markup.Xaml.MarkupExtensions",
                "FerroUI.Markup.Xaml.Styling",
                "FerroUI.Markup.Xaml.Templates",
            ]
        );
        assert!(assembly.xmlns_definitions.iter().all(|d| d.xml_namespace == FERRO_XML_NAMESPACE));
        assert!(std::ptr::eq(MarkupAssembly::of_module(module_path!()).unwrap(), assembly));
    }

    #[test]
    fn every_markup_type_is_found_by_namespace_and_name() {
        register_types();
        for markup in MARKUP_TYPES {
            let namespace = markup.namespace();
            assert!(
                namespace.starts_with("FerroUI.Markup.Xaml")
                    || namespace == "XamlX.Runtime"
                    || namespace == "System.ComponentModel",
                "{} is in namespace '{namespace}'",
                markup.name
            );
            let found = MarkupType::find(namespace, markup.name)
                .unwrap_or_else(|| panic!("{}.{} is not registered", namespace, markup.name));
            assert!(std::ptr::eq(found, *markup), "{}", markup.full_name());
        }
    }

    #[test]
    fn types_the_compiler_looks_up_by_name_exist() {
        register_types();
        for full_name in [
            "FerroUI.Markup.Xaml.XamlIl.Runtime.XamlIlRuntimeHelpers",
            "FerroUI.Markup.Xaml.IProvideValueTarget",
            "FerroUI.Markup.Xaml.IRootObjectProvider",
            "FerroUI.Markup.Xaml.IUriContext",
            "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlParentStackProvider",
            "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlEagerParentStackProvider",
            "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlXmlNamespaceInfoProvider",
            "FerroUI.Markup.Xaml.MarkupExtensions.On",
            "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindings.PropertyInfoAccessorFactory",
            "FerroUI.Markup.Xaml.MarkupExtensions.CompiledBindingExtension",
            "FerroUI.Markup.Xaml.MarkupExtensions.ResolveByNameExtension",
            "FerroUI.Markup.Xaml.MarkupExtensions.ReflectionBindingExtension",
            "FerroUI.Markup.Xaml.Templates.DataTemplate",
            "FerroUI.Markup.Xaml.Templates.ControlTemplate",
            "FerroUI.Markup.Xaml.Styling.StyleInclude",
            "FerroUI.Markup.Xaml.Styling.ResourceInclude",
            "FerroUI.Markup.Xaml.Styling.MergeResourceInclude",
            "FerroUI.Markup.Xaml.Diagnostics.XamlSourceInfo",
        ] {
            let (namespace, name) = full_name.rsplit_once('.').unwrap();
            assert!(MarkupType::find(namespace, name).is_some(), "{full_name}");
        }

        let helpers = <XamlIlRuntimeHelpers as MarkupTyped>::MARKUP;
        for method in [
            "DeferredTransformationFactoryV3",
            "CreateInnerServiceProviderV1",
            "CreateRootServiceProviderV3",
            "AsEagerParentStackProvider",
            "ApplyNonMatchingMarkupExtensionV1",
        ] {
            assert!(helpers.find_methods(method).next().is_some_and(|m| m.is_static), "{method}");
        }
    }

    #[test]
    fn the_static_type_of_source_info_is_registered() {
        register_types();
        let found = TypeInfo::find("FerroUI.Markup.Xaml.Diagnostics", "XamlSourceInfo").unwrap();
        assert!(std::ptr::eq(found, <XamlSourceInfo as StaticType>::TYPE));
    }

    /// The members the XAML compiler (the loader crate) looks up on the
    /// types of this crate, with the parameter and result types its
    /// lookups ask for: a signature that drifts fails here.
    #[test]
    fn members_the_compiler_looks_up_have_the_expected_signatures() {
        use ferroui_base::controls::IResourceDictionary;
        use ferroui_base::data::core::plugins::IPropertyAccessor;
        use ferroui_base::data::core::WeakValue;
        use ferroui_base::platform::FormFactorType;
        use ferroui_base::BoxedValue;

        register_types();
        type Types = Vec<ValueType>;
        fn types(markup: &'static MarkupType, name: &str) -> Vec<(bool, Types, Option<ValueType>)> {
            markup
                .find_methods(name)
                .map(|m| (m.is_static, m.parameters.iter().map(|p| p()).collect(), m.return_type.map(|r| r())))
                .collect()
        }
        macro_rules! t {
            ($($type_:ty),*) => { vec![$(ValueType::of::<$type_>()),*] };
        }
        macro_rules! r {
            ($type_:ty) => { Some(ValueType::of::<$type_>()) };
        }
        let object = || ValueType::of::<Option<BoxedValue>>();
        type Sp = Rc<dyn IServiceProvider>;

        // XamlSourceInfo: constructor (int, int, string), the two setters and getters.
        let info = <XamlSourceInfo as MarkupTyped>::MARKUP;
        let constructor: Types = info.constructors[0].parameters.iter().map(|p| p()).collect();
        assert_eq!(constructor, t![i32, i32, Option<String>]);
        assert_eq!((info.handles[1])(), ValueType::of::<Option<XamlSourceInfo>>());
        assert_eq!(
            types(info, "SetXamlSourceInfo"),
            [
                (true, vec![object(), ValueType::of::<Option<XamlSourceInfo>>()], None),
                (true, vec![ValueType::of::<Rc<dyn IResourceDictionary>>(), object(), ValueType::of::<Option<XamlSourceInfo>>()], None),
            ]
        );
        assert_eq!(
            types(info, "GetXamlSourceInfo"),
            [
                (true, vec![object()], r![Option<XamlSourceInfo>]),
                (true, vec![ValueType::of::<Rc<dyn IResourceDictionary>>(), object()], r![Option<XamlSourceInfo>]),
            ]
        );

        // XamlIlRuntimeHelpers.
        let helpers = <XamlIlRuntimeHelpers as MarkupTyped>::MARKUP;
        for factory in ["DeferredTransformationFactoryV1", "DeferredTransformationFactoryV2", "DeferredTransformationFactoryV3"] {
            assert_eq!(types(helpers, factory), [(true, t![DeferredContentBuilder, Sp], r![Rc<DeferredContent>])]);
        }
        assert_eq!(types(helpers, "CreateInnerServiceProviderV1"), [(true, t![Sp], r![Sp])]);
        assert_eq!(types(helpers, "CreateRootServiceProviderV2"), [(true, t![], r![Sp])]);
        assert_eq!(types(helpers, "CreateRootServiceProviderV3"), [(true, t![Option<Sp>], r![Sp])]);
        assert_eq!(
            types(helpers, "AsEagerParentStackProvider"),
            [(true, t![Rc<dyn IFerroXamlIlParentStackProvider>], r![Rc<dyn IFerroXamlIlEagerParentStackProvider>])]
        );
        assert_eq!(
            types(helpers, "ApplyNonMatchingMarkupExtensionV1"),
            [(true, vec![object(), object(), ValueType::of::<Sp>(), object()], None)]
        );

        // The eager parent stack provider contract the runtime context implements.
        let eager = <dyn IFerroXamlIlEagerParentStackProvider as MarkupTyped>::MARKUP;
        assert_eq!((eager.find_property("DirectParentsStack").unwrap().type_)(), ValueType::of::<Rc<Vec<BoxedValue>>>());
        assert_eq!(
            (eager.find_property("ParentProvider").unwrap().type_)(),
            ValueType::of::<Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>>>()
        );
        let roots = <dyn IRootObjectProvider as MarkupTyped>::MARKUP;
        assert!(roots.find_property("IntermediateRootObject").is_some() && roots.find_property("RootObject").is_some());
        assert_eq!((<dyn IUriContext as MarkupTyped>::MARKUP.find_property("BaseUri").unwrap().type_)(), ValueType::of::<Option<Uri>>());

        // The accessor factories of compiled binding paths.
        let factory = <PropertyInfoAccessorFactory as MarkupTyped>::MARKUP;
        type Info = Rc<dyn IPropertyInfo>;
        type Accessor = Rc<dyn IPropertyAccessor>;
        assert_eq!(types(factory, "CreateInpcPropertyAccessor"), [(true, t![WeakValue, Info], r![Accessor])]);
        assert_eq!(types(factory, "CreateFerroPropertyAccessor"), [(true, t![WeakValue, Info], r![Accessor])]);
        assert_eq!(types(factory, "CreateIndexerPropertyAccessor"), [(true, t![WeakValue, Info, i32], r![Accessor])]);

        // Option markup extensions: `bool ShouldProvideOption(..)` with one or two parameters.
        assert_eq!(types(<OnPlatformExtension as MarkupTyped>::MARKUP, "ShouldProvideOption"), [(true, t![String], r![bool])]);
        assert_eq!(
            types(<OnFormFactorExtension as MarkupTyped>::MARKUP, "ShouldProvideOption"),
            [(true, t![Sp, FormFactorType], r![bool])]
        );
        assert_eq!((<On as MarkupTyped>::MARKUP.find_property("Options").unwrap().type_)(), ValueType::of::<Vec<String>>());

        // Markup extensions: constructors and `ProvideValue`.
        assert_eq!(types(<StaticResourceExtension as MarkupTyped>::MARKUP, "ProvideValue"), [(false, t![Sp], Some(object()))]);
        assert_eq!(
            types(<DynamicResourceExtension as MarkupTyped>::MARKUP, "ProvideValue"),
            [(false, t![Sp], r![Rc<DynamicResourceExtension>])]
        );
        assert_eq!(types(<ResolveByNameExtension as MarkupTyped>::MARKUP, "ProvideValue"), [(false, t![Sp], Some(object()))]);
        let resolve: Types = <ResolveByNameExtension as MarkupTyped>::MARKUP.constructors[0].parameters.iter().map(|p| p()).collect();
        assert_eq!(resolve, t![String]);
        assert_eq!(
            types(<RelativeSourceExtension as MarkupTyped>::MARKUP, "ProvideValue"),
            [(false, t![Sp], r![Rc<RelativeSource>])]
        );
        let compiled = <CompiledBindingExtension as MarkupTyped>::MARKUP;
        assert_eq!(types(compiled, "ProvideValue"), [(false, t![Option<Sp>], r![Rc<CompiledBinding>])]);
        let with_path: Types = compiled.constructors[1].parameters.iter().map(|p| p()).collect();
        assert_eq!(with_path, t![CompiledBindingPath]);
        assert_eq!((compiled.find_property("DataType").unwrap().type_)(), ValueType::of::<Option<ValueType>>());
        let reflection = <ReflectionBindingExtension as MarkupTyped>::MARKUP;
        assert_eq!(types(reflection, "ProvideValue"), [(false, t![Sp], r![Rc<ReflectionBinding>])]);
        let with_path: Types = reflection.constructors[1].parameters.iter().map(|p| p()).collect();
        assert_eq!(with_path, t![String]);

        // Templates and includes.
        assert_eq!(
            (<ControlTemplate as MarkupTyped>::MARKUP.find_property("TargetType").unwrap().type_)(),
            ValueType::of::<Option<&'static TypeInfo>>()
        );
        assert_eq!(
            (<DataTemplate as MarkupTyped>::MARKUP.find_property("DataType").unwrap().type_)(),
            ValueType::of::<Option<ValueType>>()
        );
        assert_eq!(
            (<ResourceInclude as MarkupTyped>::MARKUP.find_property("Loaded").unwrap().type_)(),
            ValueType::of::<Rc<dyn IResourceDictionary>>()
        );
        assert_eq!((<StyleInclude as MarkupTyped>::MARKUP.find_property("Loaded").unwrap().type_)(), ValueType::of::<Rc<dyn IStyle>>());
        for include in [
            <ResourceInclude as MarkupTyped>::MARKUP,
            <StyleInclude as MarkupTyped>::MARKUP,
            <MergeResourceInclude as MarkupTyped>::MARKUP,
        ] {
            let constructors: Vec<Types> =
                include.constructors.iter().map(|c| c.parameters.iter().map(|p| p()).collect()).collect();
            assert_eq!(constructors, [t![Option<Uri>], t![Sp]], "{}", include.name);
        }
        assert_eq!((<ResourceInclude as MarkupTyped>::MARKUP.find_property("Source").unwrap().type_)(), ValueType::of::<Option<Uri>>());
    }

    /// A value of each concrete type of the assignability table.
    trait Sample {
        fn sample() -> Rc<Self>;
    }

    macro_rules! samples {
        ($($type_:ty => $new:expr),* $(,)?) => {
            $(impl Sample for $type_ {
                fn sample() -> Rc<Self> {
                    $new
                }
            })*
        };
    }

    /// A declaration context with an empty parent stack and no root object,
    /// built without the test support (which registers the types itself).
    struct Declaration;

    impl IFerroXamlIlParentStackProvider for Declaration {
        fn parents(&self) -> Vec<ferroui_base::BoxedValue> {
            Vec::new()
        }
    }

    impl crate::IRootObjectProvider for Declaration {
        fn root_object(&self) -> Option<ferroui_base::BoxedValue> {
            None
        }

        fn intermediate_root_object(&self) -> Option<ferroui_base::BoxedValue> {
            None
        }
    }

    impl IServiceProvider for Declaration {
        fn get_service(&self, service_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
            use ferroui_base::metadata::service;
            service(service_type, || -> Rc<dyn IFerroXamlIlParentStackProvider> { Rc::new(Declaration) })
                .or_else(|| service(service_type, || -> Rc<dyn crate::IRootObjectProvider> { Rc::new(Declaration) }))
        }
    }

    samples! {
        DeferredContent => {
            let provider: Rc<dyn IServiceProvider> = Rc::new(Declaration);
            XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<ferroui_base::BoxedValue>(
                DeferredContentBuilder::new(|_| None),
                &provider,
            )
        },
        DynamicResourceExtension => DynamicResourceExtension::new(),
        CompiledBindingExtension => CompiledBindingExtension::new(),
        ReflectionBindingExtension => ReflectionBindingExtension::new(),
        ControlTemplate => ControlTemplate::new(),
        DataTemplate => DataTemplate::new(),
        TreeDataTemplate => TreeDataTemplate::new(),
        Template => Template::new(),
        FocusAdornerTemplate => FocusAdornerTemplate::new(),
        ItemsPanelTemplate => ItemsPanelTemplate::new(),
        WindowDrawnDecorationsTemplate => WindowDrawnDecorationsTemplate::new(),
        ResourceInclude => ResourceInclude::new(None),
        MergeResourceInclude => MergeResourceInclude::new(None),
        StyleInclude => StyleInclude::new(None),
        OnPlatformExtension => OnPlatformExtension::new(),
        OnFormFactorExtension => OnFormFactorExtension::new(),
        ColorToBrushConverter => ColorToBrushConverter::new(),
        BitmapTypeConverter => BitmapTypeConverter::new(),
        FerroPropertyTypeConverter => FerroPropertyTypeConverter::new(),
        FerroUriTypeConverter => FerroUriTypeConverter::new(),
        FontFamilyTypeConverter => FontFamilyTypeConverter::new(),
        IconTypeConverter => IconTypeConverter::new(),
        PointsListTypeConverter => PointsListTypeConverter::new(),
        TimeSpanTypeConverter => TimeSpanTypeConverter::new(),
    }

    /// Every concrete type is assignable to each contract it implements and
    /// each class it derives from, in every form its value may be boxed in,
    /// on a thread that never called `register_types()`.
    #[test]
    fn every_concrete_type_is_assignable_to_its_contracts_on_any_thread() {
        use ferroui_base::metadata::{from_markup_value, into_markup_value};
        use ferroui_base::BoxedValue;

        register_types();

        std::thread::spawn(|| {
            let mut checked = 0;
            macro_rules! check {
                ($concrete:ty => $handle:ty $(, $cast:expr)?) => {{
                    let sample = <$concrete as Sample>::sample();
                    // The canonical untyped form, the handle and the object itself.
                    let canonical = into_markup_value(sample.clone());
                    let handle: BoxedValue = Rc::new(sample.clone());
                    let object: BoxedValue = sample.clone();
                    for (form, value) in [("canonical", canonical), ("handle", Some(handle)), ("object", Some(object))] {
                        let what = format!("{} ({form}) as {}", stringify!($concrete), stringify!($handle));
                        assert!(from_markup_value::<$handle>(&value).is_some(), "{what}");
                        assert!(from_markup_value::<Option<$handle>>(&value).is_some_and(|v| v.is_some()), "optional {what}");
                        assert!(
                            ValueTypes::is_assignable(
                                ValueType::of_value(&**value.as_ref().unwrap()),
                                ValueType::of::<$handle>()
                            ),
                            "assignability of {what}"
                        );
                    }
                    checked += 1;
                }};
            }
            for_each_assignability!(check);
            assert_eq!(checked, 41);

            // A binding extension provides a value that a binding-typed member takes.
            let provider: Rc<dyn IServiceProvider> = Rc::new(Declaration);
            let extension = DynamicResourceExtension::new();
            let markup = <DynamicResourceExtension as MarkupTyped>::MARKUP;
            assert_eq!((markup.base.unwrap())(), ValueType::of::<Rc<dyn BindingBase>>());
            let provide = markup.find_methods("ProvideValue").next().unwrap();
            let provided = (provide.invoke)(&[into_markup_value(extension), into_markup_value(provider)]).unwrap();
            assert!(from_markup_value::<Rc<dyn BindingBase>>(&provided).is_some());

            // Deferred content is what a resource dictionary adds as a deferred resource.
            let content = into_markup_value(<DeferredContent as Sample>::sample());
            let deferred = from_markup_value::<Rc<dyn IDeferredContent>>(&content).unwrap();
            let dictionary = ferroui_base::controls::ResourceDictionary::new();
            dictionary.add_deferred("key", deferred);
            assert!(dictionary.contains_deferred_key(&"key".into()));
        })
        .join()
        .unwrap();
    }
}
