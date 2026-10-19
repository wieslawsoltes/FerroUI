//! `ICanForceManualActivation` (`ReactiveUI.Core/Interfaces/ICanForceManualActivation.cs`).

/// A view that other code can activate and deactivate by hand, instead of through the events of
/// the system (`ICanForceManualActivation`). The views of the integration with the user interface
/// implement it with [`ActivationSignals::schedule_activation`](crate::ActivationSignals::schedule_activation).
pub trait ICanForceManualActivation {
    /// Activates the view, or deactivates it when `is_activating` is false (`Activate(bool)`).
    fn activate(&self, is_activating: bool);
}
