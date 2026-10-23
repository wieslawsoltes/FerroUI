//! Port of `ViewLocator.cs`: the data template of the window, generated at build time.
//!
//! The reference's locator declares `Build` and `Match` itself (`data is ViewModelBase`); this one
//! asks the generator for the data template, which matches a view model with a view of its own
//! that is a page (`data_template_match_types`), and shows the text of the missing view of any
//! other view model it is asked for.

use crate::view_models::ViewModelBase;
use ferroui_static_view_locator::static_view_locator;

/// The locator of the pages, the metrics and the reports.
#[static_view_locator(generate_data_template = true, generate_runtime_type_fallback_methods = false, data_template_match_types = [ViewModelBase])]
#[derive(Clone, Copy, Debug, Default)]
pub struct ViewLocator;
