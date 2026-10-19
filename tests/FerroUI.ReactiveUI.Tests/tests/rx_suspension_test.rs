//! `RxSuspensionTest` (`ReactiveUI.Tests/RxSuspensionTest.cs`).

use ferroui_reactiveui::*;
use ferroui_system::IDisposable;
use std::rc::Rc;

fn same(first: &Rc<dyn ISuspensionHost>, second: &Rc<dyn ISuspensionHost>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second))
}

// Not ported: SuspensionHost_ConcurrentFirstRead_NeverReturnsNull - real threads

// Not ported: InitializeSuspensionHost_Null_ThrowsAndKeepsDefault - argument validation

#[test]
fn initialize_suspension_host_before_first_read_is_returned() {
    RxSuspension::reset_for_testing();
    let host = SuspensionHost::new();
    let published: Rc<dyn ISuspensionHost> = host.clone();

    RxSuspension::initialize_suspension_host(published.clone());

    assert!(same(&RxSuspension::suspension_host(), &published));
    host.dispose();
    RxSuspension::reset_for_testing();
}

#[test]
fn initialize_suspension_host_after_first_read_keeps_first_host() {
    RxSuspension::reset_for_testing();
    let first = RxSuspension::suspension_host();
    let later = SuspensionHost::new();

    RxSuspension::initialize_suspension_host(later.clone());

    assert!(same(&RxSuspension::suspension_host(), &first));
    later.dispose();
    RxSuspension::reset_for_testing();
}
