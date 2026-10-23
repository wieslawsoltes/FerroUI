//! `ResolvesInterfaceMappingsByStrippingConfiguredPrefix`: the view of a trait view model is found
//! by its name without the prefix `I`, and a view model that implements the trait gets it.

use crate::prelude::*;

pub mod view_models {
    pub trait IDetailsViewModel {}

    #[derive(PartialEq)]
    pub struct ConcreteDetailsModel;

    impl IDetailsViewModel for ConcreteDetailsModel {}
}

pub mod views {
    use crate::prelude::*;

    test_view!(DetailsView);
}

#[static_view_locator]
#[derive(Default)]
pub struct InterfaceViewLocator;

#[cfg(test)]
mod tests {
    use super::view_models::{ConcreteDetailsModel, IDetailsViewModel};
    use super::InterfaceViewLocator;
    use crate::support::{class_name, start, value};
    use std::any::TypeId;

    #[test]
    fn resolves_interface_mappings_by_stripping_configured_prefix() {
        let _app = start();
        let control = InterfaceViewLocator.build(Some(&value(ConcreteDetailsModel))).expect("a view");
        assert_eq!(class_name(&control), "DetailsView");
        // The trait itself is a key of the table, as `dyn IDetailsViewModel`.
        assert!(InterfaceViewLocator::s_views().contains(TypeId::of::<dyn IDetailsViewModel>()));
    }
}
