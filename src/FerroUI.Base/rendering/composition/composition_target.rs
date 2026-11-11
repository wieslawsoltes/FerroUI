use super::animations::{ICompositionAnimation, ICompositionAnimationBase};
use super::container_visual::HIT_TEST_AABB_TREE_THRESHOLD;
use super::expressions::ExpressionVariant;
use super::generated::{CompositionTargetHooks, CompositionTargetProps, ServerCompositionTargetProps};
use super::hit_testing::{is_hit, ICompositionHitTester};
use super::server::{CompositionProperty, RenderSurfaces, ServerCompositionTarget, ServerObjectId};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::{
    AsCompositionObject, CompositionDrawItem, CompositionDrawListVisual, CompositionObject, CompositionTransparencyLevel,
    CompositionVisual, Compositor, DrawItemList, ICompositionObject, ICompositionObjectAnimations, ICompositionObjectHost,
    ICompositionTargetDebugEvents, ICompositorSerializable, PendingAnimations,
};
use crate::media::IntersectionResult;
use crate::platform::LtrbRect;
use crate::rendering::{LayoutPassTiming, RendererDebugOverlays};
use crate::{Matrix, Ref, Size, Visual};
use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicI64, Ordering};

static NEXT_ID: AtomicI64 = AtomicI64::new(1);

type VisualFilter<'a> = Option<&'a dyn Fn(&Rc<CompositionVisual>) -> bool>;

/// What a hit test of the composition tree finds: a composition visual, or what a visual
/// without one drew, which the composition visual of a visual above it shows
/// (docs/porting/desktop-performance.md, section 9.42).
pub(crate) enum CompositionHitTarget {
    Visual(Rc<CompositionVisual>),
    Item(Rc<CompositionDrawItem>),
}

impl CompositionHitTarget {
    /// The visual of the visual tree that is hit.
    pub(crate) fn element(&self) -> Option<Ref<Visual>> {
        match self {
            CompositionHitTarget::Visual(visual) => {
                CompositionDrawListVisual::from_visual(visual).and_then(|visual| visual.visual())
            }
            CompositionHitTarget::Item(item) => item.element(),
        }
    }

    /// The composition visual that shows what is hit.
    pub(crate) fn into_holder(self) -> Option<Rc<CompositionVisual>> {
        match self {
            CompositionHitTarget::Visual(visual) => Some(visual),
            CompositionHitTarget::Item(item) => item.holder(),
        }
    }
}

/// What a hit test asks before it tests something.
#[derive(Clone, Copy)]
pub(crate) struct HitFilters<'a> {
    /// A composition visual it rejects is skipped with everything under it.
    pub(crate) visual: VisualFilter<'a>,
    /// A composition visual it rejects is not the first hit; what is under it is still tested.
    pub(crate) result: VisualFilter<'a>,
    /// A drawing item it rejects is not hit. `None` accepts every item.
    pub(crate) item: Option<&'a dyn Fn(&Rc<CompositionDrawItem>) -> bool>,
}

/// Whether the composition visual that shows `item` passes `result_filter`: how the hit
/// tests that name composition visuals only treat an item.
pub(crate) fn holder_passes(item: &Rc<CompositionDrawItem>, result_filter: VisualFilter<'_>) -> bool {
    match result_filter {
        Some(result_filter) => item.holder().is_some_and(|holder| result_filter(&holder)),
        None => true,
    }
}

/// The root of a composition visual tree: what a renderer draws to a
/// render target.
pub struct CompositionTarget {
    this: Weak<CompositionTarget>,
    object: CompositionObject,
    props: CompositionTargetProps,
    id: i64,
    hit_test_child_candidates: RefCell<Vec<Rc<CompositionVisual>>>,
}

/// How a hit test enters a composition visual.
enum Entered<Input> {
    /// With the input in the coordinates of the visual.
    Local(Input),
    /// Through it to its children, with the input of its parent: the visual was created for
    /// a visual that was shown without a composition visual and has no readback yet
    /// (docs/porting/desktop-performance.md, section 9.41).
    Through,
}

