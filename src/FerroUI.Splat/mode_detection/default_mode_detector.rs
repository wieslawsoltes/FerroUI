//! `DefaultModeDetector`.

use super::IModeDetector;
use crate::logging::IEnableLogger;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// The names of the processes of known test hosts.
const KNOWN_TEST_HOST_PROCESSES: &[&str] = &[
    "testhost",
    "testhost.x86",
    "vstest.console",
    "vstest.executionengine",
    "xunit.console",
    "nunit3-console",
    "nunit-console",
    "nunit-agent",
    "resharpertestrunner",
    "jetbrains.resharper.taskrunner.clr45",
];

/// The variables of the environment whose presence means a test runner.
const EXACT_ENV_VARS: &[&str] = &["XUNIT_TEST", "NUNIT_TEST"];

/// The prefixes of the names of variables of the environment that test runners set.
const ENV_PREFIXES: &[&str] = &["VSTEST_", "NUNIT_", "XUNIT_"];

/// The detector of a unit test runner that [`ModeDetector`] starts with (`DefaultModeDetector`).
///
/// In order, the first signal wins:
/// - the variable `DOTNET_RUNNING_IN_TEST` set to `1`, `yes` or `true`;
/// - a variable of the environment that a test runner sets (`XUNIT_TEST`, `NUNIT_TEST`, or a
///   name starting with `VSTEST_`, `NUNIT_` or `XUNIT_`);
/// - the name of the executable of a known test host.
///
/// [`ModeDetector`]: super::ModeDetector
// Deviation (DEVIATIONS.md, Splat): the last check of upstream, the scan of the loaded assemblies
// for the names of test frameworks, has no counterpart without reflection; the `AppContext` switch
// and the main module of a `dotnet` host are .NET's.
#[derive(Default)]
pub struct DefaultModeDetector;

impl DefaultModeDetector {
    /// The detector.
    pub fn new() -> Self {
        Self
    }

    /// Whether `name` contains one of `markers`, ignoring case (`NameContainsAnyMarker`). For the
    /// tests of the crate.
    #[doc(hidden)]
    pub fn name_contains_any_marker(name: Option<&str>, markers: &[&str]) -> bool {
        let Some(name) = name.filter(|name| !name.is_empty()) else {
            return false;
        };
        let name = name.to_lowercase();
        markers.iter().any(|marker| name.contains(&marker.to_lowercase()))
    }

    fn has_explicit_test_signal() -> bool {
        let Ok(value) = std::env::var("DOTNET_RUNNING_IN_TEST") else {
            return false;
        };
        if value.trim().is_empty() {
            return false;
        }
        value == "1" || value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("true")
    }

    fn scan_environment_for_test_runner_signals() -> bool {
        if EXACT_ENV_VARS
            .iter()
            .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
        {
            return true;
        }
        std::env::vars_os().any(|(key, _)| {
            let Some(key) = key.to_str() else {
                return false;
            };
            !key.is_empty()
                && ENV_PREFIXES.iter().any(|prefix| {
                    key.get(..prefix.len()).is_some_and(|start| start.eq_ignore_ascii_case(prefix))
                })
        })
    }

    fn is_known_test_host_process() -> bool {
        let Ok(executable) = std::env::current_exe() else {
            return false;
        };
        let process_name = executable.file_stem().and_then(|stem| stem.to_str()).map(str::to_owned);
        Self::name_contains_any_marker(process_name.as_deref(), KNOWN_TEST_HOST_PROCESSES)
    }

    fn detect_unit_test_runner_safely(&self) -> Option<bool> {
        let detected = catch_unwind(AssertUnwindSafe(|| {
            Self::has_explicit_test_signal()
                || Self::scan_environment_for_test_runner_signals()
                || Self::is_known_test_host_process()
        }));
        match detected {
            Ok(detected) => Some(detected),
            Err(_) => {
                let error = DetectionError;
                self.log().debug_exception(&error, "Unable to find unit test runner value");
                None
            }
        }
    }
}

/// The failure of a detection, for the log.
#[derive(Debug)]
struct DetectionError;

impl std::fmt::Display for DetectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the detection of a unit test runner panicked")
    }
}

impl std::error::Error for DetectionError {}

impl IEnableLogger for DefaultModeDetector {}

impl IModeDetector for DefaultModeDetector {
    fn in_unit_test_runner(&self) -> Option<bool> {
        self.detect_unit_test_runner_safely()
    }
}
