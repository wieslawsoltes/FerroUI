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
