//! Ported from `Splat.Tests/ModeDetection/ModeTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is a `ModeDetectorScope` at the start of each
//! test; `Mode.Run` and `Mode.Test` are the constants `Mode::RUN` and `Mode::TEST`, set as an
//! `Rc<dyn IModeDetector>`.

use crate::tests::common::ModeDetectorScope;
use ferroui_splat::{Mode, ModeDetector};
use std::rc::Rc;

#[test]
fn run_mode_test() {
    let _scope = ModeDetectorScope::new();

    // Arrange
    ModeDetector::override_mode_detector(Rc::new(Mode::RUN));

    // Act
    let in_unit_test_runner = ModeDetector::in_unit_test_runner();

    // Assert
    assert!(!in_unit_test_runner);
}

#[test]
fn test_mode_test() {
    let _scope = ModeDetectorScope::new();

    // Arrange
    ModeDetector::override_mode_detector(Rc::new(Mode::TEST));

    // Act
    let in_unit_test_runner = ModeDetector::in_unit_test_runner();

    // Assert
    assert!(in_unit_test_runner);
}

#[test]
fn mode_detector_caches_results() {
    let _scope = ModeDetectorScope::new();

    // Arrange
    ModeDetector::override_mode_detector(Rc::new(Mode::TEST));

    // Act - Call multiple times
    let result1 = ModeDetector::in_unit_test_runner();
    let result2 = ModeDetector::in_unit_test_runner();
    let result3 = ModeDetector::in_unit_test_runner();

    // Assert - Should all be the same (cached)
    assert!(result1);
    assert!(result2);
    assert!(result3);
}

#[test]
fn mode_detector_override_clears_cache() {
    let _scope = ModeDetectorScope::new();

    // Arrange
    ModeDetector::override_mode_detector(Rc::new(Mode::TEST));
    let result1 = ModeDetector::in_unit_test_runner();
    assert!(result1);

    // Act - Override with different mode
    ModeDetector::override_mode_detector(Rc::new(Mode::RUN));
    let result2 = ModeDetector::in_unit_test_runner();

    // Assert - Should reflect new mode
    assert!(!result2);
}

// Not ported: ModeDetector_HandlesNullDetector - a null detector (`OverrideModeDetector(null!)`), which Rust's types make impossible
