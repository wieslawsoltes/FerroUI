use super::ManagedFileChooserItemType;

/// A place the managed file chooser can navigate to: a folder of the user
/// or a mounted volume.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ManagedFileChooserNavigationItem {
    pub display_name: Option<String>,
    pub path: Option<String>,
    pub item_type: ManagedFileChooserItemType,
}
