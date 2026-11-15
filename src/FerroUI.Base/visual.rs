use crate::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, ResetBehavior};
use crate::media::{
    affects_render_source, can_value_affect_render, CacheMode, DrawingContext, FlowDirection, Geometry, IBrush, IEffect,
    ITransform, RenderOptions, TextOptions, Transform,
};
use crate::rendering::composition::{
    CompositionDrawItem, CompositionDrawListVisual, CompositionEntry, CompositionVisual, Compositor,
    KeptCompositionVisuals,
};
use crate::reactive::{Disposable, IDisposable, Subscription};
use crate::rendering::{IPresentationSource, IRenderer};
use crate::utilities::{HandlerList, OptionCell};
use crate::{
    ferro_class, ferro_property, instantiate, AttachedProperty, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, ObjectType, PropertyChangeHandler,
    Rect, Ref, RelativePoint, StyledElement,
    StyledElementImpl, StyledElementImplExt, StyledProperty, StyledPropertyOptions, Upcast,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use rustc_hash::FxHashMap;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether a visual gets a composition visual only once it shows something or something asks
/// for it (docs/porting/desktop-performance.md, section 9.41).
static COMPOSITION_VISUALS_ON_DEMAND: AtomicBool = AtomicBool::new(true);

/// A visual without a composition visual: the offset of a composition visual below it was
/// computed through its position.
const ELIDED_OFFSET_THROUGH: u8 = 1;
/// A visual without a composition visual: its position changed since then.
const ELIDED_OFFSET_MOVED: u8 = 2;
/// A visual without a composition visual: an update pass of its renderer has seen it, so its
/// bounds are those of a layout pass.
const ELIDED_SEEN: u8 = 4;
/// A visual with a composition visual: the child composition visuals and the items of that one
/// are those of the visual tree below the visual, and nothing that decides them changed since
/// they were listed (section 9.42).
const CHILDREN_LISTED: u8 = 8;
/// How many composition visuals, and how many visuals, the move of a visual without a
/// composition visual re-positions below it before the visual gets a composition visual of
/// its own instead.
const ELIDED_MOVE_VISUALS: u32 = 8;
const ELIDED_MOVE_NODES: u32 = 32;

/// Base class for controls that provides rendering and related visual
/// properties.
///
/// The visual class represents elements that have a visual on-screen
/// representation and stores all the information needed for a renderer to
/// render the control. To traverse the visual tree, use the methods in the
/// `visual_tree` module.
#[repr(C)]
pub struct Visual {
    base: StyledElement,
    bounds: Cell<Rect>,
    presentation_source: RefCell<Option<Rc<dyn IPresentationSource>>>,
    visual_parent: RefCell<Option<WeakRef<Visual>>>,
    has_mirror_transform: Cell<bool>,
    // Deviation (DEVIATIONS.md, Visual tree): whether `RenderTransform` has a value, kept by its
    // change handler, so that the transforms of the visual tree skip the property read.
    has_render_transform: Cell<bool>,
    /// The kept value of `IsVisible`, read many times a layout pass (desktop-performance.md, 9.9).
    hot_is_visible: crate::property_store::HotValue<bool>,
    // Not upstream: what the synchronisation of the composition visual reads from the value store
    // (docs/porting/desktop-performance.md, section 9.27).
    hot_composition_inputs: crate::property_store::HotValue<CompositionStyledInputs>,
    // Not upstream: whether the visual has a manipulation transform (the zoom a scroll presenter
    // applies to its content), kept so that the transforms of the visual tree skip the lookup.
    has_manipulation_transform: Cell<bool>,
    is_effectively_visible: Cell<bool>,
    has_non_uniform_z_index_children: Cell<bool>,
    visual_children: OnceCell<FerroList<Ref<Visual>>>,
    attached_to_visual_tree: HandlerList<dyn Fn(&VisualTreeAttachmentEventArgs)>,
    detached_from_visual_tree: HandlerList<dyn Fn(&VisualTreeAttachmentEventArgs)>,
    is_effectively_visible_changed: HandlerList<dyn Fn()>,
    render_options: Cell<RenderOptions>,
    text_options: Cell<TextOptions>,
    composition_visual: OptionCell<CompositionDrawListVisual>,
    // Not upstream (docs/porting/desktop-performance.md, section 9.42): what the visual drew,
    // as an item of the composition visual of the nearest visual above it that has one, while
    // it has no composition visual of its own. The two slots are as wide as the `RefCell` the
    // first one was.
    composition_item: OptionCell<Rc<CompositionDrawItem>>,
    // Not upstream (docs/porting/desktop-performance.md, section 9.32): the visual left its
    // visual tree and its composition visual is kept, until the next update pass of the renderer
    // or for as long as what took the visual out of the tree keeps it, in case the visual comes
    // back. While it is set the visual answers that it has no
    // composition visual, as upstream does for a visual that is not attached.
    composition_visual_departed: Cell<bool>,
    // Not upstream (section 9.32): the composition visual was given to something other than the
    // synchronisation of the visual tree, which may have changed it or may hold it. Such a
    // composition visual is released when the visual leaves its tree, as upstream.
    composition_visual_handed_out: Cell<bool>,
    // Not upstream (docs/porting/desktop-performance.md, section 9.41): what a visual without a
    // composition visual of its own keeps for the composition visuals below it (`ELIDED_*`),
    // and whether one with a composition visual listed what is below it (`CHILDREN_LISTED`).
    elided_state: Cell<u8>,
    child_composition_visual: RefCell<Option<Rc<CompositionVisual>>>,
    /// What the composition API of WinUI adds to the visual (the `Translation` facade, the
    /// requests to hide the composition visual, the layer composition visual): boxed and
    /// created on first use, so that a visual that uses none of it pays one pointer.
    composition_extras: RefCell<Option<Box<CompositionExtras>>>,
    /// The subscription to the `changed` notification of the render
    /// transform, held while the visual is attached to a visual tree.
    render_transform_subscription: RefCell<Option<Subscription>>,
    affects_render_subscriptions: RefCell<Vec<(&'static FerroProperty, Rc<dyn IDisposable>)>>,
    // Not upstream (docs/porting/desktop-performance.md, section 9.40): the sets of its
    // renderer the visual is in, for the next update pass: the epoch of the sets and a bit for
    // each (`rendering::composition::compositing_renderer::VisualSets`).
    pub(crate) renderer_sets: Cell<u64>,
}

/// The state of the WinUI composition API on a visual (see
/// `docs/porting/winui-composition.md`).
/// What the synchronisation of a composition visual reads from the value store of its visual:
/// the values that are plain data, and whether each of the objects is set (an object is read
/// again at every synchronisation, since it can change without the store changing).
#[derive(Clone, Copy, Default)]
struct CompositionStyledInputs {
    opacity: f32,
    clip_to_bounds: bool,
    has_clip: bool,
    has_opacity_mask: bool,
    has_cache_mode: bool,
    has_effect: bool,
    has_render_transform: bool,
}

#[derive(Default)]
struct CompositionExtras {
    /// `UIElement.Translation`: `None` while the facade was never set, so that the translation
    /// of the composition visual is left to the application.
    translation: Option<crate::Vector3D>,
    /// The number of outstanding requests to hide the composition visual (a connected animation
    /// shows the element through a visual surface while it flies).
    hide_requests: u32,
    /// Source-free presentation suppression honored by both the compositor and immediate
    /// rendering. Unlike composition hiding, bitmap/visual-brush captures also omit it.
    render_suppression_requests: u32,
    /// The number of outstanding requests to keep the composition visual visible although the
    /// visual is not (an implicit hide animation runs).
    show_requests: u32,
    /// A composition visual shown above the children and the child composition visual: the
    /// layer of the connected animations on a top level.
    layer_visual: Option<Rc<CompositionVisual>>,
    /// Composition visuals shown after the children: the images of removed children that play
    /// their implicit hide animations (WinUI's unloading storage).
    unloading_visuals: Vec<Rc<CompositionVisual>>,
    /// The other transform facades of `UIElement` (`Rotation`, `RotationAxis`, `Scale`,
    /// `CenterPoint`, `TransformMatrix`) and the transitions (`OpacityTransition`,
    /// `RotationTransition`, `ScaleTransition`, `TranslationTransition`).
    facades: composition_facades::CompositionFacades,
    /// `UIElement.Lights`: the XAML lights the element holds (winui-materials.md), and the
    /// subscriptions that connect them while the element is in a visual tree.
    lights: Option<FerroList<Ref<crate::media::XamlLight>>>,
    lights_subscriptions: Vec<Rc<dyn IDisposable>>,
    /// The ids of the XAML lights that target the element (`XamlLight.AddTargetElement`), and
    /// the subscription that offers the element to those lights when it enters a tree.
    light_target_ids: Vec<String>,
    light_target_subscription: Option<Subscription>,
    /// The manipulation transform of the visual (see [`Visual::manipulation_transform`]).
    manipulation_transform: Option<crate::Matrix>,
    /// The theme shadow of the visual (`UIElement.Shadow`): `None` while the visual never had a
    /// `Shadow` and is not the child of a popup with one.
    theme_shadow: Option<ThemeShadowState>,
}

/// What a visual knows of the theme shadow it casts (see `visual/theme_shadow.rs`).
#[derive(Default)]
struct ThemeShadowState {
    /// The popup whose `Shadow` the visual casts as its child.
    popup: Option<WeakRef<Visual>>,
    /// Whether the visual is a popup, whose `Shadow` its child casts.
    delegates_to_child: bool,
    /// The visual that draws the shadow, shown by the parent before the composition visual of
    /// this visual.
    visual: Option<Rc<crate::rendering::composition::ThemeShadowVisual>>,
}

mod composition_facades;
mod theme_shadow;
#[cfg(test)]
mod composition_facades_tests;

pub use composition_facades::{ScalarTransition, Vector3Transition, Vector3TransitionComponents};

ferro_class! {
    Visual: StyledElement, virtuals VisualImpl: StyledElementImpl {
        /// Renders the visual to a drawing context.
        fn render(this, context: &mut DrawingContext);
        /// Calls the `on_attached_to_visual_tree` method for this control and
        /// all of its visual descendants.
        fn on_attached_to_visual_tree_core(this, e: &VisualTreeAttachmentEventArgs);
        /// Calls the `on_detached_from_visual_tree` method for this control
        /// and all of its visual descendants.
        fn on_detached_from_visual_tree_core(this, e: &VisualTreeAttachmentEventArgs);
        /// Called when the control is added to a rooted visual tree.
        fn on_attached_to_visual_tree(this, e: &VisualTreeAttachmentEventArgs);
        /// Called when the control is removed from a rooted visual tree.
        fn on_detached_from_visual_tree(this, e: &VisualTreeAttachmentEventArgs);
        /// Called when the control's visual parent changes.
        fn on_visual_parent_changed(this, old_parent: Option<&Ref<Visual>>, new_parent: Option<&Ref<Visual>>);
        /// Whether the control ignores the flow direction policies, i.e. is
        /// never mirrored.
        fn bypass_flow_direction_policies(this) -> bool;
        /// Computes the `has_mirror_transform` value according to the flow
        /// direction of this visual and its parent.
        fn invalidate_mirror_transform(this);
        /// The corner radius the visual clips its content with when
        /// `clip_to_bounds` is set; `None` for a plain rectangular clip.
        /// (Upstream: the `IVisualWithRoundRectClip` interface.)
        fn clip_to_bounds_radius(this) -> Option<crate::CornerRadius>;
        /// The custom hit test of the visual at a point in its own
        /// coordinates; `None` when the visual has none and is hit by what
        /// it occupies or draws. (Upstream: the `ICustomHitTest` interface;
        /// a class implementing it overrides this and the geometry form.)
        fn custom_hit_test(this, point: crate::Point) -> Option<bool>;
        /// The custom hit test of the visual against a geometry in its own
        /// coordinates; `None` when the visual has none.
        fn custom_hit_test_geometry(this, geometry: &Ref<crate::media::Geometry>) -> Option<crate::media::IntersectionResult>;
        /// Ensures that the visual is ready to use as the visual in a
        /// visual brush. (Upstream: the `IVisualBrushInitialize` interface;
        /// a class implementing it overrides this.)
        fn ensure_initialized_for_visual_brush(this);
        /// Creates the composition visual of the visual on `compositor`.
        fn create_composition_visual(this, compositor: &Rc<Compositor>) -> CompositionDrawListVisual;
        /// Releases the composition visual of the visual.
        fn detach_from_compositor(this);
        /// Makes the children of the composition visual match the visual
        /// children (in Z order) and the child composition visual.
        fn synchronize_composition_child_visuals(this);
        /// Copies the properties of the visual to its composition visual.
        fn synchronize_composition_properties(this);
        /// Whether the visual has a composition visual of its own whatever it draws: a class
        /// whose [`create_composition_visual`](Self::create_composition_visual) makes a
        /// composition visual that shows something itself says so here. Any other visual gets
        /// its composition visual once it sets something the compositor applies, or is asked
        /// for it (docs/porting/desktop-performance.md, section 9.41); until then what it draws
        /// is an item of the composition visual of the nearest visual above it that has one
        /// (section 9.42).
        fn requires_composition_visual(this) -> bool;
    }
}

crate::ferro_thread_local! {
    static ROOTED_VISUAL_CHILDREN_COUNT: Cell<i32> = const { Cell::new(0) };
}
crate::ferro_class_info!(Visual { new: Visual::new });

crate::ferro_thread_local! {
    // Not upstream: a version of what the effective viewports of the visuals are computed from (the
    // bounds, the visual parents, `ClipToBounds`, `RenderTransform` and `RenderTransformOrigin` of
    // the visuals of the thread), which the layout manager reads to reuse the viewports of shared
    // ancestors within one pass (DEVIATIONS.md, Layout).
    static GEOMETRY_VERSION: Cell<u64> = const { Cell::new(0) };
    // Not upstream: a version of which visuals of the thread have a composition visual and which
    // are in a tree. With the version of the geometry it says whether where the visuals without a
    // composition visual are shown changed (`CompositionItemPlacements`).
    static COMPOSITION_HOLDER_VERSION: Cell<u64> = const { Cell::new(0) };
}

/// The version of the geometry of the visuals of the thread (see `GEOMETRY_VERSION`).
#[inline]
pub(crate) fn geometry_version() -> u64 {
    GEOMETRY_VERSION.with(Cell::get)
}

#[inline]
fn bump_geometry_version() {
    GEOMETRY_VERSION.with(|version| version.set(version.get().wrapping_add(1)));
}

#[inline]
fn bump_composition_holder_version() {
    COMPOSITION_HOLDER_VERSION.with(|version| version.set(version.get().wrapping_add(1)));
}

/// Where the visuals without a composition visual are shown, for the visuals an update pass of a
/// renderer records one after another (docs/porting/desktop-performance.md, section 9.47): the
/// visual that shows what is below a visual without a composition visual, and where that visual
/// is in it, by the address of the visual. The visuals of a subtree share the visuals above them,
/// so the walk up to the composition visual is made once for them. The entries are valid while
/// the geometry of the visuals, their composition visuals and their trees do not change, which
/// the two versions say; the renderer empties the map at the end of its pass.
#[derive(Default)]
pub(crate) struct CompositionItemPlacements {
    versions: (u64, u64),
    below: FxHashMap<usize, (Ref<Visual>, f64, f64)>,
}

impl CompositionItemPlacements {
    /// Drops the entries the changes since they were computed made stale.
    fn validate(&mut self) {
        let versions = (geometry_version(), COMPOSITION_HOLDER_VERSION.with(Cell::get));
        if self.versions != versions {
            self.below.clear();
            self.versions = versions;
        }
    }

    /// Releases the visuals the entries hold.
    pub(crate) fn clear(&mut self) {
        self.below.clear();
    }

    /// The number of visuals a placement is kept for.
    #[cfg(test)]
    pub(crate) fn count(&self) -> usize {
        self.below.len()
    }
}

crate::ferro_overrides! { impl FerroObjectImpl for Visual {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // Disable transitions until we're added to the visual tree.
        this.disable_transitions();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        const HANDLER: PropertyChangeHandler = PropertyChangeHandler::Visual;
        if !change.reaches(HANDLER) {
            return;
        }

        if change.is_for(HANDLER, Self::is_visible_property().as_property()) {
            let parent_state = this.visual_parent().is_none_or(|p| p.is_effectively_visible());
            this.update_is_effectively_visible(parent_state);
        } else if change.is_for(HANDLER, Self::shadow_property().as_property()) {
            let old = change.get_old_value::<Option<Ref<crate::media::Shadow>>>().flatten();
            let new = change.get_new_value::<Option<Ref<crate::media::Shadow>>>();
            this.on_shadow_changed(old.as_ref(), new.as_ref());
        } else if change.is_for(HANDLER, crate::styling::ThemeVariant::actual_theme_variant_property().as_property()) {
            // The shadow of the theme is drawn in the theme of the element.
            if this.may_cast_theme_shadow() {
                this.invalidate_visual();
            }
        } else if change.is_for(HANDLER, Self::flow_direction_property().as_property()) {
            this.invalidate_mirror_transform();
            if let Some(children) = this.visual_children.get() {
                for child in children.snapshot().iter() {
                    child.invalidate_mirror_transform();
                }
            }
        }
    }
} }

impl StyledElementImpl for Visual {
    fn logical_children_collection_changed(this: &Self, e: &NotifyCollectionChangedEventArgs<'_, Ref<StyledElement>>) {
        Self::parent_logical_children_collection_changed(this, e);
        if let Some(source) = this.presentation_source() {
            source.renderer().recalculate_children(this);
        }
    }

    fn on_templated_parent_control_theme_changed(this: &Self) {
        Self::parent_on_templated_parent_control_theme_changed(this);
        let templated_parent = this.templated_parent();
        if let Some(children) = this.visual_children.get() {
            for child in children.snapshot().iter() {
                if child.templated_parent() == templated_parent {
                    child.on_templated_parent_control_theme_changed();
                }
            }
        }
    }
}

impl VisualImpl for Visual {
    fn render(_this: &Self, _context: &mut DrawingContext) {}

    fn on_attached_to_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        *this.presentation_source.borrow_mut() = Some(e.presentation_source().clone());
        bump_composition_holder_version();
        ROOTED_VISUAL_CHILDREN_COUNT.with(|count| count.set(count.get() + 1));

        if let Some(render_transform) = this.render_transform() {
            if let Some(mutable_transform) = render_transform.as_mutable_transform() {
                let weak = this.to_ref().downgrade();
                let subscription = mutable_transform.changed(Rc::new(move || {
                    if let Some(visual) = weak.upgrade() {
                        visual.render_transform_value_changed();
                    }
                }));
                this.set_render_transform_subscription(Some(Subscription::new(subscription)));
            }
        }

        this.enable_transitions();
        let renderer = e.presentation_source().renderer();
        if let Some(compositor) = renderer.compositor() {
            // Not upstream (section 9.41): the root, a visual that returns with the composition
            // visual it had and one that holds a composition visual are attached here. Any other
            // visual gets its composition visual from the update pass of the renderer, once it
            // is known to show something, or when something asks for it.
            if !Self::composition_visuals_on_demand()
                || this.visual_parent.borrow().is_none()
                || this.composition_visual_departed.get()
                || this.composition_visual.is_some()
            {
                this.attach_to_compositor_core(&compositor, Some(&*renderer));
            } else if let Some(item) = this.composition_item.get() {
                // It is back with what it drew (section 9.42).
                if item.is_departed() {
                    crate::perf_count!(DrawingItemsKept);
                    item.set_departed(false);
                }
            }
        }

        this.invalidate_mirror_transform();
        let parent = this.visual_parent();
        this.update_is_effectively_visible(parent.as_ref().is_none_or(|p| p.is_effectively_visible()));
        this.on_attached_to_visual_tree(e);
        if !this.attached_to_visual_tree.is_empty() {
            for (_, handler) in this.attached_to_visual_tree.snapshot().iter() {
                handler(e);
            }
        }
        this.invalidate_visual();

        if let Some(parent) = &parent {
            renderer.recalculate_children(parent);
            if this.z_index() != 0 {
                parent.has_non_uniform_z_index_children.set(true);
            }
        }

        if let Some(children) = this.visual_children.get() {
            let this_ref = this.to_ref();
            for child in children.snapshot().iter() {
                // An event handler may have modified the children: skip a
                // child which has already been attached, or which has been
                // removed from this visual (this is a snapshot).
                let attached_here = child
                    .presentation_source()
                    .is_some_and(|s| Rc::ptr_eq(&s, e.presentation_source()));
                if !attached_here && child.visual_parent().is_some_and(|p| p == this_ref) {
                    child.on_attached_to_visual_tree_core(e);
                }
            }
        }
    }

    fn on_detached_from_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        ROOTED_VISUAL_CHILDREN_COUNT.with(|count| count.set(count.get() - 1));

        this.set_render_transform_subscription(None);

        this.disable_transitions();
        this.update_is_effectively_visible(true);
        this.on_detached_from_visual_tree(e);
        this.detach_from_compositor();
        if !this.detached_from_visual_tree.is_empty() {
            for (_, handler) in this.detached_from_visual_tree.snapshot().iter() {
                handler(e);
            }
        }
        // The renderer is told once the visual is out of the tree: that is how it knows a visual
        // that left from one that has no composition visual (section 9.41).
        let source = this.presentation_source.borrow_mut().take();
        bump_composition_holder_version();
        if let Some(source) = source {
            source.renderer().add_dirty(this);
        }
        this.elided_state.set(0);

        if let Some(children) = this.visual_children.get() {
            for child in children.snapshot().iter() {
                // A child removed within an event handler has already been
                // detached by the removal itself.
                if child.presentation_source().is_some() {
                    child.on_detached_from_visual_tree_core(e);
                }
            }
        }
    }

    fn on_attached_to_visual_tree(_this: &Self, _e: &VisualTreeAttachmentEventArgs) {}

    fn on_detached_from_visual_tree(_this: &Self, _e: &VisualTreeAttachmentEventArgs) {}

    fn on_visual_parent_changed(this: &Self, old_parent: Option<&Ref<Visual>>, new_parent: Option<&Ref<Visual>>) {
        this.raise_direct_property_changed(
            Self::visual_parent_property(),
            &old_parent.cloned(),
            &new_parent.cloned(),
        );
    }

    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        false
    }

    fn clip_to_bounds_radius(_this: &Self) -> Option<crate::CornerRadius> {
        None
    }

    fn custom_hit_test(_this: &Self, _point: crate::Point) -> Option<bool> {
        None
    }

    fn custom_hit_test_geometry(
        _this: &Self,
        _geometry: &Ref<crate::media::Geometry>,
    ) -> Option<crate::media::IntersectionResult> {
        None
    }

    fn ensure_initialized_for_visual_brush(_this: &Self) {}

    fn create_composition_visual(this: &Self, compositor: &Rc<Compositor>) -> CompositionDrawListVisual {
        CompositionDrawListVisual::new(compositor, this)
    }

    fn requires_composition_visual(_this: &Self) -> bool {
        false
    }

    fn detach_from_compositor(this: &Self) {
        // The shadow visual belongs to the composition visual it follows.
        this.release_theme_shadow_visual();
        this.detach_composition_item();
        if this.composition_visual_departed.get() || this.keep_departed_composition_visual() {
            return;
        }
        let composition_visual = this.composition_visual.take();
        if let Some(composition_visual) = composition_visual {
            bump_composition_holder_version();
            this.release_composition_visual(&composition_visual);
        }
        // Nothing holds a composition visual of this visual any more: in its next tree the
        // visual gets one when it needs one (section 9.41).
        this.composition_visual_handed_out.set(false);
    }

    fn synchronize_composition_child_visuals(this: &Self) {
        let Some(composition_visual) = this.own_composition_visual() else { return };
        // Listed from here on (section 9.42): what changes the tree below while the lists are
        // made (a composition visual one of them asks for) says so again.
        this.elided_state.set(this.elided_state.get() | CHILDREN_LISTED);
        let composition_children = composition_visual.children();
        // Not upstream: a visual with nothing under it (no visual children, no child composition
        // visual, no layer and no unloading visuals) is expected to have no composition children;
        // that is the case of most visuals a frame draws, and it is answered without the lists
        // below (docs/porting/desktop-performance.md, section 9.28).
        if this.visual_children.get().is_none_or(FerroList::is_empty)
            && this.child_composition_visual.borrow().is_none()
            && this.composition_extras.borrow().is_none()
        {
            if composition_children.count() != 0 {
                composition_children.clear();
            }
            composition_visual.clear_draw_items();
            return;
        }
        crate::perf_count!(CompositionChildrenListed);
        let mut visual_children: Rc<Vec<Ref<Visual>>> =
            this.visual_children_snapshot().unwrap_or_else(|| Rc::new(Vec::new()));

        // Not upstream (section 9.41): a child without a composition visual stands for the
        // composition visuals nearest below it, in its own order. The list is then the one the
        // composition children are, already in Z order.
        //
        // What the visuals between drew is shown by this composition visual, as its items
        // (section 9.42): `items` in paint order, and for each composition child the number of
        // them painted before it.
        let flattened = visual_children.iter().any(|child| !child.has_composition_visual());
        let mut items: Vec<Rc<CompositionDrawItem>> = Vec::new();
        let mut items_before: Vec<u32> = Vec::new();
        if flattened {
            let holds_items = composition_visual.holds_drawing_items();
            let mut entries = Vec::with_capacity(visual_children.len());
            this.collect_composition_entries(&mut entries);
            if !holds_items && entries.iter().any(|entry| matches!(entry, CompositionEntry::Item(..))) {
                // This composition visual shows no drawing but its own (it became the nearest
                // one above them since they drew): they get composition visuals.
                for entry in &entries {
                    if let CompositionEntry::Item(visual, _) = entry {
                        visual.materialize_composition_visual();
                    }
                }
                entries.clear();
                this.collect_composition_entries(&mut entries);
            }
            let mut nearest = Vec::with_capacity(entries.len());
            for entry in entries {
                match entry {
                    CompositionEntry::Visual(visual) => {
                        items_before.push(items.len() as u32);
                        nearest.push(visual);
                    }
                    CompositionEntry::Item(_, item) => {
                        if holds_items {
                            items.push(item);
                        }
                    }
                }
            }
            visual_children = Rc::new(nearest);
        }

        let mut sorted_children: Option<Vec<(Ref<Visual>, usize)>> = None;
        if !flattened && this.has_non_uniform_z_index_children() && visual_children.len() > 1 {
            let mut sorted: Vec<(Ref<Visual>, usize)> =
                visual_children.iter().cloned().enumerate().map(|(index, visual)| (visual, index)).collect();
            // The indices keep the order of elements with the same Z index.
            sorted.sort_by(|lhs, rhs| lhs.0.z_index().cmp(&rhs.0.z_index()).then(lhs.1.cmp(&rhs.1)));
            sorted_children = Some(sorted);
        }

        let mut child_visual = this.child_composition_visual();

        // Check if the current visual somehow got migrated to another compositor
        if child_visual.as_ref().is_some_and(|child| !Rc::ptr_eq(child.compositor(), composition_visual.compositor())) {
            child_visual = None;
        }

        let mut layer_visual = this.composition_layer_visual();
        if layer_visual.as_ref().is_some_and(|layer| !Rc::ptr_eq(layer.compositor(), composition_visual.compositor())) {
            layer_visual = None;
        }

        let unloading_visuals: Vec<Rc<CompositionVisual>> = this
            .read_composition_extras(|extras| extras.unloading_visuals.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|unloading| Rc::ptr_eq(unloading.compositor(), composition_visual.compositor()))
            .collect();

        let child_count = visual_children.len();
        let child_at = |c: usize| -> &Ref<Visual> {
            match &sorted_children {
                Some(sorted) => &sorted[c].0,
                None => &visual_children[c],
            }
        };

        // The visuals that draw the theme shadows of the children, each shown just before the
        // visual of its child (WinUI parents it under the prepend visual of the caster), so that
        // it is under the child and clipped by the clips of this visual, not by the child's.
        // Looked at only when a child may cast one.
        let shadows: Option<Vec<Option<Rc<CompositionVisual>>>> =
            if visual_children.iter().any(|child| child.may_cast_theme_shadow()) {
                Some((0..child_count).map(|c| child_at(c).ensure_theme_shadow_composition_visual()).collect())
            } else {
                None
            };
        let shadow_at = |c: usize| -> Option<&Rc<CompositionVisual>> {
            shadows.as_ref().and_then(|shadows| shadows[c].as_ref())
        };
        let shadow_count = shadows.as_ref().map_or(0, |shadows| shadows.iter().flatten().count());

        // Where the composition children are among the items: a shadow is where its caster is,
        // and a child without an entry is after every item.
        let mut positions: Vec<u32> = Vec::new();
        if !items.is_empty() {
            for (c, before) in items_before.iter().enumerate() {
                if shadow_at(c).is_some() {
                    positions.push(*before);
                }
                positions.push(*before);
            }
            while positions.last().is_some_and(|position| *position as usize == items.len()) {
                positions.pop();
            }
        }
        composition_visual.set_draw_items(items, positions);

        let mut expected_count = child_count + shadow_count + unloading_visuals.len();
        if child_visual.is_some() {
            expected_count += 1;
        }
        if layer_visual.is_some() {
            expected_count += 1;
        }

        let same = |a: &Rc<CompositionVisual>, b: Option<CompositionDrawListVisual>| {
            b.is_some_and(|b| Rc::ptr_eq(a, &b))
        };

        if composition_children.count() == expected_count {
            let mut mismatch = false;
            let mut index = 0;
            for c in 0..child_count {
                if let Some(shadow) = shadow_at(c) {
                    if !Rc::ptr_eq(&composition_children.get(index), shadow) {
                        mismatch = true;
                        break;
                    }
                    index += 1;
                }
                if !same(&composition_children.get(index), child_at(c).own_composition_visual()) {
                    mismatch = true;
                    break;
                }
                index += 1;
            }

            if !mismatch {
                for unloading in &unloading_visuals {
                    if !Rc::ptr_eq(&composition_children.get(index), unloading) {
                        mismatch = true;
                    }
                    index += 1;
                }
                if let Some(child_visual) = &child_visual {
                    if !Rc::ptr_eq(&composition_children.get(index), child_visual) {
                        mismatch = true;
                    }
                    index += 1;
                }
                if let Some(layer_visual) = &layer_visual {
                    if !Rc::ptr_eq(&composition_children.get(index), layer_visual) {
                        mismatch = true;
                    }
                }
            }

            if !mismatch {
                return;
            }
        }

        composition_children.clear();
        for c in 0..child_count {
            if let Some(shadow) = shadow_at(c) {
                // The shadow is shown by the visual that shows its caster (section 9.41).
                if let Some(old_parent) = shadow.parent() {
                    old_parent.children().remove(shadow);
                }
                composition_children.add(shadow.clone());
            }
            if let Some(composition_child) = child_at(c).own_composition_visual() {
                let composition_child: Rc<CompositionVisual> = (*composition_child).clone();
                // A composition visual that was shown by a visual further up (this visual had
                // no composition visual then, section 9.41) leaves it: it has one parent.
                if let Some(old_parent) = composition_child.parent() {
                    old_parent.children().remove(&composition_child);
                }
                composition_children.add(composition_child);
            }
        }

        for unloading in unloading_visuals {
            composition_children.add(unloading);
        }
        if let Some(child_visual) = child_visual {
            composition_children.add(child_visual);
        }
        if let Some(layer_visual) = layer_visual {
            composition_children.add(layer_visual);
        }
    }

    fn synchronize_composition_properties(this: &Self) {
        let Some(comp) = this.own_composition_visual() else { return };
        // What was given the composition visual, or set something of the composition API on the
        // visual, sees the offset and the parent of the composition visual: they are those of
        // the visual tree, since the parent then has a composition visual too (section 9.41).
        if this.is_composition_observable() {
            this.ensure_parent_composition_visual();
        }
        let bounds = this.bounds();

        // The implicit animations of the transitions, installed before the properties are
        // written (so that a change animates), except on the first synchronization of a new
        // composition visual (so that the initial values do not).
        this.apply_composition_transitions(&comp, false);

        // Composition property assignment replaces an animation, including assignment of
        // an unchanged API-side value. Only forward actual framework changes to the handoff
        // visual, so an unrelated repaint does not replace an application's animation.
        let offset = this.composition_offset(true);
        if comp.offset() != offset { comp.set_offset(offset); }
        let size = crate::Vector::new(bounds.width, bounds.height);
        if comp.size() != size { comp.set_size(size); }
        let visible = (this.is_visible() || this.is_shown_by_composition())
            && !this.is_hidden_by_composition() && !this.is_render_suppressed();
        if comp.visible() != visible { comp.set_visible(visible); }
        // The styled values the composition visual follows, read from the value store once per
        // change of the store: an object that is absent then is not looked up again, and one
        // that is present is read as before, since it can change without the store changing.
        let inputs = this.composition_styled_inputs();
        let opacity = inputs.opacity;
        if comp.opacity() != opacity { comp.set_opacity(opacity); }
        if comp.clip_to_bounds() != inputs.clip_to_bounds { comp.set_clip_to_bounds(inputs.clip_to_bounds); }
        comp.set_clip(if inputs.has_clip { this.clip().and_then(|clip| clip.platform_impl()) } else { None });

        let opacity_mask = if inputs.has_opacity_mask { this.opacity_mask() } else { None };
        let same_mask = match (comp.opacity_mask(), &opacity_mask) {
            (Some(a), Some(b)) => *a == **b,
            (None, None) => true,
            _ => false,
        };
        if !same_mask {
            comp.set_opacity_mask(opacity_mask);
        }

        let cache_mode = if inputs.has_cache_mode {
            this.cache_mode().map(|cache_mode| cache_mode.get_for_compositor(comp.compositor()))
        } else {
            None
        };
        let same_cache_mode = match (comp.cache_mode(), &cache_mode) {
            (Some(a), Some(b)) => Rc::ptr_eq(&a, b),
            (None, None) => true,
            _ => false,
        };
        if !same_cache_mode {
            comp.set_cache_mode(cache_mode);
        }

        let effect = if inputs.has_effect { this.effect() } else { None };
        if !crate::media::EffectExtensions::effect_equals(comp.effect().as_deref(), effect.as_deref()) {
            comp.set_effect(effect.as_ref().map(crate::media::EffectExtensions::to_immutable));
        }

        comp.set_render_options(this.render_options());
        comp.set_text_options(this.text_options());

        let mut render_transform = crate::Matrix::IDENTITY;

        if this.has_mirror_transform() {
            render_transform = crate::Matrix::new(-1.0, 0.0, 0.0, 1.0, bounds.width, 0.0);
        }

        if let Some(transform) = if inputs.has_render_transform { this.render_transform() } else { None } {
            let origin = this.render_transform_origin().to_pixels(crate::Size::new(bounds.width, bounds.height));
            let offset = crate::Matrix::create_translation(origin.x, origin.y);
            render_transform = render_transform * ((-offset) * transform.value() * offset);
        }

        // The `TransformMatrix` facade follows the render transform (see
        // `composition_facade_transform_matrix`).
        if let Some(facade) = this.composition_facade_transform_matrix() {
            render_transform = render_transform * facade;
        }

        // The zoom of a scroll presenter, after the transforms of the element itself.
        if let Some(manipulation) = this.manipulation_transform() {
            render_transform = render_transform * manipulation;
        }

        if comp.transform_matrix() != render_transform { comp.set_transform_matrix(render_transform); }

        if let Some(translation) = this.composition_translation() {
            if comp.translation() != translation { comp.set_translation(translation); }
        }

        this.synchronize_composition_facades(&comp);
        this.apply_composition_transitions(&comp, true);

        // The shadow of the theme follows the size, the visibility, the elevation and the theme.
        this.update_theme_shadow_visual();
    }

    fn invalidate_mirror_transform(this: &Self) {
        let flow_direction = this.flow_direction();
        let mut parent_flow_direction = FlowDirection::LeftToRight;
        let bypass = this.bypass_flow_direction_policies();
        let mut parent_bypass = false;

        if let Some(parent) = this.visual_parent() {
            parent_flow_direction = parent.flow_direction();
            parent_bypass = parent.bypass_flow_direction_policies();
        }

        let this_should_be_mirrored = flow_direction == FlowDirection::RightToLeft && !bypass;
        let parent_should_be_mirrored = parent_flow_direction == FlowDirection::RightToLeft && !parent_bypass;

        this.set_has_mirror_transform(this_should_be_mirrored != parent_should_be_mirrored);
    }
}

