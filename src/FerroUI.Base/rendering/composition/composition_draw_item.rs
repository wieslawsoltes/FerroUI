//! The drawing of a visual that has no composition visual, in the composition visual of the
//! nearest visual above it that has one: a drawing item
//! (docs/porting/desktop-performance.md, section 9.42).
//!
//! A visual that draws, and needs nothing else of a composition visual, does not get one: what
//! it drew, where it is in the composition visual above and its size when it clips to it are
//! an item of that visual, the holder. The items and the child visuals of a holder are painted
//! in one order, that of the visual tree below it.
//!
//! Deviation (DEVIATIONS.md, Composition visuals): upstream creates a composition visual for
//! every visual.

use super::drawing::CompositionRenderData;
use super::server::{DRAW_ITEM_CLIP, DRAW_ITEM_CONTENT, DRAW_ITEM_OFFSET, DRAW_ITEM_REDRAWN};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::CompositionVisual;
use crate::platform::LtrbRect;
use crate::utilities::OptionCell;
use crate::{Ref, Visual, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// What the server is told of an item: one of the `DRAW_ITEM_*` bits each.
const CHANGES: u8 = DRAW_ITEM_CONTENT | DRAW_ITEM_REDRAWN | DRAW_ITEM_OFFSET | DRAW_ITEM_CLIP;
/// The item is in the list of its holder and the server was not told of it yet, or was told of
/// it as an item of another holder: it is written whole.
const FRESH: u8 = 0x40;
/// The item is counted among the updates that are being written.
const COLLECTED: u8 = 0x80;

/// The number of items of a holder that follow each other whose bounds are kept as one for
/// hit testing, and the number from which the items of a holder have such bounds (as the
/// children of a visual have an index from `HIT_TEST_AABB_TREE_THRESHOLD`, in buckets of as
/// many).
pub(crate) const HIT_TEST_ITEM_CHUNK: usize = 32;

/// The bounds of what a chunk of the items of a holder draws, in the coordinates of the
/// holder.
#[derive(Clone, Copy)]
enum ChunkBounds {
    /// Not calculated since an item of the chunk changed.
    Stale,
    /// The items of the chunk draw nothing.
    Empty,
    Known(LtrbRect),
}

/// The item is in no list.
const NO_INDEX: u32 = u32::MAX;

/// The visual left its visual tree and the item is kept for its return (section 9.32).
const DEPARTED: u8 = 1;
/// The drawing is clipped to `clip`.
const CLIPPED: u8 = 2;

/// What a visual without a composition visual drew, as the composition visual above shows it.
pub(crate) struct CompositionDrawItem {
    element: WeakRef<Visual>,
    data: OptionCell<Rc<CompositionRenderData>>,
    /// Where the drawing is in the holder.
    offset: Cell<(f64, f64)>,
    /// The size the drawing is clipped to, when `CLIPPED` is set.
    clip: Cell<(f64, f64)>,
    /// The composition visual whose list the item is in.
    holder: OptionCell<Weak<CompositionVisual>>,
    /// Where the item is in the list of its holder.
    index: Cell<u32>,
    /// What changed since the server was told: `DRAW_ITEM_*`, `FRESH` and `COLLECTED`.
    pending: Cell<u8>,
    flags: Cell<u8>,
}

impl CompositionDrawItem {
    pub(crate) fn new(
        element: &Visual,
        data: Rc<CompositionRenderData>,
        offset: (f64, f64),
        clip: Option<(f64, f64)>,
    ) -> Rc<CompositionDrawItem> {
        let item = CompositionDrawItem {
            element: element.to_ref().downgrade(),
            data: OptionCell::new(),
            offset: Cell::new(offset),
            clip: Cell::new(clip.unwrap_or_default()),
            holder: OptionCell::new(),
            index: Cell::new(NO_INDEX),
            pending: Cell::new(0),
            flags: Cell::new(if clip.is_some() { CLIPPED } else { 0 }),
        };
        item.data.set(Some(data));
        Rc::new(item)
    }

    /// The visual that drew the item, if it is alive.
    pub(crate) fn element(&self) -> Option<Ref<Visual>> {
        self.element.upgrade()
    }

    pub(crate) fn data(&self) -> Option<Rc<CompositionRenderData>> {
        self.data.get()
    }

    pub(crate) fn offset(&self) -> (f64, f64) {
        self.offset.get()
    }

    pub(crate) fn clip(&self) -> Option<(f64, f64)> {
        (self.flags.get() & CLIPPED != 0).then(|| self.clip.get())
    }

    /// Whether the visual left its tree and the item is kept for its return: it is neither
    /// drawn nor hit until then.
    pub(crate) fn is_departed(&self) -> bool {
        self.flags.get() & DEPARTED != 0
    }

    pub(crate) fn set_departed(&self, departed: bool) {
        let flags = self.flags.get();
        self.flags.set(if departed { flags | DEPARTED } else { flags & !DEPARTED });
    }

    /// Whether the item is in the list of `holder`.
    pub(crate) fn is_held_by(&self, holder: &CompositionVisual) -> bool {
        self.holder.with(|held| held.is_some_and(|held| std::ptr::eq(held.as_ptr(), holder)))
    }

    /// The composition visual that shows the item, if it is in the list of one.
    pub(crate) fn holder(&self) -> Option<Rc<CompositionVisual>> {
        self.holder.with(|held| held.and_then(Weak::upgrade))
    }

    /// A rectangle of the holder that what the item draws is inside: its clip, or the bounds
    /// of its drawing. `None` when it draws nothing.
    fn hit_bounds(&self) -> Option<LtrbRect> {
        let (x, y) = self.offset.get();
        if let Some((width, height)) = self.clip() {
            return self.data.is_some().then(|| LtrbRect::new(x, y, x + width, y + height));
        }
        let bounds = self.data.get()?.bounds()?;
        Some(LtrbRect::new(bounds.left + x, bounds.top + y, bounds.right + x, bounds.bottom + y))
    }

    /// Another drawing: the one the item had is disposed.
    pub(crate) fn set_data(self: &Rc<Self>, data: Rc<CompositionRenderData>) {
        if let Some(old) = self.data.replace(Some(data)) {
            old.dispose();
        }
        self.mark(DRAW_ITEM_CONTENT);
    }

    /// The same drawing, recorded again: something it draws through may have changed
    /// (section 9.14).
    pub(crate) fn mark_redrawn(self: &Rc<Self>) {
        self.mark(DRAW_ITEM_REDRAWN);
    }

    pub(crate) fn set_offset(self: &Rc<Self>, offset: (f64, f64)) {
        if self.offset.get() != offset {
            self.offset.set(offset);
            self.mark(DRAW_ITEM_OFFSET);
        }
    }

    pub(crate) fn set_clip(self: &Rc<Self>, clip: Option<(f64, f64)>) {
        if self.clip() != clip {
            let flags = self.flags.get();
            self.flags.set(if clip.is_some() { flags | CLIPPED } else { flags & !CLIPPED });
            self.clip.set(clip.unwrap_or_default());
            self.mark(DRAW_ITEM_CLIP);
        }
    }

    /// Takes the drawing out of the item, for the composition visual its visual gets.
    pub(crate) fn take_data(&self) -> Option<Rc<CompositionRenderData>> {
        self.data.take()
    }

    /// The end of the item: its visual draws nothing, or is gone. The holder is built again by
    /// the caller.
    pub(crate) fn release(&self) {
        if let Some(data) = self.data.take() {
            data.dispose();
        }
        self.set_departed(false);
    }

    fn mark(self: &Rc<Self>, bits: u8) {
        let pending = self.pending.get();
        self.pending.set(pending | bits);
        let Some(holder) = self.holder() else { return };
        let Some(list) = holder.draw_items() else { return };
        // It may be hit elsewhere.
        list.invalidate_hit_bounds(self.index.get());
        // With something pending it is already among the updates of its holder, or written
        // whole with its list.
        if pending == 0 {
            list.updated.borrow_mut().push(self.clone());
            holder.register_for_serialization();
        }
    }

    fn write_clip(&self, writer: &mut BatchStreamWriter<'_>) {
        match self.clip() {
            Some((width, height)) => {
                writer.write(1u8);
                writer.write(width);
                writer.write(height);
            }
            None => writer.write(0u8),
        }
    }

    fn write_full(&self, writer: &mut BatchStreamWriter<'_>) {
        writer.write_server_object(self.data.get().map(|data| data.server()));
        let (x, y) = self.offset.get();
        writer.write(x);
        writer.write(y);
        self.write_clip(writer);
    }
}

impl Drop for CompositionDrawItem {
    fn drop(&mut self) {
        // One that nothing released: its visual and its holder are gone.
        if let Some(data) = self.data.take() {
            data.dispose();
        }
    }
}

/// What is below a composition visual in paint order: the composition visuals of the nearest
/// visuals that have one, and the items of the visuals between that draw without one.
pub(crate) enum CompositionEntry {
    Visual(Ref<Visual>),
    Item(Ref<Visual>, Rc<CompositionDrawItem>),
}

/// The items of a composition visual, and what the server was not told of them yet.
#[derive(Default)]
pub(crate) struct DrawItemList {
    items: RefCell<Vec<Rc<CompositionDrawItem>>>,
    /// For each child visual, the number of items painted before it. A child without an entry
    /// is painted after every item.
    child_positions: RefCell<Vec<u32>>,
    /// The list the server has, when `items` is another one.
    sent: RefCell<Option<Vec<Rc<CompositionDrawItem>>>>,
    positions_changed: Cell<bool>,
    /// The items the server has that changed.
    updated: RefCell<Vec<Rc<CompositionDrawItem>>>,
    /// The bounds of every `HIT_TEST_ITEM_CHUNK` items, once a hit test asked for them: a list
    /// of fewer items has none.
    hit_bounds: OptionCell<Box<Vec<ChunkBounds>>>,
}

impl DrawItemList {
    pub(super) fn has_changes(&self) -> bool {
        self.sent.borrow().is_some() || self.positions_changed.get() || !self.updated.borrow().is_empty()
    }

    pub(crate) fn count(&self) -> usize {
        self.items.borrow().len()
    }

    pub(crate) fn item_at(&self, index: usize) -> Option<Rc<CompositionDrawItem>> {
        self.items.borrow().get(index).cloned()
    }

    /// The bounds of what the items of the chunk `chunk` draw (the items from
    /// `chunk * HIT_TEST_ITEM_CHUNK` on), in the coordinates of the holder; `None` when they
    /// draw nothing. They are calculated when an item of the chunk changed since they were
    /// last asked for: a hit test passes over the chunks it misses without a look at their
    /// items.
    pub(crate) fn chunk_bounds(&self, chunk: usize) -> Option<LtrbRect> {
        let items = self.items.borrow();
        let chunks = items.len().div_ceil(HIT_TEST_ITEM_CHUNK);
        if chunk >= chunks {
            return None;
        }
        let mut known = self.hit_bounds.take().unwrap_or_default();
        if known.len() != chunks {
            known.clear();
            known.resize(chunks, ChunkBounds::Stale);
        }
        if let ChunkBounds::Stale = known[chunk] {
            let start = chunk * HIT_TEST_ITEM_CHUNK;
            let end = items.len().min(start + HIT_TEST_ITEM_CHUNK);
            let bounds = items[start..end]
                .iter()
                .filter_map(|item| item.hit_bounds())
                .reduce(|bounds, item| bounds.union(item));
            known[chunk] = bounds.map_or(ChunkBounds::Empty, ChunkBounds::Known);
        }
        let bounds = match known[chunk] {
            ChunkBounds::Known(bounds) => Some(bounds),
            ChunkBounds::Stale | ChunkBounds::Empty => None,
        };
        self.hit_bounds.set(Some(known));
        bounds
    }

    /// Whether the list keeps the bounds of its chunks.
    #[cfg(test)]
    pub(crate) fn has_hit_bounds(&self) -> bool {
        self.hit_bounds.is_some()
    }

    /// The item at `index` draws something else, or elsewhere.
    fn invalidate_hit_bounds(&self, index: u32) {
        if let Some(mut known) = self.hit_bounds.take() {
            if let Some(chunk) = known.get_mut(index as usize / HIT_TEST_ITEM_CHUNK) {
                *chunk = ChunkBounds::Stale;
            }
            self.hit_bounds.set(Some(known));
        }
    }

    /// The items from `index` on are others, or at other places of the list.
    fn invalidate_hit_bounds_from(&self, index: usize) {
        let Some(mut known) = self.hit_bounds.take() else { return };
        let count = self.items.borrow().len();
        if count < HIT_TEST_ITEM_CHUNK {
            // The items of a short list are tested one by one.
            return;
        }
        known.truncate(index / HIT_TEST_ITEM_CHUNK);
        known.resize(count.div_ceil(HIT_TEST_ITEM_CHUNK), ChunkBounds::Stale);
        self.hit_bounds.set(Some(known));
    }

    /// The number of items painted before the child visual `child`.
    pub(crate) fn position(&self, child: usize) -> usize {
        let count = self.count();
        self.child_positions.borrow().get(child).map_or(count, |position| (*position as usize).min(count))
    }

    /// Makes `items` the items of `holder`, and `positions` where its child visuals are among
    /// them. Returns whether the server has to be told.
    pub(super) fn set(
        &self,
        holder: &CompositionVisual,
        items: Vec<Rc<CompositionDrawItem>>,
        positions: Vec<u32>,
    ) -> bool {
        let mut changed = false;
        // The items at the start of the list that stay.
        let (count, kept) = {
            let current = self.items.borrow();
            let stays = |(old, new): &(&Rc<CompositionDrawItem>, &Rc<CompositionDrawItem>)| {
                Rc::ptr_eq(old, new) && new.is_held_by(holder)
            };
            (current.len(), current.iter().zip(&items).take_while(stays).count())
        };
        if count != items.len() || kept != count {
            let old = std::mem::replace(&mut *self.items.borrow_mut(), items);
            for item in &old {
                if item.is_held_by(holder) {
                    item.index.set(NO_INDEX);
                }
            }
            for (index, item) in self.items.borrow().iter().enumerate() {
                if !item.is_held_by(holder) {
                    item.holder.set(Some(holder.this.clone()));
                    item.pending.set(item.pending.get() | FRESH);
                }
                item.index.set(index as u32);
            }
            for item in &old {
                if item.index.get() == NO_INDEX && item.is_held_by(holder) {
                    item.holder.set(None);
                }
            }
            let mut sent = self.sent.borrow_mut();
            if sent.is_none() {
                *sent = Some(old);
            }
            self.invalidate_hit_bounds_from(kept);
            changed = true;
        }
        if *self.child_positions.borrow() != positions {
            *self.child_positions.borrow_mut() = positions;
            self.positions_changed.set(true);
            changed = true;
        }
        changed
    }

    /// Writes what changed: the range of the list the server has that is replaced and the items
    /// that replace it, the positions of the child visuals when they changed, and what changed
    /// of the items that stay (`ServerDrawItems::deserialize` reads it).
    pub(super) fn serialize(&self, host: &CompositionVisual, writer: &mut BatchStreamWriter<'_>) {
        let items = self.items.borrow();

        match self.sent.borrow_mut().take() {
            Some(sent) => {
                let stays = |old: &Rc<CompositionDrawItem>, new: &Rc<CompositionDrawItem>| {
                    Rc::ptr_eq(old, new) && new.pending.get() & FRESH == 0
                };
                let prefix = sent.iter().zip(items.iter()).take_while(|(old, new)| stays(old, new)).count();
                let rest = sent.len().min(items.len()) - prefix;
                let suffix =
                    sent.iter().rev().zip(items.iter().rev()).take(rest).take_while(|(old, new)| stays(old, new)).count();
                let added = &items[prefix..items.len() - suffix];
                writer.write(prefix as u32);
                writer.write((sent.len() - prefix - suffix) as u32);
                writer.write(added.len() as u32);
                for item in added {
                    item.write_full(writer);
                    if item.is_held_by(host) {
                        item.pending.set(0);
                    }
                }
            }
            None => {
                writer.write(0u32);
                writer.write(0u32);
                writer.write(0u32);
            }
        }

        if self.positions_changed.replace(false) {
            let positions = self.child_positions.borrow();
            writer.write(1u8);
            writer.write(positions.len() as u32);
            for position in positions.iter() {
                writer.write(*position);
            }
        } else {
            writer.write(0u8);
        }

        // An item that left the list, or was written whole above, has nothing to update.
        let updated = std::mem::take(&mut *self.updated.borrow_mut());
        let mut count = 0u32;
        for item in &updated {
            let pending = item.pending.get();
            if pending & CHANGES != 0
                && pending & (COLLECTED | FRESH) == 0
                && item.is_held_by(host)
                && items.get(item.index.get() as usize).is_some_and(|listed| Rc::ptr_eq(listed, item))
            {
                item.pending.set(pending | COLLECTED);
                count += 1;
            }
        }
        writer.write(count);
        for item in &updated {
            let pending = item.pending.get();
            if pending & COLLECTED == 0 {
                continue;
            }
            item.pending.set(0);
            let bits = pending & CHANGES;
            writer.write(item.index.get());
            writer.write(bits);
            if bits & DRAW_ITEM_CONTENT != 0 {
                writer.write_server_object(item.data.get().map(|data| data.server()));
            }
            if bits & DRAW_ITEM_OFFSET != 0 {
                let (x, y) = item.offset.get();
                writer.write(x);
                writer.write(y);
            }
            if bits & DRAW_ITEM_CLIP != 0 {
                item.write_clip(writer);
            }
        }
    }
}
