use super::drawing::{RecordedDrawing, RenderDataDrawingContext};
use super::hit_testing::{GeometryCompositionHitTester, ICompositionHitTester, PointCompositionHitTester};
use super::server::RenderSurfaces;
use super::{
    release_kept_composition_visuals, CompositionDrawItem, CompositionDrawListVisual, CompositionTarget,
    CompositionEntry, CompositionHitTarget, CompositionTransparencyLevel, CompositionVisual, Compositor, HitFilters,
    KeptComposition, KeptCompositionVisual, KeptCompositionVisuals,
};
use crate::media::{DrawingContext, Geometry, GeometryHitTestResult, IntersectionResult, MediaContext};
use crate::reactive::{Disposable, IDisposable, Subscription};
use crate::rendering::{
    IHitTester, IPresentationSource, IRenderer, RendererDiagnostics, SceneInvalidatedEventArgs,
};
use crate::threading::DispatcherPriority;
use crate::utilities::HandlerList;
use crate::{Matrix, Point, Rect, Ref, Size, Visual};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_RENDERER_KEY: AtomicU64 = AtomicU64::new(1);

crate::ferro_thread_local! {
    /// The renderers of this thread by key: lets the `Send` continuation of
    /// a batch find its renderer once it is back on the thread.
    static RENDERERS: RefCell<HashMap<u64, Weak<CompositingRenderer>>> = RefCell::new(HashMap::new());
}

/// The epochs of the sets of the renderers: each is used by one renderer, from one clearing of
/// its sets to the next.
static NEXT_SETS_EPOCH: AtomicU64 = AtomicU64::new(1);

/// One of the sets of visuals a renderer keeps for its next update pass.
#[derive(Clone, Copy)]
enum VisualSet {
    /// The visuals whose drawing changed.
    Dirty = 0,
    /// The visuals whose composition properties changed but whose drawing did not.
    PropertiesDirty = 1,
    /// The visuals whose child composition visuals have to be synchronized.
    RecalculateChildren = 2,
    /// The visuals whose child composition visuals are synchronized before those of any other:
    /// each is the nearest visual with a composition visual above one that was given its
    /// composition visual since the last pass. It lets go of the composition visuals the new
    /// one takes over before the new one asks for them (section 9.41).
    ChildrenFirst = 3,
}

/// The sets of visuals of a renderer, each in the order in which its visuals were added.
///
/// A visual says itself which sets it is in (`Visual::renderer_sets`): the epoch of the sets it
/// was added to and a bit for each of them. Adding, removing and asking are a comparison of that
/// word, where the reference hashes the visual into a `HashSet<Visual>` (DEVIATIONS.md,
/// Rendering; docs/porting/desktop-performance.md, section 9.40). Clearing the sets takes a new
/// epoch, so no visual is visited to forget it. A visual is in the sets of one renderer at a
/// time: the renderer of the root it is under.
///
/// A removal clears the bit at once and leaves the entry in the list, which is compacted when
/// the stale entries are half of it or the set is read, so that removing is constant time
/// instead of a scan of the list for every visual that is invalidated while it has no
/// composition visual. The visuals a set yields are unchanged; a removed visual is released at
/// the next compaction.
struct VisualSets {
    epoch: u64,
    items: [Vec<Ref<Visual>>; Self::SETS],
    /// The visuals of each set.
    live: [usize; Self::SETS],
    /// The entries of each list whose visual was removed.
    stale: [usize; Self::SETS],
}

impl VisualSets {
    /// The bits of a word below its epoch: one for each set, and one more for each that marks a
    /// visual already kept while a list is compacted.
    const BITS: u32 = 2 * Self::SETS as u32;

    /// The number of sets.
    const SETS: usize = 4;

    fn new() -> Self {
        Self {
            epoch: Self::next_epoch(),
            items: Default::default(),
            live: [0; Self::SETS],
            stale: [0; Self::SETS],
        }
    }

    fn next_epoch() -> u64 {
        NEXT_SETS_EPOCH.fetch_add(1, Ordering::Relaxed)
    }

    /// The bits of `visual` for these sets: none when the visual was last added to other sets,
    /// or to these before they were cleared.
    fn bits(&self, visual: &Visual) -> u64 {
        let word = visual.renderer_sets.get();
        if word >> Self::BITS == self.epoch {
            word & ((1 << Self::BITS) - 1)
        } else {
            0
        }
    }

    fn set_bits(&self, visual: &Visual, bits: u64) {
        visual.renderer_sets.set(self.epoch << Self::BITS | bits);
    }

    fn add(&mut self, set: VisualSet, visual: &Visual) {
        let (bits, bit) = (self.bits(visual), 1 << set as u64);
        if bits & bit == 0 {
            self.set_bits(visual, bits | bit);
            self.items[set as usize].push(visual.to_ref());
            self.live[set as usize] += 1;
        }
    }

    fn contains(&self, set: VisualSet, visual: &Visual) -> bool {
        self.bits(visual) & (1 << set as u64) != 0
    }

    fn remove(&mut self, set: VisualSet, visual: &Visual) {
        let (bits, bit) = (self.bits(visual), 1 << set as u64);
        if bits & bit != 0 {
            self.set_bits(visual, bits & !bit);
            let index = set as usize;
            self.live[index] -= 1;
            self.stale[index] += 1;
            if self.stale[index] * 2 >= self.items[index].len() {
                self.compact(set);
            }
        }
    }

