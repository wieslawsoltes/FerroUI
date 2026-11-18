//! The drawings a composition visual shows for the visuals below its own that have no
//! composition visual: the drawing items (docs/porting/desktop-performance.md, section 9.42).
//!
//! A visual that draws, and needs nothing else of a composition visual, is an item of the
//! composition visual of the nearest visual above it that has one: its drawing, where it is in
//! that visual, and its size when it clips to it. The items and the child visuals are painted
//! in one order: `child_positions` says how many items come before each child.

use super::{IDirtyRectTracker, IServerRenderResource, IServerRenderResourceObserver, ServerCompositionVisual, ServerCompositor};
use crate::platform::{IDrawingContextImpl, IDrawingFragment, LtrbRect};
use crate::rendering::composition::drawing::ServerCompositionRenderData;
use crate::rendering::composition::transport::BatchStreamReader;
use crate::utilities::OptionCell;
use crate::{Matrix, Rect};
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::{Rc, Weak};

/// What an update of an item carries: its drawing is another one.
pub(crate) const DRAW_ITEM_CONTENT: u8 = 1;
/// What an update of an item carries: the same drawing is drawn again.
pub(crate) const DRAW_ITEM_REDRAWN: u8 = 2;
/// What an update of an item carries: it is somewhere else in its holder.
pub(crate) const DRAW_ITEM_OFFSET: u8 = 4;
/// What an update of an item carries: its clip is another one.
pub(crate) const DRAW_ITEM_CLIP: u8 = 8;

/// The rectangles of removed items that are kept apart; the ones after them are united with
/// the last.
const REMOVED_RECTS: usize = 16;

/// The items of a range whose bounds are kept (docs/porting/desktop-performance.md, section
/// 9.90): the items are in the order they are painted in, which is the order of the tree, so
/// the items of a range lie close together and their bounds are small.
const RANGE: usize = 64;

/// What is kept of a range of [`RANGE`] items.
#[derive(Clone, Copy)]
struct ItemRange {
    /// The bounds of the items of the range, in the holder, when `known`.
    bounds: Option<LtrbRect>,
    /// Whether `bounds` is what the items have now: no item of the range changed since.
    known: bool,
    /// Whether an item of the range changed since the dirty region was last told.
    changed: bool,
}

impl ItemRange {
    const CHANGED: ItemRange = ItemRange { bounds: None, known: false, changed: true };
}

/// Draws `commands` from the fragment the backend kept of them when it can, and keeps a new
/// one when it cannot.
///
/// Deviation (DEVIATIONS.md, Server composition visuals): upstream replays the commands every
/// time (`RenderCore`).
pub(super) fn draw_with_fragment(
    commands: &ServerCompositionRenderData,
    fragment: &OptionCell<Rc<dyn IDrawingFragment>>,
    canvas: &mut dyn IDrawingContextImpl,
) {
    if let Some(fragments) = canvas.as_drawing_context_impl_with_fragments() {
        if fragment.get().is_some_and(|fragment| fragments.draw_fragment(&fragment)) {
            return;
        }
        fragments.begin_fragment();
        commands.render(fragments);
        fragment.set(fragments.end_fragment());
        return;
    }
    commands.render(canvas);
}

/// The drawing of a visual without a composition visual, in the composition visual that
/// shows it.
pub struct ServerDrawItem {
    holder: Weak<ServerCompositionVisual>,
    data: OptionCell<Rc<ServerCompositionRenderData>>,
    /// What the backend kept of the drawing: dropped when the drawing or a resource of it
    /// changes.
    fragment: OptionCell<Rc<dyn IDrawingFragment>>,
    /// Where the drawing is in the holder.
    offset: Cell<(f64, f64)>,
    /// The size the drawing is clipped to.
    clip: Cell<Option<(f64, f64)>>,
    /// The bounds the dirty region was last told of, in the holder.
    announced: Cell<Option<LtrbRect>>,
    /// Whether something of the item changed since.
    dirty: Cell<bool>,
    /// Where the item is among the items of its holder: the range it belongs to is told when
    /// its drawing changes.
    index: Cell<u32>,
    /// The frame of its last change, for the count of unchanged drawings (0 when the count is
    /// off; `unchanged_drawings`, #821).
    changed: super::unchanged_drawings::ChangedFrame,
}