impl CompositionTarget {
    pub(crate) fn new(compositor: &Rc<Compositor>, surfaces: RenderSurfaces) -> Rc<CompositionTarget> {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst) + 1;
        // The render interface may have been registered after the compositor
        // was created. It is looked up here, on the thread that has the
        // services, before a frame of the render thread needs it.
        compositor.with_server(|server| {
            server.render_interface().capture_platform_render_interface();
        });
        let server =
            compositor.create_server_object(move |compositor, _| ServerCompositionTarget::new(compositor, surfaces, id));
        let target = Rc::new_cyclic(|this: &Weak<CompositionTarget>| CompositionTarget {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: CompositionTargetProps::new(),
            id,
            hit_test_child_candidates: RefCell::new(Vec::new()),
        });
        target.props.initialize_defaults(&*target);
        target
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The id of the server-side target.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    /// The identity of the target, shared with its server side.
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }

    /// Marks the target disposed after its server side has been disposed
    /// out of band.
    pub(crate) fn mark_disposed_out_of_band(&self) {
        self.object.mark_disposed();
    }

    // --- generated properties -----------------------------------------------------

    pub fn root(&self) -> Option<Rc<CompositionVisual>> {
        self.props.root().and_then(|o| o.into_any_rc().downcast::<CompositionVisual>().ok())
    }

    pub fn set_root(&self, value: Option<Rc<CompositionVisual>>) {
        self.props.set_root(self, value.map(|v| v as Rc<dyn ICompositionObject>));
    }

    pub fn is_enabled(&self) -> bool {
        self.props.is_enabled()
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.props.set_is_enabled(self, value);
    }

    pub fn debug_overlays(&self) -> RendererDebugOverlays {
        self.props.debug_overlays()
    }

    pub fn set_debug_overlays(&self, value: RendererDebugOverlays) {
        if !value.is_empty() {
            // The text of the overlays is made from the default typeface,
            // on this thread; the frames that draw it may run on another.
            self.compositor().with_server(|server| {
                server.diagnostic_text_renderer();
            });
        }
        self.props.set_debug_overlays(self, value);
    }

    pub fn last_layout_pass_timing(&self) -> LayoutPassTiming {
        self.props.last_layout_pass_timing()
    }

    pub fn set_last_layout_pass_timing(&self, value: LayoutPassTiming) {
        self.props.set_last_layout_pass_timing(self, value);
    }

    pub fn scaling(&self) -> f64 {
        self.props.scaling()
    }

    pub fn set_scaling(&self, value: f64) {
        self.props.set_scaling(self, value);
    }

    pub fn size(&self) -> Size {
        self.props.size()
    }

    pub fn set_size(&self, value: Size) {
        self.props.set_size(self, value);
    }

    pub fn transparency_level(&self) -> CompositionTransparencyLevel {
        self.props.transparency_level()
    }

    pub fn set_transparency_level(&self, value: CompositionTransparencyLevel) {
        self.props.set_transparency_level(self, value);
    }

    pub fn platform_specific_scene_info(&self) -> Option<std::sync::Arc<dyn Any + Send + Sync>> {
        self.props.platform_specific_scene_info()
    }

    pub fn set_platform_specific_scene_info(&self, value: Option<std::sync::Arc<dyn Any + Send + Sync>>) {
        self.props.set_platform_specific_scene_info(self, value);
    }

    /// Sets the receiver of the debug events of the server-side target.
    /// It is handed over with the next batch.
    pub fn set_debug_events(&self, events: Option<Arc<dyn ICompositionTargetDebugEvents>>) {
        let server = self.server();
        self.compositor().post_server_job(
            move |compositor| {
                if let Some(target) = compositor.get::<ServerCompositionTarget>(server) {
                    target.set_debug_events(events);
                }
            },
            false,
        );
    }

    // --- hit testing --------------------------------------------------------------

    /// Collects the visuals hit by `input` (in the coordinates of `root`,
    /// or of the root of the target), topmost first, with how they are
    /// hit. `None` when the tree has not been rendered yet.
    pub fn try_hit_test<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: Option<&Rc<CompositionVisual>>,
        filter: VisualFilter<'_>,
    ) -> Option<Vec<(IntersectionResult, Rc<CompositionVisual>)>> {
        let root = match root {
            Some(root) => root.clone(),
            None => self.root()?,
        };
        let filters = HitFilters { visual: filter, result: None, item: None };
        let targets = self.hit_test_targets::<H>(input, &root, filters)?;

        // What a visual without a composition visual drew is hit as the composition visual
        // that shows it, which is in the result once (section 9.42).
        let mut res: Vec<(IntersectionResult, Rc<CompositionVisual>)> = Vec::with_capacity(targets.len());
        let mut items = false;
        for (intersection, target) in targets {
            items |= matches!(target, CompositionHitTarget::Item(_));
            let Some(visual) = target.into_holder() else { continue };
            if items && res.iter().any(|(_, known)| Rc::ptr_eq(known, &visual)) {
                continue;
            }
            res.push((intersection, visual));
        }
        Some(res)
    }

    /// [`try_hit_test`](Self::try_hit_test) that says what is hit of a composition visual: the
    /// visual, or what a visual without a composition visual drew in it (section 9.42).
    pub(crate) fn hit_test_targets<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: &Rc<CompositionVisual>,
        filters: HitFilters<'_>,
    ) -> Option<Vec<(IntersectionResult, CompositionHitTarget)>> {
        self.compositor().readback().next_read();

        // Need to convert transform the point using visual's readback since HitTestCore will use its inverse matrix
        // NOTE: it can technically break hit-testing of the root visual itself if it has a non-identity transform,
        // need to investigate that possibility later. We might want a separate mode for root hit-testing.
        let parent_input = Self::root_parent_input::<H>(root, input)?;
        let layout_input = Self::root_layout_input::<H>(root, input);
        let layout_input = layout_input.as_ref().unwrap_or(&parent_input);

        let mut res = Vec::new();
        self.hit_test_core::<H>(root, &parent_input, layout_input, &mut res, filters);
        Some(res)
    }

    /// [`hit_test_targets`](Self::hit_test_targets) for a root that has no composition visual
    /// (docs/porting/desktop-performance.md, sections 9.41 and 9.42): `targets` are what shows
    /// the root and what is under it, in the order it is shown, all of one composition visual
    /// (children of it and items in it). `layout_input` is the input in the coordinates that
    /// one has in the last layout, and `parent_input` in those of the composition visual that
    /// showed the children in the last frame ([`last_frame_input`](Self::last_frame_input)).
    pub(crate) fn hit_test_siblings<H: ICompositionHitTester>(
        &self,
        targets: &[CompositionHitTarget],
        parent_input: &H::Input,
        layout_input: &H::Input,
        filters: HitFilters<'_>,
    ) -> Vec<(IntersectionResult, CompositionHitTarget)> {
        self.compositor().readback().next_read();
        let mut res = Vec::new();
        for target in targets.iter().rev() {
            match target {
                CompositionHitTarget::Visual(visual) => {
                    self.hit_test_core::<H>(visual, parent_input, layout_input, &mut res, filters)
                }
                CompositionHitTarget::Item(item) => {
                    if let Some(intersection) = Self::hit_test_item::<H>(item, layout_input, filters) {
                        res.push((intersection, CompositionHitTarget::Item(item.clone())));
                    }
                }
            }
        }
        res
    }

    /// How `input`, in the coordinates of the composition visual that shows `item`, hits it;
    /// `None` when it does not.
    ///
    /// Deviation (DEVIATIONS.md, Composition visuals): the item is where the last layout put
    /// it, while a composition visual is tested where the last frame showed it.
    fn hit_test_item<H: ICompositionHitTester>(
        item: &Rc<CompositionDrawItem>,
        input: &H::Input,
        filters: HitFilters<'_>,
    ) -> Option<IntersectionResult> {
        if item.is_departed() {
            return None;
        }
        let data = item.data()?;
        let (x, y) = item.offset();
        if let Some((width, height)) = item.clip() {
            if !H::transformed_sub_tree_bounds_match(LtrbRect::new(x, y, x + width, y + height), input) {
                return None;
            }
        }
        let bounds = || {
            let bounds = data.bounds()?;
            Some(LtrbRect::new(bounds.left + x, bounds.top + y, bounds.right + x, bounds.bottom + y))
        };
        if !H::drawing_bounds_match(&bounds, input) {
            return None;
        }
        let local = H::transform(input, Matrix::create_translation(-x, -y));
        let intersection = H::hit_test_drawing(&data, &local);
        // The filter last: it is asked for the items that are hit only.
        if !is_hit(intersection) || filters.item.is_some_and(|filter| !filter(item)) {
            return None;
        }
        Some(intersection)
    }

    /// The items of `visual`, when it shows any.
    fn items_of(visual: &CompositionVisual) -> Option<&DrawItemList> {
        visual.draw_items().filter(|items| items.count() != 0)
    }

    /// The input in the coordinates of a visual that is entered through
    /// ([`Entered::Through`]), from the one in the coordinates its parent has in the last
    /// layout: the visual is where its offset says in its parent. What it draws, and what the
    /// visuals without a composition visual drew in it, are tested with it (section 9.42).
    fn through_layout_input<H: ICompositionHitTester>(visual: &CompositionVisual, layout_input: &H::Input) -> H::Input {
        let offset = visual.offset();
        H::transform(layout_input, Matrix::create_translation(-offset.x, -offset.y))
    }

    /// The input of a hit test under `root`, given in its coordinates, in those its parent has
    /// in the last layout, when they are not those of the last frame: the root is entered
    /// through.
    fn root_layout_input<H: ICompositionHitTester>(root: &Rc<CompositionVisual>, input: &H::Input) -> Option<H::Input> {
        if root.try_get_valid_readback().is_some() || !root.hit_tests_through() {
            return None;
        }
        let offset = root.offset();
        Some(H::transform(input, Matrix::create_translation(offset.x, offset.y)))
    }

    /// `input`, given in the coordinates `visual` has in the last layout, in those of the
    /// composition visual that showed what is under `visual` in the last frame, when that is
    /// another one: no frame was rendered with `visual`, nor with the ones above it that were
    /// created with it (section 9.41).
    pub(crate) fn last_frame_input<H: ICompositionHitTester>(
        visual: &Rc<CompositionVisual>,
        input: &H::Input,
    ) -> Option<H::Input> {
        if !visual.hit_tests_through() {
            return None;
        }
        let (mut x, mut y) = (0.0, 0.0);
        let mut above = Some(visual.clone());
        while let Some(visual) = above.filter(|visual| visual.hit_tests_through()) {
            let offset = visual.offset();
            x += offset.x;
            y += offset.y;
            above = visual.parent();
        }
        Some(H::transform(input, Matrix::create_translation(x, y)))
    }

    /// The input of a hit test under `root`, given in its coordinates, in those of its parent.
    fn root_parent_input<H: ICompositionHitTester>(root: &Rc<CompositionVisual>, input: &H::Input) -> Option<H::Input> {
        match root.try_get_valid_readback() {
            Some(readback) => Some(H::transform(input, readback.matrix)),
            // In the last frame it was where its offset says, in the composition visual that
            // showed what is under it (section 9.41).
            None if root.hit_tests_through() => {
                // And so were the ones above it that were created with it, up to the one that
                // showed them all.
                let offset = root.offset();
                let (mut x, mut y) = (offset.x, offset.y);
                let mut above = root.parent();
                while let Some(visual) = above.filter(|visual| visual.hit_tests_through()) {
                    let offset = visual.offset();
                    x += offset.x;
                    y += offset.y;
                    above = visual.parent();
                }
                Some(H::transform(input, crate::Matrix::create_translation(x, y)))
            }
            None => None,
        }
    }

    /// Tests `visual` and what is under it. `parent_input` is the input in the coordinates of
    /// the composition visual that showed `visual` in the last frame, and `layout_input` in
    /// those its parent has in the last layout: the same, unless a visual above it is entered
    /// through.
    fn hit_test_core<H: ICompositionHitTester>(
        &self,
        visual: &Rc<CompositionVisual>,
        parent_input: &H::Input,
        layout_input: &H::Input,
        result: &mut Vec<(IntersectionResult, CompositionHitTarget)>,
        filters: HitFilters<'_>,
    ) {
        let input = match Self::hit_test_visual::<H>(visual, parent_input, filters.visual) {
            Some(Entered::Local(input)) => input,
            Some(Entered::Through) => {
                // The child visuals are where they were in the last frame, in the coordinates
                // of the parent (section 9.41); what the visual shows itself was an item of
                // the composition visual above, where the last layout put it (section 9.42).
                let local = Self::through_layout_input::<H>(visual, layout_input);
                self.hit_test_children::<H>(visual, parent_input, &local, result, filters);
                if let Some(intersection) = Self::hit_test_through::<H>(visual, &local) {
                    result.push((intersection, CompositionHitTarget::Visual(visual.clone())));
                }
                return;
            }
            None => return,
        };

        // Inspect children
        self.hit_test_children::<H>(visual, &input, &input, result, filters);

        // Hit-test the current node
        let intersection_result = H::hit_test(visual, &input);
        if is_hit(intersection_result) {
            result.push((intersection_result, CompositionHitTarget::Visual(visual.clone())));
        }
    }

    /// How `input`, in the coordinates of a visual that is entered through, hits what the
    /// visual shows itself; `None` when it does not.
    fn hit_test_through<H: ICompositionHitTester>(visual: &Rc<CompositionVisual>, input: &H::Input) -> Option<IntersectionResult> {
        if visual.is_departed() || (visual.clip_to_bounds() && !H::clipped_bounds_match(visual, input)) {
            return None;
        }
        let intersection = H::hit_test(visual, input);
        is_hit(intersection).then_some(intersection)
    }

    /// Tests what is under `visual`: its child visuals with `input`, and what the visuals
    /// without a composition visual drew in it with `item_input`, the input in the coordinates
    /// `visual` has in the last layout.
    fn hit_test_children<H: ICompositionHitTester>(
        &self,
        visual: &Rc<CompositionVisual>,
        input: &H::Input,
        item_input: &H::Input,
        result: &mut Vec<(IntersectionResult, CompositionHitTarget)>,
        filters: HitFilters<'_>,
    ) {
        // The children of a visual kept for a visual that left its tree are those of a released
        // one: none (section 9.32).
        if visual.is_departed() {
            return;
        }
        if let Some(items) = Self::items_of(visual) {
            // The items and the child visuals are shown in one order: from the last, the
            // items after a child, then the child (section 9.42). The children are not
            // looked up in their index: the items between them are tested one by one anyway.
            let children = visual.children();
            let mut item_end = items.count();
            let mut index = children.count();
            while index > 0 {
                index -= 1;
                let position = items.position(index).min(item_end);
                Self::hit_test_items::<H>(items, position, item_end, item_input, result, filters);
                item_end = position;
                self.hit_test_core::<H>(&children.get(index), input, item_input, result, filters);
            }
            Self::hit_test_items::<H>(items, 0, item_end, item_input, result, filters);
            return;
        }
        if visual.children().count() >= HIT_TEST_AABB_TREE_THRESHOLD {
            // The candidate list of the target is reused unless a query is
            // already running (a nested container).
            let mut candidates = match self.hit_test_child_candidates.try_borrow_mut() {
                Ok(mut pooled) => std::mem::take(&mut *pooled),
                Err(_) => Vec::new(),
            };
            candidates.clear();
            let queried = visual.try_query_hit_test_children::<H>(input, &mut candidates);
            if queried {
                for child in &candidates {
                    self.hit_test_core::<H>(child, input, item_input, result, filters);
                }
            }
            candidates.clear();
            if let Ok(mut pooled) = self.hit_test_child_candidates.try_borrow_mut() {
                if pooled.capacity() < candidates.capacity() {
                    *pooled = candidates;
                }
            }
            if queried {
                return;
            }
        }

        // By index, from the last child, as the original: no copy of the
        // children of every visual the hit test enters.
        let children = visual.children();
        let mut index = children.count();
        while index > 0 {
            index -= 1;
            self.hit_test_core::<H>(&children.get(index), input, item_input, result, filters);
        }
    }

    /// Tests the items `start..end` of a holder, from the last.
    fn hit_test_items<H: ICompositionHitTester>(
        items: &DrawItemList,
        start: usize,
        end: usize,
        input: &H::Input,
        result: &mut Vec<(IntersectionResult, CompositionHitTarget)>,
        filters: HitFilters<'_>,
    ) {
        for index in (start..end).rev() {
            let Some(item) = items.item_at(index) else { continue };
            if let Some(intersection) = Self::hit_test_item::<H>(&item, input, filters) {
                result.push((intersection, CompositionHitTarget::Item(item)));
            }
        }
    }

    /// The last of the items `start..end` of a holder that is hit.
    fn hit_test_first_item<H: ICompositionHitTester>(
        items: &DrawItemList,
        start: usize,
        end: usize,
        input: &H::Input,
        filters: HitFilters<'_>,
    ) -> Option<(CompositionHitTarget, IntersectionResult)> {
        for index in (start..end).rev() {
            let Some(item) = items.item_at(index) else { continue };
            if let Some(intersection) = Self::hit_test_item::<H>(&item, input, filters) {
                return Some((CompositionHitTarget::Item(item), intersection));
            }
        }
        None
    }

    fn hit_test_visual<H: ICompositionHitTester>(
        visual: &Rc<CompositionVisual>,
        parent_input: &H::Input,
        filter: VisualFilter<'_>,
    ) -> Option<Entered<H::Input>> {
        // A visual that is not hit test visible (WinUI `IsHitTestVisible`)
        // is skipped with its subtree.
        if !visual.visible() || !visual.is_hit_test_visible() {
            return None;
        }

        if let Some(filter) = filter {
            if !filter(visual) {
                return None;
            }
        }

        let Some(readback) = visual.try_get_valid_readback() else {
            return visual.hit_tests_through().then_some(Entered::Through);
        };

        if !visual.disable_sub_tree_bounds_hit_test_optimization() {
            let transformed_subtree_bounds = readback.transformed_subtree_bounds?;
            if !H::transformed_sub_tree_bounds_match(transformed_subtree_bounds, parent_input) {
                return None;
            }
        }

        let inv_matrix = readback.matrix.try_invert()?;

        let input = H::transform(parent_input, inv_matrix);

        if visual.clip_to_bounds() && !H::clipped_bounds_match(visual, &input) {
            return None;
        }

        if let Some(clip) = visual.clip() {
            if !H::clip_matches(&*clip, &input) {
                return None;
            }
        }

        Some(Entered::Local(input))
    }

    /// Finds the topmost visual hit by `input` that passes `result_filter`
    /// and returns it with how it is hit.
    pub fn try_hit_test_first<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: Option<&Rc<CompositionVisual>>,
        filter: VisualFilter<'_>,
        result_filter: VisualFilter<'_>,
    ) -> (Option<Rc<CompositionVisual>>, IntersectionResult) {
        let none = (None, IntersectionResult::NotCalculated);
        let root = match root {
            Some(root) => root.clone(),
            None => match self.root() {
                Some(root) => root,
                None => return none,
            },
        };
        // What a visual without a composition visual drew is hit as the composition visual
        // that shows it (section 9.42).
        let item_filter = |item: &Rc<CompositionDrawItem>| holder_passes(item, result_filter);
        let filters = HitFilters { visual: filter, result: result_filter, item: Some(&item_filter) };
        let (hit, intersection) = self.hit_test_first_target::<H>(input, &root, filters);
        (hit.and_then(CompositionHitTarget::into_holder), intersection)
    }

    /// [`try_hit_test_first`](Self::try_hit_test_first) that says what is hit of a composition
    /// visual: the visual, or what a visual without a composition visual drew in it
    /// (section 9.42).
    pub(crate) fn hit_test_first_target<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: &Rc<CompositionVisual>,
        filters: HitFilters<'_>,
    ) -> (Option<CompositionHitTarget>, IntersectionResult) {
        self.compositor().readback().next_read();

        // Need to convert transform the point using visual's readback since HitTestCore will use its inverse matrix
        // NOTE: it can technically break hit-testing of the root visual itself if it has a non-identity transform,
        // need to investigate that possibility later. We might want a separate mode for root hit-testing.
        let Some(parent_input) = Self::root_parent_input::<H>(root, input) else {
            return (None, IntersectionResult::NotCalculated);
        };
        let layout_input = Self::root_layout_input::<H>(root, input);
        let layout_input = layout_input.as_ref().unwrap_or(&parent_input);

        self.hit_test_first_core::<H>(root, &parent_input, layout_input, filters)
    }

    /// [`hit_test_first_target`](Self::hit_test_first_target) for a root that has no
    /// composition visual: see [`hit_test_siblings`](Self::hit_test_siblings).
    pub(crate) fn hit_test_first_sibling<H: ICompositionHitTester>(
        &self,
        targets: &[CompositionHitTarget],
        parent_input: &H::Input,
        layout_input: &H::Input,
        filters: HitFilters<'_>,
    ) -> (Option<CompositionHitTarget>, IntersectionResult) {
        self.compositor().readback().next_read();
        for target in targets.iter().rev() {
            match target {
                CompositionHitTarget::Visual(visual) => {
                    let (hit, intersection) =
                        self.hit_test_first_core::<H>(visual, parent_input, layout_input, filters);
                    if hit.is_some() {
                        return (hit, intersection);
                    }
                }
                CompositionHitTarget::Item(item) => {
                    if let Some(intersection) = Self::hit_test_item::<H>(item, layout_input, filters) {
                        return (Some(CompositionHitTarget::Item(item.clone())), intersection);
                    }
                }
            }
        }
        (None, IntersectionResult::NotCalculated)
    }

    /// The first hit of `visual` and what is under it: the inputs are those of
    /// [`hit_test_core`](Self::hit_test_core).
    pub(crate) fn hit_test_first_core<H: ICompositionHitTester>(
        &self,
        visual: &Rc<CompositionVisual>,
        parent_input: &H::Input,
        layout_input: &H::Input,
        filters: HitFilters<'_>,
    ) -> (Option<CompositionHitTarget>, IntersectionResult) {
        let (input, through) = match Self::hit_test_visual::<H>(visual, parent_input, filters.visual) {
            Some(Entered::Local(input)) => (input, None),
            // The child visuals are where they were in the last frame, in the coordinates of
            // the parent (section 9.41); what the visual shows itself was an item of the
            // composition visual above, where the last layout put it (section 9.42).
            Some(Entered::Through) => {
                (parent_input.clone(), Some(Self::through_layout_input::<H>(visual, layout_input)))
            }
            None => return (None, IntersectionResult::NotCalculated),
        };
        // The input in the coordinates the visual has in the last layout.
        let layout_input = through.as_ref().unwrap_or(&input);

        // A visual kept for a visual that left its tree has no children here (section 9.32).
        let mut queried_indexed_children = visual.is_departed();
        if !queried_indexed_children {
            if let Some(items) = Self::items_of(visual) {
                // The items and the child visuals are shown in one order: from the last, the
                // items after a child, then the child (section 9.42).
                queried_indexed_children = true;
                let item_input = layout_input;
                let children = visual.children();
                let mut item_end = items.count();
                let mut index = children.count();
                while index > 0 {
                    index -= 1;
                    let position = items.position(index).min(item_end);
                    if let Some((hit, intersection_result)) =
                        Self::hit_test_first_item::<H>(items, position, item_end, item_input, filters)
                    {
                        return (Some(hit), intersection_result);
                    }
                    item_end = position;
                    let child = children.get(index);
                    let (hit, intersection_result) =
                        self.hit_test_first_core::<H>(&child, &input, layout_input, filters);
                    if hit.is_some() {
                        return (hit, intersection_result);
                    }
                }
                if let Some((hit, intersection_result)) =
                    Self::hit_test_first_item::<H>(items, 0, item_end, item_input, filters)
                {
                    return (Some(hit), intersection_result);
                }
            }
        }
        if !queried_indexed_children && visual.children().count() >= HIT_TEST_AABB_TREE_THRESHOLD {
            if let Some((hit, intersection_result)) =
                visual.try_query_first_hit_test_child::<H>(self, &input, layout_input, filters)
            {
                queried_indexed_children = true;
                if hit.is_some() {
                    return (hit, intersection_result);
                }
            }
        }

        if !queried_indexed_children {
            // By index, from the last child, as the original: no copy of the
            // children of every visual the hit test enters.
            let children = visual.children();
            let mut index = children.count();
            while index > 0 {
                index -= 1;
                let child = children.get(index);
                let (hit, intersection_result) = self.hit_test_first_core::<H>(&child, &input, layout_input, filters);
                if hit.is_some() {
                    return (hit, intersection_result);
                }
            }
        }

        if through.is_some() {
            return match Self::hit_test_through::<H>(visual, layout_input) {
                Some(intersection) if filters.result.is_none_or(|result_filter| result_filter(visual)) => {
                    (Some(CompositionHitTarget::Visual(visual.clone())), intersection)
                }
                _ => (None, IntersectionResult::NotCalculated),
            };
        }
        let intersection_result = H::hit_test(visual, &input);
        let hit = is_hit(intersection_result) && filters.result.is_none_or(|result_filter| result_filter(visual));
        (hit.then(|| CompositionHitTarget::Visual(visual.clone())), intersection_result)
    }

    pub fn request_redraw(&self) {
        self.register_for_serialization();
    }
}

