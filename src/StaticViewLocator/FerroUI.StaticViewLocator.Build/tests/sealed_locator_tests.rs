//! `StaticViewLocatorGeneratorSealedLocatorTests.cs`.

use super::*;

#[test]
fn sealed_locator_compiles_with_generated_data_template_hooks() {
    // A Rust struct has no subclasses: the generated hooks are private methods of the locator
    // (the defaults of a sealed C# locator), replaced by declaring a method of the name.
    let outcome = generated(
        "#[ferroui_static_view_locator::static_view_locator(generate_data_template = true, generate_runtime_type_fallback_methods = false)]
pub struct ViewLocator;
",
    );
    let source = file(&outcome, "ViewLocator.rs");
    assert!(source.contains("    fn build_resolved_view(&self, param: "), "{source}");
    assert!(!source.contains("pub fn build_resolved_view"), "{source}");
    assert!(!source.contains("pub fn build_fallback_view"), "{source}");
}
