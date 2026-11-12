//! The drawings a composition visual shows for the visuals below its own that have no
//! composition visual: the drawing items (docs/porting/desktop-performance.md, section 9.42).
//!
//! A visual that draws, and needs nothing else of a composition visual, is an item of the
//! composition visual of the nearest visual above it that has one: its drawing, where it is in
//! that visual, and its size when it clips to it. The items and the child visuals are painted
//! in one order: `child_positions` says how many items come before each child.

use super::{IDirtyRectTracker, IServerRenderResource, IServerRenderResourceObserver, ServerCompositionVisual, ServerCompositor};
use crate::platform::{FragmentKeeping, IDrawingContextImpl, IDrawingFragment, LtrbRect};
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

/// How many times a drawing is drawn from its commands before a backend that keeps settled
/// drawings ([`FragmentKeeping::Settled`]) makes a fragment of it.
const SETTLED_AFTER: u8 = 1;
/// The most drawings from the commands between two fragments of a drawing whose fragments the
/// backend refuses to draw.
const LONGEST_WAIT: u8 = 64;
/// The backend made no fragment of the drawing: none is asked for until the drawing changes.
const NO_FRAGMENT: u8 = u8::MAX;

/// What the backend kept of a drawing, and when the next fragment of it is made: the part of
/// [`draw_with_fragment`] that lives as long as the drawing does.
pub(super) struct KeptFragment {
    fragment: OptionCell<Rc<dyn IDrawingFragment>>,
    /// How many times the drawing is still drawn from its commands before a fragment is made
    /// ([`FragmentKeeping::Settled`]), or [`NO_FRAGMENT`].
    wait: Cell<u8>,
    /// What `wait` was set to when the backend last refused to draw the fragment.
    refused_wait: Cell<u8>,
}

impl Default for KeptFragment {
    fn default() -> Self {
        Self { fragment: OptionCell::new(), wait: Cell::new(SETTLED_AFTER), refused_wait: Cell::new(SETTLED_AFTER) }
    }
}

impl KeptFragment {

    /// The drawing is another one, or was drawn again: what was kept of it is dropped.
    pub(super) fn clear(&self) {
        self.fragment.set(None);
        self.wait.set(SETTLED_AFTER);
        self.refused_wait.set(SETTLED_AFTER);
    }

    #[cfg(test)]
    pub(super) fn is_kept(&self) -> bool {
        self.fragment.get().is_some()
    }
}

/// Draws `commands` from the fragment the backend kept of them when it can, and has a new one
/// made when the backend wants one ([`FragmentKeeping`]).
///
/// The policy is the compositor's, one for every backend: a backend says when a fragment is
/// worth making, draws one, and refuses the ones it cannot draw as the commands would.
///
/// Deviation (DEVIATIONS.md, Server composition visuals): upstream replays the commands every
/// time (`RenderCore`).
pub(super) fn draw_with_fragment(
    commands: &ServerCompositionRenderData,
    kept: &KeptFragment,
    canvas: &mut dyn IDrawingContextImpl,
) {
    draw_kept_or(kept, canvas, |canvas| commands.render(canvas));
}

/// [`draw_with_fragment`] for a drawing that `render` draws.
fn draw_kept_or(
    kept: &KeptFragment,
    canvas: &mut dyn IDrawingContextImpl,
    render: impl Fn(&mut dyn IDrawingContextImpl),
) {
    let Some(fragments) = canvas.as_drawing_context_impl_with_fragments() else {
        render(canvas);
        return;
    };
    let keeping = fragments.fragment_keeping();
    let refused = match kept.fragment.get() {
        Some(fragment) => {
            if fragments.draw_fragment(&fragment) {
                crate::perf_count!(RenderFragmentsDrawn);
                return;
            }
            true
        }
        None => false,
    };
    let make = match keeping {
        FragmentKeeping::None => false,
        FragmentKeeping::Each => true,
        FragmentKeeping::Settled if refused => {
            // The drawing is at a state the fragment was not made at (it is scaled, it
            // fades): while that lasts a fragment a frame would be made and refused.
            let wait = kept.refused_wait.get().saturating_mul(2).min(LONGEST_WAIT);
            kept.fragment.set(None);
            kept.refused_wait.set(wait);
            kept.wait.set(wait);
            false
        }
        FragmentKeeping::Settled => match kept.wait.get() {
            0 => true,
            NO_FRAGMENT => false,
            wait => {
                kept.wait.set(wait - 1);
                false
            }
        },
    };
    if !make {
        render(fragments);
        return;
    }
    fragments.begin_fragment();
    render(fragments);
    let fragment = fragments.end_fragment();
    crate::perf_count!(RenderFragmentsMade);
    if fragment.is_none() && keeping == FragmentKeeping::Settled {
        kept.wait.set(NO_FRAGMENT);
    }
    kept.fragment.set(fragment);
}

/// The drawing of a visual without a composition visual, in the composition visual that
/// shows it.
pub struct ServerDrawItem {
    holder: Weak<ServerCompositionVisual>,
    data: OptionCell<Rc<ServerCompositionRenderData>>,
    /// What the backend kept of the drawing: dropped when the drawing or a resource of it
    /// changes.
    fragment: KeptFragment,
    /// Where the drawing is in the holder.
    offset: Cell<(f64, f64)>,
    /// The size the drawing is clipped to.
    clip: Cell<Option<(f64, f64)>>,
    /// The bounds the dirty region was last told of, in the holder.
    announced: Cell<Option<LtrbRect>>,
    /// Whether something of the item changed since.
    dirty: Cell<bool>,
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
            fragment: KeptFragment::default(),
            offset: Cell::new((0.0, 0.0)),
            clip: Cell::new(None),
            announced: Cell::new(None),
            dirty: Cell::new(true),
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
        self.fragment.clear();
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
        self.fragment.clear();
        self.dirty.set(true);
        if let Some(holder) = self.holder.upgrade() {
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
                item.fragment.clear();
            }
            if bits & DRAW_ITEM_OFFSET != 0 {
                item.read_offset(reader);
            }
            if bits & DRAW_ITEM_CLIP != 0 {
                item.read_clip(reader);
            }
            item.dirty.set(true);
        }

        visual.set_has_draw_items(!self.items.borrow().is_empty());
        visual.content_items_changed();
    }

    /// The bounds of the items, in the holder.
    pub(super) fn bounds(&self) -> Option<LtrbRect> {
        let mut bounds = None;
        for item in self.items.borrow().iter() {
            bounds = LtrbRect::full_union(bounds, item.bounds());
        }
        bounds
    }

    /// Tells `add` what the changes of the items since the last time changed on screen, in the
    /// holder: where the removed items were, and where each changed item was and is.
    pub(super) fn drain_dirty_rects(&self, add: &mut dyn FnMut(LtrbRect)) {
        for rect in self.removed.borrow_mut().drain(..) {
            add(rect);
        }
        for item in self.items.borrow().iter() {
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
        let mut moved = false;
        for item in &items[range] {
            let Some(bounds) = item.bounds() else { continue };
            let world = bounds.transform_to_aabb(transform);
            if !clip.intersects(world) || dirty_rects.is_some_and(|dirty_rects| !dirty_rects.intersects(world)) {
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
            draw_with_fragment(&data, &item.fragment, canvas);
            if clip.is_some() {
                canvas.pop_clip();
            }
        }
        moved
    }

    pub(super) fn clear(&self) {
        self.items.borrow_mut().clear();
        self.child_positions.borrow_mut().clear();
        self.removed.borrow_mut().clear();
    }
}

#[cfg(test)]
#[path = "server_draw_items_tests.rs"]
mod tests;