impl CompositionTargetHooks for CompositionTarget {
    fn on_root_changed(&self) {
        if let Some(root) = self.root() {
            root.set_root(self.this.upgrade());
        }
    }

    fn on_root_changing(&self) {
        if let Some(root) = self.root() {
            root.set_root(None);
        }
    }
}

impl IRegisterForSerialization for CompositionTarget {
    fn register_for_serialization(&self) {
        self.object
            .register_for_serialization(|| self.this.upgrade().map(|this| this as Rc<dyn ICompositorSerializable>));
    }
}

impl ICompositionObjectHost for CompositionTarget {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn pending_animations(&self) -> &PendingAnimations {
        self.object.pending_animations()
    }

    fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.object.implicit_animation(property_name)
    }

    fn start_animation_group(
        &self,
        grp: &Rc<dyn ICompositionAnimationBase>,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        self.start_animation_group_for(&**grp, target, final_value)
    }
}

/// The class has no animated property: upstream does not override
/// `StartAnimation`, whose base implementation rejects every property.
impl ICompositionObjectAnimations for CompositionTarget {
    fn try_start_animation(
        &self,
        _property_name: &str,
        _animation: &dyn ICompositionAnimation,
        _final_value: Option<ExpressionVariant>,
    ) -> bool {
        false
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        ServerCompositionTargetProps::get_composition_property(property_name)
    }

    fn composition_object(&self) -> &CompositionObject {
        &self.object
    }
}

impl AsCompositionObject for CompositionTarget {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionTarget"
    }
}

impl ICompositionObject for CompositionTarget {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl ICompositorSerializable for CompositionTarget {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.object.try_get_server(c)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.object.begin_serialize_changes(c);
        self.props.serialize_changes_core(self, writer);
    }
}
