//! Feature usage tracking (`Splat/ApplicationPerformanceMonitoring`): the interfaces that the
//! adapters of monitoring services implement, and the default implementation, which logs.

mod default_feature_usage_tracking;
mod guid;

pub use default_feature_usage_tracking::{
    DefaultFeatureUsageTrackingManager, DefaultFeatureUsageTrackingSession,
    FuncFeatureUsageTrackingManager,
};
pub use guid::Guid;

use crate::service_location::{AppLocator, ReadonlyDependencyResolverExt};
use ferroui_system::IDisposable;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// A session of the usage of a feature (`IFeatureUsageTrackingSession`); disposing it ends the
/// session.
pub trait IFeatureUsageTrackingSession: IDisposable {
    /// The name of the feature (`FeatureName`).
    fn feature_name(&self) -> &str;

    /// A session of a sub-feature of this one (`SubFeature`).
    fn sub_feature(&self, description: &str) -> Rc<dyn IFeatureUsageTrackingSession>;

    /// Records a failure of the feature (`OnException`).
    fn on_exception(&self, exception: &dyn std::error::Error);
}

/// A session with references of the type `R` to itself and to its parent
/// (`IFeatureUsageTrackingSession<TReferenceType>`).
pub trait IFeatureUsageTrackingSessionOf<R>: IFeatureUsageTrackingSession {
    /// The reference of the session (`FeatureReference`).
    fn feature_reference(&self) -> R;

    /// The reference of the parent session (`ParentReference`).
    fn parent_reference(&self) -> R;
}

/// Starts the sessions of features (`IFeatureUsageTrackingManager`).
pub trait IFeatureUsageTrackingManager {
    /// A session for the feature `feature_name` (`GetFeatureUsageTrackingSession`).
    fn get_feature_usage_tracking_session(&self, feature_name: &str) -> Rc<dyn IFeatureUsageTrackingSession>;
}

/// Tracks the navigation between views (`IViewTracking`).
pub trait IViewTracking {
    /// Records the navigation to the view `name` (`OnViewNavigation`).
    fn on_view_navigation(&self, name: &str);
}

/// A type that tracks the usage of its features (`IEnableFeatureUsageTracking`), through the
/// methods of [`EnableFeatureUsageTrackingExt`].
pub trait IEnableFeatureUsageTracking {}

/// The failure of a feature, for [`IFeatureUsageTrackingSession::on_exception`]: the message of
/// the panic of the action.
#[derive(Debug)]
struct ActionPanicked(String);

impl std::fmt::Display for ActionPanicked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ActionPanicked {}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "panic".to_owned()
    }
}

/// Runs `action` in `session`; a panic of the action is recorded with `on_exception` and
/// propagates; the session is disposed in either case.
fn run_in_session(
    session: Rc<dyn IFeatureUsageTrackingSession>,
    action: impl FnOnce(&dyn IFeatureUsageTrackingSession),
) {
    let result = catch_unwind(AssertUnwindSafe(|| action(&*session)));
    if let Err(payload) = result {
        session.on_exception(&ActionPanicked(panic_message(payload.as_ref())));
        session.dispose();
        resume_unwind(payload);
    }
    session.dispose();
}

/// The feature usage tracking of a type (`EnableFeatureUsageTrackingExtensions`).
// Deviation (DEVIATIONS.md, Splat): an exception of the action is a panic; it is recorded with the
// message of the panic and propagates.
pub trait EnableFeatureUsageTrackingExt: IEnableFeatureUsageTracking {
    /// A session for the feature `feature_name`, from the `dyn IFeatureUsageTrackingManager` of
    /// the current resolver (`FeatureUsageTrackingSession`).
    ///
    /// Panics when the resolver has no manager (`InvalidOperationException`).
    fn feature_usage_tracking_session(&self, feature_name: &str) -> Rc<dyn IFeatureUsageTrackingSession> {
        match AppLocator::current().get_service::<dyn IFeatureUsageTrackingManager>() {
            None => panic!(
                "Feature Usage Tracking Manager is null. This should never happen, your dependency resolver is broken"
            ),
            Some(manager) => manager.get_feature_usage_tracking_session(feature_name),
        }
    }

    /// Runs `action` in a session for the feature `feature_name`
    /// (`WithFeatureUsageTrackingSession`).
    fn with_feature_usage_tracking_session(
        &self,
        feature_name: &str,
        action: impl FnOnce(&dyn IFeatureUsageTrackingSession),
    ) {
        run_in_session(self.feature_usage_tracking_session(feature_name), action);
    }
}

impl<T: IEnableFeatureUsageTracking + ?Sized> EnableFeatureUsageTrackingExt for T {}

/// The sub-features of a session (`EnableFeatureUsageTrackingExtensions`).
pub trait FeatureUsageTrackingSessionExt: IFeatureUsageTrackingSession {
    /// Runs `action` in a session of the sub-feature `feature_name`
    /// (`WithSubFeatureUsageTrackingSession`).
    fn with_sub_feature_usage_tracking_session(
        &self,
        feature_name: &str,
        action: impl FnOnce(&dyn IFeatureUsageTrackingSession),
    ) {
        run_in_session(self.sub_feature(feature_name), action);
    }
}

impl<T: IFeatureUsageTrackingSession + ?Sized> FeatureUsageTrackingSessionExt for T {}
