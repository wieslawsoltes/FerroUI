/// Defines how a container is queried.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContainerSizing {
    /// The container is not included in any size queries.
    #[default]
    Normal,
    /// The container size can be queried for width.
    Width,
    /// The container size can be queried for height.
    Height,
    /// The container size can be queried for width and height.
    WidthAndHeight,
}
