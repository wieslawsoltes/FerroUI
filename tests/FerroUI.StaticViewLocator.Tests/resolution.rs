//! The run-time half of `GeneratedResolverCoversAllPathsWithoutAllocatingOrConstructingDuringMatch`:
//! the generated data template resolves exact, inherited and trait view models through the table
//! of the views, `match_` creates no view and allocates nothing.
//!
//! The reference's generic view model is left out: a generated adapter reports an open generic view
//! model (SVL0008), which the tests of the generator check.

use crate::prelude::*;

pub mod view_models {
    use crate::prelude::*;

    #[repr(C)]
    pub struct RootModel {
        base: FerroObject,
    }

    ferro_class!(RootModel: FerroObject);
    ferro_impl_classes!(RootModel: FerroObjectImpl);

    impl RootModel {
        pub fn construct() -> Self {
            Self { base: FerroObject::construct() }
        }
    }

    #[repr(C)]
    pub struct DerivedModel {
        base: RootModel,
    }

    ferro_class!(DerivedModel: RootModel);
    ferro_impl_classes!(DerivedModel: FerroObjectImpl);

    impl DerivedModel {
        pub fn new() -> Ref<Self> {
            instantiate(Self { base: RootModel::construct() })
        }
    }

    pub trait IContractModel {}

    #[derive(PartialEq)]
    pub struct ContractModel;

    impl IContractModel for ContractModel {}

    #[derive(PartialEq)]
    pub struct ExplicitModel;

    #[derive(PartialEq)]
    pub struct FilteredModel;

    #[derive(PartialEq)]
    pub struct UnresolvedModel;
}

pub mod views {
    use crate::prelude::*;

    test_view!(RootView);
    test_view!(ContractView);
    test_view!(ExplicitOverrideView);
    test_view!(FilteredView);
}

#[static_view_locator(
    generate_data_template = true,
    generated_adapter_resolution_mode = ExactThenBaseTypesThenInterfaces,
    data_template_match_types = [view_models::RootModel, view_models::IContractModel, view_models::ExplicitModel],
)]
#[static_view_mapping(view_models::RootModel, views::RootView)]
#[static_view_mapping(view_models::IContractModel, views::ContractView)]
#[static_view_mapping(view_models::ExplicitModel, views::ExplicitOverrideView)]
#[static_view_mapping(view_models::FilteredModel, views::FilteredView)]
#[derive(Default)]
pub struct ResolutionViewLocator;

impl ResolutionViewLocator {
    /// The allocations of `count` calls of `match_` with `value`, after as many to warm up
    /// (`MeasureMatch`).
    #[cfg(test)]
    fn measure_match(&self, value: &BoxedValue, count: usize) -> u64 {
        use ferroui_controls::templates::IDataTemplate;
        for _ in 0..count {
            std::hint::black_box(self.match_(Some(value)));
        }
        let before = crate::support::allocations::current();
        for _ in 0..count {
            std::hint::black_box(self.match_(Some(value)));
        }
        crate::support::allocations::current() - before
    }
}

#[cfg(test)]
mod tests {
    use super::view_models::{ContractModel, DerivedModel, ExplicitModel, FilteredModel, UnresolvedModel};
    use super::ResolutionViewLocator;
    use crate::support::{class_name, start, value};
    use ferroui_base::{BoxedValue, FerroObject};
    use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
    use std::rc::Rc;

    #[test]
    fn generated_resolver_covers_all_paths_without_allocating_or_constructing_during_match() {
        let _app = start();
        let locator = ResolutionViewLocator;
        let exact = value(ExplicitModel);
        let inherited: BoxedValue = Rc::new(DerivedModel::new().upcast::<FerroObject>());
        let through_interface = value(ContractModel);
        let filtered = value(FilteredModel);
        let unresolved = value(UnresolvedModel);

        let constructed = crate::constructed();
        assert!(locator.match_(Some(&exact)));
        assert!(locator.match_(Some(&inherited)));
        assert!(locator.match_(Some(&through_interface)));
        assert!(!locator.match_(Some(&filtered)));
        assert!(!locator.match_(Some(&unresolved)));
        assert!(!locator.match_(None));
        assert_eq!(crate::constructed(), constructed, "match_ creates no view");

        assert_eq!(locator.measure_match(&exact, 10_000), 0);
        assert_eq!(locator.measure_match(&unresolved, 10_000), 0);
        assert_eq!(crate::constructed(), constructed, "match_ creates no view");

        let view = |data: &BoxedValue| class_name(&locator.build(&Some(data.clone())).expect("a view"));
        assert_eq!(view(&inherited), "RootView");
        assert_eq!(view(&through_interface), "ContractView");
        assert_eq!(view(&exact), "ExplicitOverrideView");
        assert_eq!(view(&unresolved), "TextBlock");
    }
}
