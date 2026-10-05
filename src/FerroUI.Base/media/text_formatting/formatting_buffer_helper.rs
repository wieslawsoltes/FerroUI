use std::collections::HashMap;
use std::hash::Hash;

use crate::utilities::ArrayBuilder;

/// Keeps the scratch buffers of the text formatter from holding on to large
/// allocations after formatting unusually long text.
pub(crate) struct FormattingBufferHelper;

/// 1MB, arbitrary, that's 512K characters or 128K object references on a 64-bit target.
const MAX_KEPT_BUFFER_SIZE_IN_BYTES: u64 = 1024 * 1024;

impl FormattingBufferHelper {
    pub fn clear_then_reset_if_too_large_builder<T>(array_builder: &mut ArrayBuilder<T>) {
        array_builder.clear();

        if Self::is_buffer_too_large::<T>(array_builder.capacity()) {
            array_builder.reset();
        }
    }

    pub fn clear_then_reset_if_too_large<T>(list: &mut Vec<T>) {
        list.clear();

        if Self::is_buffer_too_large::<T>(list.capacity()) {
            list.shrink_to_fit();
        }
    }

    #[allow(dead_code)] // upstream member without a user yet
    pub fn clear_then_reset_if_too_large_map<K: Eq + Hash, V>(dictionary: &mut HashMap<K, V>) {
        let approximate_capacity = dictionary.len().next_power_of_two();

        dictionary.clear();

        // The dictionary is in fact larger than that: it has entries and
        // buckets, but let's only count our data here.
        if Self::is_buffer_too_large::<(K, V)>(approximate_capacity) {
            dictionary.shrink_to_fit();
        }
    }

    #[inline]
    fn is_buffer_too_large<T>(capacity: usize) -> bool {
        std::mem::size_of::<T>() as u64 * capacity as u64 > MAX_KEPT_BUFFER_SIZE_IN_BYTES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL_SIZES: [usize; 4] = [1, 500, 10_000, 125_000];
    const LARGE_SIZES: [usize; 2] = [500_000, 1_000_000];

    fn fill_and_clear_list(item_count: usize) -> usize {
        let mut list: Vec<i32> = Vec::new();

        for i in 0..item_count {
            list.push(i as i32);
        }

        FormattingBufferHelper::clear_then_reset_if_too_large(&mut list);

        list.capacity()
    }

    #[test]
    fn should_keep_small_buffer_list() {
        for item_count in SMALL_SIZES {
            assert!(fill_and_clear_list(item_count) >= item_count);
        }
    }

    #[test]
    fn should_reset_large_buffer_list() {
        for item_count in LARGE_SIZES {
            assert_eq!(fill_and_clear_list(item_count), 0);
        }
    }

    fn fill_and_clear_array_builder(item_count: usize) -> usize {
        let mut array_builder: ArrayBuilder<i32> = ArrayBuilder::new();

        for i in 0..item_count {
            array_builder.add_item(i as i32);
        }

        FormattingBufferHelper::clear_then_reset_if_too_large_builder(&mut array_builder);

        array_builder.capacity()
    }

    #[test]
    fn should_keep_small_buffer_array_builder() {
        for item_count in SMALL_SIZES {
            assert!(fill_and_clear_array_builder(item_count) >= item_count);
        }
    }

    #[test]
    fn should_reset_large_buffer_array_builder() {
        for item_count in LARGE_SIZES {
            assert_eq!(fill_and_clear_array_builder(item_count), 0);
        }
    }

    fn fill_and_clear_dictionary(item_count: usize) -> usize {
        let mut dictionary: HashMap<i32, i32> = HashMap::new();

        for i in 0..item_count {
            dictionary.insert(i as i32, i as i32);
        }

        FormattingBufferHelper::clear_then_reset_if_too_large_map(&mut dictionary);

        dictionary.capacity()
    }

    #[test]
    fn should_keep_small_buffer_dictionary() {
        for item_count in SMALL_SIZES {
            assert!(fill_and_clear_dictionary(item_count) >= item_count);
        }
    }

    #[test]
    fn should_reset_large_buffer_dictionary() {
        for item_count in LARGE_SIZES {
            // Upstream's dictionary trims to its smallest size (3); the map here releases everything.
            assert!(fill_and_clear_dictionary(item_count) <= 3);
        }
    }
}
