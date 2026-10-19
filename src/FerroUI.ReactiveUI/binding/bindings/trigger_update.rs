//! `TriggerUpdate`.

/// The direction driven by the update stream of a binding (`TriggerUpdate`), for
/// [`bind_with_trigger`](crate::PropertyBindingMixins::bind_with_trigger).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerUpdate {
    /// The stream replaces the notifications of the view and requests writes to the view model
    /// (`ViewToViewModel`).
    #[default]
    ViewToViewModel = 0,
    /// The stream requests writes to the view in place of the notifications of the view model
    /// after the first; the notifications of the view are still observed (`ViewModelToView`).
    ViewModelToView = 1,
}
