//! Ported from `Splat.Tests/ModeDetection/DefaultModeDetectorTests.cs` of Splat.
//! Adapted: `IsAssignableTo<IModeDetector>` and `IsAssignableTo<IEnableLogger>` are checks of trait
//! bounds at compile time. `NameContainsAnyMarker` takes no `StringComparison` (it always ignores
//! case); its `[Arguments]` rows are the cases of one test. The port has no scan of the loaded
//! assemblies (reflection), which is how upstream detects TUnit, and `cargo test` sets none of the
//! signals the port reads (`DOTNET_RUNNING_IN_TEST`, the variables of test runners, the name of a
//! test host), so the tests that expect the runner to be detected are not ported. The tests that
//! set variables of the environment are not ported either: the test harness runs the tests on
//! threads of one process, which share the environment, so a variable set by one test changes
//! what the detectors of the others see.

use crate::tests::common::panics;
use ferroui_splat::{DefaultModeDetector, IEnableLogger, IModeDetector};

fn assert_mode_detector<T: IModeDetector>(_detector: &T) {}

fn assert_enable_logger<T: IEnableLogger>(_detector: &T) {}

#[test]
fn default_mode_detector_implements_i_mode_detector() {
    // Arrange & Act
    let detector = DefaultModeDetector::new();

    // Assert
    assert_mode_detector(&detector);
}

#[test]
fn default_mode_detector_implements_i_enable_logger() {
    // Arrange & Act
    let detector = DefaultModeDetector::new();

    // Assert
    assert_enable_logger(&detector);
}

#[test]
fn default_mode_detector_handles_exceptions_gracefully() {
    // Arrange
    let detector = DefaultModeDetector::new();

    // Act & Assert - should not panic
    assert!(!panics(|| {
        let _ = detector.in_unit_test_runner();
    }));
}

#[test]
fn default_mode_detector_returns_consistent_results() {
    // Arrange
    let detector = DefaultModeDetector::new();

    // Act
    let result1 = detector.in_unit_test_runner();
    let result2 = detector.in_unit_test_runner();
    let result3 = detector.in_unit_test_runner();

    // Assert - Should return consistent results
    assert_eq!(result2, result1);
    assert_eq!(result3, result2);
}

#[test]
fn name_contains_any_marker_matches_expected() {
    let markers = ["testhost", "vstest"];
    let cases: [(Option<&str>, bool); 5] = [
        (None, false),
        (Some(""), false),
        (Some("just-an-ordinary-app"), false),
        (Some("MyTestHostRunner"), true),
        (Some("some/path/to/VSTEST.console"), true),
    ];

    for (name, expected) in cases {
        let actual = DefaultModeDetector::name_contains_any_marker(name, &markers);

        assert_eq!(actual, expected, "{name:?}");
    }
}

// Not ported: DefaultModeDetector_CanDetectUnitTestRunner - upstream detects TUnit by scanning the loaded assemblies (reflection), which the port has not; cargo test sets none of the signals the port reads
// Not ported: DefaultModeDetector_ExplicitEnvVar_DotnetRunningInTest_TrueVariants - sets a process environment variable, shared by the tests that run in parallel threads of one process
// Not ported: DefaultModeDetector_AppContext_DotnetRunningInTest_ReturnsTrue - the AppContext switch is .NET's, and the test sets a process environment variable
// Not ported: DefaultModeDetector_ExactEnvVar_NUnitTest_ReturnsTrue - sets a process environment variable, shared by the tests that run in parallel threads of one process
// Not ported: DefaultModeDetector_EnvPrefix_VSTEST_ReturnsTrue - sets a process environment variable, shared by the tests that run in parallel threads of one process
// Not ported: DefaultModeDetector_ExplicitEnvVar_NonAffirmativeValues_StillDetectViaRunner - sets a process environment variable, and relies on the scan of the loaded assemblies (reflection)
// Not ported: DefaultModeDetector_DetectsMicrosoftTestingPlatform - reflection over the loaded assemblies (Microsoft.Testing.Platform)
// Not ported: DefaultModeDetector_IncludesMTPAssemblyMarker - reflection over the loaded assemblies (Microsoft.Testing.Platform)