impl ServerDrawItem {
    fn read(
        holder: &Weak<ServerCompositionVisual>,
        compositor: Option<&Rc<ServerCompositor>>,
        reader: &mut BatchStreamReader<'_>,
    ) -> Rc<ServerDrawItem> {
        let item = Rc::new(ServerDrawItem {
            holder: holder.clone(),
            data: OptionCell::new(),
            fragment: OptionCell::new(),
            offset: Cell::new((0.0, 0.0)),
            clip: Cell::new(None),
            announced: Cell::new(None),
            dirty: Cell::new(true),
            index: Cell::new(0),
            changed: super::unchanged_drawings::ChangedFrame::now(),
        });
        item.read_data(compositor, reader);
        item.read_offset(reader);
        item.read_clip(reader);
        item
    }

    fn read_data(self: &Rc<Self>, compositor: Option<&Rc<ServerCompositor>>, reader: &mut BatchStreamReader<'_>) {
        let data = reader.read_server_object().map(|id| {
            match compositor.and_then(|compositor| compositor.get::<ServerCompositionRenderData>(id)) {
                Some(data) => data,
                None => panic!("a batch refers to render data {id:?}, which does not exist"),
            }
        });
        let observer: Rc<dyn IServerRenderResourceObserver> = self.clone();
        if let Some(old) = self.data.get() {
            old.remove_observer(&observer);
        }
        if let Some(data) = &data {
            data.add_observer(&observer);
        }
        self.data.set(data);
        self.fragment.set(None);
    }

    fn read_offset(&self, reader: &mut BatchStreamReader<'_>) {
        let x = reader.read::<f64>();
        let y = reader.read::<f64>();
        self.offset.set((x, y));
    }

    fn read_clip(&self, reader: &mut BatchStreamReader<'_>) {
        self.clip.set(if reader.read::<u8>() != 0 {
            let width = reader.read::<f64>();
            let height = reader.read::<f64>();
            Some((width, height))
        } else {
            None
        });
    }

    /// The bounds of the item in the holder, as it is now.
    fn bounds(&self) -> Option<LtrbRect> {
        let bounds = self.data.get()?.bounds()?;
        let (x, y) = self.offset.get();
        let bounds = LtrbRect::new(bounds.left + x, bounds.top + y, bounds.right + x, bounds.bottom + y);
        match self.clip.get() {
            Some((width, height)) => bounds.intersect_or_null(LtrbRect::new(x, y, x + width, y + height)),
            None => Some(bounds),
        }
    }

    pub fn data(&self) -> Option<Rc<ServerCompositionRenderData>> {
        self.data.get()
    }

    pub fn offset(&self) -> (f64, f64) {
        self.offset.get()
    }

    pub fn clip(&self) -> Option<(f64, f64)> {
        self.clip.get()
    }
}

impl IServerRenderResourceObserver for ServerDrawItem {
    fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {
        self.fragment.set(None);
        self.dirty.set(true);
        self.changed.note();
        if let Some(holder) = self.holder.upgrade() {
            // The bounds of the drawing may be other ones.
            if let Some(items) = holder.draw_items() {
                items.item_changed(self.index.get() as usize);
            }
            holder.content_items_changed();
        }
    }
}

/// The items of a composition visual.
#[derive(Default)]
pub struct ServerDrawItems {
    items: RefCell<Vec<Rc<ServerDrawItem>>>,
    /// For each child visual, the number of items painted before it. A child without an entry
    /// is painted after every item.
    child_positions: RefCell<Vec<u32>>,
    /// The bounds of the items that were removed since the dirty region was last told.
    removed: RefCell<Vec<LtrbRect>>,
    /// What is kept of each range of [`RANGE`] items, in their order: a frame asks the ranges
    /// and looks only at the items of those that changed or that it draws, where it looked at
    /// every item three times (their bounds, their changes, what of them meets the dirty area).
    ranges: RefCell<Vec<ItemRange>>,
}

