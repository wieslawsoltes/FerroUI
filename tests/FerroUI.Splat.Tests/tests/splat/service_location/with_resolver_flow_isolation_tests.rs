//! Ported from `Splat.Tests/ServiceLocation/WithResolverFlowIsolationTests.cs` of Splat.
//! Adapted: the override of `with_resolver` belongs to the thread in the port (upstream's flows
//! with the asynchronous context, `AsyncLocal`), so only the test that runs in one flow is ported;
//! the tests of overlapping asynchronous flows have no counterpart on one thread, where every scope
//! is the current one of the single flow, and another thread has a locator of its own. The
//! `[Before(Test)]`/`[After(Test)]` scope is an `AppLocatorScope` at the start of the test, and the
//! `using` blocks are the disposal of the scope `with_resolver` returns.

use crate::tests::common::{same, AppLocatorScope};
use ferroui_splat::{AppLocator, DependencyResolverMixins, InstanceGenericFirstDependencyResolver};
use std::rc::Rc;

#[test]
fn set_locator_inside_a_scope_is_confined_to_the_flow() {
    let _scope = AppLocatorScope::new();
    let process_wide_resolver = AppLocator::get_locator();

    let flow_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());
    let replacement_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());

    let flow_scope = flow_resolver.with_resolver();
    AppLocator::set_locator(replacement_resolver.clone());

    assert!(same(&AppLocator::get_locator(), &replacement_resolver));
    flow_scope.dispose();

    assert!(same(&AppLocator::get_locator(), &process_wide_resolver));
}

// Not ported: WithResolver_WhenScopesOverlap_EachFlowResolvesFromItsOwnResolver - overlapping asynchronous flows (`AsyncLocal`); the override is per thread in the port
// Not ported: WithResolver_WhenScopesOverlap_SuppressionDoesNotLeakIntoAnotherFlow - overlapping asynchronous flows (`AsyncLocal`); the override is per thread in the port
// Not ported: WithResolver_WhileScopeIsActive_UnrelatedFlowSeesTheProcessWideResolver - overlapping asynchronous flows (`AsyncLocal`); the override is per thread in the port, and another thread has a locator of its own
