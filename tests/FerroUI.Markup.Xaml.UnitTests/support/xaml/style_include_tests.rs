//! The test types declared by the upstream test files `Xaml/StyleIncludeTests.cs`
//! (`TestServiceProvider`) and `Xaml/StyleWithServiceProvider.xaml.cs`
//! (`StyleWithServiceProvider`, the class of the embedded document
//! `Xaml/StyleWithServiceProvider.xaml`).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{IServiceProvider, MarkupTyped};
use ferroui_base::styling::{Style, StyleBaseImpl};
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl,
    Ref,
};
use ferroui_controls::{ContentControl, Control};
use ferroui_markup_xaml::xaml_il::runtime::{
    IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider, XamlIlRuntimeHelpers,
};
use ferroui_markup_xaml::{FerroXamlLoader, IUriContext};

use crate::support::TypeModule;

// --- TestServiceProvider -----------------------------------------------------

/// A service provider that is its own URI context and parent stack
/// provider; every other service comes from a root service provider.
pub struct TestServiceProvider {
    this: Weak<TestServiceProvider>,
    root: Rc<dyn IServiceProvider>,
    base_uri: RefCell<Option<Uri>>,
    parents_stack: RefCell<Rc<Vec<BoxedValue>>>,
}

crate::test_identity_eq!(TestServiceProvider);

impl TestServiceProvider {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            root: XamlIlRuntimeHelpers::create_root_service_provider_v2(),
            base_uri: RefCell::new(None),
            parents_stack: RefCell::new(Rc::new(vec![Control::boxed(ContentControl::new())])),
        })
    }

    pub fn parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        self.parents_stack.borrow().clone()
    }

    pub fn set_parents_stack(&self, value: Rc<Vec<BoxedValue>>) {
        *self.parents_stack.borrow_mut() = value;
    }

    fn this(&self) -> Rc<TestServiceProvider> {
        self.this.upgrade().expect("the object is alive while it is used")
    }

    fn as_service_provider(&self) -> Rc<dyn IServiceProvider> {
        self.this()
    }

    fn as_uri_context(&self) -> Rc<dyn IUriContext> {
        self.this()
    }

    fn as_parent_stack_provider(&self) -> Rc<dyn IFerroXamlIlParentStackProvider> {
        self.this()
    }

    fn as_eager_parent_stack_provider(&self) -> Rc<dyn IFerroXamlIlEagerParentStackProvider> {
        self.this()
    }
}

impl IServiceProvider for TestServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if service_type == TypeId::of::<Rc<dyn IUriContext>>() {
            return Some(Rc::new(self.as_uri_context()));
        }
        if service_type == TypeId::of::<Rc<dyn IFerroXamlIlParentStackProvider>>() {
            return Some(Rc::new(self.as_parent_stack_provider()));
        }
        self.root.get_service(service_type)
    }
}

impl IUriContext for TestServiceProvider {
    fn base_uri(&self) -> Option<Uri> {
        self.base_uri.borrow().clone()
    }

    fn set_base_uri(&self, value: Option<Uri>) {
        *self.base_uri.borrow_mut() = value;
    }
}

impl IFerroXamlIlParentStackProvider for TestServiceProvider {
    fn parents(&self) -> Vec<BoxedValue> {
        self.parents_stack().iter().rev().cloned().collect()
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

impl IFerroXamlIlEagerParentStackProvider for TestServiceProvider {
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        self.parents_stack()
    }

    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        None
    }
}

ferro_markup_type!(class TestServiceProvider {
    this: Rc<TestServiceProvider>,
    handles: [TestServiceProvider, Rc<TestServiceProvider>, Option<Rc<TestServiceProvider>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    interfaces: [
        Rc<dyn IServiceProvider>,
        Rc<dyn IUriContext>,
        Rc<dyn IFerroXamlIlEagerParentStackProvider>,
    ],
    constructors: [() => TestServiceProvider::new],
    properties: [
        BaseUri: Option<Uri> {
            get: |this: &Rc<TestServiceProvider>| this.base_uri(),
            set: |this: &Rc<TestServiceProvider>, value: Option<Uri>| this.set_base_uri(value)
        },
        ParentsStack: Rc<Vec<BoxedValue>> {
            get: |this: &Rc<TestServiceProvider>| this.parents_stack(),
            set: |this: &Rc<TestServiceProvider>, value: Rc<Vec<BoxedValue>>| this.set_parents_stack(value)
        },
    ],
});

// --- StyleWithServiceProvider ------------------------------------------------

/// A style whose content is the embedded document of its class: it keeps
/// the service provider it was created with and loads its document through
/// it.
#[repr(C)]
pub struct StyleWithServiceProvider {
    base: Style,
    service_provider: Option<Rc<dyn IServiceProvider>>,
}

ferro_class!(StyleWithServiceProvider: Style);
ferro_impl_classes!(StyleWithServiceProvider: FerroObjectImpl, StyleBaseImpl);
ferro_class_info!(StyleWithServiceProvider {
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
        constructors: [(Option<Rc<dyn IServiceProvider>>) => StyleWithServiceProvider::new],
        properties: [
            ServiceProvider: Option<Rc<dyn IServiceProvider>> { get: StyleWithServiceProvider::service_provider },
        ],
    },
});

impl StyleWithServiceProvider {
    pub fn new(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        let this = instantiate(Self { base: Style::construct(), service_provider: sp.clone() });
        let object: BoxedValue = Rc::new(this.clone());
        if let Err(error) = FerroXamlLoader::load_object_with_service_provider(sp.as_ref(), &object) {
            panic!("{error}");
        }
        this
    }

    pub fn service_provider(&self) -> Option<Rc<dyn IServiceProvider>> {
        self.service_provider.clone()
    }
}

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[StyleWithServiceProvider::TYPE],
    markup_types: &[<TestServiceProvider as MarkupTyped>::MARKUP],
    value_types: || {
        ValueTypes::register_reference::<TestServiceProvider>();
        ValueTypes::register_cast::<TestServiceProvider, Rc<dyn IServiceProvider>>(
            TestServiceProvider::as_service_provider,
        );
        ValueTypes::register_cast::<TestServiceProvider, Rc<dyn IUriContext>>(TestServiceProvider::as_uri_context);
        ValueTypes::register_cast::<TestServiceProvider, Rc<dyn IFerroXamlIlParentStackProvider>>(
            TestServiceProvider::as_parent_stack_provider,
        );
        ValueTypes::register_cast::<TestServiceProvider, Rc<dyn IFerroXamlIlEagerParentStackProvider>>(
            TestServiceProvider::as_eager_parent_stack_provider,
        );
    },
};
