use crate::items_source::ItemsChangedEventArgs;

/// Collection helpers shared by the items controls.
pub struct CollectionUtils;

impl CollectionUtils {
    /// The arguments of a reset notification.
    pub const RESET_EVENT_ARGS: ItemsChangedEventArgs<'static> = ItemsChangedEventArgs::RESET;

    /// Inserts `count` copies of `item` at `index`.
    pub fn insert_many<T: Clone>(list: &mut Vec<T>, index: usize, item: T, count: usize) {
        list.splice(index..index, std::iter::repeat_n(item, count));
    }
}
