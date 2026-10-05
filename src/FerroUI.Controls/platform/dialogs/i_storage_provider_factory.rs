use crate::TopLevel;
use ferroui_base::platform::storage::IStorageProvider;
use ferroui_base::Ref;
use std::rc::Rc;

/// Factory allows to register custom storage provider instead of native
/// implementation.
///
/// This API is unstable: it may change with the platform backends.
pub trait IStorageProviderFactory {
    /// Creates the storage provider of a top-level.
    fn create_provider(&self, top_level: &Ref<TopLevel>) -> Rc<dyn IStorageProvider>;
}
