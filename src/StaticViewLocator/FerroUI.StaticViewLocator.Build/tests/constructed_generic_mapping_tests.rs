//! `StaticViewLocatorGeneratorConstructedGenericMappingTests.cs`.

use super::*;

#[test]
fn reactive_ui_mapping_preserves_constructed_generic_view_model_arguments() {
    let outcome = generated(&format!(
        "{VIEW_IMPORTS}
use ferroui_reactiveui_controls::IControlViewFor;

pub struct Item;
pub struct GenericViewModel<T>(T);

{}
impl IControlViewFor for ClosedGenericScreen {{
    type ViewModel = GenericViewModel<Item>;
}}

#[ferroui_static_view_locator::static_view_locator(generate_view_locator = true, generate_runtime_type_fallback_methods = false)]
pub struct ViewLocator;
",
        view("ClosedGenericScreen")
    ));
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains(&view_entry("crate::GenericViewModel<crate::Item>", "crate::ClosedGenericScreen::new")), "{source}");
}