    /// Takes a visual out of every set.
    fn remove_all(&mut self, visual: &Visual) {
        if self.bits(visual) != 0 {
            for set in
                [VisualSet::Dirty, VisualSet::PropertiesDirty, VisualSet::RecalculateChildren, VisualSet::ChildrenFirst]
            {
                self.remove(set, visual);
            }
        }
    }

    /// Drops the stale entries of a list (and an entry added again after its removal is kept
    /// once, at its last place).
    fn compact(&mut self, set: VisualSet) {
        let index = set as usize;
        let (bit, kept) = (1 << set as u64, 1 << (set as u64 + Self::SETS as u64));
        let mut items = std::mem::take(&mut self.items[index]);
        items.reverse();
        items.retain(|item| {
            let bits = self.bits(item);
            let keep = bits & bit != 0 && bits & kept == 0;
            if keep {
                self.set_bits(item, bits | kept);
            }
            keep
        });
        items.reverse();
        for item in &items {
            self.set_bits(item, self.bits(item) & !kept);
        }
        self.live[index] = items.len();
        self.items[index] = items;
        self.stale[index] = 0;
    }

    /// The visuals of a set, in the order in which they were added.
    ///
    /// A visual that the sets of another renderer took since (it moved to another root between
    /// two passes) says so in its word and is not in the set any more.
    fn snapshot(&mut self, set: VisualSet) -> Vec<Ref<Visual>> {
        if self.stale[set as usize] > 0 {
            self.compact(set);
        }
        self.items[set as usize].iter().filter(|item| self.contains(set, item)).cloned().collect()
    }

    fn is_empty(&self, set: VisualSet) -> bool {
        self.live[set as usize] == 0
    }

    /// Empties every set.
    fn clear(&mut self) {
        self.epoch = Self::next_epoch();
        for items in &mut self.items {
            items.clear();
        }
        self.live = [0; Self::SETS];
        self.stale = [0; Self::SETS];
    }
}

/// A renderer that utilizes a [`Compositor`] to render the visual tree.
pub struct CompositingRenderer {
    this: Weak<CompositingRenderer>,
    key: u64,
    root: Weak<dyn IPresentationSource>,
    compositor: Rc<Compositor>,
    recorder: RefCell<RenderDataDrawingContext>,
    /// The visuals of the next update pass.
    sets: RefCell<VisualSets>,
    /// The composition visuals kept for visuals that left the tree since the last update pass,
    /// each with its visual: released in the next pass unless the visual is in a tree again
    /// (docs/porting/desktop-performance.md, section 9.32). The visuals are referred to weakly:
    /// the list keeps no visual alive.
    departed: RefCell<Vec<KeptCompositionVisual>>,
    /// The composition visuals of the visuals leaving the tree inside
    /// `keep_composition_visuals_departing_in`, while one runs: they are handed to its caller
    /// instead of being released in the next update pass (section 9.32).
    keeping: RefCell<Option<Vec<KeptCompositionVisual>>>,
    /// The visuals that were given a composition visual while the renderer could not place it:
    /// during an update pass, or while it placed another one (section 9.41).
    late: RefCell<Vec<Ref<Visual>>>,
    /// Whether the renderer is placing a composition visual created outside an update pass.
    placing: Cell<bool>,
    queued_update: Cell<bool>,
    queued_scene_invalidation: Cell<bool>,
    updating: Cell<bool>,
    is_disposed: Cell<bool>,
    composition_target: Rc<CompositionTarget>,
    diagnostics: Rc<RendererDiagnostics>,
    diagnostics_subscription: RefCell<Option<Subscription>>,
    scene_invalidated: HandlerList<dyn Fn(&SceneInvalidatedEventArgs)>,
}

impl CompositingRenderer {
    /// Creates the renderer of the tree hosted by `root`. `surfaces`
    /// returns the platform surfaces to render to.
    ///
    /// The renderer refers to its presentation source weakly: the source
    /// owns its renderer.
    pub fn new(
        root: &Rc<dyn IPresentationSource>,
        compositor: &Rc<Compositor>,
        surfaces: RenderSurfaces,
    ) -> Rc<CompositingRenderer> {
        Self::with_weak_root(Rc::downgrade(root), compositor, surfaces)
    }

    /// Like [`new`](Self::new), for a presentation source that is still
    /// being constructed.
    pub fn with_weak_root(
        root: Weak<dyn IPresentationSource>,
        compositor: &Rc<Compositor>,
        surfaces: RenderSurfaces,
    ) -> Rc<CompositingRenderer> {
        let key = NEXT_RENDERER_KEY.fetch_add(1, Ordering::SeqCst);
        let renderer = Rc::new_cyclic(|this: &Weak<CompositingRenderer>| CompositingRenderer {
            this: this.clone(),
            key,
            root,
            compositor: compositor.clone(),
            recorder: RefCell::new(RenderDataDrawingContext::new(Some(compositor.clone()))),
            sets: RefCell::new(VisualSets::new()),
            departed: RefCell::new(Vec::new()),
            keeping: RefCell::new(None),
            late: RefCell::new(Vec::new()),
            placing: Cell::new(false),
            queued_update: Cell::new(false),
            queued_scene_invalidation: Cell::new(false),
            updating: Cell::new(false),
            is_disposed: Cell::new(false),
            composition_target: compositor.create_composition_target(surfaces),
            diagnostics: RendererDiagnostics::new(),
            diagnostics_subscription: RefCell::new(None),
            scene_invalidated: HandlerList::new(),
        });
        let weak = Rc::downgrade(&renderer);
        let subscription = renderer.diagnostics.property_changed(move |diagnostics, property_name| {
            if let Some(renderer) = weak.upgrade() {
                renderer.on_diagnostics_property_changed(diagnostics, property_name);
            }
        });
        *renderer.diagnostics_subscription.borrow_mut() = Some(subscription);
        RENDERERS.with(|renderers| {
            let mut renderers = renderers.borrow_mut();
            renderers.retain(|_, r| r.strong_count() > 0);
            renderers.insert(key, Rc::downgrade(&renderer));
        });
        renderer
    }

