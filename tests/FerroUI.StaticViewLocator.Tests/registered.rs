//! `CreatesRegisteredViewInstances`: a legacy locator creates the view of a view model with a view
//! and a text block for one without.

use crate::prelude::*;

pub mod view_models {
    #[derive(PartialEq)]
    pub struct SampleViewModel;

    #[derive(PartialEq)]
    pub struct MissingViewModel;
}

pub mod views {
    use crate::prelude::*;

    test_view!(SampleView);
}

#[static_view_locator]
#[derive(Default)]
pub struct RegisteredViewLocator;

#[cfg(test)]
mod tests {
    use super::view_models::{MissingViewModel, SampleViewModel};
    use super::RegisteredViewLocator;
    use crate::support::{class_name, start, value};

    #[test]
    fn creates_registered_view_instances() {
        let _app = start();
        let locator = RegisteredViewLocator;

        let sample = locator.build(Some(&value(SampleViewModel))).expect("a view");
        let missing = locator.build(Some(&value(MissingViewModel))).expect("a view");

        assert_eq!(class_name(&sample), "SampleView");
        assert_eq!(class_name(&missing), "TextBlock");
        assert!(locator.build(None).is_none());
    }
}