impl ServerDrawItems {
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    pub fn items(&self) -> Vec<Rc<ServerDrawItem>> {
        self.items.borrow().clone()
    }

    pub fn child_positions(&self) -> Vec<u32> {
        self.child_positions.borrow().clone()
    }

    /// Reads the changes of the items: the range that is replaced, the positions of the child
    /// visuals when they changed, and what changed of the items that stay.
    pub(super) fn deserialize(&self, visual: &ServerCompositionVisual, reader: &mut BatchStreamReader<'_>) {
        let compositor = visual.compositor();
        let holder = visual.weak();

        let start = reader.read::<u32>() as usize;
        let remove = reader.read::<u32>() as usize;
        let count = reader.read::<u32>() as usize;
        if remove != 0 || count != 0 {
            let mut added = Vec::with_capacity(count);
            for _ in 0..count {
                added.push(ServerDrawItem::read(&holder, compositor.as_ref(), reader));
            }
            let mut items = self.items.borrow_mut();
            let start = start.min(items.len());
            let end = (start + remove).min(items.len());
            let mut removed = self.removed.borrow_mut();
            for item in items.splice(start..end, added) {
                if let Some(bounds) = item.announced.get() {
                    if removed.len() < REMOVED_RECTS {
                        removed.push(bounds);
                    } else if let Some(last) = removed.last_mut() {
                        *last = last.union(bounds);
                    }
                }
            }
            // The items from the first new one on are somewhere else among the items, and so
            // in other ranges.
            for (index, item) in items.iter().enumerate().skip(start) {
                item.index.set(index as u32);
            }
            let mut ranges = self.ranges.borrow_mut();
            ranges.truncate(start / RANGE);
            ranges.resize(items.len().div_ceil(RANGE), ItemRange::CHANGED);
        }

        if reader.read::<u8>() != 0 {
            let len = reader.read::<u32>() as usize;
            let mut positions = self.child_positions.borrow_mut();
            positions.clear();
            positions.reserve_exact(len);
            for _ in 0..len {
                positions.push(reader.read::<u32>());
            }
        }

        let updates = reader.read::<u32>();
        for _ in 0..updates {
            let index = reader.read::<u32>() as usize;
            let bits = reader.read::<u8>();
            let item = match self.items.borrow().get(index) {
                Some(item) => item.clone(),
                None => panic!("a batch updates the drawing item {index}, which does not exist"),
            };
            if bits & DRAW_ITEM_CONTENT != 0 {
                item.read_data(compositor.as_ref(), reader);
            }
            if bits & DRAW_ITEM_REDRAWN != 0 {
                item.fragment.set(None);
            }
            if bits & DRAW_ITEM_OFFSET != 0 {
                item.read_offset(reader);
            }
            if bits & DRAW_ITEM_CLIP != 0 {
                item.read_clip(reader);
            }
            item.dirty.set(true);
            item.changed.note();
            self.item_changed(index);
        }

        visual.set_has_draw_items(!self.items.borrow().is_empty());
        visual.content_items_changed();
    }

    /// The item `index` changed: what is kept of its range is not what the items have.
    fn item_changed(&self, index: usize) {
        if let Some(range) = self.ranges.borrow_mut().get_mut(index / RANGE) {
            range.known = false;
            range.changed = true;
        }
    }

    /// The bounds of the items of the range `range`, in the holder: the kept ones, or those of
    /// its items now, kept.
    fn range_bounds(&self, items: &[Rc<ServerDrawItem>], range: usize) -> Option<LtrbRect> {
        if let Some(kept) = self.ranges.borrow().get(range).filter(|kept| kept.known) {
            return kept.bounds;
        }
        let start = range * RANGE;
        let end = (start + RANGE).min(items.len());
        let mut bounds = None;
        for item in &items[start.min(end)..end] {
            bounds = LtrbRect::full_union(bounds, item.bounds());
        }
        if let Some(kept) = self.ranges.borrow_mut().get_mut(range) {
            kept.bounds = bounds;
            kept.known = true;
        }
        bounds
    }