    /// The composition target the renderer draws the tree to.
    pub fn composition_target(&self) -> &Rc<CompositionTarget> {
        &self.composition_target
    }

    /// The compositor of the renderer.
    pub fn compositor_handle(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// Sets the visual whose composition visual is the root of the
    /// composition target; `None` detaches the tree.
    pub fn set_root(&self, root: Option<Ref<Visual>>) {
        let visual = root.and_then(|root| root.composition_visual()).map(|visual| (*visual).clone());
        self.composition_target.set_root(visual);
    }

    /// Sets the transparency level the composition target renders with.
    pub fn set_transparency_level(&self, level: CompositionTransparencyLevel) {
        self.composition_target.set_transparency_level(level);
    }

    /// Sets the platform-specific scene info passed to the render target.
    pub fn set_platform_specific_scene_info(&self, info: Option<std::sync::Arc<dyn Any + Send + Sync>>) {
        self.composition_target.set_platform_specific_scene_info(info);
    }

    /// The renderer as the hit tester of its scene.
    pub fn hit_tester(&self) -> Option<Rc<dyn IHitTester>> {
        self.this.upgrade().map(|this| this as Rc<dyn IHitTester>)
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }

    fn on_diagnostics_property_changed(&self, diagnostics: &RendererDiagnostics, property_name: &str) {
        if property_name == RendererDiagnostics::DEBUG_OVERLAYS_PROPERTY_NAME {
            self.composition_target.set_debug_overlays(diagnostics.debug_overlays());
        } else if property_name == RendererDiagnostics::LAST_LAYOUT_PASS_TIMING_PROPERTY_NAME {
            self.composition_target.set_last_layout_pass_timing(diagnostics.last_layout_pass_timing());
        }
    }

    fn queue_update(&self) {
        if self.queued_update.get() {
            return;
        }
        self.queued_update.set(true);
        let weak = self.this.clone();
        self.compositor.request_composition_update(move || {
            if let Some(renderer) = weak.upgrade() {
                renderer.update();
            }
        });
    }

    /// The filter of a hit test, for the composition visuals. A visual without a composition
    /// visual is shown by the composition visuals of the visuals under it, which are children
    /// of the composition visual of a visual further up: the filter is asked for it with each
    /// of them, so that what it rejects is rejected with everything under it, as it is when
    /// the visual has a composition visual (section 9.41). Nothing above `root` is asked.
    fn composition_filter<'a>(
        filter: Option<&'a dyn Fn(&Visual) -> bool>,
        root: &'a Visual,
    ) -> Option<impl Fn(&Rc<CompositionVisual>) -> bool + 'a> {
        filter.map(move |filter| {
            move |v: &Rc<CompositionVisual>| match CompositionDrawListVisual::from_visual(v) {
                Some(dlv) => dlv.visual().is_some_and(|visual| Self::passes_filter(filter, &visual, root)),
                None => true,
            }
        })
    }

    /// The filter of a hit test, for what the visuals without a composition visual drew
    /// (section 9.42): it is asked for the visual that drew, and for the visuals above it as
    /// [`composition_filter`](Self::composition_filter) asks.
    fn composition_item_filter<'a>(
        filter: Option<&'a dyn Fn(&Visual) -> bool>,
        root: &'a Visual,
    ) -> Option<impl Fn(&Rc<CompositionDrawItem>) -> bool + 'a> {
        filter.map(move |filter| {
            move |item: &Rc<CompositionDrawItem>| {
                item.element().is_some_and(|visual| Self::passes_filter(filter, &visual, root))
            }
        })
    }

    /// Whether `filter` accepts `visual` and the visuals above it that have no composition
    /// visual, up to `root`.
    fn passes_filter(filter: &dyn Fn(&Visual) -> bool, visual: &Ref<Visual>, root: &Visual) -> bool {
        if !filter(visual) {
            return false;
        }
        if std::ptr::eq::<Visual>(&**visual, root) {
            return true;
        }
        let mut parent = visual.visual_parent();
        while let Some(ancestor) = parent {
            if ancestor.has_composition_visual() {
                break;
            }
            if !filter(&ancestor) {
                return false;
            }
            if std::ptr::eq::<Visual>(&*ancestor, root) {
                break;
            }
            parent = ancestor.visual_parent();
        }
        true
    }

    /// What a hit test under a root without a composition visual tests (sections 9.41 and
    /// 9.42): what the root drew, and the composition visuals and the drawings nearest below
    /// it, in the order they are shown. All of them are of the composition visual of the
    /// nearest visual above the root that has one, children of it or items in it. The input is
    /// given in the coordinates that visual has in the last layout, and, when they are others,
    /// in those of the composition visual that showed its children in the last frame. `None`
    /// when the root is in no tree or the filter rejects it.
    #[allow(clippy::type_complexity)]
    fn hit_test_below<H: ICompositionHitTester>(
        input: &H::Input,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<(Vec<CompositionHitTarget>, H::Input, Option<H::Input>)> {
        if !root.is_attached_to_visual_tree()
            || root.composition_visual_departed()
            || filter.is_some_and(|filter| !filter(root))
        {
            return None;
        }
        let mut entries = Vec::new();
        root.collect_composition_entries(&mut entries);
        let mut targets = Vec::with_capacity(entries.len() + 1);
        if let Some(item) = root.composition_item().filter(|item| !item.is_departed()) {
            targets.push(CompositionHitTarget::Item(item));
        }
        for entry in entries {
            match entry {
                CompositionEntry::Visual(visual) => {
                    if let Some(visual) = visual.own_composition_visual() {
                        targets.push(CompositionHitTarget::Visual((*visual).clone()));
                    }
                }
                CompositionEntry::Item(_, item) => targets.push(CompositionHitTarget::Item(item)),
            }
        }
        let offset = root.composition_offset(false);
        let layout_input = H::transform(input, Matrix::create_translation(offset.x, offset.y));
        let parent_input = root
            .nearest_composition_ancestor()
            .and_then(|ancestor| ancestor.own_composition_visual())
            .and_then(|holder| CompositionTarget::last_frame_input::<H>(&holder, &layout_input));
        Some((targets, layout_input, parent_input))
    }

    fn hit_test_all<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<(Ref<Visual>, IntersectionResult)> {
        let f = Self::composition_filter(filter, root);
        let item_f = Self::composition_item_filter(filter, root);
        let filters = HitFilters {
            visual: f.as_ref().map(|f| f as &dyn Fn(&Rc<CompositionVisual>) -> bool),
            result: None,
            item: item_f.as_ref().map(|f| f as &dyn Fn(&Rc<CompositionDrawItem>) -> bool),
        };

        let res = match root.own_composition_visual() {
            Some(root_visual) => {
                let root_visual: Rc<CompositionVisual> = (*root_visual).clone();
                let Some(res) = self.composition_target.hit_test_targets::<H>(input, &root_visual, filters) else {
                    return Vec::new();
                };
                res
            }
            None => {
                let Some((targets, layout_input, parent_input)) = Self::hit_test_below::<H>(input, root, filter) else {
                    return Vec::new();
                };
                let parent_input = parent_input.as_ref().unwrap_or(&layout_input);
                self.composition_target.hit_test_siblings::<H>(&targets, parent_input, &layout_input, filters)
            }
        };

        let mut result = Vec::new();
        for (intersection, target) in res {
            if let Some(visual) = target.element() {
                if filter.is_none_or(|filter| filter(&visual)) {
                    result.push((visual, intersection));
                }
            }
        }
        result
    }

    fn hit_test_first_core<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<(Ref<Visual>, IntersectionResult)> {
        let f = Self::composition_filter(filter, root);
        let item_f = Self::composition_item_filter(filter, root);
        let result_filter = |v: &Rc<CompositionVisual>| CompositionDrawListVisual::from_visual(v).is_some();
        let filters = HitFilters {
            visual: f.as_ref().map(|f| f as &dyn Fn(&Rc<CompositionVisual>) -> bool),
            result: Some(&result_filter),
            item: item_f.as_ref().map(|f| f as &dyn Fn(&Rc<CompositionDrawItem>) -> bool),
        };

        let (hit, intersection) = match root.own_composition_visual() {
            Some(root_visual) => {
                let root_visual: Rc<CompositionVisual> = (*root_visual).clone();
                self.composition_target.hit_test_first_target::<H>(input, &root_visual, filters)
            }
            None => {
                let (targets, layout_input, parent_input) = Self::hit_test_below::<H>(input, root, filter)?;
                let parent_input = parent_input.as_ref().unwrap_or(&layout_input);
                self.composition_target.hit_test_first_sibling::<H>(&targets, parent_input, &layout_input, filters)
            }
        };
        let visual = hit?.element()?;
        Some((visual, intersection))
    }

    /// Releases the composition visuals kept for visuals that left the tree and are not back
    /// (section 9.32): what the visuals would have done when they left.
    fn release_departed(&self) {
        if self.departed.borrow().is_empty() {
            return;
        }
        // The list is taken first: a release changes composition visuals and nothing else, but
        // nothing here relies on that.
        let mut departed = std::mem::take(&mut *self.departed.borrow_mut());
        release_kept_composition_visuals(&mut departed);
        // The storage is kept for the next pass, unless something was queued meanwhile.
        let mut list = self.departed.borrow_mut();
        if list.is_empty() {
            *list = departed;
        }
    }

    /// Creates the composition visual of a visual of the tree in the update pass (section 9.41).
    fn create_in_pass(&self, visual: &Visual) -> CompositionDrawListVisual {
        visual.attach_to_compositor_core(&self.compositor, Some(self as &dyn IRenderer))
    }

    /// `visual` was given its composition visual in this pass: its child composition visuals
    /// are synchronized in it, after those of the nearest visual above it that has a
    /// composition visual, which shows the new one in place of what the new one shows now.
    fn created_in_pass(&self, visual: &Visual) {
        let ancestor = visual.nearest_composition_ancestor();
        let mut sets = self.sets.borrow_mut();
        if let Some(ancestor) = &ancestor {
            sets.add(VisualSet::ChildrenFirst, ancestor);
        }
        // The children of the visuals whose drawing changed are synchronized anyway.
        if !sets.contains(VisualSet::Dirty, visual) {
            sets.add(VisualSet::RecalculateChildren, visual);
        }
    }

    /// Gives `visual` its composition visual in the update pass, with its properties.
    fn materialize_in_pass(&self, visual: &Visual) -> CompositionDrawListVisual {
        let comp = self.create_in_pass(visual);
        visual.synchronize_composition_properties();
        visual.settle_composition_offsets();
        self.created_in_pass(visual);
        comp
    }

    /// What `visual`, which has no composition visual, drew in the update pass is an item of
    /// the composition visual of the nearest visual above it that has one (section 9.42):
    /// `item` is the one it has. Gives the drawing back when that composition visual shows no
    /// drawing but its own: the visual then gets a composition visual.
    fn record_item_in_pass(
        &self,
        visual: &Visual,
        item: Option<Rc<CompositionDrawItem>>,
        drawing: RecordedDrawing,
    ) -> Option<RecordedDrawing> {
        let Some(holder) = visual.nearest_composition_ancestor() else { return Some(drawing) };
        let Some(holder_visual) = holder.own_composition_visual().filter(|comp| comp.holds_drawing_items()) else {
            return Some(drawing);
        };
        let offset = visual.composition_offset(true);
        let offset = (offset.x, offset.y);
        let clip = visual.composition_item_clip();
        let item = match (drawing, item) {
            (RecordedDrawing::New(Some(data)), Some(item)) => {
                item.set_data(data);
                Some(item)
            }
            (RecordedDrawing::New(Some(data)), None) => {
                let item = CompositionDrawItem::new(visual, data, offset, clip);
                visual.set_composition_item(Some(item.clone()));
                Some(item)
            }
            (RecordedDrawing::Updated, Some(item)) => {
                crate::perf_count!(VisualsRecordedInPlace);
                item.mark_redrawn();
                Some(item)
            }
            (RecordedDrawing::Same, Some(item)) => {
                crate::perf_count!(VisualsRecordedUnchanged);
                item.mark_redrawn();
                Some(item)
            }
            // Nothing to show.
            (_, item) => {
                if item.is_some() {
                    visual.release_composition_item();
                    self.sets.borrow_mut().add(VisualSet::ChildrenFirst, &holder);
                }
                None
            }
        };
        if let Some(item) = item {
            crate::perf_count!(VisualsRecordedAsItems);
            item.set_offset(offset);
            item.set_clip(clip);
            // A new one, and one that another composition visual showed, is listed.
            if !item.is_held_by(&holder_visual) {
                self.sets.borrow_mut().add(VisualSet::ChildrenFirst, &holder);
            }
        }
        visual.mark_seen_without_composition_visual();
        // The composition visuals below follow it when it moved; when there are too many of
        // them it gets a composition visual, which moves them all.
        if !visual.move_composition_offsets_below() {
            self.materialize_in_pass(visual);
        }
        None
    }

    /// The composition visuals something asked for during the pass (a drawing, the
    /// synchronisation of another visual) get their properties and their place.
    fn place_late_in_pass(&self) {
        loop {
            let Some(visual) = self.late.borrow_mut().pop() else { break };
            if visual.own_composition_visual().is_some() {
                visual.synchronize_composition_properties();
                visual.settle_composition_offsets();
                self.created_in_pass(&visual);
            }
        }
    }

    /// Places a composition visual created outside an update pass (section 9.41). The next
    /// pass synchronizes the children around it, and the properties of one no pass has seen
    /// yet. A visual that a pass has already seen is synchronized here instead: what asked for
    /// the composition visual finds it as it would be had the visual had it all along, and
    /// what it then sets on it (an `Offset` an animation starts from) is not written over by
    /// the next pass, which has nothing of the visual to forward.
    fn place_materialized(&self, visual: &Visual) {
        let ancestor = visual.nearest_composition_ancestor();
        // Read before the offsets are settled, which forgets it.
        let seen = visual.was_seen_without_composition_visual();
        {
            let mut sets = self.sets.borrow_mut();
            if let Some(ancestor) = &ancestor {
                sets.add(VisualSet::ChildrenFirst, ancestor);
            }
            sets.add(VisualSet::RecalculateChildren, visual);
            if !seen {
                sets.add(VisualSet::PropertiesDirty, visual);
            }
        }
        if seen {
            visual.synchronize_composition_properties();
            visual.settle_composition_offsets();
            if let Some(ancestor) = &ancestor {
                ancestor.synchronize_composition_child_visuals();
            }
            visual.synchronize_composition_child_visuals();
        }
    }

    fn update_core(&self) {
        self.queued_update.set(false);
        // Before anything is synchronized: a visual that is still out of the tree lets go of
        // its composition visual here, and its former parent is synchronized below.
        self.release_departed();

        // The visuals whose drawing changed. A visual that has no composition visual gets one
        // here when it needs one; one that needs none stays without (section 9.41), and what
        // it draws is an item of the composition visual above (section 9.42). The child
        // composition visuals and the items are synchronized after every composition visual of
        // the pass exists.
        let dirty: Vec<Ref<Visual>> = self.sets.borrow_mut().snapshot(VisualSet::Dirty);
        for visual in &dirty {
            // It left the tree since.
            if !visual.is_attached_to_visual_tree() {
                continue;
            }
            let mut comp = visual.own_composition_visual();
            // What it drew as an item of the composition visual above (section 9.42).
            let mut item = if comp.is_none() { visual.composition_item() } else { None };
            if let Some(held) = &item {
                held.set_departed(false);
                if held.data().is_some_and(|data| !data.is_of(&self.compositor)) {
                    // Drawn for another compositor.
                    visual.release_composition_item();
                    item = None;
                }
            }
            if comp.is_none() && visual.needs_own_composition_visual() {
                // The drawing of the item is the one of the new composition visual.
                comp = Some(self.create_in_pass(visual));
                self.created_in_pass(visual);
                item = None;
            }
            if comp.is_some() {
                visual.synchronize_composition_properties();
                visual.settle_composition_offsets();
            }
            crate::perf_count!(VisualsRecorded);

            let previous = match &comp {
                Some(comp) => comp.draw_list(),
                None => item.as_ref().and_then(|item| item.data()),
            };
            let draw_list = {
                struct ResetRecorder<'a>(&'a RefCell<RenderDataDrawingContext>);
                impl Drop for ResetRecorder<'_> {
                    fn drop(&mut self) {
                        if let Ok(mut recorder) = self.0.try_borrow_mut() {
                            recorder.reset();
                        }
                    }
                }
                let _reset = ResetRecorder(&self.recorder);
                let mut recorder = self.recorder.borrow_mut();
                {
                    let mut context = DrawingContext::new(&mut *recorder);
                    visual.render(&mut context);
                }
                if comp.is_none() {
                    // Its drawing asked for a composition visual, which took the drawing of
                    // the item: it is placed with the late ones.
                    comp = visual.own_composition_visual();
                    if comp.is_some() {
                        item = None;
                    }
                }
                if comp.is_none() && recorder.recorded_nothing() {
                    None
                } else {
                    // A drawing equal to the one the visual has is not sent again (section
                    // 9.14), and a different one is given to the render data the visual has,
                    // unless something besides the visual and `previous` holds it (section
                    // 9.28).
                    let previous_is_shared =
                        previous.as_ref().is_some_and(|previous| Rc::strong_count(previous) > 2);
                    Some(recorder.get_render_results_for(previous.as_ref(), previous_is_shared))
                }
            };
            let Some(draw_list) = draw_list else {
                // It draws nothing and has no composition visual: what it drew before is gone.
                if item.is_some() {
                    visual.release_composition_item();
                    if let Some(holder) = visual.nearest_composition_ancestor() {
                        self.sets.borrow_mut().add(VisualSet::ChildrenFirst, &holder);
                    }
                }
                visual.mark_seen_without_composition_visual();
                // The composition visuals below follow it when it moved; when there are too
                // many of them it gets a composition visual, which moves them all.
                if !visual.move_composition_offsets_below() {
                    self.materialize_in_pass(visual);
                }
                continue;
            };
            let (comp, draw_list) = match comp {
                Some(comp) => (comp, draw_list),
                // It draws and needs no composition visual: its drawing is an item of the
                // composition visual above, unless that one shows no drawing but its own.
                None => match self.record_item_in_pass(visual, item, draw_list) {
                    None => continue,
                    Some(draw_list) => (self.materialize_in_pass(visual), draw_list),
                },
            };
            match draw_list {
                RecordedDrawing::New(draw_list) => comp.set_draw_list(draw_list),
                RecordedDrawing::Updated => {
                    crate::perf_count!(VisualsRecordedInPlace);
                    comp.invalidate_draw_list_content();
                }
                RecordedDrawing::Same => {
                    crate::perf_count!(VisualsRecordedUnchanged);
                    comp.invalidate_draw_list_content();
                }
            }
        }
        // The visuals whose properties alone changed: their drawing is kept. One without a
        // composition visual gets it when it needs one now.
        let properties_dirty: Vec<Ref<Visual>> = self.sets.borrow_mut().snapshot(VisualSet::PropertiesDirty);
        for visual in &properties_dirty {
            if !visual.is_attached_to_visual_tree() || self.sets.borrow().contains(VisualSet::Dirty, visual) {
                continue;
            }
            if visual.own_composition_visual().is_some() {
                visual.synchronize_composition_properties();
                visual.settle_composition_offsets();
            } else if visual.needs_own_composition_visual() {
                self.materialize_in_pass(visual);
            } else {
                // What it drew follows it (section 9.42), and so do the composition visuals
                // below.
                visual.synchronize_composition_item();
                if !visual.move_composition_offsets_below() {
                    self.materialize_in_pass(visual);
                }
            }
        }
        self.place_late_in_pass();
        // The visuals whose children changed. One without a composition visual has its children
        // shown by the nearest visual above it that has one, unless it needs one now (something
        // of the composition API was set on it without another notice).
        let recalculate: Vec<Ref<Visual>> = self.sets.borrow_mut().snapshot(VisualSet::RecalculateChildren);
        for visual in &recalculate {
            if !visual.is_attached_to_visual_tree() || visual.own_composition_visual().is_some() {
                continue;
            }
            if visual.needs_own_composition_visual() {
                self.materialize_in_pass(visual);
            } else if let Some(ancestor) = visual.nearest_composition_ancestor() {
                self.sets.borrow_mut().add(VisualSet::RecalculateChildren, &ancestor);
            }
        }
        self.place_late_in_pass();

        // The child composition visuals: first of the visuals above the new composition
        // visuals, then of the visuals whose drawing changed, then of the others.
        let children_first: Vec<Ref<Visual>> = self.sets.borrow_mut().snapshot(VisualSet::ChildrenFirst);
        for visual in &children_first {
            visual.synchronize_composition_child_visuals();
        }
        for visual in &dirty {
            if !self.sets.borrow().contains(VisualSet::ChildrenFirst, visual) {
                visual.synchronize_composition_child_visuals();
            }
        }
        let recalculate: Vec<Ref<Visual>> = self.sets.borrow_mut().snapshot(VisualSet::RecalculateChildren);
        for visual in &recalculate {
            let synchronized = {
                let sets = self.sets.borrow();
                sets.contains(VisualSet::Dirty, visual) || sets.contains(VisualSet::ChildrenFirst, visual)
            };
            if !synchronized {
                visual.synchronize_composition_child_visuals();
            }
        }
        // A composition visual asked for while the children were synchronized (the shadow of a
        // child is one) is placed at once: the lists above are done.
        loop {
            let Some(visual) = self.late.borrow_mut().pop() else { break };
            if visual.own_composition_visual().is_none() {
                continue;
            }
            visual.synchronize_composition_properties();
            visual.settle_composition_offsets();
            if let Some(ancestor) = visual.nearest_composition_ancestor() {
                ancestor.synchronize_composition_child_visuals();
            }
            visual.synchronize_composition_child_visuals();
        }
        // Nothing else is added during the pass (`updating`), so the sets are emptied together.
        self.sets.borrow_mut().clear();
        if let Some(root) = self.root.upgrade() {
            self.composition_target.set_size(root.client_size());
            self.composition_target.set_scaling(root.render_scaling());
        }

        let commit = self.compositor.request_composition_batch_commit_async();
        if !self.queued_scene_invalidation.get() {
            self.queued_scene_invalidation.set(true);
            // Updated hit-test information is available after full render
            let key = self.key;
            let dispatcher = self.compositor.dispatcher().clone();
            commit.rendered().on_completed(move || {
                dispatcher.post(
                    move || {
                        let renderer =
                            RENDERERS.with(|renderers| renderers.borrow().get(&key).and_then(Weak::upgrade));
                        if let Some(renderer) = renderer {
                            renderer.queued_scene_invalidation.set(false);
                            let size = renderer.root.upgrade().map_or(Size::default(), |root| root.client_size());
                            renderer.raise_scene_invalidated(&SceneInvalidatedEventArgs::new(Rect::from_size(size)));
                        }
                    },
                    DispatcherPriority::INPUT,
                );
            });
        }
    }

    fn raise_scene_invalidated(&self, e: &SceneInvalidatedEventArgs) {
        if self.scene_invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.scene_invalidated.snapshot().iter() {
            handler(e);
        }
    }

    pub fn trigger_scene_invalidated_for_unit_tests(&self, rect: Rect) {
        self.raise_scene_invalidated(&SceneInvalidatedEventArgs::new(rect));
    }

    /// Whether the renderer holds `visual` for its next update pass. For
    /// tests only.
    pub fn has_pending_update_for_unit_tests(&self, visual: &Visual) -> bool {
        let sets = self.sets.borrow();
        sets.contains(VisualSet::Dirty, visual) || sets.contains(VisualSet::RecalculateChildren, visual)
    }

    fn update(&self) {
        if self.updating.get() {
            return;
        }
        if !self.composition_target.is_enabled() {
            self.queued_update.set(false);
            return;
        }

        struct ResetUpdating<'a>(&'a Cell<bool>);
        impl Drop for ResetUpdating<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        self.updating.set(true);
        let _reset = ResetUpdating(&self.updating);
        self.update_core();
    }

    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }
}

