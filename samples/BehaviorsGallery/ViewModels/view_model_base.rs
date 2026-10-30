//! Port of `ViewModels/ViewModelBase.cs`.

use ferroui_reactiveui::ReactiveObject;

/// The base of the view models of the gallery (`ViewModelBase : ReactiveObject`).
///
/// Deviation (DEVIATIONS.md, Behaviors gallery): the base class of the reference is the part a view
/// model holds (the `ReactiveObject` of the port is one too); `reactive_object!` is invoked for the
/// view model with the field that holds this part.
#[derive(Default)]
pub struct ViewModelBase {
    reactive: ReactiveObject,
}

impl ViewModelBase {
    /// The part of a new view model.
    pub fn new() -> Self {
        Self {
            reactive: ReactiveObject::new(),
        }
    }
}

impl AsRef<ReactiveObject> for ViewModelBase {
    fn as_ref(&self) -> &ReactiveObject {
        &self.reactive
    }
}
