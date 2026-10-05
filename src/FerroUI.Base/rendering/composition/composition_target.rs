use super::animations::ICompositionAnimationBase;
use super::container_visual::HIT_TEST_AABB_TREE_THRESHOLD;
use super::expressions::ExpressionVariant;
use super::generated::{CompositionTargetHooks, CompositionTargetProps};
use super::hit_testing::{is_hit, ICompositionHitTester};
use super::server::{RenderSurfaces, ServerCompositionTarget, ServerObjectId};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::{
    CompositionObject, CompositionTransparencyLevel, CompositionVisual, Compositor, ICompositionObject,
    ICompositionObjectHost, ICompositionTargetDebugEvents, ICompositorSerializable, PendingAnimations,
};
use crate::media::IntersectionResult;
use crate::rendering::{LayoutPassTiming, RendererDebugOverlays};
use crate::Size;
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicI64, Ordering};

static NEXT_ID: AtomicI64 = AtomicI64::new(1);

type VisualFilter<'a> = Option<&'a dyn Fn(&Rc<CompositionVisual>) -> bool>;

/// The root of a composition visual tree: what a renderer draws to a
/// render target.
pub struct CompositionTarget {
    this: Weak<CompositionTarget>,
    object: CompositionObject,
    props: CompositionTargetProps,
    id: i64,
    hit_test_child_candidates: RefCell<Vec<Rc<CompositionVisual>>>,
}

impl CompositionTarget {
    pub(crate) fn new(compositor: &Rc<Compositor>, surfaces: RenderSurfaces) -> Rc<CompositionTarget> {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst) + 1;
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

    pub fn platform_specific_scene_info(&self) -> Option<Rc<dyn Any>> {
        self.props.platform_specific_scene_info()
    }

    pub fn set_platform_specific_scene_info(&self, value: Option<Rc<dyn Any>>) {
        self.props.set_platform_specific_scene_info(self, value);
    }

    /// Sets the receiver of the debug events of the server-side target.
    /// It is handed over with the next batch.
    pub fn set_debug_events(&self, events: Option<Rc<dyn ICompositionTargetDebugEvents>>) {
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
        self.compositor().server().readback().next_read();
        let root = match root {
            Some(root) => root.clone(),
            None => self.root()?,
        };

        // Need to convert transform the point using visual's readback since HitTestCore will use its inverse matrix
        // NOTE: it can technically break hit-testing of the root visual itself if it has a non-identity transform,
        // need to investigate that possibility later. We might want a separate mode for root hit-testing.
        let readback = root.try_get_valid_readback()?;
        let parent_input = H::transform(input, readback.matrix);

        let mut res = Vec::new();
        self.hit_test_core::<H>(&root, &parent_input, &mut res, filter);
        Some(res)
    }

    fn hit_test_core<H: ICompositionHitTester>(
        &self,
        visual: &Rc<CompositionVisual>,
        parent_input: &H::Input,
        result: &mut Vec<(IntersectionResult, Rc<CompositionVisual>)>,
        filter: VisualFilter<'_>,
    ) {
        let Some(input) = Self::hit_test_visual::<H>(visual, parent_input, filter) else { return };

        // Inspect children
        self.hit_test_children::<H>(visual, &input, result, filter);

        // Hit-test the current node
        let intersection_result = H::hit_test(visual, &input);
        if is_hit(intersection_result) {
            result.push((intersection_result, visual.clone()));
        }
    }

    fn hit_test_children<H: ICompositionHitTester>(
        &self,
        visual: &Rc<CompositionVisual>,
        input: &H::Input,
        result: &mut Vec<(IntersectionResult, Rc<CompositionVisual>)>,
        filter: VisualFilter<'_>,
    ) {
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
                    self.hit_test_core::<H>(child, input, result, filter);
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

        for child in visual.children().items().iter().rev() {
            self.hit_test_core::<H>(child, input, result, filter);
        }
    }

    fn hit_test_visual<H: ICompositionHitTester>(
        visual: &Rc<CompositionVisual>,
        parent_input: &H::Input,
        filter: VisualFilter<'_>,
    ) -> Option<H::Input> {
        if !visual.visible() {
            return None;
        }

        if let Some(filter) = filter {
            if !filter(visual) {
                return None;
            }
        }

        let readback = visual.try_get_valid_readback()?;

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

        Some(input)
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
        self.compositor().server().readback().next_read();
        let none = (None, IntersectionResult::NotCalculated);
        let root = match root {
            Some(root) => root.clone(),
            None => match self.root() {
                Some(root) => root,
                None => return none,
            },
        };

        // Need to convert transform the point using visual's readback since HitTestCore will use its inverse matrix
        // NOTE: it can technically break hit-testing of the root visual itself if it has a non-identity transform,
        // need to investigate that possibility later. We might want a separate mode for root hit-testing.
        let Some(readback) = root.try_get_valid_readback() else { return none };
        let parent_input = H::transform(input, readback.matrix);

        self.hit_test_first_core::<H>(&root, &parent_input, filter, result_filter)
    }

    pub(crate) fn hit_test_first_core<H: ICompositionHitTester>(
        &self,
        visual: &Rc<CompositionVisual>,
        parent_input: &H::Input,
        filter: VisualFilter<'_>,
        result_filter: VisualFilter<'_>,
    ) -> (Option<Rc<CompositionVisual>>, IntersectionResult) {
        let Some(input) = Self::hit_test_visual::<H>(visual, parent_input, filter) else {
            return (None, IntersectionResult::NotCalculated);
        };

        let mut queried_indexed_children = false;
        if visual.children().count() >= HIT_TEST_AABB_TREE_THRESHOLD {
            if let Some((hit, intersection_result)) =
                visual.try_query_first_hit_test_child::<H>(self, &input, filter, result_filter)
            {
                queried_indexed_children = true;
                if hit.is_some() {
                    return (hit, intersection_result);
                }
            }
        }

        if !queried_indexed_children {
            for child in visual.children().items().iter().rev() {
                let (hit, intersection_result) = self.hit_test_first_core::<H>(child, &input, filter, result_filter);
                if hit.is_some() {
                    return (hit, intersection_result);
                }
            }
        }

        let intersection_result = H::hit_test(visual, &input);
        let hit = is_hit(intersection_result) && result_filter.is_none_or(|result_filter| result_filter(visual));
        (hit.then(|| visual.clone()), intersection_result)
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

    fn implicit_animation(&self, _property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        None
    }

    fn start_animation_group(
        &self,
        _grp: &Rc<dyn ICompositionAnimationBase>,
        _target: &str,
        _final_value: ExpressionVariant,
    ) -> bool {
        false
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
