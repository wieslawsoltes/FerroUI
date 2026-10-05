/// Defines possible binding modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BindingMode {
    /// Uses the default binding mode specified for the property.
    #[default]
    Default,
    /// Binds one way from source to target.
    OneWay,
    /// Binds two-way with the initial value coming from the target.
    TwoWay,
    /// Updates the target when the application starts or when the data context changes.
    OneTime,
    /// Binds one way from target to source.
    OneWayToSource,
}
