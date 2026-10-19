//! `ReactiveUIBuilderRxAppMigrationTests` (`ReactiveUI.Builder.Tests/ReactiveUIBuilderRxAppMigrationTests.cs`).
//!
//! `IsTypeOf<SuspensionHost<TestAppState>>()`: the suspension host is a trait object, whose
//! concrete type cannot be read without reflection; the tests that check only that type are not
//! ported, and the test that chains all the methods checks the rest.

use super::executors::{configure_builder, execute_test, reset_state};
use ferroui_reactive::Observer;
use ferroui_reactiveui::*;
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// `CustomSmallCacheSize`.
const CUSTOM_SMALL_CACHE_SIZE: usize = 128;
/// `CustomBigCacheSize`.
const CUSTOM_BIG_CACHE_SIZE: usize = 512;
/// `ChainedSmallCacheSize`.
const CHAINED_SMALL_CACHE_SIZE: usize = 100;
/// `ChainedBigCacheSize`.
const CHAINED_BIG_CACHE_SIZE: usize = 400;
/// `LastSmallCacheSize`.
const LAST_SMALL_CACHE_SIZE: usize = 300;
/// `LastBigCacheSize`.
const LAST_BIG_CACHE_SIZE: usize = 600;
/// `InvalidCacheSize`.
const INVALID_CACHE_SIZE: usize = 0;
/// `ValidCacheSize`.
const VALID_CACHE_SIZE: usize = 100;
/// `MobileSmallCacheDefault`.
#[cfg(any(target_os = "android", target_os = "ios"))]
const SMALL_CACHE_DEFAULT: usize = 32;
/// `MobileBigCacheDefault`.
#[cfg(any(target_os = "android", target_os = "ios"))]
const BIG_CACHE_DEFAULT: usize = 64;
/// `DesktopSmallCacheDefault`.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const SMALL_CACHE_DEFAULT: usize = 64;
/// `DesktopBigCacheDefault`.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const BIG_CACHE_DEFAULT: usize = 256;

ferroui_system::ferro_thread_local! {
    /// `CapturedEx` of the executors that set one handler.
    static CAPTURED: RefCell<Option<ObservableError>> = const { RefCell::new(None) };
    /// `FirstCaptured` of `WithMultipleExceptionHandlersExecutor`.
    static FIRST_CAPTURED: RefCell<Option<ObservableError>> = const { RefCell::new(None) };
}

/// An error of the tests (`InvalidOperationException`).
#[derive(Debug)]
struct InvalidOperationException(&'static str);

impl fmt::Display for InvalidOperationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for InvalidOperationException {}

/// `TestAppState`.
#[allow(dead_code)]
struct TestAppState {
    name: Option<String>,
    counter: i32,
}

/// `RxAppMigrationTestExecutor.ResetState`: also the cache sizes, the handler of the errors and
/// the suspension host.
fn reset_migration_state() {
    reset_state();
    RxCacheSize::reset_for_testing();
    RxState::reset_for_testing();
    RxSuspension::reset_for_testing();
}

/// Runs `action` with the state reset and the builder configured by `configure`
/// (`RxAppMigrationTestExecutor` and its subclasses).
fn migration_test(configure: impl FnOnce(), action: impl FnOnce()) {
    execute_test(reset_migration_state, configure, action);
}

fn captured() -> Option<ObservableError> {
    CAPTURED.with(|captured| captured.borrow().clone())
}

/// `Witness.Create<Exception>(ex => CapturedEx = ex)`.
fn capturing_handler() -> Rc<dyn IObserver<ObservableError>> {
    Observer::create(|error: ObservableError| CAPTURED.with(|captured| *captured.borrow_mut() = Some(error)))
}

fn same_error(left: &Option<ObservableError>, right: &ObservableError) -> bool {
    left.as_ref().is_some_and(|left| Rc::ptr_eq(left, right))
}

#[test]
fn with_exception_handler_should_set_custom_exception_handler() {
    migration_test(
        // `WithExceptionHandlerExecutor`.
        || {
            CAPTURED.with(|captured| *captured.borrow_mut() = None);
            let _ = RxAppBuilder::create_reactive_ui_builder().with_exception_handler(capturing_handler()).with_core_services().build_app();
        },
        || {
            let test_exception: ObservableError = Rc::new(InvalidOperationException("Test exception"));
            RxState::default_exception_handler().on_next(test_exception.clone());
            assert!(same_error(&captured(), &test_exception));
        },
    );
}

// Not ported: WithExceptionHandler_With_Null_Handler_Should_Throw - argument validation

// Not ported: WithSuspensionHost_NonGeneric_Should_Create_Default_Host - reflection (the concrete type of the published host)

// Not ported: WithSuspensionHost_Generic_Should_Create_Typed_Host - reflection (the concrete type of the published host)

#[test]
fn with_cache_sizes_should_set_custom_cache_sizes() {
    migration_test(
        // `WithCacheSizesExecutor`.
        || {
            let _ = RxAppBuilder::create_reactive_ui_builder()
                .with_cache_sizes(CUSTOM_SMALL_CACHE_SIZE, CUSTOM_BIG_CACHE_SIZE)
                .with_core_services()
                .build_app();
        },
        || {
            assert_eq!(RxCacheSize::small_cache_limit(), CUSTOM_SMALL_CACHE_SIZE);
            assert_eq!(RxCacheSize::big_cache_limit(), CUSTOM_BIG_CACHE_SIZE);
        },
    );
}

#[test]
fn with_cache_sizes_with_zero_or_negative_values_should_throw() {
    migration_test(configure_builder, || {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let _ = builder.with_cache_sizes(INVALID_CACHE_SIZE, VALID_CACHE_SIZE);
        }))
        .is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let _ = builder.with_cache_sizes(VALID_CACHE_SIZE, INVALID_CACHE_SIZE);
        }))
        .is_err());
        // The negative sizes of upstream cannot be expressed: a size is a `usize`.
    });
}

