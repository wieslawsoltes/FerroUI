//! The prototype of #1038 (feature `layout-columns`, off by default): the geometry and the layout
//! flags of the elements in flat columns of their window, behind the same methods of `Visual` and
//! `Layoutable`.
//!
//! - **Slots.** An element holds a [`LayoutSlot`] of eight bytes: an index, its generation and the
//!   store. It takes a row at its first write, so an element never laid out costs no row and its
//!   construction does not reach the stores. A row that is released gets a new generation, so a
//!   slot cannot read the row of another element.
//! - **Windows.** Each window (by the identity of its presentation source) has its own columns,
//!   made when its first element attaches and released with its last. An element without a window
//!   is in the detached store, and moving between stores copies its row, so an element keeps its
//!   values across a detach, as it does with fields.
//! - **Worker threads.** [`with_frozen_columns`] lends the columns of a window as plain slices that
//!   are `Send` and `Sync`. The UI thread is inside the call for as long as they are lent, and a
//!   write in that time is refused by the borrow, so a worker can read them without a lock. No
//!   parallel layout is implemented here.

use crate::{Rect, Size};
use std::cell::{Cell, RefCell};

/// `IsMeasureValid`.
pub(crate) const MEASURE_VALID: u8 = 1 << 0;
/// The measure of the element and of every layoutable below it is invalid.
pub(crate) const SUBTREE_MEASURE_INVALID: u8 = 1 << 1;
/// `IsArrangeValid`.
pub(crate) const ARRANGE_VALID: u8 = 1 << 2;
/// The element is inside its measure.
pub(crate) const MEASURING: u8 = 1 << 3;
/// The element was given the size it had at another place.
pub(crate) const ARRANGE_MOVED_ONLY: u8 = 1 << 4;

/// The index of a slot without a row.
const NO_ROW: u32 = u32::MAX;
/// The store of the elements without a window.
const DETACHED: u16 = 0;

/// The columns of one window.
#[derive(Default)]
struct LayoutColumns {
    generations: Vec<u16>,
    desired_sizes: Vec<Size>,
    bounds: Vec<Rect>,
    flags: Vec<u8>,
    free: Vec<u32>,
    live: u32,
}

impl LayoutColumns {
    /// A row with default values, and its generation.
    fn allocate(&mut self) -> (u32, u16) {
        self.live += 1;
        if let Some(index) = self.free.pop() {
            return (index, self.generations[index as usize]);
        }
        let index = self.generations.len() as u32;
        self.generations.push(0);
        self.desired_sizes.push(Size::default());
        self.bounds.push(Rect::default());
        self.flags.push(0);
        (index, 0)
    }

    /// Gives a row back. A row whose generation is used up is never given out again.
    fn release(&mut self, index: u32) {
        let row = index as usize;
        self.live -= 1;
        self.desired_sizes[row] = Size::default();
        self.bounds[row] = Rect::default();
        self.flags[row] = 0;
        if let Some(next) = self.generations[row].checked_add(1) {
            self.generations[row] = next;
            self.free.push(index);
        } else {
            self.generations[row] = u16::MAX;
        }
    }

    #[inline]
    fn holds(&self, index: u32, generation: u16) -> bool {
        self.generations.get(index as usize) == Some(&generation)
    }
}

/// The stores of a thread.
struct Stores {
    /// By store; the first is [`DETACHED`].
    columns: Vec<Option<LayoutColumns>>,
    /// The store of each window.
    windows: Vec<(usize, u16)>,
    free: Vec<u16>,
}

impl Stores {
    fn new() -> Self {
        Self { columns: vec![Some(LayoutColumns::default())], windows: Vec::new(), free: Vec::new() }
    }

    /// The store of `window`, made if it has none; the detached store when no id is left.
    fn store_of(&mut self, window: usize) -> u16 {
        if let Some((_, store)) = self.windows.iter().find(|(key, _)| *key == window) {
            return *store;
        }
        let store = match self.free.pop() {
            Some(store) => {
                self.columns[store as usize] = Some(LayoutColumns::default());
                store
            }
            None if self.columns.len() <= usize::from(u16::MAX) => {
                self.columns.push(Some(LayoutColumns::default()));
                (self.columns.len() - 1) as u16
            }
            None => return DETACHED,
        };
        self.windows.push((window, store));
        store
    }

    /// Lets a window's store go once its last element has left.
    fn release_if_empty(&mut self, store: u16) {
        if store == DETACHED || self.columns[store as usize].as_ref().is_none_or(|columns| columns.live != 0) {
            return;
        }
        self.columns[store as usize] = None;
        self.windows.retain(|(_, held)| *held != store);
        self.free.push(store);
    }
}

crate::ferro_thread_local! {
    static STORES: RefCell<Stores> = RefCell::new(Stores::new());
}

/// The handle of an element to its row.
pub(crate) struct LayoutSlot {
    index: Cell<u32>,
    generation: Cell<u16>,
    store: Cell<u16>,
}

impl LayoutSlot {
    /// A slot without a row: it reads the default values.
    #[inline]
    pub(crate) const fn new() -> Self {
        Self { index: Cell::new(NO_ROW), generation: Cell::new(0), store: Cell::new(DETACHED) }
    }

