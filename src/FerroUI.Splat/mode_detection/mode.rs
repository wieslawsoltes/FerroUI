//! `Mode`.

use super::IModeDetector;

/// A fixed mode (`Mode`): [`RUN`](Self::RUN) outside a test runner, [`TEST`](Self::TEST) under
/// one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    in_unit_test_runner: bool,
}

impl Mode {
    /// The mode of an application (`Mode.Run`).
    pub const RUN: Mode = Mode { in_unit_test_runner: false };

    /// The mode of unit tests (`Mode.Test`).
    pub const TEST: Mode = Mode { in_unit_test_runner: true };
}

impl IModeDetector for Mode {
    fn in_unit_test_runner(&self) -> Option<bool> {
        Some(self.in_unit_test_runner)
    }
}
