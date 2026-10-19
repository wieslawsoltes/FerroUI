//! Ported from
//! `Splat.Tests/ApplicationPerformanceMonitoring/EnableFeatureUsageTrackingExtensionsCoverageTests.cs`
//! of Splat.
//! Adapted: an exception of the action is a panic, which the extensions record with
//! `on_exception` and let propagate (`InvalidOperationException("boom")` is `panic!("boom")`);
//! `IsTypeOf<DefaultFeatureUsageTrackingSession>` has no form on a `Rc<dyn
//! IFeatureUsageTrackingSession>`, so the type of the session is told by the message a
//! `DefaultFeatureUsageTrackingSession` logs when it starts, written to a `TextLogger` registered
//! as the `dyn ILogger` of the locator; the same logger shows the report of the exception.
//! `using var` is an explicit `dispose`.

use crate::tests::common::{
    panic_message, panic_payload, panics, AppLocatorScope, IMockLogTarget, TextLogger,
};
use ferroui_splat::{
    AppLocator, DefaultFeatureUsageTrackingSession, EnableFeatureUsageTrackingExt,
    FeatureUsageTrackingSessionExt, Guid, IEnableFeatureUsageTracking, IFeatureUsageTrackingManager,
    IFeatureUsageTrackingSession, ILogger,
};
use ferroui_system::IDisposable;
use std::rc::Rc;

const FEATURE_NAME: &str = "feature";

/// A type with feature usage tracking (`TrackingEnabledFixture`).
struct TrackingEnabledFixture;

impl IEnableFeatureUsageTracking for TrackingEnabledFixture {}

#[test]
fn feature_usage_tracking_session_returns_session_with_feature_name() {
    let _scope = AppLocatorScope::new();
    let fixture = TrackingEnabledFixture;
    let feature_name = Guid::new_guid().to_string();

    let session = fixture.feature_usage_tracking_session(&feature_name);

    assert_eq!(session.feature_name(), feature_name);
    session.dispose();
}

#[test]
fn feature_usage_tracking_session_returns_default_session_type() {
    let _scope = AppLocatorScope::new();
    let logger = Rc::new(TextLogger::new());
    AppLocator::register_constant::<dyn ILogger>(logger.clone());
    let fixture = TrackingEnabledFixture;

    let session = fixture.feature_usage_tracking_session(&Guid::new_guid().to_string());

    assert!(logger
        .logs()
        .iter()
        .any(|(_, message)| message.starts_with("DefaultFeatureUsageTrackingSession: Feature Start.")));
    session.dispose();
}

#[test]
fn feature_usage_tracking_session_throws_when_manager_missing() {
    let _scope = AppLocatorScope::new();
    AppLocator::unregister_current::<dyn IFeatureUsageTrackingManager>();
    let fixture = TrackingEnabledFixture;

    assert!(panics(|| {
        let _ = fixture.feature_usage_tracking_session(FEATURE_NAME);
    }));
}

#[test]
fn with_feature_usage_tracking_session_invokes_action() {
    let _scope = AppLocatorScope::new();
    let fixture = TrackingEnabledFixture;
    let feature_name = Guid::new_guid().to_string();
    let mut captured: Option<String> = None;

    fixture.with_feature_usage_tracking_session(&feature_name, |session: &dyn IFeatureUsageTrackingSession| {
        captured = Some(session.feature_name().to_owned());
    });

    assert_eq!(captured.as_deref(), Some(feature_name.as_str()));
}

#[test]
fn with_feature_usage_tracking_session_reports_and_rethrows() {
    let _scope = AppLocatorScope::new();
    let logger = Rc::new(TextLogger::new());
    AppLocator::register_constant::<dyn ILogger>(logger.clone());
    let fixture = TrackingEnabledFixture;

    let payload = panic_payload(|| {
        fixture.with_feature_usage_tracking_session(FEATURE_NAME, |_: &dyn IFeatureUsageTrackingSession| {
            panic!("boom")
        });
    });

    assert_eq!(payload.map(|payload| panic_message(&*payload)).as_deref(), Some("boom"));
    assert!(logger.logs().iter().any(|(_, message)| message.contains("Feature Usage Tracking Exception")));
}

#[test]
fn with_sub_feature_usage_tracking_session_invokes_action_with_sub_feature() {
    let _scope = AppLocatorScope::new();
    let session = DefaultFeatureUsageTrackingSession::new(&Guid::new_guid().to_string());
    let sub_feature_name = Guid::new_guid().to_string();
    let mut captured: Option<String> = None;

    session.with_sub_feature_usage_tracking_session(&sub_feature_name, |sub: &dyn IFeatureUsageTrackingSession| {
        captured = Some(sub.feature_name().to_owned());
    });

    assert_eq!(captured.as_deref(), Some(sub_feature_name.as_str()));
    session.dispose();
}

#[test]
fn with_sub_feature_usage_tracking_session_reports_and_rethrows() {
    let _scope = AppLocatorScope::new();
    let logger = Rc::new(TextLogger::new());
    AppLocator::register_constant::<dyn ILogger>(logger.clone());
    let session = DefaultFeatureUsageTrackingSession::new(&Guid::new_guid().to_string());

    let payload = panic_payload(|| {
        session.with_sub_feature_usage_tracking_session("sub", |_: &dyn IFeatureUsageTrackingSession| {
            panic!("boom")
        });
    });

    assert_eq!(payload.map(|payload| panic_message(&*payload)).as_deref(), Some("boom"));
    assert!(logger.logs().iter().any(|(_, message)| message.contains("Feature Usage Tracking Exception")));
    session.dispose();
}

// Not ported: WithFeatureUsageTrackingSession_ThrowsForNullAction - argument-null validation, which Rust's types make impossible
// Not ported: WithSubFeatureUsageTrackingSession_ThrowsForNullAction - argument-null validation, which Rust's types make impossible
