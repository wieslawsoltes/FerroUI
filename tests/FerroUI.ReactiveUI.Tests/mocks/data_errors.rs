//! The messages of a collection of validation errors (`errors.OfType<string>()`), which the tests
//! of reactive properties read.

use ferroui_reactiveui::DataErrors;

/// The messages among `errors` (`OfType<string>()`); empty when there are none.
pub fn string_errors(errors: &Option<DataErrors>) -> Vec<String> {
    errors
        .iter()
        .flatten()
        .filter_map(|error| error.downcast_ref::<String>().cloned())
        .collect()
}
