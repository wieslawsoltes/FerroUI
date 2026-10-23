//! `CovariantBuildHookReturnTypeCompiles`: a hook of the locator returns a type that converts into
//! the result of the generated `build` (`Ref<Control>`, where the generated hook returns
//! `Option<Ref<Control>>`).

use crate::prelude::*;
use ferroui_controls::Border;

#[static_view_locator(generate_data_template = true, generate_runtime_type_fallback_methods = false)]
#[derive(Default)]
pub struct CovariantViewLocator;

impl CovariantViewLocator {
    fn build_resolved_view(&self, _param: Option<&BoxedValue>) -> Ref<Control> {
        Border::new().upcast()
    }
}

#[cfg(test)]
mod tests {
    use super::CovariantViewLocator;
    use crate::support::{class_name, start, value};
    use ferroui_controls::templates::ITemplateWithParam;

    #[test]
    fn covariant_build_hook_return_type_compiles() {
        let _app = start();
        let view = CovariantViewLocator.build(&Some(value(1_i32))).expect("a view");
        assert_eq!(class_name(&view), "Border");
    }
}