    #[inline]
    fn read<R: Copy>(&self, default: R, read: impl FnOnce(&LayoutColumns, usize) -> R) -> R {
        let index = self.index.get();
        if index == NO_ROW {
            return default;
        }
        STORES
            .try_with(|stores| {
                let stores = stores.borrow();
                match stores.columns.get(usize::from(self.store.get())) {
                    Some(Some(columns)) if columns.holds(index, self.generation.get()) => {
                        read(columns, index as usize)
                    }
                    _ => default,
                }
            })
            .unwrap_or(default)
    }

    #[inline]
    fn write(&self, write: impl FnOnce(&mut LayoutColumns, usize)) {
        let _ = STORES.try_with(|stores| {
            let mut stores = stores.borrow_mut();
            let store = usize::from(self.store.get());
            let Some(Some(columns)) = stores.columns.get_mut(store) else {
                return;
            };
            if self.index.get() == NO_ROW || !columns.holds(self.index.get(), self.generation.get()) {
                let (index, generation) = columns.allocate();
                self.index.set(index);
                self.generation.set(generation);
            }
            write(columns, self.index.get() as usize);
        });
    }

    #[inline]
    pub(crate) fn desired_size(&self) -> Size {
        self.read(Size::default(), |columns, row| columns.desired_sizes[row])
    }

    #[inline]
    pub(crate) fn set_desired_size(&self, value: Size) {
        self.write(|columns, row| columns.desired_sizes[row] = value);
    }

    #[inline]
    pub(crate) fn bounds(&self) -> Rect {
        self.read(Rect::default(), |columns, row| columns.bounds[row])
    }

    #[inline]
    pub(crate) fn set_bounds(&self, value: Rect) {
        self.write(|columns, row| columns.bounds[row] = value);
    }

    #[inline]
    pub(crate) fn flag(&self, flag: u8) -> bool {
        self.read(false, |columns, row| columns.flags[row] & flag != 0)
    }

    #[inline]
    pub(crate) fn set_flag(&self, flag: u8, value: bool) {
        // A flag that is cleared on an element without a row stays what it reads.
        if !value && self.index.get() == NO_ROW {
            return;
        }
        self.write(|columns, row| {
            if value {
                columns.flags[row] |= flag;
            } else {
                columns.flags[row] &= !flag;
            }
        });
    }

    /// Moves the row to the columns of `window`.
    pub(crate) fn move_to_window(&self, window: usize) {
        let _ = STORES.try_with(|stores| {
            let mut stores = stores.borrow_mut();
            let store = stores.store_of(window);
            self.move_to(&mut stores, store);
        });
    }

    /// Moves the row to the columns of the elements without a window.
    pub(crate) fn move_to_detached(&self) {
        let _ = STORES.try_with(|stores| self.move_to(&mut stores.borrow_mut(), DETACHED));
    }

    fn move_to(&self, stores: &mut Stores, store: u16) {
        let old_store = self.store.get();
        if old_store == store {
            return;
        }
        let old_index = self.index.get();
        let row = (old_index != NO_ROW)
            .then(|| stores.columns[usize::from(old_store)].as_mut())
            .flatten()
            .filter(|columns| columns.holds(old_index, self.generation.get()))
            .map(|columns| {
                let row = old_index as usize;
                let values = (columns.desired_sizes[row], columns.bounds[row], columns.flags[row]);
                columns.release(old_index);
                values
            });
        self.store.set(store);
        match (row, stores.columns[usize::from(store)].as_mut()) {
            (Some((desired_size, bounds, flags)), Some(columns)) => {
                let (index, generation) = columns.allocate();
                let new_row = index as usize;
                columns.desired_sizes[new_row] = desired_size;
                columns.bounds[new_row] = bounds;
                columns.flags[new_row] = flags;
                self.index.set(index);
                self.generation.set(generation);
            }
            // Nothing written yet: the first write takes a row in the new store, which an attached
            // element's window keeps alive through the row it takes here.
            (None, Some(columns)) if store != DETACHED => {
                let (index, generation) = columns.allocate();
                self.index.set(index);
                self.generation.set(generation);
            }
            _ => self.index.set(NO_ROW),
        }
        stores.release_if_empty(old_store);
    }
}

impl Drop for LayoutSlot {
    fn drop(&mut self) {
        let index = self.index.get();
        if index == NO_ROW {
            return;
        }
        let _ = STORES.try_with(|stores| {
            let mut stores = stores.borrow_mut();
            let store = self.store.get();
            if let Some(Some(columns)) = stores.columns.get_mut(usize::from(store)) {
                if columns.holds(index, self.generation.get()) {
                    columns.release(index);
                }
            }
            stores.release_if_empty(store);
        });
    }
}

/// The columns of a window lent to readers on other threads.
#[derive(Clone, Copy)]
#[allow(dead_code)] // See `with_frozen_columns`.
pub(crate) struct FrozenLayoutColumns<'a> {
    /// The generation of each row.
    pub(crate) generations: &'a [u16],
    /// The desired size of each row.
    pub(crate) desired_sizes: &'a [Size],
    /// The bounds of each row.
    pub(crate) bounds: &'a [Rect],
    /// The layout flags of each row.
    pub(crate) flags: &'a [u8],
}

