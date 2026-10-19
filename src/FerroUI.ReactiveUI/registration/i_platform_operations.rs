//! `IPlatformOperations` (`ReactiveUI.Core/Interfaces/IPlatformOperations.cs`).

/// The details that each platform of ReactiveUI implements (`IPlatformOperations`).
pub trait IPlatformOperations {
    /// A description of the orientation of the screen, where the platform has one
    /// (`GetOrientation`).
    fn get_orientation(&self) -> Option<String>;
}
