//! Ported from `Splat.Tests/ApplicationPerformanceMonitoring/BaseFeatureUsageTrackingTests.cs` of
//! Splat.
//! Adapted: the abstract test classes `BaseConstructorTests<T>` and `BaseSubFeatureMethodTests<T>`
//! are the macros `base_constructor_tests!` and `base_sub_feature_method_tests!`, which expand to
//! the test `returns_instance` given an expression that makes the session under test from a
//! feature name (`GetFeatureUsageTrackingSession`). Upstream only the adapter test projects
//! (Exceptionless, Raygun, ApplicationInsights), which are not ported, inherit them; here they are
//! instantiated for `DefaultFeatureUsageTrackingSession`, with an `AppLocatorScope`, as the session
//! logs through the log manager of the locator. The sub-feature is a `Rc<dyn
//! IFeatureUsageTrackingSession>`, which does not convert to `IFeatureUsageTrackingSessionOf<Guid>`
//! (`subfeature as IFeatureUsageTrackingSession<Guid>`), so the base checks its name only; the
//! references of a sub-feature of a `DefaultFeatureUsageTrackingSession` are checked in
//! `default_feature_usage_tracking_session_coverage_tests`.

/// The tests of the construction of a session (`BaseConstructorTests<TFeatureUsageTracking>`);
/// `$get_session` makes a session that implements `IFeatureUsageTrackingSessionOf<Guid>` from a
/// feature name (`&str`).
macro_rules! base_constructor_tests {
    ($get_session:expr) => {
        #[test]
        fn returns_instance() {
            use ::ferroui_splat::{Guid, IFeatureUsageTrackingSession, IFeatureUsageTrackingSessionOf};

            let _scope = $crate::tests::common::AppLocatorScope::new();
            let feature_name = Guid::new_guid().to_string();
            let instance = ($get_session)(feature_name.as_str());

            assert_eq!(instance.feature_name(), feature_name);
            assert_ne!(instance.feature_reference(), Guid::EMPTY);
            assert_eq!(instance.parent_reference(), Guid::EMPTY);
        }
    };
}

pub(crate) use base_constructor_tests;

/// The tests of the sub-features of a session (`BaseSubFeatureMethodTests<TFeatureUsageTracking>`);
/// `$get_session` makes a session that implements `IFeatureUsageTrackingSessionOf<Guid>` from a
/// feature name (`&str`).
macro_rules! base_sub_feature_method_tests {
    ($get_session:expr) => {
        #[test]
        fn returns_instance() {
            use ::ferroui_splat::{Guid, IFeatureUsageTrackingSession, IFeatureUsageTrackingSessionOf};

            let _scope = $crate::tests::common::AppLocatorScope::new();
            let feature_name = Guid::new_guid().to_string();
            let instance = ($get_session)(feature_name.as_str());

            assert_eq!(instance.feature_name(), feature_name);
            assert_ne!(instance.feature_reference(), Guid::EMPTY);
            assert_eq!(instance.parent_reference(), Guid::EMPTY);

            let subfeature_name = Guid::new_guid().to_string();
            let subfeature = instance.sub_feature(&subfeature_name);

            assert_eq!(subfeature.feature_name(), subfeature_name);
        }
    };
}

pub(crate) use base_sub_feature_method_tests;

/// `BaseConstructorTests<DefaultFeatureUsageTrackingSession>`.
mod default_feature_usage_tracking_session_constructor_tests {
    super::base_constructor_tests!(::ferroui_splat::DefaultFeatureUsageTrackingSession::new);
}

/// `BaseSubFeatureMethodTests<DefaultFeatureUsageTrackingSession>`.
mod default_feature_usage_tracking_session_sub_feature_method_tests {
    super::base_sub_feature_method_tests!(::ferroui_splat::DefaultFeatureUsageTrackingSession::new);
}
