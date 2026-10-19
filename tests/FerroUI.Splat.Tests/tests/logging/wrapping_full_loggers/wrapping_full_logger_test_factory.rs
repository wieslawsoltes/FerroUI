//! Ported from `Splat.Logging.Tests/WrappingFullLoggers/WrappingFullLoggerTestFactory.cs` of Splat.
//! Adapted: the logger and its target are handed out in `Rc`, the target being shared with the
//! wrapping logger. `AboveFatal` (`LogLevel.Fatal + 1`, a level that disables every level) is not
//! ported: `LogLevel` has no value above `Fatal`, so the tests that use it are not ported.

use crate::tests::common::TextLogger;
use ferroui_splat::{LogLevel, WrappingFullLogger};
use std::rc::Rc;

/// A `WrappingFullLogger` over a `TextLogger` at `level`, and that `TextLogger`, which records
/// what the wrapping logger forwards (`CreateLogger`).
pub(super) fn create_logger(level: LogLevel) -> (Rc<WrappingFullLogger>, Rc<TextLogger>) {
    let target = Rc::new(TextLogger::with_level(level));
    (Rc::new(WrappingFullLogger::new(target.clone())), target)
}
