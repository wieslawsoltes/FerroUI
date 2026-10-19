//! Mode detection (`Splat/ModeDetection`): whether the code runs under a unit test runner.

mod default_mode_detector;
mod mode;
mod mode_detector;

pub use default_mode_detector::DefaultModeDetector;
pub use mode::Mode;
pub use mode_detector::ModeDetector;

/// Detects whether the code runs under a unit test runner (`IModeDetector`).
pub trait IModeDetector {
    /// `Some(true)` under a unit test runner, `Some(false)` outside one, `None` when it cannot
    /// tell (`InUnitTestRunner`).
    fn in_unit_test_runner(&self) -> Option<bool>;
}
