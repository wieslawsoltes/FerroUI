//! `BindingDirectionTests` (`ReactiveUI.Binding.Tests/Bindings/BindingDirectionTests.cs`).
//!
//! `Enum.IsDefined` has no counterpart: a Rust enum holds only its variants. The test checks the
//! values of both directions instead.

use ferroui_reactiveui::*;

#[test]
fn values_define_both_binding_directions() {
    assert_eq!(BindingDirection::OneWay as i32, 0);
    assert_eq!(BindingDirection::TwoWay as i32, 1);
    assert_eq!(BindingDirection::default(), BindingDirection::OneWay);
}
