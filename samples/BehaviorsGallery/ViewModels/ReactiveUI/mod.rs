//! The view models of the ReactiveUI pages (namespace `BehaviorsGallery.ViewModels.ReactiveUI`):
//! one module per reference file of `ViewModels/ReactiveUI/`. A view model is registered by the
//! page that uses it; its module line stays a comment until that page is ported (`PAGES.md`).

pub mod detail_page_view_model; // owner: ReactiveNavigationView
pub mod home_page_view_model; // owner: ReactiveNavigationView
pub mod interaction_trigger_behavior_view_model; // owner: InteractionTriggerBehaviorView
pub mod reactive_navigation_view_model; // owner: ReactiveNavigationView
