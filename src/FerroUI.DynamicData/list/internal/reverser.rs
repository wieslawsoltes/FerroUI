//! `Reverser<T>` (`List/Linq/Reverser.cs`).

use crate::list::{Change, ChangeSet, ListChangeReason};

/// The changes of a list as the changes of the list in reverse order (`Reverser<T>`), keeping the
/// length of the list between change sets.
#[derive(Default)]
pub(crate) struct Reverser {
    length: isize,
}

/// An index of upstream, where `-1` is none.
fn signed(index: Option<usize>) -> isize {
    index.map_or(-1, |index| index as isize)
}

/// The index of the port: none for a negative index.
fn unsigned(index: isize) -> Option<usize> {
    usize::try_from(index).ok()
}

impl Reverser {
    /// The changes reversed (`Reverse`).
    pub(crate) fn reverse<T: Clone>(&mut self, changes: &ChangeSet<T>) -> Vec<Change<T>> {
        let mut result = Vec::with_capacity(changes.count());
        for change in changes {
            match change.reason() {
                ListChangeReason::Add => {
                    let item = change.item();
                    result.push(Change::new(ListChangeReason::Add, item.current().clone(), unsigned(self.length - signed(item.current_index()))));
                    self.length += 1;
                }
                ListChangeReason::AddRange => {
                    let range = change.range();
                    let offset = match range.index() {
                        None => 0,
                        Some(index) => self.length - index as isize,
                    };
                    result.push(Change::range_change(ListChangeReason::AddRange, range.items().iter().rev().cloned().collect(), unsigned(offset)));
                    self.length += range.count() as isize;
                }
                ListChangeReason::Replace => {
                    let item = change.item();
                    let new_index = unsigned(self.length - signed(item.current_index()) - 1);
                    result.push(Change::with_previous(
                        ListChangeReason::Replace,
                        item.current().clone(),
                        Some(item.previous().expect("a replacement has a previous item").clone()),
                        new_index,
                        new_index,
                    ));
                }
                ListChangeReason::Remove => {
                    let item = change.item();
                    result.push(Change::new(ListChangeReason::Remove, item.current().clone(), unsigned(self.length - signed(item.current_index()) - 1)));
                    self.length -= 1;
                }
                ListChangeReason::RemoveRange => {
                    let range = change.range();
                    let offset = self.length - signed(range.index()) - range.count() as isize;
                    result.push(Change::range_change(ListChangeReason::RemoveRange, range.items().iter().rev().cloned().collect(), unsigned(offset)));
                    self.length -= range.count() as isize;
                }
                ListChangeReason::Moved => {
                    let item = change.item();
                    let current_index = self.length - signed(item.current_index()) - 1;
                    let previous_index = self.length - signed(item.previous_index()) - 1;
                    result.push(Change::moved(
                        item.current().clone(),
                        usize::try_from(current_index).expect("a move has a current index"),
                        usize::try_from(previous_index).expect("a move has a previous index"),
                    ));
                }
                ListChangeReason::Clear => {
                    result.push(Change::range_change(ListChangeReason::Clear, change.range().items().iter().rev().cloned().collect(), None));
                    self.length = 0;
                }
                // Not handled upstream.
                ListChangeReason::Refresh => {}
            }
        }
        result
    }
}
