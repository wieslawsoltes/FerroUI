//! `InferMappingFromConfiguredGenericContractAndGeneratesFactoryMethod`, compiled: a view states
//! its model through a contract of the locator, and the exact factory creates it.

use crate::prelude::*;

pub mod models {
    #[derive(PartialEq)]
    pub struct DashboardModel;
}

pub mod framework {
    /// A contract of a view for its model (`IViewFor<TViewModel>` of an MVVM framework).
    pub trait IViewFor<VM> {}
}

pub mod screens {
    use crate::prelude::*;

    test_view!(DashboardScreen);

    impl super::framework::IViewFor<super::models::DashboardModel> for DashboardScreen {}
}

#[static_view_locator(
    generate_runtime_type_fallback_methods = false,
    generate_view_factory_methods = true,
    view_model_mapping_contracts = [framework::IViewFor],
)]
#[derive(Default)]
pub struct FactoryViewLocator;

impl FactoryViewLocator {
    pub fn build(&self, data: Option<&BoxedValue>) -> Option<Ref<Control>> {
        Self::try_create_view_exact(ferroui_static_view_locator::RuntimeType::of_value(data?).id())
    }
}

#[cfg(test)]
mod tests {
    use super::models::DashboardModel;
    use super::FactoryViewLocator;
    use crate::support::{class_name, start, value};
    use std::any::TypeId;

    #[test]
    fn infers_mapping_from_configured_generic_contract_and_generates_factory_method() {
        let _app = start();
        let view = FactoryViewLocator::try_create_view_exact(TypeId::of::<DashboardModel>()).expect("a view");
        assert_eq!(class_name(&view), "DashboardScreen");
        let view = FactoryViewLocator.build(Some(&value(DashboardModel))).expect("a view");
        assert_eq!(class_name(&view), "DashboardScreen");
        assert!(FactoryViewLocator::try_create_view_exact(TypeId::of::<String>()).is_none());
    }
}
