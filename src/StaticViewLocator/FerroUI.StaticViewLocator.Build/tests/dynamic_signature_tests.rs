//! `StaticViewLocatorGeneratorDynamicSignatureTests.cs`.

use super::*;

#[test]
fn dynamic_methods_are_treated_as_object_signatures() {
    // The reference treats `dynamic` parameters as `object`. Rust has no `dynamic`; the generator
    // does not compare parameter types (the compiler does), so members that take any type stand
    // for the adapter members: the data template the locator implements itself, and a hook that
    // takes the data as `&dyn Any`.
    let outcome = generated(
        "use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam};
use ferroui_controls::Control;
use std::any::Any;

#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generate_runtime_type_fallback_methods = false)]
pub struct ViewLocator;

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ViewLocator {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        None
    }
}

impl IDataTemplate for ViewLocator {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        false
    }
}

impl ViewLocator {
    fn build_fallback_view(&self, value: &dyn Any) -> Option<Ref<Control>> {
        None
    }
}
",
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(!source.contains("ITemplateWithParam<"), "{source}");
    assert!(!source.contains("IDataTemplate for ViewLocator"), "{source}");
    assert!(!source.contains("fn build_fallback_view("), "{source}");
}
