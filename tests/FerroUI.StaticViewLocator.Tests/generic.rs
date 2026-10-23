//! `ResolvesGenericViewModelsUsingGenericTypeDefinition`: the legacy fallback finds the view of a
//! closed generic view model through its generic type definition.
//!
//! The reference maps `WidgetViewModel<T>` by the conventions. The other locators of this crate
//! generate adapters, which report an open generic view model they cannot resolve (SVL0008); the
//! generic type of this module is mapped explicitly, by this locator only, and is named so that
//! the conventions do not take it.

use crate::prelude::*;

#[derive(PartialEq)]
pub struct WidgetModel<T>(pub T);

pub mod views {
    use crate::prelude::*;

    test_view!(WidgetView);
}

#[static_view_locator]
#[static_view_mapping(WidgetModel, views::WidgetView)]
#[derive(Default)]
pub struct GenericViewLocator;

#[cfg(test)]
mod tests {
    use super::{GenericViewLocator, WidgetModel};
    use crate::support::{class_name, start, value};

    #[test]
    fn resolves_generic_view_models_using_generic_type_definition() {
        let _app = start();
        let locator = GenericViewLocator;

        let control = locator.build(Some(&value(WidgetModel(1_i32)))).expect("a view");
        assert_eq!(class_name(&control), "WidgetView");
        let control = locator.build(Some(&value(WidgetModel("text")))).expect("a view");
        assert_eq!(class_name(&control), "WidgetView");
    }
}
