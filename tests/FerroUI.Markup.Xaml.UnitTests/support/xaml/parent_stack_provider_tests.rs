//! The test types declared by the upstream test file `Xaml/ParentStackProviderTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::Colors;
use ferroui_base::metadata::{IServiceProvider, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroLocator, LocatorExtensions};
use ferroui_markup_xaml::xaml_il::runtime::{IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider};
use ferroui_markup_xaml::{EagerParentStackEnumerator, ServiceProviderExtensions};

use crate::support::TypeModule;

// --- CapturedParents ---------------------------------------------------------

/// What [`CapturingParentsMarkupExtension`] saw: the parents as the parent
/// stack provider lists them and as the eager enumeration yields them.
pub struct CapturedParents {
    lazy_parents: RefCell<Option<Vec<BoxedValue>>>,
    eager_parents: RefCell<Option<Vec<BoxedValue>>>,
}

crate::test_identity_eq!(CapturedParents);

impl CapturedParents {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { lazy_parents: RefCell::new(None), eager_parents: RefCell::new(None) })
    }

    pub fn lazy_parents(&self) -> Option<Vec<BoxedValue>> {
        self.lazy_parents.borrow().clone()
    }

    pub fn set_lazy_parents(&self, value: Option<Vec<BoxedValue>>) {
        *self.lazy_parents.borrow_mut() = value;
    }

    pub fn eager_parents(&self) -> Option<Vec<BoxedValue>> {
        self.eager_parents.borrow().clone()
    }

    pub fn set_eager_parents(&self, value: Option<Vec<BoxedValue>>) {
        *self.eager_parents.borrow_mut() = value;
    }
}

ferro_markup_type!(class CapturedParents {
    this: Rc<CapturedParents>,
    handles: [CapturedParents, Rc<CapturedParents>, Option<Rc<CapturedParents>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => CapturedParents::new],
    properties: [
        LazyParents: Option<Vec<BoxedValue>> {
            get: |this: &Rc<CapturedParents>| this.lazy_parents(),
            set: |this: &Rc<CapturedParents>, value: Option<Vec<BoxedValue>>| this.set_lazy_parents(value)
        },
        EagerParents: Option<Vec<BoxedValue>> {
            get: |this: &Rc<CapturedParents>| this.eager_parents(),
            set: |this: &Rc<CapturedParents>, value: Option<Vec<BoxedValue>>| this.set_eager_parents(value)
        },
    ],
});

// --- CapturingParentsMarkupExtension -----------------------------------------

/// A markup extension that records the parents of the place it is used at
/// in the [`CapturedParents`] service, and provides a color.
pub struct CapturingParentsMarkupExtension;

crate::test_identity_eq!(CapturingParentsMarkupExtension);

impl CapturingParentsMarkupExtension {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Option<BoxedValue> {
        let parents_provider = service_provider.get_required_service::<Rc<dyn IFerroXamlIlParentStackProvider>>();
        let eager_parents_provider = parents_provider
            .clone()
            .as_eager_parent_stack_provider()
            .expect("the parent stack provider is not an eager parent stack provider");

        let captured_parents = FerroLocator::current().get_required_service::<CapturedParents>();
        captured_parents.set_lazy_parents(Some(parents_provider.parents()));
        captured_parents.set_eager_parents(Some(Self::enumerate_eager_parents(eager_parents_provider)));

        Some(Rc::new(Colors::BLUE))
    }

    fn enumerate_eager_parents(provider: Rc<dyn IFerroXamlIlEagerParentStackProvider>) -> Vec<BoxedValue> {
        let mut parents = Vec::new();

        let mut enumerator = EagerParentStackEnumerator::new(Some(provider));
        while let Some(parent) = enumerator.try_get_next() {
            parents.push(parent);
        }

        parents
    }
}

ferro_markup_type!(class CapturingParentsMarkupExtension {
    this: Rc<CapturingParentsMarkupExtension>,
    handles: [
        CapturingParentsMarkupExtension,
        Rc<CapturingParentsMarkupExtension>,
        Option<Rc<CapturingParentsMarkupExtension>>,
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => CapturingParentsMarkupExtension::new],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |this: &Rc<CapturingParentsMarkupExtension>, service_provider: Rc<dyn IServiceProvider>| {
                this.provide_value(&service_provider)
            },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[
        <CapturedParents as MarkupTyped>::MARKUP,
        <CapturingParentsMarkupExtension as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<CapturedParents>();
        ValueTypes::register_reference::<CapturingParentsMarkupExtension>();
    },
};
