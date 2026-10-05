//! Port of `CompilerExtensions/XamlDocumentUsage.cs`.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum XamlDocumentUsage {
    #[default]
    Unknown,
    Merged,
    Used,
}
