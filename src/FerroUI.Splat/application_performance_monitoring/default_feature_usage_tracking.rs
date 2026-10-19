//! `DefaultFeatureUsageTrackingSession`, `FuncFeatureUsageTrackingManager` and
//! `DefaultFeatureUsageTrackingManager`.

use super::{Guid, IFeatureUsageTrackingManager, IFeatureUsageTrackingSession, IFeatureUsageTrackingSessionOf};
use crate::logging::IEnableLogger;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The session of [`DefaultFeatureUsageTrackingManager`] (`DefaultFeatureUsageTrackingSession`):
/// it logs the start and the end of the feature, and its failures, at the level `Info`.
pub struct DefaultFeatureUsageTrackingSession {
    parent_reference: Guid,
    feature_reference: Guid,
    feature_name: String,
}

impl DefaultFeatureUsageTrackingSession {
    /// A session for the feature `feature_name`, without a parent.
    ///
    /// Panics when the name is empty or white space (`ArgumentException`).
    pub fn new(feature_name: &str) -> Self {
        Self::with_parent(feature_name, Guid::EMPTY)
    }

    /// A session for the feature `feature_name` under the session `parent_reference`.
    pub(crate) fn with_parent(feature_name: &str, parent_reference: Guid) -> Self {
        assert!(!feature_name.trim().is_empty(), "The value cannot be an empty string or composed entirely of whitespace. (Parameter 'featureName')");
        let session = Self {
            parent_reference,
            feature_reference: Guid::new_guid(),
            feature_name: feature_name.to_owned(),
        };
        session.log().info_fn(&|| session.session_start_log_message());
        session
    }

    fn session_start_log_message(&self) -> String {
        let mut message = format!("Feature Start. Reference={}", self.feature_reference);
        if self.parent_reference != Guid::EMPTY {
            message += &format!(", Parent Reference={}", self.parent_reference);
        }
        message
    }
}

impl IEnableLogger for DefaultFeatureUsageTrackingSession {}

impl IFeatureUsageTrackingSession for DefaultFeatureUsageTrackingSession {
    fn feature_name(&self) -> &str {
        &self.feature_name
    }

    fn sub_feature(&self, description: &str) -> Rc<dyn IFeatureUsageTrackingSession> {
        Rc::new(Self::with_parent(description, self.feature_reference))
    }

    fn on_exception(&self, exception: &dyn std::error::Error) {
        self.log().info_exception_fn(exception, &|| "Feature Usage Tracking Exception".to_owned());
    }
}

impl IFeatureUsageTrackingSessionOf<Guid> for DefaultFeatureUsageTrackingSession {
    fn feature_reference(&self) -> Guid {
        self.feature_reference
    }

    fn parent_reference(&self) -> Guid {
        self.parent_reference
    }
}

impl IDisposable for DefaultFeatureUsageTrackingSession {
    fn dispose(&self) {
        self.log().info_fn(&|| format!("Feature Finish: {}", self.feature_reference));
    }
}

/// A manager over a function that starts the sessions (`FuncFeatureUsageTrackingManager`).
pub struct FuncFeatureUsageTrackingManager {
    feature_usage_tracking_session_func: Box<dyn Fn(&str) -> Rc<dyn IFeatureUsageTrackingSession>>,
}

impl FuncFeatureUsageTrackingManager {
    /// A manager whose sessions `feature_usage_tracking_session_func` starts.
    pub fn new(
        feature_usage_tracking_session_func: impl Fn(&str) -> Rc<dyn IFeatureUsageTrackingSession> + 'static,
    ) -> Self {
        Self { feature_usage_tracking_session_func: Box::new(feature_usage_tracking_session_func) }
    }
}

impl IFeatureUsageTrackingManager for FuncFeatureUsageTrackingManager {
    fn get_feature_usage_tracking_session(&self, feature_name: &str) -> Rc<dyn IFeatureUsageTrackingSession> {
        (self.feature_usage_tracking_session_func)(feature_name)
    }
}

/// The manager Splat registers (`DefaultFeatureUsageTrackingManager`): its sessions are
/// [`DefaultFeatureUsageTrackingSession`]s, which log.
pub struct DefaultFeatureUsageTrackingManager {
    inner: FuncFeatureUsageTrackingManager,
}

impl Default for DefaultFeatureUsageTrackingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultFeatureUsageTrackingManager {
    /// The manager.
    pub fn new() -> Self {
        Self {
            inner: FuncFeatureUsageTrackingManager::new(|feature_name| {
                Rc::new(DefaultFeatureUsageTrackingSession::new(feature_name)) as Rc<dyn IFeatureUsageTrackingSession>
            }),
        }
    }
}

impl IFeatureUsageTrackingManager for DefaultFeatureUsageTrackingManager {
    fn get_feature_usage_tracking_session(&self, feature_name: &str) -> Rc<dyn IFeatureUsageTrackingSession> {
        self.inner.get_feature_usage_tracking_session(feature_name)
    }
}
