//! The test types declared by the upstream test file
//! `Converters/FerroPropertyConverterTest.cs`: the two nested classes, and
//! the services the test hands to the converter (mocks in the original).

use std::any::{Any, TypeId};
use std::rc::Rc;

use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::metadata::{service, IServiceProvider, MarkupTyped};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, ferro_static_type,
    instantiate, AttachedProperty, BoxedValue, FerroObjectImpl, FerroProperty, Ref, StaticType, StyledElement,
    StyledElementImpl, StyledProperty,
};
use ferroui_markup_xaml::converters::ITypeDescriptorContext;
use ferroui_markup_xaml::xaml_il::runtime::IFerroXamlIlParentStackProvider;
use ferroui_markup_xaml::{IXamlTypeResolver, XamlLoadException};

use crate::support::TypeModule;

// --- Class1 ------------------------------------------------------------------

/// A styled element with the registered property `Foo`.
#[repr(C)]
pub struct Class1 {
    base: StyledElement,
}

ferro_class!(Class1: StyledElement);
ferro_impl_classes!(Class1: FerroObjectImpl, StyledElementImpl);
ferro_class_info!(Class1 {
    new: Class1::new,
    markup: { namespace: "FerroUI.Markup.Xaml.UnitTests.Converters" },
});

ferro_properties! {
    impl Class1 {
        pub fn foo_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Class1, _>("Foo", None)
        }
    }
}

impl Class1 {
    pub fn construct() -> Self {
        Self { base: StyledElement::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

// --- AttachedOwner -----------------------------------------------------------

/// The owner of the attached property `Attached`, hosted by [`Class1`].
pub struct AttachedOwner;

ferro_static_type!(AttachedOwner);

ferro_properties! {
    impl AttachedOwner {
        pub fn attached_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<AttachedOwner, Class1, _>("Attached", None)
        }
    }
}

ferro_markup_type!(static AttachedOwner {
    type_info: AttachedOwner,
    namespace: "FerroUI.Markup.Xaml.UnitTests.Converters",
});

// --- The context of the conversion -------------------------------------------

/// The type resolver of the test: it resolves the names of the two classes
/// and nothing else.
pub struct TestTypeResolver;

impl IXamlTypeResolver for TestTypeResolver {
    fn resolve(&self, qualified_type_name: &str) -> Result<CastTarget, XamlLoadException> {
        match qualified_type_name {
            "Class1" => Ok(CastTarget::Class(<Class1 as StaticType>::TYPE)),
            "AttachedOwner" => Ok(CastTarget::Class(<AttachedOwner as StaticType>::TYPE)),
            other => Err(XamlLoadException::with_message(format!("Unable to resolve type {other}"))),
        }
    }
}

/// The parent stack of the test: it only enumerates its parents.
pub struct TestParentStackProvider {
    parents: Vec<BoxedValue>,
}

impl IFerroXamlIlParentStackProvider for TestParentStackProvider {
    fn parents(&self) -> Vec<BoxedValue> {
        self.parents.clone()
    }
}

/// The type descriptor context of the test: a type resolver and a parent
/// stack.
pub struct TestTypeDescriptorContext {
    type_resolver: Rc<dyn IXamlTypeResolver>,
    parent_stack: Rc<dyn IFerroXamlIlParentStackProvider>,
}

impl TestTypeDescriptorContext {
    pub fn new(parents: Vec<BoxedValue>) -> Rc<dyn ITypeDescriptorContext> {
        Rc::new(Self { type_resolver: Rc::new(TestTypeResolver), parent_stack: Rc::new(TestParentStackProvider { parents }) })
    }
}

impl IServiceProvider for TestTypeDescriptorContext {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        service(service_type, || self.type_resolver.clone())
            .or_else(|| service(service_type, || self.parent_stack.clone()))
    }
}

impl ITypeDescriptorContext for TestTypeDescriptorContext {}

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[Class1::TYPE, <AttachedOwner as StaticType>::TYPE],
    markup_types: &[<AttachedOwner as MarkupTyped>::MARKUP],
    value_types: || {},
};
