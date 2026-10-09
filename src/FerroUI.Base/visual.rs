use crate::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, ResetBehavior};
use crate::media::{
    can_value_affect_render, subscribe_invalidated, CacheMode, DrawingContext, FlowDirection, Geometry, IBrush, IEffect,
    ITransform, RenderOptions, TextOptions, Transform,
};
use crate::rendering::composition::{CompositionDrawListVisual, CompositionVisual, Compositor};
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::IPresentationSource;
use crate::utilities::HandlerList;
use crate::{
    ferro_class, ferro_property, instantiate, AttachedProperty, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, ObjectType, Rect, Ref,
    RelativePoint, StyledElement,
    StyledElementImpl, StyledElementImplExt, StyledProperty, StyledPropertyOptions, Upcast,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

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
    is_effectively_visible: Cell<bool>,
    has_non_uniform_z_index_children: Cell<bool>,
    visual_children: OnceCell<FerroList<Ref<Visual>>>,
    attached_to_visual_tree: HandlerList<dyn Fn(&VisualTreeAttachmentEventArgs)>,
    detached_from_visual_tree: HandlerList<dyn Fn(&VisualTreeAttachmentEventArgs)>,
    is_effectively_visible_changed: HandlerList<dyn Fn()>,
    render_options: Cell<RenderOptions>,
    text_options: Cell<TextOptions>,
    composition_visual: RefCell<Option<CompositionDrawListVisual>>,
    child_composition_visual: RefCell<Option<Rc<CompositionVisual>>>,
    /// The subscription to the `changed` notification of the render
    /// transform, held while the visual is attached to a visual tree.
    render_transform_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    affects_render_subscriptions: RefCell<Vec<(&'static FerroProperty, Rc<dyn IDisposable>)>>,
}

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
    }
}

thread_local! {
    static ROOTED_VISUAL_CHILDREN_COUNT: Cell<i32> = const { Cell::new(0) };
}
crate::ferro_class_info!(Visual { new: Visual::new });

