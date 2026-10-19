//! `AppBuilderExtensionsOverloadsTests` (`AppBuilderExtensionsOverloadsTests.cs`): the overloads
//! of the extensions of the application builder.

use super::view_support::{same_builder, start};
use crate::AppBuilderExtensions;
use ferroui_controls::{AppBuilder, Application};

// Not ported: UseReactiveUI_WithBuilderOverload_ThrowsOnNulls - argument validation (the builder
// and the callback are values, never null).
// Not ported: RegisterReactiveUIViews_ThrowsOnNullBuilder - argument validation.

#[test]
fn register_reactive_ui_views_returns_builder_no_assemblies_no_throw() {
    let _app = start();
    let builder = AppBuilder::configure::<Application>();

    let result = builder.register_reactive_ui_views(Vec::new());

    assert!(same_builder(&result, &builder));
}
