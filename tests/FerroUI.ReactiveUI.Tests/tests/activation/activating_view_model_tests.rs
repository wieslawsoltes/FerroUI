//! `ActivatingViewModelTests` (`ReactiveUI.Tests/Activation/ActivatingViewModelTests.cs`).

use super::{ActivatingViewModel, DerivedActivatingViewModel};
use ferroui_reactiveui::*;

#[test]
fn activations_get_ref_counted() {
    let fixture = ActivatingViewModel::new();
    assert_eq!(fixture.is_active_count(), 0);

    let _ = fixture.activator().activate();
    assert_eq!(fixture.is_active_count(), 1);

    let _ = fixture.activator().activate();
    assert_eq!(fixture.is_active_count(), 1);

    fixture.activator().deactivate();
    assert_eq!(fixture.is_active_count(), 1);

    // RefCount drops to zero
    fixture.activator().deactivate();
    assert_eq!(fixture.is_active_count(), 0);
}

#[test]
fn derived_activations_dont_get_stomped() {
    let fixture = DerivedActivatingViewModel::new();
    assert_eq!(fixture.is_active_count(), 0);
    assert_eq!(fixture.is_active_count_also(), 0);

    let _ = fixture.activator().activate();
    assert_eq!(fixture.is_active_count(), 1);
    assert_eq!(fixture.is_active_count_also(), 1);

    let _ = fixture.activator().activate();
    assert_eq!(fixture.is_active_count(), 1);
    assert_eq!(fixture.is_active_count_also(), 1);

    fixture.activator().deactivate();
    assert_eq!(fixture.is_active_count(), 1);
    assert_eq!(fixture.is_active_count_also(), 1);

    fixture.activator().deactivate();
    assert_eq!(fixture.is_active_count(), 0);
    assert_eq!(fixture.is_active_count_also(), 0);
}
