use super::ActivationKind;
use ferroui_base::platform::storage::IStorageItem;
use std::rc::Rc;

/// The arguments of an activation in which the application is passed one or
/// more files to open. Convert them to
/// [`ActivatedEventArgs`](super::ActivatedEventArgs) to raise the event.
#[derive(Clone)]
pub struct FileActivatedEventArgs {
    files: Rc<[Rc<dyn IStorageItem>]>,
}

impl FileActivatedEventArgs {
    /// Creates the arguments with the files the application was activated
    /// with.
    pub fn new(files: Vec<Rc<dyn IStorageItem>>) -> Self {
        Self { files: files.into() }
    }

    /// The kind of activation: [`ActivationKind::File`].
    pub fn kind(&self) -> ActivationKind {
        ActivationKind::File
    }

    /// The files the application was activated with.
    pub fn files(&self) -> &[Rc<dyn IStorageItem>] {
        &self.files
    }
}
