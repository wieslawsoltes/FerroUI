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
    let session = SESSION.get_or_init(|| HeadlessUnitTestSession::start_new(builder));
    // A panic of the action happens on the thread of the session, which the first test started:
    // its message goes to the output of that test, and `wait` resumes it here without printing
    // it. Panic again on the thread of this test with the message, so that it is reported here.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.dispatch(action, CancellationToken::none()).wait()
    }));
    match outcome {
        Ok(result) => result.unwrap(),
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "a panic without a message".to_owned());
            panic!("the test panicked on the thread of the headless session: {message}");
        }
    }
}