#[test]
fn rx_cache_size_should_use_platform_defaults_when_not_configured() {
    migration_test(configure_builder, || {
        assert_eq!(RxCacheSize::small_cache_limit(), SMALL_CACHE_DEFAULT);
        assert_eq!(RxCacheSize::big_cache_limit(), BIG_CACHE_DEFAULT);
    });
}

#[test]
fn builder_should_support_chaining_all_rx_app_migration_methods() {
    migration_test(
        // `WithAllMigrationMethodsExecutor`.
        || {
            CAPTURED.with(|captured| *captured.borrow_mut() = None);
            let _ = RxAppBuilder::create_reactive_ui_builder()
                .with_exception_handler(capturing_handler())
                .with_suspension_host_of::<TestAppState>()
                .with_cache_sizes(CHAINED_SMALL_CACHE_SIZE, CHAINED_BIG_CACHE_SIZE)
                .with_core_services()
                .build_app();
        },
        || {
            // The handler of the errors.
            let test_exception: ObservableError = Rc::new(InvalidOperationException("Test"));
            RxState::default_exception_handler().on_next(test_exception.clone());
            assert!(same_error(&captured(), &test_exception));

            // The type of the suspension host is not checked: it cannot be read from the trait
            // object without reflection.

            // The cache sizes.
            assert_eq!(RxCacheSize::small_cache_limit(), CHAINED_SMALL_CACHE_SIZE);
            assert_eq!(RxCacheSize::big_cache_limit(), CHAINED_BIG_CACHE_SIZE);
        },
    );
}

#[test]
fn rx_schedulers_default_exception_handler_should_not_be_null() {
    migration_test(configure_builder, || {
        let handler = RxState::default_exception_handler();
        assert!(Rc::ptr_eq(&handler, &RxState::default_exception_handler()));
    });
}

#[test]
fn rx_schedulers_suspension_host_should_not_be_null() {
    migration_test(configure_builder, || {
        let host = RxSuspension::suspension_host();
        assert!(Rc::ptr_eq(&host, &RxSuspension::suspension_host()));
    });
}

#[test]
fn with_exception_handler_called_multiple_times_should_use_last_handler() {
    migration_test(
        // `WithMultipleExceptionHandlersExecutor`.
        || {
            CAPTURED.with(|captured| *captured.borrow_mut() = None);
            FIRST_CAPTURED.with(|captured| *captured.borrow_mut() = None);
            let first_handler =
                Observer::create(|error: ObservableError| FIRST_CAPTURED.with(|captured| *captured.borrow_mut() = Some(error)));
            let _ = RxAppBuilder::create_reactive_ui_builder()
                .with_exception_handler(first_handler)
                .with_exception_handler(capturing_handler())
                .with_core_services()
                .build_app();
        },
        || {
            let test_exception: ObservableError = Rc::new(InvalidOperationException("Test"));
            RxState::default_exception_handler().on_next(test_exception.clone());
            // `SecondCaptured`.
            assert!(same_error(&captured(), &test_exception));
            assert!(FIRST_CAPTURED.with(|captured| captured.borrow().is_none()));
        },
    );
}

#[test]
fn with_cache_sizes_called_multiple_times_should_use_last_values() {
    const FIRST_SMALL_CACHE_SIZE: usize = 100;
    const FIRST_BIG_CACHE_SIZE: usize = 200;
    migration_test(
        // `WithMultipleCacheSizesExecutor`.
        || {
            let _ = RxAppBuilder::create_reactive_ui_builder()
                .with_cache_sizes(FIRST_SMALL_CACHE_SIZE, FIRST_BIG_CACHE_SIZE)
                .with_cache_sizes(LAST_SMALL_CACHE_SIZE, LAST_BIG_CACHE_SIZE)
                .with_core_services()
                .build_app();
        },
        || {
            assert_eq!(RxCacheSize::small_cache_limit(), LAST_SMALL_CACHE_SIZE);
            assert_eq!(RxCacheSize::big_cache_limit(), LAST_BIG_CACHE_SIZE);
        },
    );
}

// Not ported: WithSuspensionHost_Generic_Overrides_NonGeneric - reflection (the concrete type of the published host)
