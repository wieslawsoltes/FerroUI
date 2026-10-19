//! `AppBuilderExtensionsViewsApiTests` (`AppBuilderExtensionsViewsApiTests.cs`): the views API of
//! the extensions of the application builder.

use super::view_support::{same_builder, start};
use crate::AppBuilderExtensions;
use ferroui_controls::{AppBuilder, Application};

#[test]
fn register_reactive_ui_views_returns_same_builder_and_handles_null_or_empty() {
    let _app = start();
    let builder = AppBuilder::configure::<Application>();

    let result = builder.register_reactive_ui_views(Vec::new());

    assert!(same_builder(&result, &builder));
}

// Not ported: RegisterReactiveUIViewsFromAssemblyOf_Registers_Types - reflection (the assembly of a
// type, scanned for its views).
// Not ported: RegisterReactiveUIViewsFromEntryAssembly_Returns_Same_Builder - reflection (the entry
// assembly of the process); the overload that takes the entry assembly is tested by
// `AppBuilderExtensionsRegistrationTests`.
