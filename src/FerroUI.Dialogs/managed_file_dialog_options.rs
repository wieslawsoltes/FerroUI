use ferroui_base::Ref;
use ferroui_controls::platform::IMountedVolumeInfoProvider;
use ferroui_controls::ContentControl;
use std::rc::Rc;

/// The options of the managed file dialogs.
///
/// A record: a copy with a member changed is made with `Clone` and the
/// setter (the `with` expression of the original).
#[derive(Clone, Default)]
pub struct ManagedFileDialogOptions {
    allow_directory_selection: bool,
    custom_volume_info_provider: Option<Rc<dyn IMountedVolumeInfoProvider>>,
    content_root_factory: Option<Rc<dyn Fn() -> Ref<ContentControl>>>,
}

impl ManagedFileDialogOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_directory_selection(&self) -> bool {
        self.allow_directory_selection
    }

    pub fn set_allow_directory_selection(&mut self, value: bool) {
        self.allow_directory_selection = value;
    }

    /// Allows to redefine how root volumes are populated in the dialog.
    pub fn custom_volume_info_provider(&self) -> Option<&Rc<dyn IMountedVolumeInfoProvider>> {
        self.custom_volume_info_provider.as_ref()
    }

    pub fn set_custom_volume_info_provider(&mut self, value: Option<Rc<dyn IMountedVolumeInfoProvider>>) {
        self.custom_volume_info_provider = value;
    }

    /// Allows to redefine content root.
    /// Can be a custom Window or any ContentControl (Popup hosted).
    pub fn content_root_factory(&self) -> Option<&Rc<dyn Fn() -> Ref<ContentControl>>> {
        self.content_root_factory.as_ref()
    }

    pub fn set_content_root_factory(&mut self, value: Option<Rc<dyn Fn() -> Ref<ContentControl>>>) {
        self.content_root_factory = value;
    }
}

impl PartialEq for ManagedFileDialogOptions {
    /// Member-wise, as the equality of a record; the provider and the
    /// factory compare by reference.
    fn eq(&self, other: &Self) -> bool {
        self.allow_directory_selection == other.allow_directory_selection
            && match (&self.custom_volume_info_provider, &other.custom_volume_info_provider) {
                (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
                (None, None) => true,
                _ => false,
            }
            && match (&self.content_root_factory, &other.content_root_factory) {
                (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
                (None, None) => true,
                _ => false,
            }
    }
}
