//! Observable collections.

mod ferro_dictionary;
mod ferro_dictionary_extensions;
mod ferro_list;
mod ferro_list_extensions;
mod i_ferro_dictionary;
mod i_ferro_list;
mod i_ferro_list_item_validator;
mod i_ferro_read_only_dictionary;
mod i_ferro_read_only_list;
mod notify_collection_changed_extensions;

pub use ferro_dictionary::{FerroDictionary, WeakFerroDictionary};
pub use ferro_dictionary_extensions::FerroDictionaryExtensions;
pub use ferro_list_extensions::FerroListExtensions;
pub use i_ferro_dictionary::IFerroDictionary;
pub use i_ferro_list::IFerroList;
pub use i_ferro_list_item_validator::IFerroListItemValidator;
pub use i_ferro_read_only_dictionary::IFerroReadOnlyDictionary;
pub use i_ferro_read_only_list::IFerroReadOnlyList;
pub use notify_collection_changed_extensions::NotifyCollectionChangedExtensions;
pub use ferro_list::{
    SharedNotifyCollectionChangedEventArgs,
    CollectionChangedHandler, FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs,
    ResetBehavior, WeakFerroList,
};
