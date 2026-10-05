//! Observable collections.

mod ferro_dictionary;
mod ferro_list;
mod i_ferro_dictionary;
mod i_ferro_list;
mod i_ferro_list_item_validator;
mod i_ferro_read_only_dictionary;
mod i_ferro_read_only_list;

pub use ferro_dictionary::{FerroDictionary, WeakFerroDictionary};
pub use i_ferro_dictionary::IFerroDictionary;
pub use i_ferro_list::IFerroList;
pub use i_ferro_list_item_validator::IFerroListItemValidator;
pub use i_ferro_read_only_dictionary::IFerroReadOnlyDictionary;
pub use i_ferro_read_only_list::IFerroReadOnlyList;
pub use ferro_list::{
    SharedNotifyCollectionChangedEventArgs,
    CollectionChangedHandler, FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs,
    ResetBehavior, WeakFerroList,
};
