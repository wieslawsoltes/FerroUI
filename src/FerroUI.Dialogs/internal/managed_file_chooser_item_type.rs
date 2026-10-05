use ferroui_base::ferro_markup_enum;

/// The kind of an entry of the managed file chooser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ManagedFileChooserItemType {
    #[default]
    File = 0,
    Folder = 1,
    Volume = 2,
}

ferro_markup_enum!(ManagedFileChooserItemType { File, Folder, Volume });