impl IRenderer for CompositingRenderer {
    fn diagnostics(&self) -> Rc<RendererDiagnostics> {
        self.diagnostics.clone()
    }

    fn scene_invalidated(&self, handler: Rc<dyn Fn(&SceneInvalidatedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.scene_invalidated.add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.scene_invalidated.remove(token);
            }
        })
    }

    fn add_dirty(&self, visual: &Visual) {
        if self.is_disposed.get() {
            return;
        }
        if self.updating.get() {
            panic!("Visual was invalidated during the render pass");
        }

        // A visual says so when it leaves the tree. One that is in the tree without a
        // composition visual is queued like any other: the pass finds out whether it needs one
        // (section 9.41).
        if !visual.is_attached_to_visual_tree() {
            // Queuing a detached visual would keep it alive for as long as rendering is stopped.
            self.sets.borrow_mut().remove_all(visual);
        } else {
            self.sets.borrow_mut().add(VisualSet::Dirty, visual);
        }

        self.queue_update();
    }

    fn add_dirty_properties(&self, visual: &Visual) {
        if self.is_disposed.get() {
            return;
        }
        if self.updating.get() {
            panic!("Visual was invalidated during the render pass");
        }
        if !visual.is_attached_to_visual_tree() {
            self.sets.borrow_mut().remove(VisualSet::PropertiesDirty, visual);
            return;
        }
        self.sets.borrow_mut().add(VisualSet::PropertiesDirty, visual);
        self.queue_update();
    }

    fn recalculate_children(&self, visual: &Visual) {
        if self.is_disposed.get() {
            return;
        }
        if self.updating.get() {
            panic!("Visual was invalidated during the render pass");
        }
        self.sets.borrow_mut().add(VisualSet::RecalculateChildren, visual);
        self.queue_update();
    }

    fn resized(&self, _size: Size) {}

    fn paint(&self, _rect: Rect) {
        if self.is_disposed.get() {
            return;
        }

        self.queue_update();
        self.composition_target.request_redraw();
        MediaContext::instance().immediate_render_requested(&self.compositor);
    }

    fn start(&self) {
        if self.is_disposed.get() {
            return;
        }

        self.composition_target.set_is_enabled(true);
        let pending = {
            let sets = self.sets.borrow();
            !sets.is_empty(VisualSet::Dirty) || !sets.is_empty(VisualSet::RecalculateChildren)
        };
        if pending {
            self.queue_update();
        }
    }

    fn stop(&self) {
        self.composition_target.set_is_enabled(false);
        // No update pass runs while the renderer is stopped: nothing is kept for one.
        self.release_departed();
    }

    fn try_get_render_interface_feature(
        &self,
        feature_type: TypeId,
    ) -> super::LocalFuture<Option<super::RenderInterfaceFeature>> {
        self.compositor.try_get_render_interface_feature(feature_type)
    }

    fn keep_departed_composition_visual(&self, visual: &Visual, composition_visual: &CompositionDrawListVisual) -> bool {
        // Kept only when an update pass will run to release it.
        if self.is_disposed.get() || self.updating.get() || !self.composition_target.is_enabled() {
            return false;
        }
        let kept = (visual.to_ref().downgrade(), KeptComposition::Visual(composition_visual.clone()));
        // Inside `keep_composition_visuals_departing_in` it is kept for the caller of that.
        if let Some(keeping) = self.keeping.borrow_mut().as_mut() {
            keeping.push(kept);
            return true;
        }
        self.departed.borrow_mut().push(kept);
        self.queue_update();
        true
    }

    fn keep_departed_composition_item(&self, visual: &Visual) -> bool {
        // Kept only when an update pass will run to release it.
        if self.is_disposed.get() || self.updating.get() || !self.composition_target.is_enabled() {
            return false;
        }
        let Some(item) = visual.composition_item() else { return false };
        let kept = (visual.to_ref().downgrade(), KeptComposition::Item(item));
        if let Some(keeping) = self.keeping.borrow_mut().as_mut() {
            keeping.push(kept);
            return true;
        }
        self.departed.borrow_mut().push(kept);
        self.queue_update();
        true
    }

    fn keep_composition_visuals_departing_in(&self, departure: &mut dyn FnMut()) -> Option<KeptCompositionVisuals> {
        /// Ends the scope when `departure` unwinds: what was collected is released in the next
        /// update pass, as it is for a visual that leaves outside of a scope.
        struct Scope<'a> {
            renderer: &'a CompositingRenderer,
            outer: Option<Vec<KeptCompositionVisual>>,
            ended: bool,
        }
        impl Drop for Scope<'_> {
            fn drop(&mut self) {
                if self.ended {
                    return;
                }
                let collected = self.renderer.keeping.replace(self.outer.take());
                if let Some(mut collected) = collected.filter(|collected| !collected.is_empty()) {
                    self.renderer.departed.borrow_mut().append(&mut collected);
                    self.renderer.queue_update();
                }
            }
        }

        // A scope inside another collects for itself, and the outer one goes on after it.
        let outer = self.keeping.replace(Some(Vec::new()));
        let mut scope = Scope { renderer: self, outer, ended: false };
        departure();
        let collected = self.keeping.replace(scope.outer.take());
        scope.ended = true;
        collected.filter(|collected| !collected.is_empty()).map(KeptCompositionVisuals::new)
    }

    fn rejoins_departed_composition_visual(&self, composition_visual: &CompositionDrawListVisual) -> bool {
        // One that is in the tree of another target of the compositor is not moved to this one.
        !self.is_disposed.get()
            && composition_visual.root().is_none_or(|root| Rc::ptr_eq(&root, &self.composition_target))
    }

    fn composition_visual_materialized(&self, visual: &Visual) {
        if self.is_disposed.get() {
            return;
        }
        // During a pass, and while another one is placed (placing one can ask for more: that
        // of the visual parent, those of the shadows), it waits its turn.
        if self.updating.get() || self.placing.get() {
            self.late.borrow_mut().push(visual.to_ref());
            return;
        }
        struct ResetPlacing<'a>(&'a Cell<bool>);
        impl Drop for ResetPlacing<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        self.placing.set(true);
        let _reset = ResetPlacing(&self.placing);
        self.place_materialized(visual);
        loop {
            let Some(late) = self.late.borrow_mut().pop() else { break };
            if late.own_composition_visual().is_some() {
                self.place_materialized(&late);
            }
        }
        self.queue_update();
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.compositor.clone())
    }

    fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }
        self.sets.borrow_mut().clear();
        self.late.borrow_mut().clear();
        self.release_departed();
        for (token, _) in self.scene_invalidated.snapshot().iter() {
            self.scene_invalidated.remove(*token);
        }
        self.stop();
        // The tree is released with the target: the root visual and the
        // target refer to each other.
        self.composition_target.set_root(None);
        self.composition_target.mark_disposed_out_of_band();
        MediaContext::instance().sync_dispose_composition_target(&self.compositor, self.composition_target.server());
        let key = self.key;
        let _ = RENDERERS.try_with(|renderers| {
            if let Ok(mut renderers) = renderers.try_borrow_mut() {
                renderers.remove(&key);
            }
        });
    }
}

impl IHitTester for CompositingRenderer {
    fn hit_test(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        self.hit_test_all::<PointCompositionHitTester>(&p, root, filter).into_iter().map(|(visual, _)| visual).collect()
    }

    fn hit_test_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        self.hit_test_all::<GeometryCompositionHitTester>(&geometry.to_ref(), root, filter)
            .into_iter()
            .map(|(visual, intersection)| GeometryHitTestResult::new(visual, intersection))
            .collect()
    }

    fn hit_test_first(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Option<Ref<Visual>> {
        self.hit_test_first_core::<PointCompositionHitTester>(&p, root, filter).map(|(visual, _)| visual)
    }

    fn hit_test_first_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        self.hit_test_first_core::<GeometryCompositionHitTester>(&geometry.to_ref(), root, filter)
            .map(|(visual, intersection)| GeometryHitTestResult::new(visual, intersection))
    }
}
