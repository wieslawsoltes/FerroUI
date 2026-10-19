//! Ported from `Splat.Tests/ServiceLocation/InstanceGenericFirst/ResolverStateTests.cs` of Splat.

use crate::service_location::instance_generic_first::ResolverState;

#[test]
fn resolver_state_should_have_unique_ids() {
    let state1 = ResolverState::new();
    let state2 = ResolverState::new();
    let state3 = ResolverState::new();
    assert_ne!(state1.id(), state2.id());
    assert_ne!(state2.id(), state3.id());
    assert_ne!(state1.id(), state3.id());
}

#[test]
fn resolver_state_should_have_monotonically_increasing_ids() {
    let state1 = ResolverState::new();
    let state2 = ResolverState::new();
    let state3 = ResolverState::new();
    assert!(state2.id() > state1.id());
    assert!(state3.id() > state2.id());
}
