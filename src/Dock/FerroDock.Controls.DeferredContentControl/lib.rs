//! Native deferred content presentation and shared scheduling timelines.
mod presentation;
mod register_types;
mod scheduling;
mod settings;
pub use scheduling::DeferredContentScheduling;
mod timeline;
mod timeline_queue;
pub use presentation::IDeferredContentPresentation;
pub use register_types::{register_types, ASSEMBLY};
pub use settings::DeferredContentPresentationSettings;
pub use timeline::{DeferredContentPresentationBudgetMode, DeferredContentPresentationTimeline};

#[cfg(test)]
mod timeline_tests;

mod deferred_content_control;
mod deferred_content_presenter;
mod reveal;
mod target;
pub use deferred_content_control::DeferredContentControl;
pub use deferred_content_presenter::DeferredContentPresenter;

#[cfg(test)]
mod control_tests;
