//! One UI owner for unit tests, using the pinned framework's actual isolation/session machinery.
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use ferroui_system::threading::CancellationToken;
use std::sync::{Arc, OnceLock};
fn builder() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
pub fn run(action: impl FnOnce() + Send + 'static) {
    static SESSION: OnceLock<Arc<HeadlessUnitTestSession>> = OnceLock::new();
    SESSION
        .get_or_init(|| HeadlessUnitTestSession::start_new(builder))
        .dispatch(action, CancellationToken::none())
        .wait()
        .unwrap();
}
