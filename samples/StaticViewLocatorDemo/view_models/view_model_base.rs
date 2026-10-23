//! Port of `ViewModels/ViewModelBase.cs`: what every page of the sample has, its title.

/// A page of the sample (`ViewModelBase`). A trait: a Rust struct derives from no class.
pub trait ViewModelBase {
    /// The title of the tab of the page.
    fn title(&self) -> String;
}
