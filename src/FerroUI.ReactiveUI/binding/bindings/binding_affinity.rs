//! `BindingAffinity`.

/// The affinity scores shared by the type converters, the property observers and the command
/// binders: a higher value is a stronger match, and zero means that the candidate does not apply
/// (`BindingAffinity`).
///
/// `DEFAULT_INTERNAL_TYPE_CONVERTER`, `DEFAULT_EVENT`, `EXPLICIT` and `EXACT_TYPE` carry the
/// values of ReactiveUI's scores of those names; the scores of the platforms are upstream's own,
/// kept so that an observer of another platform ranks as it does there.
pub struct BindingAffinity;

impl BindingAffinity {
    /// The affinity of the observer of plain objects, the lowest match (`Fallback`).
    pub const FALLBACK: i32 = 1;

    /// The affinity of the built-in value and string converters (`DefaultInternalTypeConverter`).
    pub const DEFAULT_INTERNAL_TYPE_CONVERTER: i32 = 2;

    /// The affinity of a binding to the conventional default event of a type (`DefaultEvent`).
    pub const DEFAULT_EVENT: i32 = 3;

    /// The affinity of a dependency property of WPF (`WpfDependencyObject`).
    pub const WPF_DEPENDENCY_OBJECT: i32 = 4;

    /// The affinity of the command binding of a control with an enabled event
    /// (`EventEnabledControl`).
    pub const EVENT_ENABLED_CONTROL: i32 = 4;

    /// The affinity of an explicit or interface-based match, such as `INotifyPropertyChanged` or a
    /// named event (`Explicit`).
    pub const EXPLICIT: i32 = 5;

    /// The affinity of a dependency property of WinUI (`WinUiDependencyObject`).
    pub const WIN_UI_DEPENDENCY_OBJECT: i32 = 6;

    /// The affinity of the event-based property observation of WinForms (`WinFormsEvent`).
    pub const WIN_FORMS_EVENT: i32 = 8;

    /// The affinity of a strong, exact-type match, such as `IReactiveObject` (`ExactType`).
    pub const EXACT_TYPE: i32 = 10;

    /// The affinity of key-value observing on Apple platforms (`Kvo`).
    pub const KVO: i32 = 15;
}
