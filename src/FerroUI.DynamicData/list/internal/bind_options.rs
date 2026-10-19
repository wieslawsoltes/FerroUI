//! The options of the forms of `Bind` of lists that take a threshold
//! (`List/ObservableListEx.Bind.cs`).

use crate::binding::BindingOptions;
use crate::DynamicDataOptions;

/// The defaults of `DynamicDataOptions.Binding`, with `reset_threshold` unless it is upstream's
/// default (the retrofit of the system wide defaults of upstream).
pub(crate) fn bind_options(reset_threshold: usize) -> BindingOptions {
    let defaults = DynamicDataOptions::binding();
    if reset_threshold == BindingOptions::DEFAULT_RESET_THRESHOLD {
        defaults
    } else {
        defaults.with_reset_threshold(reset_threshold)
    }
}