/// Calls `read` with the columns of `window`; `None` when the window has none. A write on this
/// thread during the call panics at the borrow, rather than race with the readers.
#[allow(dead_code)] // The route a worker would take: nothing of the prototype runs on a worker.
pub(crate) fn with_frozen_columns<R>(window: usize, read: impl FnOnce(FrozenLayoutColumns<'_>) -> R) -> Option<R> {
    STORES.with(|stores| {
        let stores = stores.borrow();
        let (_, store) = stores.windows.iter().find(|(key, _)| *key == window)?;
        let columns = stores.columns[usize::from(*store)].as_ref()?;
        Some(read(FrozenLayoutColumns {
            generations: &columns.generations,
            desired_sizes: &columns.desired_sizes,
            bounds: &columns.bounds,
            flags: &columns.flags,
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW_A: usize = 0x1000;
    const WINDOW_B: usize = 0x2000;

    fn stores_of(window: usize) -> bool {
        STORES.with(|stores| stores.borrow().windows.iter().any(|(key, _)| *key == window))
    }

    #[test]
    fn a_slot_is_eight_bytes() {
        assert_eq!(8, std::mem::size_of::<LayoutSlot>());
    }

    #[test]
    fn a_slot_without_a_row_reads_the_defaults_and_takes_no_row() {
        let slot = LayoutSlot::new();
        assert_eq!(Size::default(), slot.desired_size());
        assert_eq!(Rect::default(), slot.bounds());
        assert!(!slot.flag(MEASURE_VALID));
        slot.set_flag(MEASURE_VALID, false);
        assert_eq!(NO_ROW, slot.index.get());
    }

    #[test]
    fn a_released_row_is_not_read_by_its_old_generation() {
        let first = LayoutSlot::new();
        first.set_desired_size(Size::new(3.0, 4.0));
        let (index, generation) = (first.index.get(), first.generation.get());
        drop(first);

        let second = LayoutSlot::new();
        second.set_bounds(Rect::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(index, second.index.get());
        assert_ne!(generation, second.generation.get());
        assert_eq!(Size::default(), second.desired_size());

        let stale = LayoutSlot::new();
        stale.index.set(index);
        stale.generation.set(generation);
        assert_eq!(Rect::default(), stale.bounds());
        // The stale handle owns nothing, so its drop does not release the row of `second`.
        drop(stale);
        assert_eq!(Rect::new(1.0, 2.0, 3.0, 4.0), second.bounds());
    }

    #[test]
    fn an_element_keeps_its_values_between_windows_and_when_detached() {
        let slot = LayoutSlot::new();
        slot.set_desired_size(Size::new(5.0, 6.0));
        slot.set_bounds(Rect::new(1.0, 1.0, 5.0, 6.0));
        slot.set_flag(MEASURE_VALID, true);
        slot.set_flag(ARRANGE_VALID, true);

        for window in [WINDOW_A, WINDOW_B] {
            slot.move_to_window(window);
            assert_eq!(Size::new(5.0, 6.0), slot.desired_size());
            assert_eq!(Rect::new(1.0, 1.0, 5.0, 6.0), slot.bounds());
            assert!(slot.flag(MEASURE_VALID) && slot.flag(ARRANGE_VALID) && !slot.flag(MEASURING));
        }
        assert!(!stores_of(WINDOW_A), "the window its last element left has no columns");
        slot.move_to_detached();
        assert_eq!(Size::new(5.0, 6.0), slot.desired_size());
        assert!(!stores_of(WINDOW_B));
    }

    #[test]
    fn a_window_is_released_with_its_last_element() {
        let (first, second) = (LayoutSlot::new(), LayoutSlot::new());
        first.move_to_window(WINDOW_A);
        second.move_to_window(WINDOW_A);
        drop(first);
        assert!(stores_of(WINDOW_A));
        drop(second);
        assert!(!stores_of(WINDOW_A));
    }

    #[test]
    fn a_worker_reads_the_frozen_columns_of_a_window() {
        let slots: Vec<LayoutSlot> = (0..64).map(|_| LayoutSlot::new()).collect();
        for (at, slot) in slots.iter().enumerate() {
            slot.move_to_window(WINDOW_A);
            slot.set_desired_size(Size::new(at as f64, 1.0));
        }
        let expected: f64 = (0..64).map(|at| at as f64).sum();
        let total = with_frozen_columns(WINDOW_A, |columns| {
            std::thread::scope(|scope| {
                let (left, right) = columns.desired_sizes.split_at(32);
                let sum = |sizes: &'_ [Size]| sizes.iter().map(|size| size.width).sum::<f64>();
                let (a, b) = (scope.spawn(move || sum(left)), scope.spawn(move || sum(right)));
                a.join().expect("the first worker") + b.join().expect("the second worker")
            })
        });
        assert_eq!(Some(expected), total);
    }
}
