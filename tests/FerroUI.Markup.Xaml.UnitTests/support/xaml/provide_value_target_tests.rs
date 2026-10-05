//! The test types declared by the upstream test file `Xaml/ProvideValueTargetTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::metadata::{IServiceProvider, MarkupTyped};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroLocator, LocatorExtensions};
use ferroui_markup_xaml::{IProvideValueTarget, ServiceProviderExtensions};

use crate::support::TypeModule;

/// A captured target: the target object and the target property.
pub type CapturedTarget = (Option<BoxedValue>, Option<BoxedValue>);

// --- CapturedTargets ---------------------------------------------------------

/// What [`CapturingTargetsMarkupExtension`] saw: the target object and the
/// target property of each place it was used at, in order.
pub struct CapturedTargets {
    targets: RefCell<Vec<CapturedTarget>>,
}

crate::test_identity_eq!(CapturedTargets);

impl CapturedTargets {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { targets: RefCell::new(Vec::new()) })
    }

    pub fn targets(&self) -> Vec<CapturedTarget> {
        self.targets.borrow().clone()
    }

    pub fn add(&self, target: CapturedTarget) {
        self.targets.borrow_mut().push(target);
    }
}

ferro_markup_type!(class CapturedTargets {
    this: Rc<CapturedTargets>,
    handles: [CapturedTargets, Rc<CapturedTargets>, Option<Rc<CapturedTargets>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => CapturedTargets::new],
    properties: [
        Targets: Vec<CapturedTarget> { get: |this: &Rc<CapturedTargets>| this.targets() },
    ],
});

// --- CapturingTargetsMarkupExtension -----------------------------------------

/// A markup extension that records what its value is provided for in the
/// [`CapturedTargets`] service, and provides a brush.
pub struct CapturingTargetsMarkupExtension;

crate::test_identity_eq!(CapturingTargetsMarkupExtension);

impl CapturingTargetsMarkupExtension {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Option<BoxedValue> {
        let parents_provider = service_provider.get_required_service::<Rc<dyn IProvideValueTarget>>();
        let captured_targets = FerroLocator::current().get_required_service::<CapturedTargets>();
        captured_targets.add((parents_provider.target_object(), parents_provider.target_property()));
        let brush: Rc<dyn IBrush> = Brushes::dark_violet();
        Some(Rc::new(brush))
    }
}

ferro_markup_type!(class CapturingTargetsMarkupExtension {
    this: Rc<CapturingTargetsMarkupExtension>,
    handles: [
        CapturingTargetsMarkupExtension,
        Rc<CapturingTargetsMarkupExtension>,
        Option<Rc<CapturingTargetsMarkupExtension>>,
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => CapturingTargetsMarkupExtension::new],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |this: &Rc<CapturingTargetsMarkupExtension>, service_provider: Rc<dyn IServiceProvider>| {
                this.provide_value(&service_provider)
            },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[],
    markup_types: &[
        <CapturedTargets as MarkupTyped>::MARKUP,
        <CapturingTargetsMarkupExtension as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<CapturedTargets>();
        ValueTypes::register_reference::<CapturingTargetsMarkupExtension>();
    },
};
