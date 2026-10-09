//! Port of `HeadlessUnitTestSessionTests.cs` of the upstream unit test
//! project of the headless platform (tests of the NUnit projects, which do
//! not run on the session: they use it).

use super::block_on;
use super::test_application::{TestApplication, PER_ASSEMBLY, PER_TEST};
use crate::{FerroTestAssembly, HeadlessUnitTestSession};
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherPriority};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Duration;

fn session_should_support_immediate_disposal(asynchronously: bool) {
    for _ in 0..10000 {
        let session = HeadlessUnitTestSession::start_new(TestApplication::build_ferro_app);
        if asynchronously {
            block_on(session.dispose_async());
        } else {
            session.dispose();
        }
    }
}

mod session_should_support_immediate_disposal {
    #[test]
    fn synchronously() {
        super::session_should_support_immediate_disposal(false);
    }

    #[test]
    fn asynchronously() {
        super::session_should_support_immediate_disposal(true);
    }
}

/// `assembly` is the assembly of the test class in the original.
fn dispatch_should_report_cleanup_exceptions_and_continue(assembly: &'static FerroTestAssembly) {
    let session = HeadlessUnitTestSession::get_or_start_for_assembly(Some(assembly));

    const MESSAGE: &str = "Thrown by a dispatcher job during cleanup.";
    let poisoned_dispatch = session.dispatch(
        || Dispatcher::ui_thread().post(|| panic!("{}", MESSAGE), DispatcherPriority::DEFAULT),
        CancellationToken::none(),
    );

    let exception = catch_unwind(AssertUnwindSafe(|| poisoned_dispatch.wait_timeout(Duration::from_secs(10))))
        .expect_err("the cleanup of the dispatch panics");

    let message = exception
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| exception.downcast_ref::<&str>().copied());
    assert_eq!(Some(MESSAGE), message);

    let result = session.dispatch_result(|| 42, CancellationToken::none()).wait_timeout(Duration::from_secs(10));

    assert_eq!(Ok(42), result);
}

mod dispatch_should_report_cleanup_exceptions_and_continue {
    use super::{PER_ASSEMBLY, PER_TEST};

    #[test]
    fn per_test() {
        super::dispatch_should_report_cleanup_exceptions_and_continue(&PER_TEST);
    }

    #[test]
    fn per_assembly() {
        super::dispatch_should_report_cleanup_exceptions_and_continue(&PER_ASSEMBLY);
    }
}
