//! Port of `Diagnostics/DevToolsViewKind.cs`.

/// Kinds of diagnostic views available in DevTools
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DevToolsViewKind {
    /// The Logical Tree diagnostic view
    LogicalTree = 0,
    /// The Visual Tree diagnostic view
    VisualTree = 1,
    /// Events diagnostic view
    Events = 2,
    /// The combined tree: the logical tree with the parts of the templates, greyed out.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): an addition of the port, and the default view.
    #[default]
    CombinedTree = 3,
}