// Deviation (DEVIATIONS.md, Visual tree): the subscriptions to the values of
// the render-affecting properties are weak events in the managed original (`InvalidatedWeakEvent`): the value does not
// keep the visual, and the entries of visuals that were collected are removed
// from its list when the list is compacted. Here the handler of a visual holds
// it weakly too, and the visual takes its handlers out of the lists when it is
// dropped: a value many visuals share (the geometry of an icon that is a
// resource) would otherwise keep one entry, and the memory of the visual its
// weak reference holds, for every visual that ever drew it.
impl Drop for Visual {
    fn drop(&mut self) {
        for (_, subscription) in self.affects_render_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

crate::ferro_properties! { impl Visual {
    ferro_property!(
        /// Defines the `Bounds` property.
        pub fn bounds_property() -> DirectProperty<Visual, Rect> {
            FerroProperty::register_direct::<Visual, _>("Bounds", |o| o.bounds(), None, Rect::default())
        }
    );

    ferro_property!(
        /// Defines the `ClipToBounds` property.
        pub fn clip_to_bounds_property() -> StyledProperty<bool> {
            FerroProperty::register::<Visual, _>("ClipToBounds", false)
        }
    );

    ferro_property!(
        /// Defines the `Clip` property.
        pub fn clip_property() -> StyledProperty<Option<Ref<Geometry>>> {
            FerroProperty::register::<Visual, _>("Clip", None)
        }
    );

    ferro_property!(
        /// Defines the `Shadow` property (WinUI `UIElement.Shadow`): the shadow the element casts,
        /// a [`ThemeShadow`](crate::media::ThemeShadow).
        pub fn shadow_property() -> StyledProperty<Option<Ref<crate::media::Shadow>>> {
            FerroProperty::register::<Visual, _>("Shadow", None)
        }
    );

    ferro_property!(
        /// Defines the `Translation` property (WinUI `UIElement.Translation`): the facade
        /// [`translation`](Visual::translation) as a property markup can set
        /// (`Translation="0,0,32"`).
        pub fn translation_property() -> DirectProperty<Visual, crate::Vector3D> {
            FerroProperty::register_direct::<Visual, _>(
                "Translation",
                |o| o.translation(),
                Some(|o, value| o.set_translation(value)),
                crate::Vector3D::default(),
            )
        }
    );

    ferro_property!(
        /// Defines the `CacheMode` property.
        pub fn cache_mode_property() -> StyledProperty<Option<Ref<CacheMode>>> {
            FerroProperty::register::<Visual, _>("CacheMode", None)
        }
    );

    ferro_property!(
        /// Defines the `Effect` property.
        pub fn effect_property() -> StyledProperty<Option<Rc<dyn IEffect>>> {
            FerroProperty::register::<Visual, _>("Effect", None)
        }
    );

    ferro_property!(
        /// Defines the `OpacityMask` property.
        pub fn opacity_mask_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<Visual, _>("OpacityMask", None)
        }
    );

    ferro_property!(
        /// Defines the `RenderTransform` property.
        pub fn render_transform_property() -> StyledProperty<Option<Rc<dyn ITransform>>> {
            FerroProperty::register::<Visual, _>("RenderTransform", None)
        }
    );

    ferro_property!(
        /// Defines the `RenderTransformOrigin` property.
        pub fn render_transform_origin_property() -> StyledProperty<RelativePoint> {
            FerroProperty::register::<Visual, _>("RenderTransformOrigin", RelativePoint::CENTER)
        }
    );

    ferro_property!(
        /// Defines the `IsVisible` property.
        pub fn is_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<Visual, _>("IsVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `Opacity` property.
        pub fn opacity_property() -> StyledProperty<f64> {
            FerroProperty::register::<Visual, _>("Opacity", 1.0)
        }
    );

    ferro_property!(
        /// Defines the `HasMirrorTransform` property.
        pub fn has_mirror_transform_property() -> DirectProperty<Visual, bool> {
            FerroProperty::register_direct::<Visual, _>("HasMirrorTransform", |o| o.has_mirror_transform(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `FlowDirection` attached property.
        pub fn flow_direction_property() -> AttachedProperty<FlowDirection> {
            FerroProperty::register_attached_with::<Visual, Visual, _>(
                "FlowDirection",
                StyledPropertyOptions::new(FlowDirection::LeftToRight).inherits(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `VisualParent` property.
        pub fn visual_parent_property() -> DirectProperty<Visual, Option<Ref<Visual>>> {
            FerroProperty::register_direct::<Visual, _>("VisualParent", |o| o.visual_parent(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `ZIndex` property.
        pub fn z_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<Visual, _>("ZIndex", 0)
        }
    );
} }

impl Visual {
    /// The styled values [`Self::synchronize_composition_properties`] reads, kept while the value
    /// store of the visual is unchanged (none of the properties inherits).
    fn composition_styled_inputs(&self) -> CompositionStyledInputs {
        self.hot_composition_inputs.get(self.values().value_generation(), || CompositionStyledInputs {
            opacity: self.opacity() as f32,
            clip_to_bounds: self.clip_to_bounds(),
            has_clip: self.clip().is_some(),
            has_opacity_mask: self.opacity_mask().is_some(),
            has_cache_mode: self.cache_mode().is_some(),
            has_effect: self.effect().is_some(),
            has_render_transform: self.render_transform().is_some(),
        })
    }

    fn static_constructor() {
        PropertyChangeHandler::Visual.reacts_to(&[
            Self::is_visible_property().as_property(),
            Self::shadow_property().as_property(),
            crate::styling::ThemeVariant::actual_theme_variant_property().as_property(),
            Self::flow_direction_property().as_property(),
        ]);
        Self::affects_render::<Visual>(&[
            Self::clip_property().as_property(),
            Self::clip_to_bounds_property().as_property(),
            Self::is_visible_property().as_property(),
            Self::opacity_mask_property().as_property(),
            Self::effect_property().as_property(),
            Self::has_mirror_transform_property().as_property(),
        ]);
        // A visual draws in its own coordinates and its position reaches the compositor as the
        // offset of its composition visual: a change of position alone does not record the
        // drawing again, it synchronizes the composition properties (FerroUI deviation,
        // desktop-performance.md section 9.13; upstream's bounds affect render). A class whose
        // drawing reads its position names the bounds in its own `affects_render`.
        Self::bounds_property()
            .changed()
            .subscribe(|e| {
                if let Some(visual) = e.sender().downcast_ref::<Visual>() {
                    let old = e.get_old_value::<Rect>();
                    let new = e.get_new_value::<Rect>();
                    let resized = old.is_none_or(|old| old.size() != new.size());
                    // The composition visuals below a visual that has none are positioned
                    // through it (section 9.41).
                    let state = visual.elided_state.get();
                    if state & ELIDED_OFFSET_THROUGH != 0 && old.is_none_or(|old| old.position() != new.position()) {
                        visual.elided_state.set(state | ELIDED_OFFSET_MOVED);
                    }
                    if resized {
                        visual.invalidate_visual();
                    } else {
                        visual.invalidate_composition_properties();
                    }
                }
            })
            .detach();
        Self::render_transform_property().changed().subscribe(Self::render_transform_changed).detach();
        // Opacity is a composition property: its change does not record the drawing again.
        Self::opacity_property()
            .changed()
            .subscribe(|e| {
                if let Some(sender) = e.sender().downcast_ref::<Visual>() {
                    sender.invalidate_composition_properties();
                }
            })
            .detach();
        // The cache mode is a composition property too. Nothing followed its change before a
        // visual could be without a composition visual; a visual that gets a cache mode gets
        // one (section 9.41).
        Self::cache_mode_property()
            .changed()
            .subscribe(|e| {
                if let Some(sender) = e.sender().downcast_ref::<Visual>() {
                    sender.invalidate_composition_properties();
                }
            })
            .detach();
        Self::clip_to_bounds_property().changed().subscribe(|_| bump_geometry_version()).detach();
        Self::render_transform_origin_property().changed().subscribe(|_| bump_geometry_version()).detach();
        Self::z_index_property().changed().subscribe(|e| {
            let Some(sender) = e.sender().downcast_ref::<Visual>() else { return };
            let parent = sender.visual_parent();
            if let Some(parent) = &parent {
                if sender.z_index() != 0 {
                    parent.has_non_uniform_z_index_children.set(true);
                }
            }
            sender.invalidate_visual();
            if let Some(parent) = &parent {
                if let Some(source) = parent.presentation_source() {
                    source.renderer().recalculate_children(parent);
                }
            }
        }).detach();
    }

    /// Creates the class data; see [`crate::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            bounds: Cell::new(Rect::default()),
            presentation_source: RefCell::new(None),
            visual_parent: RefCell::new(None),
            has_mirror_transform: Cell::new(false),
            has_render_transform: Cell::new(false),
            hot_is_visible: Default::default(),
            hot_composition_inputs: Default::default(),
            has_manipulation_transform: Cell::new(false),
            is_effectively_visible: Cell::new(true),
            has_non_uniform_z_index_children: Cell::new(false),
            visual_children: OnceCell::new(),
            attached_to_visual_tree: HandlerList::new(),
            detached_from_visual_tree: HandlerList::new(),
            is_effectively_visible_changed: HandlerList::new(),
            render_options: Cell::new(RenderOptions::default()),
            text_options: Cell::new(TextOptions::default()),
            composition_visual: OptionCell::new(),
            composition_item: OptionCell::new(),
            composition_visual_departed: Cell::new(false),
            composition_visual_handed_out: Cell::new(false),
            elided_state: Cell::new(0),
            child_composition_visual: RefCell::new(None),
            composition_extras: RefCell::new(None),
            render_transform_subscription: RefCell::new(None),
            affects_render_subscriptions: RefCell::new(Vec::new()),
            renderer_sets: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // --- events -----------------------------------------------------------

    /// Raised when the control is attached to a rooted visual tree.
    pub fn attached_to_visual_tree(
        &self,
        handler: impl Fn(&VisualTreeAttachmentEventArgs) + 'static,
    ) -> Subscription {
        let token = self.attached_to_visual_tree.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.attached_to_visual_tree.remove(token);
            }
        }))
    }

    /// Raised when the control is detached from a rooted visual tree.
    pub fn detached_from_visual_tree(
        &self,
        handler: impl Fn(&VisualTreeAttachmentEventArgs) + 'static,
    ) -> Subscription {
        let token = self.detached_from_visual_tree.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.detached_from_visual_tree.remove(token);
            }
        }))
    }

    /// Raised when `is_effectively_visible` changes.
    pub fn is_effectively_visible_changed(&self, handler: impl Fn() + 'static) -> Subscription {
        let token = self.is_effectively_visible_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.is_effectively_visible_changed.remove(token);
            }
        }))
    }

    // --- properties -------------------------------------------------------

    /// The bounds of the control relative to its parent.
    #[inline]
    pub fn bounds(&self) -> Rect {
        self.bounds.get()
    }

    /// Sets the bounds of the control. Called by the layout system.
    pub fn set_bounds(&self, value: Rect) {
        if self.bounds.get() != value {
            bump_geometry_version();
        }
        self.set_and_raise_cell(Self::bounds_property(), &self.bounds, value);
    }

    /// Whether the control should be clipped to its bounds.
    pub fn clip_to_bounds(&self) -> bool {
        self.get_value(Self::clip_to_bounds_property())
    }

    pub fn set_clip_to_bounds(&self, value: bool) {
        self.set_value(Self::clip_to_bounds_property(), value)
    }

    /// The geometry clip for this visual.
    pub fn clip(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::clip_property())
    }

    pub fn set_clip(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::clip_property(), value.into().0)
    }

    /// The cache mode of the visual.
    pub fn cache_mode(&self) -> Option<Ref<CacheMode>> {
        self.get_value(Self::cache_mode_property())
    }

    pub fn set_cache_mode(&self, value: impl Into<Nullable<CacheMode>>) {
        self.set_value(Self::cache_mode_property(), value.into().0)
    }

    /// The effect of the control.
    pub fn effect(&self) -> Option<Rc<dyn IEffect>> {
        self.get_value(Self::effect_property())
    }

    pub fn set_effect(&self, value: Option<Rc<dyn IEffect>>) {
        self.set_value(Self::effect_property(), value)
    }

    /// The opacity mask of the control.
    pub fn opacity_mask(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::opacity_mask_property())
    }

    pub fn set_opacity_mask(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::opacity_mask_property(), value)
    }

    /// The render transform of the control.
    pub fn render_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.get_value(Self::render_transform_property())
    }

    pub fn set_render_transform(&self, value: Option<Rc<dyn ITransform>>) {
        self.set_value(Self::render_transform_property(), value)
    }

    /// The transform origin of the control.
    pub fn render_transform_origin(&self) -> RelativePoint {
        self.get_value(Self::render_transform_origin_property())
    }

    pub fn set_render_transform_origin(&self, value: RelativePoint) {
        self.set_value(Self::render_transform_origin_property(), value)
    }

    /// Whether this control and all its parents are visible.
    #[inline]
    pub fn is_effectively_visible(&self) -> bool {
        self.is_effectively_visible.get()
    }

    /// Whether this control is visible.
    pub fn is_visible(&self) -> bool {
        self.hot_is_visible.get(self.values().value_generation(), || self.get_value(Self::is_visible_property()))
    }

    pub fn set_is_visible(&self, value: bool) {
        self.set_value(Self::is_visible_property(), value)
    }

    /// The opacity of the control.
    pub fn opacity(&self) -> f64 {
        self.get_value(Self::opacity_property())
    }

    pub fn set_opacity(&self, value: f64) {
        self.set_value(Self::opacity_property(), value)
    }

    /// Whether `RenderTransform` has a value (not upstream: the transforms of the visual tree
    /// skip the property read when it has none).
    #[inline]
    pub(crate) fn has_render_transform(&self) -> bool {
        self.has_render_transform.get()
    }

    /// The manipulation transform of the visual (not upstream): a transform its parent applies
    /// to it after its render transform and before its position, about its top left corner. The
    /// scroll presenter of a zooming scroll viewer scales its content this way (WinUI applies the
    /// zoom of DirectManipulation to the content of a scroll viewer); it takes part in rendering,
    /// hit testing and the transforms between visuals as the render transform does. `None` by
    /// default.
    pub fn manipulation_transform(&self) -> Option<crate::Matrix> {
        if !self.has_manipulation_transform.get() {
            return None;
        }
        self.read_composition_extras(|extras| extras.manipulation_transform).flatten()
    }

    /// Sets [`manipulation_transform`](Self::manipulation_transform); the identity is no
    /// transform.
    pub fn set_manipulation_transform(&self, value: Option<crate::Matrix>) {
        let value = value.filter(|matrix| !matrix.is_identity());
        if self.manipulation_transform() == value {
            return;
        }
        self.has_manipulation_transform.set(value.is_some());
        self.with_composition_extras(|extras| extras.manipulation_transform = value);
        bump_geometry_version();
        self.invalidate_visual();
    }

    /// Whether the control is mirrored relative to its parent because of its
    /// flow direction.
    #[inline]
    pub fn has_mirror_transform(&self) -> bool {
        self.has_mirror_transform.get()
    }

    /// Sets the mirror transform flag.
    pub fn set_has_mirror_transform(&self, value: bool) {
        self.set_and_raise_cell(Self::has_mirror_transform_property(), &self.has_mirror_transform, value);
    }

    /// The text flow direction.
    pub fn flow_direction(&self) -> FlowDirection {
        self.get_value(Self::flow_direction_property())
    }

    pub fn set_flow_direction(&self, value: FlowDirection) {
        self.set_value(Self::flow_direction_property(), value)
    }

    /// Gets the value of the attached `FlowDirection` property on a visual.
    pub fn get_flow_direction(visual: &Visual) -> FlowDirection {
        visual.get_value(Self::flow_direction_property())
    }

    /// Sets the value of the attached `FlowDirection` property on a visual.
    pub fn set_flow_direction_on(visual: &Visual, value: FlowDirection) {
        visual.set_value(Self::flow_direction_property(), value)
    }

    /// The Z index of the control.
    ///
    /// Controls with a higher Z index appear in front of controls with a
    /// lower one. Among controls with the same value, order of appearance in
    /// the visual children decides.
    pub fn z_index(&self) -> i32 {
        self.get_value(Self::z_index_property())
    }

    pub fn set_z_index(&self, value: i32) {
        self.set_value(Self::z_index_property(), value)
    }

    /// The control's child visuals.
    pub fn visual_children(&self) -> &FerroList<Ref<Visual>> {
        self.visual_children.get_or_init(|| {
            let list = FerroList::new();
            list.set_reset_behavior(ResetBehavior::Remove);
            let weak = self.to_ref().downgrade();
            let validator_owner = weak.clone();
            list.set_validate(Some(Rc::new(move |item: &Ref<Visual>| {
                if let Some(parent) = item.visual_parent() {
                    let owner = validator_owner.upgrade();
                    panic!(
                        "The control {:?} already has a visual parent {:?} while trying to add it as a child of {:?}.",
                        item, parent, owner
                    );
                }
            })));
            list.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Ref<Visual>>| {
                if let Some(this) = weak.upgrade() {
                    this.visual_children_changed(e);
                }
            }));
            list
        })
    }

    /// The number of visual children, without creating the collection.
    #[inline]
    pub fn visual_children_count(&self) -> usize {
        self.visual_children.get().map_or(0, FerroList::count)
    }

    /// A snapshot of the visual children, without creating the collection.
    #[inline]
    pub fn visual_children_snapshot(&self) -> Option<Rc<Vec<Ref<Visual>>>> {
        self.visual_children.get().map(FerroList::snapshot)
    }

    /// The presentation source the visual is attached to.
    #[inline]
    pub fn presentation_source(&self) -> Option<Rc<dyn IPresentationSource>> {
        self.presentation_source.borrow().clone()
    }

    /// The layout scaling of the presentation source the visual is attached
    /// to, read in place: layout asks for every element it rounds.
    #[inline]
    pub(crate) fn layout_scaling_of_presentation_source(&self) -> Option<f64> {
        self.presentation_source.borrow().as_ref().map(|source| source.layout_root_scaling())
    }

    /// The root of the visual tree, if the control is attached to one.
    pub fn visual_root(&self) -> Option<Ref<Visual>> {
        self.presentation_source().and_then(|s| s.root_visual())
    }

    /// The render options of the visual; see the accessors on
    /// [`RenderOptions`].
    #[inline]
    pub(crate) fn render_options(&self) -> RenderOptions {
        self.render_options.get()
    }

    pub(crate) fn set_render_options(&self, value: RenderOptions) {
        self.render_options.set(value);
        self.invalidate_visual();
    }

    /// The text options of the visual; see the accessors on
    /// [`TextOptions`].
    #[inline]
    pub(crate) fn text_options(&self) -> TextOptions {
        self.text_options.get()
    }

    pub(crate) fn set_text_options(&self, value: TextOptions) {
        self.text_options.set(value);
        self.invalidate_visual();
    }

    /// The composition visual of the visual, while it is attached to a
    /// visual tree rendered by a compositor.
    ///
    /// A visual that shows nothing itself has none until it is asked for here (section 9.41 of
    /// docs/porting/desktop-performance.md): it is created then, and so is the one of the
    /// visual parent, so that the offset and the parent of what is returned are those of the
    /// visual tree. [`has_composition_visual`](Self::has_composition_visual) asks without
    /// creating one.
    pub fn composition_visual(&self) -> Option<CompositionDrawListVisual> {
        let composition_visual = match self.own_composition_visual() {
            Some(composition_visual) => composition_visual,
            None => self.materialize_composition_visual()?,
        };
        // What is given out here may be changed or held by the caller: it is not kept when the
        // visual leaves its tree (section 9.32).
        self.composition_visual_handed_out.set(true);
        self.ensure_parent_composition_visual();
        Some(composition_visual)
    }

    /// Whether the visual has a composition visual of its own now. A visual attached to a tree
    /// that has none is shown by the composition visual of the nearest visual above it that has
    /// one: what the visual draws is an item of that composition visual
    /// ([`has_composition_item`](Self::has_composition_item)), and the composition visuals of
    /// the visuals under it are its children (docs/porting/desktop-performance.md, sections
    /// 9.41 and 9.42).
    #[inline]
    pub fn has_composition_visual(&self) -> bool {
        !self.composition_visual_departed.get() && self.composition_visual.is_some()
    }

    /// Whether what the visual draws is shown now by the composition visual of the nearest
    /// visual above it that has one, as an item of it: the visual has no composition visual of
    /// its own (docs/porting/desktop-performance.md, section 9.42).
    #[inline]
    pub fn has_composition_item(&self) -> bool {
        self.composition_item.get().is_some_and(|item| !item.is_departed())
    }

    /// Whether visuals get their composition visuals on demand (the default), or each of them
    /// when it is attached to a visual tree.
    #[inline]
    pub fn composition_visuals_on_demand() -> bool {
        COMPOSITION_VISUALS_ON_DEMAND.load(Ordering::Relaxed)
    }

    /// Sets whether visuals get their composition visuals on demand. With `false` every visual
    /// has a composition visual from the moment it is attached to a visual tree, as upstream.
    /// It applies to the visuals attached from then on; it is meant to be set once, before the
    /// first window.
    pub fn set_composition_visuals_on_demand(value: bool) {
        COMPOSITION_VISUALS_ON_DEMAND.store(value, Ordering::Relaxed);
    }

    /// The nearest visual above this one that has a composition visual.
    pub(crate) fn nearest_composition_ancestor(&self) -> Option<Ref<Visual>> {
        let mut parent = self.visual_parent();
        while let Some(ancestor) = parent {
            if ancestor.has_composition_visual() {
                return Some(ancestor);
            }
            parent = ancestor.visual_parent();
        }
        None
    }

    /// Whether the visual needs a composition visual of its own for something other than its
    /// drawing: it is the root, it sets something the compositor applies (a transform, an
    /// opacity, a clip, an effect, a cache mode, options, its visibility), it uses the
    /// composition API, or it answers hit tests itself. A visual that needs none and draws
    /// nothing has none (section 9.41).
    pub(crate) fn needs_own_composition_visual(&self) -> bool {
        if !Self::composition_visuals_on_demand()
            || self.visual_parent.borrow().is_none()
            || self.composition_extras.borrow().is_some()
            || self.child_composition_visual.borrow().is_some()
            || self.has_mirror_transform.get()
            || self.has_manipulation_transform.get()
            || self.render_options.get() != RenderOptions::default()
            || self.text_options.get() != TextOptions::default()
        {
            return true;
        }
        let inputs = self.composition_styled_inputs();
        inputs.opacity != 1.0
            // A visual that clips and has nothing under it is drawn clipped, as an item of
            // the composition visual above (section 9.42).
            || (inputs.clip_to_bounds && self.visual_children_count() != 0)
            || inputs.has_clip
            || inputs.has_opacity_mask
            || inputs.has_cache_mode
            || inputs.has_effect
            || inputs.has_render_transform
            || !self.is_visible()
            || self.requires_composition_visual()
            || self.custom_hit_test(crate::Point::default()).is_some()
    }

    /// Whether something outside the synchronisation of the visual tree may read or animate the
    /// composition visual: it was handed out, or the visual has state of the composition API.
    #[inline]
    fn is_composition_observable(&self) -> bool {
        self.composition_visual_handed_out.get() || self.composition_extras.borrow().is_some()
    }

    /// Gives the visual parent a composition visual when it has none.
    fn ensure_parent_composition_visual(&self) {
        if let Some(parent) = self.visual_parent() {
            if !parent.has_composition_visual() {
                parent.materialize_composition_visual();
            }
        }
    }

    /// Creates the composition visual of a visual that is attached to a tree without one, and
    /// tells the renderer, which places it in the composition tree (section 9.41). `None` when
    /// the visual is in no tree rendered by a compositor.
    pub(crate) fn materialize_composition_visual(&self) -> Option<CompositionDrawListVisual> {
        if self.composition_visual_departed.get() {
            return None;
        }
        let existing = self.composition_visual.get();
        if existing.is_some() {
            return existing;
        }
        let source = self.presentation_source()?;
        let renderer = source.renderer();
        let compositor = renderer.compositor()?;
        let created = self.attach_to_compositor_core(&compositor, Some(&*renderer));
        renderer.composition_visual_materialized(self);
        Some(created)
    }

    /// The offset of the composition visual of the visual: its position in the nearest visual
    /// above it that has a composition visual (section 9.41). With `mark`, the visuals between
    /// the two remember that an offset was computed through them.
    ///
    /// The positions are added from the visual that has the composition visual down to this one,
    /// so that the position of a visual in between is a term of the sum, which
    /// [`composition_item_placement`](Self::composition_item_placement) keeps for the visuals
    /// below it: both give the same number, to the last bit.
    pub(crate) fn composition_offset(&self, mark: bool) -> crate::Vector3D {
        let bounds = self.bounds.get();
        let (x, y) = self.visual_parent().map_or((0.0, 0.0), |parent| parent.elided_origin(mark));
        crate::Vector3D::new(x + bounds.x, y + bounds.y, 0.0)
    }

    /// Where the visual is in the nearest visual above it that has a composition visual; nothing
    /// is added for a visual that has a composition visual, or that is out of the tree.
    fn elided_origin(&self, mark: bool) -> (f64, f64) {
        if self.has_composition_visual() || !self.is_attached_to_visual_tree() {
            return (0.0, 0.0);
        }
        let (x, y) = self.visual_parent().map_or((0.0, 0.0), |parent| parent.elided_origin(mark));
        if mark {
            self.elided_state.set(self.elided_state.get() | ELIDED_OFFSET_THROUGH);
        }
        let position = self.bounds.get();
        (x + position.x, y + position.y)
    }

    /// For a visual without a composition visual: the nearest visual above it that has one, whose
    /// composition visual shows what this one draws as an item, and where the item is in it:
    /// [`nearest_composition_ancestor`](Self::nearest_composition_ancestor) and
    /// [`composition_offset`](Self::composition_offset) with `mark`, in one walk that ends at the
    /// first visual `placements` knows (section 9.47).
    pub(crate) fn composition_item_placement(
        &self,
        placements: &mut CompositionItemPlacements,
    ) -> Option<(Ref<Visual>, (f64, f64))> {
        placements.validate();
        let (holder, x, y) = Self::placement_below(&self.visual_parent()?, placements)?;
        let bounds = self.bounds.get();
        let offset = (x + bounds.x, y + bounds.y);
        debug_assert!(
            self.nearest_composition_ancestor().is_some_and(|ancestor| ancestor.ptr_eq(&holder))
                && self.composition_offset(false) == crate::Vector3D::new(offset.0, offset.1, 0.0),
            "the placement kept for the visuals below a visual is not the one a walk gives"
        );
        Some((holder, offset))
    }

    /// The visual that shows what is below `visual`, and where `visual` is in it: `visual` itself
    /// when it has a composition visual.
    fn placement_below(
        visual: &Ref<Visual>,
        placements: &mut CompositionItemPlacements,
    ) -> Option<(Ref<Visual>, f64, f64)> {
        if visual.has_composition_visual() {
            return Some((visual.clone(), 0.0, 0.0));
        }
        if !visual.is_attached_to_visual_tree() {
            // Nothing is positioned through a visual out of the tree, and nothing is kept for it.
            return visual.nearest_composition_ancestor().map(|holder| (holder, 0.0, 0.0));
        }
        let key = &**visual as *const Visual as usize;
        if let Some((holder, x, y)) = placements.below.get(&key) {
            return Some((holder.clone(), *x, *y));
        }
        let (holder, x, y) = Self::placement_below(&visual.visual_parent()?, placements)?;
        // An entry says that offsets of the visuals below were computed through the visual, as
        // the mark does: the mark is taken away where the entries are dropped (the visual gets a
        // composition visual, or leaves its tree).
        visual.elided_state.set(visual.elided_state.get() | ELIDED_OFFSET_THROUGH);
        let position = visual.bounds.get();
        let (x, y) = (x + position.x, y + position.y);
        placements.below.insert(key, (holder.clone(), x, y));
        Some((holder, x, y))
    }

    /// Writes the offset of the composition visual again: a visual above it that has no
    /// composition visual moved, or got one.
    ///
    /// `origin` is where the parent of the visual is in the nearest visual above that has a
    /// composition visual ([`elided_origin`](Self::elided_origin)): the walk down the visuals
    /// that moved carries it, so no visual walks up for it (section 9.47).
    fn synchronize_composition_offset(&self, origin: (f64, f64)) {
        let bounds = self.bounds.get();
        let offset = (origin.0 + bounds.x, origin.1 + bounds.y);
        debug_assert!(
            self.composition_offset(false) == crate::Vector3D::new(offset.0, offset.1, 0.0),
            "the offset carried down the visuals is not the one a walk up gives"
        );
        if let Some(comp) = self.own_composition_visual() {
            let offset = crate::Vector3D::new(offset.0, offset.1, 0.0);
            if comp.offset() != offset {
                comp.set_offset(offset);
            }
        } else if let Some(item) = self.composition_item.get() {
            item.set_offset(offset);
        }
    }

    /// Writes the offsets of the composition visuals nearest below this visual again. `false`
    /// when it stopped because there are more of them than `visuals_left`, or more visuals on
    /// the way than `nodes_left`.
    ///
    /// `origin` is where this visual is in the nearest visual above that has a composition
    /// visual, or this visual itself when it has one.
    fn synchronize_composition_offsets_below(
        &self,
        origin: (f64, f64),
        visuals_left: &mut u32,
        nodes_left: &mut u32,
    ) -> bool {
        let Some(children) = self.visual_children_snapshot() else { return true };
        for child in children.iter() {
            if *nodes_left == 0 {
                return false;
            }
            *nodes_left -= 1;
            let has_composition_visual = child.has_composition_visual();
            // What a visual without a composition visual drew is positioned as a composition
            // visual is (section 9.42).
            if has_composition_visual || child.composition_item.is_some() {
                if *visuals_left == 0 {
                    return false;
                }
                *visuals_left -= 1;
                child.synchronize_composition_offset(origin);
            }
            if !has_composition_visual && child.elided_state.get() & ELIDED_OFFSET_THROUGH != 0 {
                // As `elided_origin` adds them: nothing for a visual out of the tree.
                let below = if child.is_attached_to_visual_tree() {
                    let position = child.bounds.get();
                    (origin.0 + position.x, origin.1 + position.y)
                } else {
                    (0.0, 0.0)
                };
                if !child.synchronize_composition_offsets_below(below, visuals_left, nodes_left) {
                    return false;
                }
            }
        }
        true
    }

    /// For a visual that has a composition visual: the composition visuals below it that were
    /// positioned through it while it had none are positioned in it (section 9.41).
    pub(crate) fn settle_composition_offsets(&self) {
        let state = self.elided_state.get();
        if state & !CHILDREN_LISTED == 0 {
            return;
        }
        self.elided_state.set(state & CHILDREN_LISTED);
        if state & ELIDED_OFFSET_THROUGH != 0 {
            let (mut visuals, mut nodes) = (u32::MAX, u32::MAX);
            self.synchronize_composition_offsets_below(self.elided_origin(true), &mut visuals, &mut nodes);
        }
    }

    /// For a visual without a composition visual, in the update pass of its renderer: the
    /// composition visuals below it follow a change of its position. `false` when there are
    /// too many of them: the renderer then gives the visual a composition visual, whose offset
    /// is the one thing a later move changes.
    ///
    /// Where the visual is comes from `placements`, which the pass keeps for the visuals above
    /// the ones it records and moves (section 9.47).
    pub(crate) fn move_composition_offsets_below(&self, placements: &mut CompositionItemPlacements) -> bool {
        let state = self.elided_state.get();
        if state & ELIDED_OFFSET_MOVED == 0 {
            return true;
        }
        self.elided_state.set(state & !ELIDED_OFFSET_MOVED);
        // As `elided_origin(true)` of the visual gives it.
        let origin = if self.has_composition_visual() || !self.is_attached_to_visual_tree() {
            (0.0, 0.0)
        } else {
            self.elided_state.set(self.elided_state.get() | ELIDED_OFFSET_THROUGH);
            self.composition_item_origin(placements)
        };
        debug_assert!(origin == self.elided_origin(false), "the origin kept for a visual is not the one a walk gives");
        let (mut visuals, mut nodes) = (ELIDED_MOVE_VISUALS, ELIDED_MOVE_NODES);
        self.synchronize_composition_offsets_below(origin, &mut visuals, &mut nodes)
    }

    /// Where the visual is in the nearest visual above it that has a composition visual:
    /// [`composition_offset`](Self::composition_offset) with the mark, from `placements`.
    fn composition_item_origin(&self, placements: &mut CompositionItemPlacements) -> (f64, f64) {
        match self.composition_item_placement(placements) {
            Some((_, origin)) => origin,
            None => {
                let offset = self.composition_offset(true);
                (offset.x, offset.y)
            }
        }
    }

    /// The update pass of the renderer saw the visual without a composition visual: its bounds
    /// are those of a layout pass from here on.
    #[inline]
    pub(crate) fn mark_seen_without_composition_visual(&self) {
        self.elided_state.set(self.elided_state.get() | ELIDED_SEEN);
    }

    /// Whether an update pass saw the visual without a composition visual.
    #[inline]
    pub(crate) fn was_seen_without_composition_visual(&self) -> bool {
        self.elided_state.get() & ELIDED_SEEN != 0
    }

    /// Whether the child composition visuals and the items of the composition visual of the
    /// visual are those of the visual tree below it: they were listed, and nothing that decides
    /// them changed since. The update pass of the renderer does not list them again for a
    /// visual that is only drawn again (section 9.42).
    #[inline]
    pub(crate) fn composition_children_listed(&self) -> bool {
        self.elided_state.get() & CHILDREN_LISTED != 0
    }

    /// Something that decides what the composition visual of the visual shows below it changed
    /// (a visual child, the order of the children, a composition visual or an item below it):
    /// the next synchronisation lists it again.
    #[inline]
    pub(crate) fn forget_composition_children(&self) {
        let state = self.elided_state.get();
        if state & CHILDREN_LISTED != 0 {
            self.elided_state.set(state & !CHILDREN_LISTED);
        }
    }

    /// Debug builds: lists what is below the composition visual of a visual that says it is
    /// listed, and fails when the lists were not the ones it had: something changed the tree
    /// below without a notice ([`forget_composition_children`](Self::forget_composition_children)).
    /// Not with the feature `perf-counters`, whose counts are those of a release build.
    #[cfg(all(debug_assertions, not(feature = "perf-counters")))]
    pub(crate) fn assert_composition_children_listed(&self) {
        let Some(composition_visual) = self.own_composition_visual() else { return };
        let shown = || {
            let children = composition_visual.children();
            let children: Vec<*const CompositionVisual> =
                (0..children.count()).map(|index| Rc::as_ptr(&children.get(index))).collect();
            let (items, positions) = composition_visual.draw_items().map_or((Vec::new(), Vec::new()), |list| {
                let items: Vec<*const CompositionDrawItem> =
                    (0..list.count()).filter_map(|index| list.item_at(index)).map(|item| Rc::as_ptr(&item)).collect();
                let positions: Vec<usize> = (0..children.len()).map(|child| list.position(child)).collect();
                (items, positions)
            });
            (children, items, positions)
        };
        let before = shown();
        self.synchronize_composition_child_visuals();
        assert!(
            before == shown(),
            "what is below the composition visual of a visual changed without a notice to the visual"
        );
    }

    /// What is below the composition visual of this visual, in the order it is shown: for each
    /// visual child in Z order its composition visual, or, when it has none, what it drew and
    /// then what is below it (section 9.42).
    pub(crate) fn collect_composition_entries(&self, out: &mut Vec<CompositionEntry>) {
        let Some(children) = self.visual_children_snapshot() else { return };
        if self.has_non_uniform_z_index_children() && children.len() > 1 {
            let mut sorted: Vec<(i32, usize)> =
                children.iter().enumerate().map(|(index, child)| (child.z_index(), index)).collect();
            // The indices keep the order of elements with the same Z index.
            sorted.sort_unstable();
            for (_, index) in sorted {
                children[index].collect_composition_entry(out);
            }
        } else {
            for child in children.iter() {
                child.collect_composition_entry(out);
            }
        }
    }

    fn collect_composition_entry(&self, out: &mut Vec<CompositionEntry>) {
        if self.has_composition_visual() {
            out.push(CompositionEntry::Visual(self.to_ref()));
            return;
        }
        if let Some(item) = self.composition_item.get() {
            if !item.is_departed() {
                out.push(CompositionEntry::Item(self.to_ref(), item));
            }
        }
        self.collect_composition_entries(out);
    }

    /// What the visual drew, as the composition visual of the nearest visual above it that has
    /// one shows it: the visual has no composition visual of its own (section 9.42).
    #[inline]
    pub(crate) fn composition_item(&self) -> Option<Rc<CompositionDrawItem>> {
        self.composition_item.get()
    }

    pub(crate) fn set_composition_item(&self, item: Option<Rc<CompositionDrawItem>>) {
        self.composition_item.set(item);
    }

    /// The visual draws nothing any more, or what it drew is of another compositor: the
    /// composition visual that shows the item is synchronized by the caller.
    pub(crate) fn release_composition_item(&self) {
        if let Some(item) = self.composition_item.take() {
            item.release();
        }
    }

    /// The size the drawing of the visual is clipped to as an item.
    pub(crate) fn composition_item_clip(&self) -> Option<(f64, f64)> {
        self.clip_to_bounds().then(|| {
            let bounds = self.bounds.get();
            (bounds.width, bounds.height)
        })
    }

    /// Writes where the item of the visual is, and its clip, again.
    pub(crate) fn synchronize_composition_item(&self, placements: &mut CompositionItemPlacements) {
        if let Some(item) = self.composition_item.get() {
            item.set_offset(self.composition_item_origin(placements));
            item.set_clip(self.composition_item_clip());
        }
    }

    /// The visual is leaving its visual tree: the renderer keeps what it drew for its return,
    /// as it keeps a composition visual (section 9.32), or the visual lets go of it. The
    /// composition visual that shows it is synchronized because its visual children changed.
    fn detach_composition_item(&self) {
        let Some(item) = self.composition_item.get() else { return };
        if item.is_departed() {
            return;
        }
        let kept = self
            .presentation_source
            .borrow()
            .as_ref()
            .is_some_and(|source| source.renderer().keep_departed_composition_item(self));
        if kept {
            item.set_departed(true);
        } else {
            self.composition_item.set(None);
            item.release();
        }
    }

    /// What kept the item of the visual lets go of it: if the visual is still out of its tree
    /// the item is released, as it would have been when the visual left (section 9.32).
    pub(crate) fn release_departed_composition_item(&self, kept: &Rc<CompositionDrawItem>) {
        let held = self.composition_item.with(|held| held.is_some_and(|held| Rc::ptr_eq(held, kept)));
        if held && kept.is_departed() {
            self.composition_item.set(None);
            kept.release();
        }
    }

    /// The composition visual of the visual for the synchronisation of the visual tree with the
    /// composition tree, which writes to it only what it writes again when the visual is
    /// synchronized (section 9.32). Anything else asks [`composition_visual`](Self::composition_visual).
    #[inline]
    pub(crate) fn own_composition_visual(&self) -> Option<CompositionDrawListVisual> {
        if self.composition_visual_departed.get() {
            return None;
        }
        self.composition_visual.get()
    }

    /// Whether the visual left its visual tree and its composition visual is kept for its return
    /// (section 9.32).
    #[inline]
    pub(crate) fn composition_visual_departed(&self) -> bool {
        self.composition_visual_departed.get()
    }

    /// Whether the composition visual of the visual is one that nothing but the synchronisation
    /// of the visual tree wrote to, so that it is the same as one created now once the visual is
    /// synchronized again: it was not handed out, it is a plain draw list visual, and the visual
    /// has no child composition visual and no state of the composition API. What a control adds
    /// to its composition visual is kept when the control says that it writes it whether or not
    /// it is in a tree (`ICompositionDrawListVisualExtension::follows_control_out_of_tree`).
    fn composition_visual_is_untouched(&self, composition_visual: &CompositionDrawListVisual) -> bool {
        !self.composition_visual_handed_out.get()
            && self.child_composition_visual.borrow().is_none()
            && self.composition_extras.borrow().is_none()
            && composition_visual.can_be_kept_out_of_tree()
    }

    /// The visual is leaving its visual tree: asks the renderer to keep the composition visual,
    /// and reports whether it does (section 9.32). The renderer releases it in its next update
    /// pass unless the visual is back by then; inside
    /// [`leave_tree_keeping_composition_visuals`](Self::leave_tree_keeping_composition_visuals)
    /// it is kept for the caller of that instead. Whatever takes an element out of the tree and
    /// puts it back (a panel that moves a child, a virtualizing panel that recycles a row) then
    /// keeps the composition visuals of the element and of everything under it, on both sides of
    /// the compositor.
    fn keep_departed_composition_visual(&self) -> bool {
        let Some(composition_visual) = self.composition_visual.get() else { return false };
        let kept = self.composition_visual_is_untouched(&composition_visual)
            && self
                .presentation_source
                .borrow()
                .as_ref()
                .is_some_and(|source| source.renderer().keep_departed_composition_visual(self, &composition_visual));
        if kept {
            self.composition_visual_departed.set(true);
            bump_composition_holder_version();
            composition_visual.set_departed(true);
        }
        kept
    }

    /// What a visual does to its composition visual when it lets go of it.
    fn release_composition_visual(&self, composition_visual: &CompositionDrawListVisual) {
        if let Some(child) = self.child_composition_visual() {
            composition_visual.children().remove(&child);
        }
        if let Some(layer) = self.composition_layer_visual() {
            composition_visual.children().remove(&layer);
        }
        Self::release_orphaned_composition_visual(composition_visual);
    }

    /// Releases a composition visual whose visual is gone.
    pub(crate) fn release_orphaned_composition_visual(composition_visual: &CompositionDrawListVisual) {
        composition_visual.set_departed(false);
        composition_visual.set_draw_list(None);
        composition_visual.clear_draw_items();
        composition_visual.set_opacity_mask(None);
        // The composition visual and its children refer to each other;
        // nothing collects such a cycle here, so the visual lets go of
        // its children. They are re-added when their visuals are
        // synchronized again.
        composition_visual.children().clear();
    }

    /// What kept the composition visual of the visual lets go of it (the update pass of the
    /// renderer, or the holder of a [`KeptCompositionVisuals`]): if the visual is still out of
    /// its tree the composition visual is released, as it would have been when the visual left
    /// (section 9.32). Nothing is done when the visual is back in a tree, or holds another
    /// composition visual. A visual that left twice is released by whichever of the two lets go
    /// first: it is then created again when the visual returns, as it always was.
    pub(crate) fn release_departed_composition_visual(&self, kept: &CompositionDrawListVisual) {
        if !self.composition_visual_departed.get() {
            return;
        }
        let held = self.composition_visual.with(|held| held.is_some_and(|held| Rc::ptr_eq(held, kept)));
        if !held {
            return;
        }
        self.composition_visual_departed.set(false);
        let composition_visual = self.composition_visual.take();
        bump_composition_holder_version();
        if let Some(composition_visual) = composition_visual {
            self.release_composition_visual(&composition_visual);
        }
    }

    /// Runs `leave`, which takes this visual (or visuals in the same visual tree) out of the
    /// tree, and returns the composition visuals that were kept for what left: those of the
    /// visuals that nothing but the synchronisation of the visual tree wrote to (FerroUI
    /// addition, docs/porting/desktop-performance.md, section 9.32).
    ///
    /// They live until what is returned is released or dropped. A visual attached to a tree of
    /// the same compositor before that has the composition visual it had, with what the
    /// compositor holds for it on the render thread; one that is not lets go of it then, as it
    /// would have when it left. What recycles elements (the pool of a virtualizing panel) holds
    /// the result for as long as the element is in its pool, and nothing is kept longer than
    /// the holder decides. `None` when nothing was kept: the visual is in no tree, or its
    /// renderer keeps nothing.
    pub fn leave_tree_keeping_composition_visuals(&self, leave: impl FnOnce()) -> Option<KeptCompositionVisuals> {
        let mut leave = Some(leave);
        let mut departure = || {
            if let Some(leave) = leave.take() {
                leave();
            }
        };
        let kept = match self.presentation_source() {
            Some(source) => source.renderer().keep_composition_visuals_departing_in(&mut departure),
            None => None,
        };
        // A renderer that did not run it.
        departure();
        kept
    }

    /// The composition visual shown as the last child of the composition
    /// visual of this visual (see `ElementComposition`).
    pub fn child_composition_visual(&self) -> Option<Rc<CompositionVisual>> {
        self.child_composition_visual.borrow().clone()
    }

    pub fn set_child_composition_visual(&self, value: Option<Rc<CompositionVisual>>) {
        *self.child_composition_visual.borrow_mut() = value;
    }

    fn with_composition_extras<R>(&self, f: impl FnOnce(&mut CompositionExtras) -> R) -> R {
        let (result, created) = {
            let mut extras = self.composition_extras.borrow_mut();
            let created = extras.is_none();
            (f(extras.get_or_insert_with(Default::default)), created)
        };
        if created {
            self.composition_extras_created();
        }
        result
    }

    /// The visual has state of the composition API from here on: it has a composition visual
    /// while it is in a tree (section 9.41). What set the state need not tell the renderer.
    #[cold]
    fn composition_extras_created(&self) {
        if self.is_attached_to_visual_tree() && !self.has_composition_visual() {
            self.materialize_composition_visual();
        }
    }

    fn read_composition_extras<R>(&self, f: impl FnOnce(&CompositionExtras) -> R) -> Option<R> {
        self.composition_extras.borrow().as_deref().map(f)
    }

    /// Changes the composition extras when the visual has them. Taking something away from a
    /// visual that never had extras leaves it without them: the visuals without extras, nearly
    /// all of them, are the ones the synchronisation answers on its short paths (sections 9.28
    /// and 9.32 of docs/porting/desktop-performance.md).
    fn change_composition_extras<R>(&self, f: impl FnOnce(&mut CompositionExtras) -> R) -> Option<R> {
        self.composition_extras.borrow_mut().as_deref_mut().map(f)
    }

    /// Whether the visual has composition extras.
    #[cfg(test)]
    pub(crate) fn has_composition_extras(&self) -> bool {
        self.composition_extras.borrow().is_some()
    }

    /// `UIElement.Lights`: the XAML lights the element holds. A light added
    /// to the collection is connected to the element (`OnConnected`) while
    /// the element is in a visual tree, and lights the targets of its id in
    /// the subtree of the element (winui-materials.md).
    pub fn lights(&self) -> FerroList<Ref<crate::media::XamlLight>> {
        if let Some(lights) = self.read_composition_extras(|extras| extras.lights.clone()).flatten() {
            return lights;
        }
        let lights: FerroList<Ref<crate::media::XamlLight>> = FerroList::new();
        let subscriptions = crate::media::xaml_light::track_lights(self, &lights);
        self.with_composition_extras(|extras| {
            extras.lights = Some(lights.clone());
            extras.lights_subscriptions = subscriptions;
        });
        lights
    }

    /// Whether the element holds a lights collection.
    pub fn has_lights(&self) -> bool {
        self.read_composition_extras(|extras| extras.lights.as_ref().is_some_and(|lights| lights.count() > 0))
            .unwrap_or(false)
    }

    /// The ids of the XAML lights that target the element.
    pub fn light_target_ids(&self) -> Vec<String> {
        self.read_composition_extras(|extras| extras.light_target_ids.clone()).unwrap_or_default()
    }

    /// Adds `id` to the ids of the XAML lights that target the element;
    /// returns whether it was added.
    pub(crate) fn add_light_target_id(&self, id: &str) -> bool {
        let added = self.with_composition_extras(|extras| {
            if extras.light_target_ids.iter().any(|existing| existing == id) {
                return false;
            }
            extras.light_target_ids.push(id.to_owned());
            true
        });
        if added && self.read_composition_extras(|extras| extras.light_target_subscription.is_none()).unwrap_or(true) {
            let subscription = crate::media::xaml_light::track_light_target(self);
            self.with_composition_extras(|extras| extras.light_target_subscription = Some(subscription));
        }
        added
    }

    /// Removes `id` from the ids of the XAML lights that target the element;
    /// returns whether it was there.
    pub(crate) fn remove_light_target_id(&self, id: &str) -> bool {
        self.read_composition_extras(|extras| extras.light_target_ids.iter().any(|existing| existing == id))
            .unwrap_or(false)
            && self.with_composition_extras(|extras| {
                extras.light_target_ids.retain(|existing| existing != id);
                true
            })
    }

    /// `UIElement.Translation`: the translation of the composition visual of the visual, applied
    /// after its render transform. Zero until set.
    ///
    /// As in WinUI the translation is not a styled property; setting it hands the `Translation`
    /// of the composition visual to this facade, which writes it whenever the visual is
    /// rendered again. While it was never set, the application may animate the `Translation` of
    /// the composition visual itself.
    pub fn translation(&self) -> crate::Vector3D {
        self.composition_translation().unwrap_or_default()
    }

    pub fn set_translation(&self, value: crate::Vector3D) {
        let old = self.translation();
        self.with_composition_extras(|extras| extras.translation = Some(value));
        self.invalidate_visual();
        if old != value {
            self.raise_direct_property_changed(Self::translation_property(), &old, &value);
        }
    }

    fn composition_translation(&self) -> Option<crate::Vector3D> {
        self.read_composition_extras(|extras| extras.translation).flatten()
    }

    /// Whether the composition visual of the visual is hidden by requests of the composition
    /// API (see [`add_composition_hide_request`](Self::add_composition_hide_request)).
    pub fn is_hidden_by_composition(&self) -> bool {
        self.read_composition_extras(|extras| extras.hide_requests > 0).unwrap_or(false)
    }

    /// Hides the composition visual of the visual until the matching
    /// [`remove_composition_hide_request`](Self::remove_composition_hide_request), whatever
    /// `IsVisible` says. Requests are counted. A connected animation hides its source and
    /// destination this way while it shows them through visual surfaces, which render a hidden
    /// visual (WinUI hides the element's visual with `SetConnectedAnimationRunning`).
    pub fn add_composition_hide_request(&self) {
        self.with_composition_extras(|extras| extras.hide_requests += 1);
        self.invalidate_visual();
    }

    /// Ends a request of [`add_composition_hide_request`](Self::add_composition_hide_request).
    pub fn remove_composition_hide_request(&self) {
        self.change_composition_extras(|extras| extras.hide_requests = extras.hide_requests.saturating_sub(1));
        self.invalidate_visual();
    }

    /// Whether presentation owners have suppressed this visual and its descendants in all
    /// render paths. Suppression changes no styled property, layout or logical visibility.
    /// It also applies to immediate bitmap and visual-brush captures; composition-only
    /// hiding remains independently capturable for connected animations.
    pub fn is_render_suppressed(&self) -> bool {
        self.read_composition_extras(|extras| extras.render_suppression_requests > 0)
            .unwrap_or(false)
    }

    /// Suppresses drawing until a matching remove request. Independent owners may hold
    /// requests concurrently. Input behavior is deliberately separate: editors that suppress
    /// presentation may additionally install their own temporary hit-test policy.
    pub fn add_render_suppression_request(&self) {
        let changed = self.with_composition_extras(|extras| {
            let previous = extras.render_suppression_requests;
            extras.render_suppression_requests = previous
                .checked_add(1)
                .expect("Render suppression request count exhausted");
            previous == 0
        });
        if changed {
            self.invalidate_visual();
        }
    }

    /// Releases one request without changing another owner's suppression or authored values.
    /// Removing a request when none is active is harmless.
    pub fn remove_render_suppression_request(&self) {
        if !self.is_render_suppressed() {
            return;
        }
        let changed = self.with_composition_extras(|extras| {
            extras.render_suppression_requests -= 1;
            extras.render_suppression_requests == 0
        });
        if changed {
            self.invalidate_visual();
        }
    }

    /// Whether the composition visual of the visual is kept visible by requests of the
    /// composition API although `IsVisible` is `false` (see
    /// [`add_composition_show_request`](Self::add_composition_show_request)).
    pub fn is_shown_by_composition(&self) -> bool {
        self.read_composition_extras(|extras| extras.show_requests > 0).unwrap_or(false)
    }

    /// Keeps the composition visual of the visual visible until the matching
    /// [`remove_composition_show_request`](Self::remove_composition_show_request), whatever
    /// `IsVisible` says. Requests are counted. An implicit hide animation keeps the element
    /// visible this way until it completes. A hide request wins over a show request.
    pub fn add_composition_show_request(&self) {
        self.with_composition_extras(|extras| extras.show_requests += 1);
        self.invalidate_visual();
    }

    /// Ends a request of [`add_composition_show_request`](Self::add_composition_show_request).
    pub fn remove_composition_show_request(&self) {
        self.change_composition_extras(|extras| extras.show_requests = extras.show_requests.saturating_sub(1));
        self.invalidate_visual();
    }

    /// A composition visual shown above the children of the visual and above its child
    /// composition visual (the layer of the connected animations of a top level, WinUI's
    /// `ConnectedAnimationRoot`).
    pub fn composition_layer_visual(&self) -> Option<Rc<CompositionVisual>> {
        self.read_composition_extras(|extras| extras.layer_visual.clone()).flatten()
    }

    /// Shows `visual` after the children of this visual until
    /// [`remove_composition_unloading_visual`](Self::remove_composition_unloading_visual): the
    /// image of a removed child that plays its implicit hide animation.
    pub(crate) fn add_composition_unloading_visual(&self, visual: Rc<CompositionVisual>) {
        self.with_composition_extras(|extras| extras.unloading_visuals.push(visual));
        if let Some(source) = self.presentation_source() {
            source.renderer().recalculate_children(self);
        }
    }

    /// Ends [`add_composition_unloading_visual`](Self::add_composition_unloading_visual).
    pub(crate) fn remove_composition_unloading_visual(&self, visual: &Rc<CompositionVisual>) {
        self.change_composition_extras(|extras| extras.unloading_visuals.retain(|v| !Rc::ptr_eq(v, visual)));
        if let Some(own) = self.own_composition_visual() {
            own.children().remove(visual);
        }
        if let Some(source) = self.presentation_source() {
            source.renderer().recalculate_children(self);
        }
    }

    /// Sets the visual of [`composition_layer_visual`](Self::composition_layer_visual).
    pub fn set_composition_layer_visual(&self, value: Option<Rc<CompositionVisual>>) {
        let old = self.with_composition_extras(|extras| std::mem::replace(&mut extras.layer_visual, value.clone()));
        if let (Some(old), Some(own)) = (&old, self.own_composition_visual()) {
            if !value.as_ref().is_some_and(|new| Rc::ptr_eq(new, old)) {
                own.children().remove(old);
            }
        }
        if let Some(source) = self.presentation_source() {
            source.renderer().recalculate_children(self);
        }
    }

    /// Makes sure the visual has a composition visual on `compositor` and
    /// returns it.
    pub fn attach_to_compositor(&self, compositor: &Rc<Compositor>) -> CompositionDrawListVisual {
        let had_one = self.has_composition_visual();
        let composition_visual = self.attach_to_compositor_core(compositor, None);
        self.composition_visual_handed_out.set(true);
        if !had_one {
            // A visual in a tree that had no composition visual: its renderer places the new
            // one in the composition tree (section 9.41).
            if let Some(source) = self.presentation_source() {
                source.renderer().composition_visual_materialized(self);
            }
        }
        self.ensure_parent_composition_visual();
        composition_visual
    }

    /// [`attach_to_compositor`](Self::attach_to_compositor) for the visual tree, which hands the
    /// composition visual to nothing. `renderer` is the renderer of the tree the visual is
    /// attached to.
    pub(crate) fn attach_to_compositor_core(
        &self,
        compositor: &Rc<Compositor>,
        renderer: Option<&dyn IRenderer>,
    ) -> CompositionDrawListVisual {
        let current = self.composition_visual.get();
        if self.composition_visual_departed.replace(false) {
            // It has the composition visual it left with again, or a new one.
            bump_composition_holder_version();
            // The visual left a tree and is in one again before the renderer released its
            // composition visual (section 9.32).
            if let Some(kept) = current {
                if Rc::ptr_eq(kept.compositor(), compositor)
                    && self.composition_visual_is_untouched(&kept)
                    && renderer.is_some_and(|renderer| renderer.rejoins_departed_composition_visual(&kept))
                {
                    crate::perf_count!(CompositionVisualsKept);
                    kept.set_departed(false);
                    self.rejoin_composition_tree(&kept);
                    return kept;
                }
                self.composition_visual.take();
                self.release_composition_visual(&kept);
            }
        } else if let Some(current) = current {
            if Rc::ptr_eq(current.compositor(), compositor) {
                return current;
            }
        }
        crate::perf_count!(CompositionVisualsCreated);
        let created = self.create_composition_visual(compositor);
        if self.was_seen_without_composition_visual() {
            // What is under the visual was shown without it until now: a hit test goes on to
            // find it where it was shown, until a frame is rendered with the new one.
            created.set_shown_through();
        }
        // What the visual drew as an item of the composition visual above is the drawing of
        // its own from here on (section 9.42): the caller has that one synchronized.
        if let Some(item) = self.composition_item.take() {
            item.set_departed(false);
            if let Some(data) = item.take_data() {
                if data.is_of(compositor) {
                    created.set_draw_list(Some(data));
                } else {
                    data.dispose();
                }
            }
        }
        self.composition_visual.set(Some(created.clone()));
        bump_composition_holder_version();
        self.composition_visual_handed_out.set(false);
        self.composition_visual_created(&created);
        created
    }

    /// A kept composition visual is still a child of the composition visual its visual was under.
    /// When the visual is now under another one, it leaves the old one here: the old parent may
    /// be synchronized after the new one, and a composition visual has one parent.
    fn rejoin_composition_tree(&self, kept: &CompositionDrawListVisual) {
        let Some(old_parent) = kept.parent() else { return };
        // The parent of a composition visual is the one of the nearest visual above that has a
        // composition visual (section 9.41).
        let same_parent = self
            .nearest_composition_ancestor()
            .and_then(|parent| parent.own_composition_visual())
            .is_some_and(|new_parent| Rc::ptr_eq(&new_parent, &old_parent));
        if !same_parent {
            old_parent.children().remove(kept);
        }
    }

    /// Whether any child has a non-default Z index.
    pub fn has_non_uniform_z_index_children(&self) -> bool {
        self.has_non_uniform_z_index_children.get()
    }

    /// Whether this control is attached to a visual root.
    #[inline]
    pub fn is_attached_to_visual_tree(&self) -> bool {
        self.presentation_source.borrow().is_some()
    }

    /// The control's parent visual.
    #[inline]
    pub fn visual_parent(&self) -> Option<Ref<Visual>> {
        self.visual_parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// The number of visuals attached to a rooted visual tree (on this
    /// thread; visuals are UI-thread-affine).
    pub(crate) fn rooted_visual_children_count() -> i32 {
        ROOTED_VISUAL_CHILDREN_COUNT.with(Cell::get)
    }

    // --- methods ----------------------------------------------------------

    /// Invalidates the visual and queues a repaint.
    pub fn invalidate_visual(&self) {
        if let Some(source) = self.presentation_source() {
            source.renderer().add_dirty(self);
        }
    }

    /// Invalidates the visual because its layout was invalidated: its drawing is recorded
    /// again, and its area is drawn again only when it draws something else than it drew. A
    /// FerroUI addition (docs/porting/desktop-performance.md, section 9.53): upstream
    /// invalidates the visual, whose area is then drawn again whatever it draws.
    pub(crate) fn invalidate_visual_for_layout(&self) {
        if let Some(source) = self.presentation_source() {
            source.renderer().add_dirty_for_layout(self);
        }
    }

    /// Invalidates the composition properties of this visual (its render transform and opacity)
    /// but not its drawing: the renderer synchronizes the properties without calling `render`
    /// again. A FerroUI addition (docs/porting/desktop-performance.md, section 9.5): upstream
    /// invalidates the visual, which records its drawing again.
    pub fn invalidate_composition_properties(&self) {
        if let Some(source) = self.presentation_source() {
            source.renderer().add_dirty_properties(self);
        }
    }

    /// Indicates that a property change should cause `invalidate_visual` to
    /// be called on objects of class `T`.
    ///
    /// When the value of such a property is a media object that can change
    /// (a brush, pen, geometry, transform, effect or image), its
    /// invalidation invalidates the visual as well.
    pub fn affects_render<T: ObjectType + Upcast<Visual>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            if can_value_affect_render(property.property_type()) {
                property.changed().subscribe(|e| {
                    if let Some(sender) = e.sender().downcast_ref::<T>() {
                        let visual: &Visual = sender.upcast();
                        visual.update_affects_render_subscription(e);
                        visual.invalidate_visual();
                    }
                }).detach();
            } else {
                property.changed().subscribe(|e| {
                    if let Some(sender) = e.sender().downcast_ref::<T>() {
                        let visual: &Visual = sender.upcast();
                        visual.invalidate_visual();
                    }
                }).detach();
            }
        }
    }

    /// Moves the invalidation subscription of a render-affecting property
    /// from its old value to its new value.
    fn update_affects_render_subscription(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let property = e.property();
        let old = {
            let mut subscriptions = self.affects_render_subscriptions.borrow_mut();
            let position = subscriptions.iter().position(|(p, _)| *p == property);
            position.map(|position| subscriptions.swap_remove(position))
        };
        if let Some((_, subscription)) = old {
            subscription.dispose();
        }

        // The handler is made for a value that can change only: an immutable
        // brush, the common value of a brush property, has nothing to
        // subscribe to.
        let Some(source) = affects_render_source(e.new_value()) else { return };
        let weak = self.to_ref().downgrade();
        let subscription = source.invalidated(Rc::new(move || {
            if let Some(target) = weak.upgrade() {
                target.invalidate_visual();
            }
        }));
        self.affects_render_subscriptions.borrow_mut().push((property, subscription));
    }

    /// Called when a visual's render transform property changes: while the
    /// visual is attached, the subscription to the transform's `changed`
    /// notification moves from the old value to the new one.
    fn render_transform_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(sender) = e.sender().downcast_ref::<Visual>() else { return };
        let (_, new_value) = e.get_old_and_new_value::<Option<Rc<dyn ITransform>>>();
        sender.has_render_transform.set(new_value.is_some());
        bump_geometry_version();

        if sender.visual_root().is_some() {

            // The old transform's subscription is the one held.
            let subscription = new_value.as_ref().and_then(|new_value| {
                let new_transform = new_value.as_any().downcast_ref::<Transform>()?;
                let weak = sender.to_ref().downgrade();
                Some(new_transform.changed(move || {
                    if let Some(visual) = weak.upgrade() {
                        visual.render_transform_value_changed();
                    }
                }))
            });
            sender.set_render_transform_subscription(subscription);

            sender.invalidate_composition_properties();
        }
    }

    /// Called when the render transform's `changed` notification is raised.
    fn render_transform_value_changed(&self) {
        bump_geometry_version();
        self.invalidate_composition_properties();
    }

    fn set_render_transform_subscription(&self, value: Option<Subscription>) {
        let previous = self.render_transform_subscription.replace(value);
        if let Some(previous) = previous {
            previous.dispose();
        }
    }

    /// Attaches or detaches the root visual of a presentation source.
    pub fn set_presentation_source_for_root_visual(&self, presentation_source: Option<Rc<dyn IPresentationSource>>) {
        let current = self.presentation_source();
        let same = match (&current, &presentation_source) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }

        if let Some(current) = current {
            assert!(
                presentation_source.is_none(),
                "Visual is already attached to a presentation source. Only one presentation source can be \
                 attached to a visual tree."
            );
            self.on_detached_from_visual_tree_core(&VisualTreeAttachmentEventArgs::new(None, current));
        }

        *self.presentation_source.borrow_mut() = presentation_source.clone();
        bump_composition_holder_version();

        if let Some(source) = presentation_source {
            let e = VisualTreeAttachmentEventArgs::new(None, source);
            self.on_attached_to_visual_tree_core(&e);
        }
    }

    fn update_is_effectively_visible(&self, parent_state: bool) {
        let is_effectively_visible = parent_state && self.is_visible();
        if self.is_effectively_visible.get() == is_effectively_visible {
            return;
        }
        self.is_effectively_visible.set(is_effectively_visible);
        if !self.is_effectively_visible_changed.is_empty() {
            for (_, handler) in self.is_effectively_visible_changed.snapshot().iter() {
                handler();
            }
        }
        // PERF-SENSITIVE: this is called on the entire hierarchy.
        if let Some(children) = self.visual_children.get() {
            for child in children.snapshot().iter() {
                child.update_is_effectively_visible(is_effectively_visible);
            }
        }
    }

    fn set_visual_parent(&self, value: Option<&Ref<Visual>>) {
        let old = self.visual_parent();
        if old.as_ref() == value {
            return;
        }
        *self.visual_parent.borrow_mut() = value.map(Ref::downgrade);
        bump_geometry_version();

        if let (Some(source), Some(old)) = (self.presentation_source(), &old) {
            let e = VisualTreeAttachmentEventArgs::new(Some(old.clone()), source);
            self.on_detached_from_visual_tree_core(&e);
            // A parent without a composition visual does not show its children itself: the
            // visual above it that does is told that one left (section 9.41).
            if old.is_attached_to_visual_tree() && !old.has_composition_visual() {
                e.presentation_source().renderer().recalculate_children(old);
            }
        }

        if let Some(parent) = value {
            if let Some(source) = parent.presentation_source() {
                let e = VisualTreeAttachmentEventArgs::new(Some(parent.clone()), source);
                self.on_attached_to_visual_tree_core(&e);
            }
        }

        self.on_visual_parent_changed(old.as_ref(), value);
    }

    fn visual_children_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Visual>>) {
        // What the composition visual of the visual shows below it is listed again (section
        // 9.42); a visual without one has the one above it told below.
        self.forget_composition_children();
        match e.action {
            NotifyCollectionChangedAction::Add => {
                let this = self.to_ref();
                for visual in e.new_items {
                    visual.set_visual_parent(Some(&this));
                }
            }
            NotifyCollectionChangedAction::Remove => {
                for visual in e.old_items {
                    visual.set_visual_parent(None);
                }
            }
            NotifyCollectionChangedAction::Replace => {
                for visual in e.old_items {
                    visual.set_visual_parent(None);
                }
                let this = self.to_ref();
                for visual in e.new_items {
                    visual.set_visual_parent(Some(&this));
                }
            }
            NotifyCollectionChangedAction::Move | NotifyCollectionChangedAction::Reset => {
                // The order of the children is the order of what shows them: a visual without
                // a composition visual has the one above it that has one told (section 9.41).
                if !self.has_composition_visual() {
                    if let Some(source) = self.presentation_source() {
                        source.renderer().recalculate_children(self);
                    }
                }
            }
        }
    }
}