    /// The bounds of the items, in the holder.
    pub(super) fn bounds(&self) -> Option<LtrbRect> {
        let items = self.items.borrow();
        let mut bounds = None;
        for range in 0..items.len().div_ceil(RANGE) {
            bounds = LtrbRect::full_union(bounds, self.range_bounds(&items, range));
        }
        bounds
    }

    /// Tells `add` what the changes of the items since the last time changed on screen, in the
    /// holder: where the removed items were, and where each changed item was and is.
    pub(super) fn drain_dirty_rects(&self, add: &mut dyn FnMut(LtrbRect)) {
        for rect in self.removed.borrow_mut().drain(..) {
            add(rect);
        }
        let items = self.items.borrow();
        for range in 0..items.len().div_ceil(RANGE) {
            // Only a range that was told of a change has an item that changed.
            let changed = match self.ranges.borrow_mut().get_mut(range) {
                Some(kept) => std::mem::replace(&mut kept.changed, false),
                None => true,
            };
            if !changed {
                continue;
            }
            let start = range * RANGE;
            for item in &items[start..(start + RANGE).min(items.len())] {
                if !item.dirty.replace(false) {
                    continue;
                }
                let bounds = item.bounds();
                if let Some(old) = item.announced.replace(bounds) {
                    add(old);
                }
                if let Some(bounds) = bounds {
                    add(bounds);
                }
            }
        }
    }

    fn position(&self, child: usize, count: usize) -> usize {
        self.child_positions.borrow().get(child).map_or(count, |position| (*position as usize).min(count))
    }

    /// The items painted just before the child visual `child`: after the child before it.
    pub(super) fn range_before_child(&self, child: usize) -> Range<usize> {
        let count = self.count();
        let end = self.position(child, count);
        let start = if child == 0 { 0 } else { self.position(child - 1, count).min(end) };
        start..end
    }

    /// The items painted after the last of the `children` child visuals.
    pub(super) fn range_after_children(&self, children: usize) -> Range<usize> {
        let count = self.count();
        let start = if children == 0 { 0 } else { self.position(children - 1, count) };
        start..count
    }

    /// Draws the items of `range` onto `canvas`, whose transform is `transform`, that of the
    /// holder: the ones that meet `clip` and the dirty region. Returns whether the transform of
    /// the canvas is another one now.
    pub(super) fn render(
        &self,
        range: Range<usize>,
        canvas: &mut dyn IDrawingContextImpl,
        transform: Matrix,
        clip: LtrbRect,
        dirty_rects: Option<&dyn IDirtyRectTracker>,
    ) -> bool {
        let items = self.items.borrow();
        let meets = |bounds: LtrbRect| {
            let world = bounds.transform_to_aabb(transform);
            clip.intersects(world) && dirty_rects.is_none_or(|dirty_rects| dirty_rects.intersects(world))
        };
        let mut moved = false;
        let mut next = range.start.min(range.end);
        while next < range.end {
            // The items of `range` that are of one range of kept bounds: none of them is drawn
            // when the bounds of the range are outside the clip and the dirty region.
            let kept = next / RANGE;
            let end = ((kept + 1) * RANGE).min(range.end);
            let part = next..end;
            next = end;
            if !self.range_bounds(&items, kept).is_some_and(meets) {
                continue;
            }
            for item in &items[part] {
                if !item.bounds().is_some_and(meets) {
                    continue;
                }
                let Some(data) = item.data.get() else { continue };
                crate::perf_count!(RenderItemsDrawn);
                let (x, y) = item.offset.get();
                canvas.set_transform(Matrix::create_translation(x, y) * transform);
                moved = true;
                let clip = item.clip.get();
                if let Some((width, height)) = clip {
                    canvas.push_clip(Rect::new(0.0, 0.0, width, height));
                }
                super::unchanged_drawings::replay(
                    item.changed.get(),
                    || data.recorded_bytes(),
                    || draw_with_fragment(&data, &item.fragment, canvas),
                );
                if clip.is_some() {
                    canvas.pop_clip();
                }
            }
        }
        moved
    }

    pub(super) fn clear(&self) {
        self.items.borrow_mut().clear();
        self.child_positions.borrow_mut().clear();
        self.removed.borrow_mut().clear();
        self.ranges.borrow_mut().clear();
    }
}