crate::ferro_overrides! { impl FerroObjectImpl for Visual {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // Disable transitions until we're added to the visual tree.
        this.disable_transitions();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::is_visible_property().as_property() {
            let parent_state = this.visual_parent().is_none_or(|p| p.is_effectively_visible());
            this.update_is_effectively_visible(parent_state);
        } else if change.property() == Self::flow_direction_property().as_property() {
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
        ROOTED_VISUAL_CHILDREN_COUNT.with(|count| count.set(count.get() + 1));

        if let Some(render_transform) = this.render_transform() {
            if let Some(mutable_transform) = render_transform.as_mutable_transform() {
                let weak = this.to_ref().downgrade();
                let subscription = mutable_transform.changed(Rc::new(move || {
                    if let Some(visual) = weak.upgrade() {
                        visual.render_transform_value_changed();
                    }
                }));
                this.set_render_transform_subscription(Some(subscription));
            }
        }

        this.enable_transitions();
        let renderer = e.presentation_source().renderer();
        if let Some(compositor) = renderer.compositor() {
            this.attach_to_compositor(&compositor);
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
        if let Some(source) = this.presentation_source() {
            source.renderer().add_dirty(this);
        }
        *this.presentation_source.borrow_mut() = None;

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

    fn detach_from_compositor(this: &Self) {
        let composition_visual = this.composition_visual.borrow_mut().take();
        if let Some(composition_visual) = composition_visual {
            if let Some(child) = this.child_composition_visual() {
                composition_visual.children().remove(&child);
            }
            composition_visual.set_draw_list(None);
            composition_visual.set_opacity_mask(None);
            // The composition visual and its children refer to each other;
            // nothing collects such a cycle here, so the visual lets go of
            // its children. They are re-added when their visuals are
            // synchronized again.
            composition_visual.children().clear();
        }
    }

    fn synchronize_composition_child_visuals(this: &Self) {
        let Some(composition_visual) = this.composition_visual() else { return };
        let composition_children = composition_visual.children();
        let visual_children: Rc<Vec<Ref<Visual>>> =
            this.visual_children_snapshot().unwrap_or_else(|| Rc::new(Vec::new()));

        let mut sorted_children: Option<Vec<(Ref<Visual>, usize)>> = None;
        if this.has_non_uniform_z_index_children() && visual_children.len() > 1 {
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

        let mut expected_count = visual_children.len();
        if child_visual.is_some() {
            expected_count += 1;
        }

        let same = |a: &Rc<CompositionVisual>, b: Option<CompositionDrawListVisual>| {
            b.is_some_and(|b| Rc::ptr_eq(a, &b))
        };

        if composition_children.count() == expected_count {
            let mut mismatch = false;
            for c in 0..visual_children.len() {
                let expected = match &sorted_children {
                    Some(sorted) => sorted[c].0.composition_visual(),
                    None => visual_children[c].composition_visual(),
                };
                if !same(&composition_children.get(c), expected) {
                    mismatch = true;
                    break;
                }
            }

            if let Some(child_visual) = &child_visual {
                if !Rc::ptr_eq(&composition_children.get(composition_children.count() - 1), child_visual) {
                    mismatch = true;
                }
            }

            if !mismatch {
                return;
            }
        }

        composition_children.clear();
        match &sorted_children {
            Some(sorted) => {
                for (child, _) in sorted {
                    if let Some(composition_child) = child.composition_visual() {
                        composition_children.add((*composition_child).clone());
                    }
                }
            }
            None => {
                for child in visual_children.iter() {
                    if let Some(composition_child) = child.composition_visual() {
                        composition_children.add((*composition_child).clone());
                    }
                }
            }
        }

        if let Some(child_visual) = child_visual {
            composition_children.add(child_visual);
        }
    }

    fn synchronize_composition_properties(this: &Self) {
        let Some(comp) = this.composition_visual() else { return };
        let bounds = this.bounds();

        // TODO: Introduce a dirty mask like WPF has, so we don't overwrite properties every time
        comp.set_offset(crate::Vector3D::new(bounds.x, bounds.y, 0.0));
        comp.set_size(crate::Vector::new(bounds.width, bounds.height));
        comp.set_visible(this.is_visible());
        comp.set_opacity(this.opacity() as f32);
        comp.set_clip_to_bounds(this.clip_to_bounds());
        comp.set_clip(this.clip().and_then(|clip| clip.platform_impl()));

        let opacity_mask = this.opacity_mask();
        let same_mask = match (comp.opacity_mask(), &opacity_mask) {
            (Some(a), Some(b)) => *a == **b,
            (None, None) => true,
            _ => false,
        };
        if !same_mask {
            comp.set_opacity_mask(opacity_mask);
        }

        let cache_mode = this.cache_mode().map(|cache_mode| cache_mode.get_for_compositor(comp.compositor()));
        let same_cache_mode = match (comp.cache_mode(), &cache_mode) {
            (Some(a), Some(b)) => Rc::ptr_eq(&a, b),
            (None, None) => true,
            _ => false,
        };
        if !same_cache_mode {
            comp.set_cache_mode(cache_mode);
        }

        let effect = this.effect();
        if !crate::media::EffectExtensions::effect_equals(comp.effect().as_deref(), effect.as_deref()) {
            comp.set_effect(effect.as_ref().map(crate::media::EffectExtensions::to_immutable));
        }

        comp.set_render_options(this.render_options());
        comp.set_text_options(this.text_options());

        let mut render_transform = crate::Matrix::IDENTITY;

        if this.has_mirror_transform() {
            render_transform = crate::Matrix::new(-1.0, 0.0, 0.0, 1.0, bounds.width, 0.0);
        }

        if let Some(transform) = this.render_transform() {
            let origin = this.render_transform_origin().to_pixels(crate::Size::new(bounds.width, bounds.height));
            let offset = crate::Matrix::create_translation(origin.x, origin.y);
            render_transform = render_transform * ((-offset) * transform.value() * offset);
        }

        comp.set_transform_matrix(render_transform);
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
    fn static_constructor() {
        Self::affects_render::<Visual>(&[
            Self::bounds_property().as_property(),
            Self::clip_property().as_property(),
            Self::clip_to_bounds_property().as_property(),
            Self::is_visible_property().as_property(),
            Self::opacity_property().as_property(),
            Self::opacity_mask_property().as_property(),
            Self::effect_property().as_property(),
            Self::has_mirror_transform_property().as_property(),
        ]);
        Self::render_transform_property().changed().subscribe(Self::render_transform_changed);
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
        });
    }

    /// Creates the class data; see [`crate::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            bounds: Cell::new(Rect::default()),
            presentation_source: RefCell::new(None),
            visual_parent: RefCell::new(None),
            has_mirror_transform: Cell::new(false),
            is_effectively_visible: Cell::new(true),
            has_non_uniform_z_index_children: Cell::new(false),
            visual_children: OnceCell::new(),
            attached_to_visual_tree: HandlerList::new(),
            detached_from_visual_tree: HandlerList::new(),
            is_effectively_visible_changed: HandlerList::new(),
            render_options: Cell::new(RenderOptions::default()),
            text_options: Cell::new(TextOptions::default()),
            composition_visual: RefCell::new(None),
            child_composition_visual: RefCell::new(None),
            render_transform_subscription: RefCell::new(None),
            affects_render_subscriptions: RefCell::new(Vec::new()),
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
    ) -> Rc<dyn IDisposable> {
        let token = self.attached_to_visual_tree.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.attached_to_visual_tree.remove(token);
            }
        })
    }

    /// Raised when the control is detached from a rooted visual tree.
    pub fn detached_from_visual_tree(
        &self,
        handler: impl Fn(&VisualTreeAttachmentEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.detached_from_visual_tree.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.detached_from_visual_tree.remove(token);
            }
        })
    }

    /// Raised when `is_effectively_visible` changes.
    pub fn is_effectively_visible_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.is_effectively_visible_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.is_effectively_visible_changed.remove(token);
            }
        })
    }

    // --- properties -------------------------------------------------------

    /// The bounds of the control relative to its parent.
    #[inline]
    pub fn bounds(&self) -> Rect {
        self.bounds.get()
    }

    /// Sets the bounds of the control. Called by the layout system.
    pub fn set_bounds(&self, value: Rect) {
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
        self.get_value(Self::is_visible_property())
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
    pub fn composition_visual(&self) -> Option<CompositionDrawListVisual> {
        self.composition_visual.borrow().clone()
    }

    /// The composition visual shown as the last child of the composition
    /// visual of this visual (see `ElementComposition`).
    pub fn child_composition_visual(&self) -> Option<Rc<CompositionVisual>> {
        self.child_composition_visual.borrow().clone()
    }

    pub fn set_child_composition_visual(&self, value: Option<Rc<CompositionVisual>>) {
        *self.child_composition_visual.borrow_mut() = value;
    }

    /// Makes sure the visual has a composition visual on `compositor` and
    /// returns it.
    pub fn attach_to_compositor(&self, compositor: &Rc<Compositor>) -> CompositionDrawListVisual {
        let current = self.composition_visual();
        match current {
            Some(current) if Rc::ptr_eq(current.compositor(), compositor) => current,
            _ => {
                let created = self.create_composition_visual(compositor);
                *self.composition_visual.borrow_mut() = Some(created.clone());
                created
            }
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
                });
            } else {
                property.changed().subscribe(|e| {
                    if let Some(sender) = e.sender().downcast_ref::<T>() {
                        let visual: &Visual = sender.upcast();
                        visual.invalidate_visual();
                    }
                });
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

        let weak = self.to_ref().downgrade();
        let subscription = subscribe_invalidated(
            e.new_value(),
            Rc::new(move || {
                if let Some(target) = weak.upgrade() {
                    target.invalidate_visual();
                }
            }),
        );
        if let Some(subscription) = subscription {
            self.affects_render_subscriptions.borrow_mut().push((property, subscription));
        }
    }

    /// Called when a visual's render transform property changes: while the
    /// visual is attached, the subscription to the transform's `changed`
    /// notification moves from the old value to the new one.
    fn render_transform_changed(e: &FerroPropertyChangedEventArgs<'_>) {
        let Some(sender) = e.sender().downcast_ref::<Visual>() else { return };

        if sender.visual_root().is_some() {
            let (_, new_value) = e.get_old_and_new_value::<Option<Rc<dyn ITransform>>>();

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

            sender.invalidate_visual();
        }
    }

    /// Called when the render transform's `changed` notification is raised.
    fn render_transform_value_changed(&self) {
        self.invalidate_visual();
    }

    fn set_render_transform_subscription(&self, value: Option<Rc<dyn IDisposable>>) {
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

        if let (Some(source), Some(old)) = (self.presentation_source(), &old) {
            let e = VisualTreeAttachmentEventArgs::new(Some(old.clone()), source);
            self.on_detached_from_visual_tree_core(&e);
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
            NotifyCollectionChangedAction::Move | NotifyCollectionChangedAction::Reset => {}
        }
    }
}
