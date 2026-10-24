//! `CustomBuildCanCallGeneratedHelpers`: a locator that declares its `build` calls the legacy
//! helpers the generator writes for it.

use crate::prelude::*;
use ferroui_static_view_locator::RuntimeType;

pub mod view_models {
    #[derive(PartialEq)]
    pub struct CustomSampleViewModel;
}

pub mod views {
    use crate::prelude::*;

    test_view!(CustomSampleView);
}

#[static_view_locator]
#[derive(Default)]
pub struct CustomBuildViewLocator;

impl CustomBuildViewLocator {
    pub fn build(&self, data: Option<&BoxedValue>) -> Option<Ref<Control>> {
        let type_ = RuntimeType::of_value(data?);
        let factory = Self::try_get_factory(Some(&type_)).or_else(|| Self::try_get_factory_from_interfaces(&type_));
        factory.map(|factory| factory())
    }
}

#[cfg(test)]
mod tests {
    use super::view_models::CustomSampleViewModel;
    use super::CustomBuildViewLocator;
    use crate::support::{class_name, start, value};

    #[test]
    fn custom_build_can_call_generated_helpers() {
        let _app = start();
        let control = CustomBuildViewLocator.build(Some(&value(CustomSampleViewModel))).expect("a view");
        assert_eq!(class_name(&control), "CustomSampleView");
    }
}
