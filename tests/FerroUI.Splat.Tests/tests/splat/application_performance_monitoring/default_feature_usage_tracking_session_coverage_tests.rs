//! Ported from
//! `Splat.Tests/ApplicationPerformanceMonitoring/DefaultFeatureUsageTrackingSessionCoverageTests.cs`
//! of Splat.
//! Adapted: `subFeature as IFeatureUsageTrackingSession<Guid>` has no form on the `Rc<dyn
//! IFeatureUsageTrackingSession>` that `sub_feature` returns, so the references of the sub-feature
//! are read from the message it logs when it starts ("Feature Start. Reference=..., Parent
//! Reference=..."), written to a `TextLogger` registered as the `dyn ILogger` of the locator. An
//! `InvalidOperationException` is a `TestException`; `ArgumentException` is a panic; `using var` is
//! an explicit `dispose`.

use crate::tests::common::{panics, AppLocatorScope, IMockLogTarget, TestException, TextLogger};
use ferroui_splat::{
    AppLocator, DefaultFeatureUsageTrackingSession, Guid, ILogger, IFeatureUsageTrackingSession,
    IFeatureUsageTrackingSessionOf,
};
use ferroui_system::IDisposable;
use std::rc::Rc;

#[test]
fn constructor_sets_root_session_properties() {
    let _scope = AppLocatorScope::new();
    let feature_name = Guid::new_guid().to_string();
    let instance = DefaultFeatureUsageTrackingSession::new(&feature_name);

    assert_eq!(instance.feature_name(), feature_name);
    assert_ne!(instance.feature_reference(), Guid::EMPTY);
    assert_eq!(instance.parent_reference(), Guid::EMPTY);
    instance.dispose();
}

#[test]
fn constructor_throws_for_invalid_feature_name() {
    assert!(panics(|| {
        let _ = DefaultFeatureUsageTrackingSession::new("  ");
    }));
}

#[test]
fn sub_feature_returns_child_session_with_parent_reference() {
    let _scope = AppLocatorScope::new();
    let logger = Rc::new(TextLogger::new());
    AppLocator::register_constant::<dyn ILogger>(logger.clone());
    let feature_name = Guid::new_guid().to_string();
    let instance = DefaultFeatureUsageTrackingSession::new(&feature_name);

    let sub_feature_name = Guid::new_guid().to_string();
    let sub_feature = instance.sub_feature(&sub_feature_name);

    assert_eq!(sub_feature.feature_name(), sub_feature_name);
    let start_message = logger.logs().last().map(|(_, message)| message.clone()).unwrap_or_default();
    // The reference of the sub-feature is not the empty one.
    assert!(start_message.contains("Feature Start. Reference="));
    assert!(!start_message.contains(&format!("Reference={},", Guid::EMPTY)));
    // Its parent is the session.
    assert!(start_message.contains(&format!(", Parent Reference={}", instance.feature_reference())));

    sub_feature.dispose();
    instance.dispose();
}

#[test]
fn on_exception_does_not_throw() {
    let _scope = AppLocatorScope::new();
    let instance = DefaultFeatureUsageTrackingSession::new(&Guid::new_guid().to_string());

    assert!(!panics(|| instance.on_exception(&TestException::with_message("boom"))));
    instance.dispose();
}

#[test]
fn dispose_does_not_throw() {
    let _scope = AppLocatorScope::new();
    let instance = DefaultFeatureUsageTrackingSession::new(&Guid::new_guid().to_string());

    assert!(!panics(|| instance.dispose()));
}
