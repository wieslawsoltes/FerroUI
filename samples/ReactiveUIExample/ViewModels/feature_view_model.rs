//! Port of `ViewModels/FeatureViewModel.cs`: an immutable feature description resolved through an
//! automatic item template.

use ferroui_base::ferro_markup_type;
use std::rc::Rc;

/// An immutable feature description resolved through an automatic item template
/// (`FeatureViewModel(string Name)`): a record, compared by value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureViewModel {
    name: String,
}

impl FeatureViewModel {
    /// The feature `name`.
    pub fn new(name: impl Into<String>) -> Rc<Self> {
        Rc::new(Self { name: name.into() })
    }

    /// The feature description (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

ferro_markup_type!(class FeatureViewModel {
    this: Rc<FeatureViewModel>,
    handles: [FeatureViewModel, Rc<FeatureViewModel>, Option<Rc<FeatureViewModel>>],
    properties: [
        Name: String { get: |this: &Rc<FeatureViewModel>| this.name() },
    ],
});
