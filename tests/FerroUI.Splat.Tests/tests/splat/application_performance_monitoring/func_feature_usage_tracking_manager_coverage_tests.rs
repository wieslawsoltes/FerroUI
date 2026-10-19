//! Ported from
//! `Splat.Tests/ApplicationPerformanceMonitoring/FuncFeatureUsageTrackingManagerCoverageTests.cs` of
//! Splat.
//! Adapted: the name the factory receives is captured in a `RefCell` the factory shares, as the
//! factory is `'static`; `using var` is an explicit `dispose`.

use crate::tests::common::AppLocatorScope;
use ferroui_splat::{
    DefaultFeatureUsageTrackingSession, FuncFeatureUsageTrackingManager, Guid,
    IFeatureUsageTrackingManager, IFeatureUsageTrackingSession,
};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn get_feature_usage_tracking_session_returns_session_from_factory() {
    let _scope = AppLocatorScope::new();
    let feature_name = Guid::new_guid().to_string();
    let captured_name: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let captured = captured_name.clone();
    let manager = FuncFeatureUsageTrackingManager::new(move |name| {
        *captured.borrow_mut() = Some(name.to_owned());
        Rc::new(DefaultFeatureUsageTrackingSession::new(name)) as Rc<dyn IFeatureUsageTrackingSession>
    });

    let session = manager.get_feature_usage_tracking_session(&feature_name);

    assert_eq!(captured_name.borrow().as_deref(), Some(feature_name.as_str()));
    assert_eq!(session.feature_name(), feature_name);
    session.dispose();
}

// Not ported: Constructor_ThrowsForNullFactory - argument-null validation, which Rust's types make impossible
