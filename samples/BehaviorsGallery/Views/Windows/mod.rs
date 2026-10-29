//! The windows of the pages (namespace `BehaviorsGallery.Views.Windows`): one module per reference
//! file of `Views/Windows/`. A window is registered by its owner page (its entry of the page table,
//! `pages/mod.rs`); its module line stays a comment until that page is ported (`PAGES.md`).

pub mod sample_window; // owner: WindowInteractionsView
pub mod window_demo; // owner: WindowDragMoveBehaviorView
