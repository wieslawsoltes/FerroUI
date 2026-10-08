use crate::collections::NotifyCollectionChangedAction;
use crate::media::effects::{EffectExtensions, IEffect};
use crate::media::{
    BoxShadows, Drawing, DrawingCollection, DrawingContext, DrawingImpl, Geometry, GeometryDrawing, GlyphRun,
    GlyphRunDrawing, IBrush, IDrawingContextCore, IPen, MatrixTransform, PlatformGeometry, PushedState,
    RectangleGeometry, RenderOptions, TextOptions, Transform,
};
use crate::platform::{self, IBitmapImpl, IGeometryImpl, IPlatformRenderInterface};
use crate::reactive::IDisposable;
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    Matrix, Nullable, Point, Rect, Ref, RoundedRect, StyledProperty,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

type ChildSubscription = (Ref<Drawing>, Rc<dyn IDisposable>);

/// Represents a collection of drawings that can be operated upon as a single
/// drawing.
#[repr(C)]
pub struct DrawingGroup {
    base: Drawing,
    children: RefCell<DrawingCollection>,
    children_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    child_subscriptions: RefCell<Vec<ChildSubscription>>,
    render_options: Cell<Option<RenderOptions>>,
    text_options: Cell<Option<TextOptions>>,
    effect_bounds: Cell<Option<Rect>>,
}

ferro_class!(DrawingGroup: Drawing);
crate::ferro_class_info!(DrawingGroup { new: DrawingGroup::new });

impl FerroObjectImpl for DrawingGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let children = this.children();
        this.subscribe_collection(&children);
    }
}

impl DrawingImpl for DrawingGroup {
    fn draw_core(this: &Self, context: &mut DrawingContext) {
        // Compute local bounds from children only when the effect bounds
        // are not explicitly set. The effect bounds store content bounds
        // (pre-inflation); pushing the effect handles inflation.
        let children = this.children();
        let content_bounds = match this.effect_bounds() {
            Some(bounds) => bounds,
            None => {
                let mut bounds = Rect::default();
                for drawing in children.iter() {
                    bounds = bounds.union(drawing.get_bounds());
                }
                bounds
            }
        };

        let effect = this.effect();
        let effect_bounds = match &effect {
            Some(effect) => {
                content_bounds.inflate_thickness(EffectExtensions::get_effect_output_padding(Some(&**effect)))
            }
            None => content_bounds,
        };

        let s_transform =
            context.push_transform(this.transform().map(|transform| transform.value()).unwrap_or(Matrix::IDENTITY));
        let s_opacity = context.push_opacity(this.opacity());
        let s_clip = match this.clip_geometry() {
            Some(clip) => context.push_geometry_clip(&clip),
            None => PushedState::NONE,
        };
        let s_opacity_mask = match this.opacity_mask() {
            Some(mask) => context.push_opacity_mask(&mask, effect_bounds),
            None => PushedState::NONE,
        };
        let s_render_options = match this.render_options() {
            Some(render_options) => context.push_render_options(render_options),
            None => PushedState::NONE,
        };
        let s_text_options = match this.text_options() {
            Some(text_options) => context.push_text_options(text_options),
            None => PushedState::NONE,
        };
        let s_effect = match &effect {
            Some(effect) => context.push_effect(effect, content_bounds),
            None => PushedState::NONE,
        };

        for drawing in children.iter() {
            drawing.draw(context);
        }

        context.pop(s_effect);
        context.pop(s_text_options);
        context.pop(s_render_options);
        context.pop(s_opacity_mask);
        context.pop(s_clip);
        context.pop(s_opacity);
        context.pop(s_transform);
    }

    fn get_bounds(this: &Self) -> Rect {
        let mut rect = Rect::default();

        for drawing in this.children().iter() {
            rect = rect.union(drawing.get_bounds());
        }

        // Note: the effect is intentionally not reflected here. The bounds
        // are the content/geometric bounds only. Effects render additively
        // outside the content bounds and must not shift the coordinate
        // origin (e.g. when used inside a drawing image).
        if let Some(transform) = this.transform() {
            rect = rect.transform_to_aabb(transform.value());
        }

        rect
    }
}

crate::ferro_properties! { impl DrawingGroup {
    ferro_property!(
        /// Defines the `Opacity` property.
        pub fn opacity_property() -> StyledProperty<f64> {
            FerroProperty::register::<DrawingGroup, _>("Opacity", 1.0)
        }
    );

    ferro_property!(
        /// Defines the `Transform` property.
        pub fn transform_property() -> StyledProperty<Option<Ref<Transform>>> {
            FerroProperty::register::<DrawingGroup, _>("Transform", None)
        }
    );

    ferro_property!(
        /// Defines the `ClipGeometry` property.
        pub fn clip_geometry_property() -> StyledProperty<Option<Ref<Geometry>>> {
            FerroProperty::register::<DrawingGroup, _>("ClipGeometry", None)
        }
    );

    ferro_property!(
        /// Defines the `OpacityMask` property.
        pub fn opacity_mask_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<DrawingGroup, _>("OpacityMask", None)
        }
    );

    ferro_property!(
        /// Defines the `Effect` property.
        pub fn effect_property() -> StyledProperty<Option<Rc<dyn IEffect>>> {
            FerroProperty::register::<DrawingGroup, _>("Effect", None)
        }
    );

    ferro_property!(
        /// Defines the `Children` property.
        pub fn children_property() -> DirectProperty<DrawingGroup, DrawingCollection> {
            FerroProperty::register_direct::<DrawingGroup, _>(
                "Children",
                |o| o.children(),
                Some(|o, v| o.set_children(v)),
                DrawingCollection::new(),
            )
        }
    );
} }

impl DrawingGroup {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Drawing::construct(),
            children: RefCell::new(DrawingCollection::new()),
            children_subscription: RefCell::new(None),
            child_subscriptions: RefCell::new(Vec::new()),
            render_options: Cell::new(None),
            text_options: Cell::new(None),
            effect_bounds: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The opacity the children are drawn with.
    pub fn opacity(&self) -> f64 {
        self.get_value(Self::opacity_property())
    }

    pub fn set_opacity(&self, value: f64) {
        self.set_value(Self::opacity_property(), value)
    }

    /// The transform applied to the children.
    pub fn transform(&self) -> Option<Ref<Transform>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: impl Into<Nullable<Transform>>) {
        self.set_value(Self::transform_property(), value.into().0)
    }

    /// The geometry the children are clipped to.
    pub fn clip_geometry(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::clip_geometry_property())
    }

    pub fn set_clip_geometry(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::clip_geometry_property(), value.into().0)
    }

    /// The opacity mask the children are drawn through.
    pub fn opacity_mask(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::opacity_mask_property())
    }

    pub fn set_opacity_mask(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::opacity_mask_property(), value)
    }

    /// The effect applied to the children.
    pub fn effect(&self) -> Option<Rc<dyn IEffect>> {
        self.get_value(Self::effect_property())
    }

    pub fn set_effect(&self, value: Option<Rc<dyn IEffect>>) {
        self.set_value(Self::effect_property(), value)
    }

    /// The render options the children are drawn with. Set by the drawing
    /// context that records into a group.
    pub(crate) fn render_options(&self) -> Option<RenderOptions> {
        self.render_options.get()
    }

    pub(crate) fn set_render_options(&self, value: Option<RenderOptions>) {
        self.render_options.set(value)
    }

    /// The text options the children are drawn with. Set by the drawing
    /// context that records into a group.
    pub(crate) fn text_options(&self) -> Option<TextOptions> {
        self.text_options.get()
    }

    pub(crate) fn set_text_options(&self, value: Option<TextOptions>) {
        self.text_options.set(value)
    }

    /// The bounds of the content the effect applies to; computed from the
    /// children when not set. Set by the drawing context that records into a
    /// group.
    pub(crate) fn effect_bounds(&self) -> Option<Rect> {
        self.effect_bounds.get()
    }

    pub(crate) fn set_effect_bounds(&self, value: Option<Rect>) {
        self.effect_bounds.set(value)
    }

    /// The collection that contains the child drawings.
    pub fn children(&self) -> DrawingCollection {
        self.children.borrow().clone()
    }

    pub fn set_children(&self, value: DrawingCollection) {
        if self.children.borrow().ptr_eq(&value) {
            return;
        }

        if let Some(subscription) = self.children_subscription.take() {
            subscription.dispose();
        }
        let old_subscriptions = std::mem::take(&mut *self.child_subscriptions.borrow_mut());
        for (_, subscription) in old_subscriptions {
            subscription.dispose();
        }

        self.set_and_raise(Self::children_property(), &self.children, value.clone());

        self.subscribe_collection(&value);
        for child in value.iter() {
            self.subscribe_child(&child);
        }
    }

    /// Opens the group for populating: what is drawn on the returned
    /// context replaces the children of the group when the context is
    /// disposed (or dropped).
    ///
    /// Panics when no platform render interface is registered.
    pub fn open(&self) -> DrawingContext<'static> {
        DrawingContext::owned(Box::new(DrawingGroupDrawingContext::new(self.to_ref())))
    }

    fn subscribe_collection(&self, children: &DrawingCollection) {
        let weak = self.to_ref().downgrade();
        let subscription = children.collection_changed(move |e| {
            let Some(this) = weak.upgrade() else { return };

            if e.action == NotifyCollectionChangedAction::Reset {
                panic!("Collection reset is not supported.");
            }

            for child in e.old_items {
                this.unsubscribe_child(child);
            }

            for child in e.new_items {
                this.subscribe_child(child);
            }

            this.raise_invalidated();
        });
        *self.children_subscription.borrow_mut() = Some(subscription);
    }

    fn subscribe_child(&self, child: &Ref<Drawing>) {
        let weak = self.to_ref().downgrade();
        let subscription = child.invalidated(move || {
            if let Some(this) = weak.upgrade() {
                this.raise_invalidated();
            }
        });
        self.child_subscriptions.borrow_mut().push((child.clone(), subscription));
    }

    fn unsubscribe_child(&self, child: &Ref<Drawing>) {
        let position = self.child_subscriptions.borrow().iter().position(|(c, _)| c.ptr_eq(child));
        if let Some(position) = position {
            let (_, subscription) = self.child_subscriptions.borrow_mut().remove(position);
            subscription.dispose();
        }
    }
}

/// The drawing context core that records what is drawn as the children of
/// a [`DrawingGroup`].
struct DrawingGroupDrawingContext {
    drawing_group: Ref<DrawingGroup>,
    platform_render_interface: Rc<dyn IPlatformRenderInterface>,

    disposed: bool,

    // Root drawing created by this drawing context.
    //
    // If there is only a single child of the root drawing group,
    // `root_drawing` references the single child, and the root
    // `current_drawing_group` value is `None`. Otherwise, `root_drawing`
    // references the root drawing group, and is the same value as the root
    // `current_drawing_group`.
    //
    // Either way, `root_drawing` always references the root drawing.
    root_drawing: Option<Ref<Drawing>>,

    // Current drawing group that new children are added to.
    current_drawing_group: Option<Ref<DrawingGroup>>,

    // Previous values of `current_drawing_group`.
    previous_drawing_group_stack: Option<Vec<Option<Ref<DrawingGroup>>>>,
}

impl DrawingGroupDrawingContext {
    fn new(drawing_group: Ref<DrawingGroup>) -> Self {
        Self {
            drawing_group,
            platform_render_interface: platform::render_interface(),
            disposed: false,
            root_drawing: None,
            current_drawing_group: None,
            previous_drawing_group_stack: None,
        }
    }

    fn pop(&mut self) {
        // Verify that pop hasn't been called too many times.
        let Some(previous) = self.previous_drawing_group_stack.as_mut().and_then(Vec::pop) else {
            panic!("DrawingGroupStack count missmatch.");
        };

        // Restore the previous value of the current drawing group.
        self.current_drawing_group = previous;
    }

    fn push_new_drawing_group(&mut self) -> Ref<DrawingGroup> {
        // Instantiate a new drawing group.
        let drawing_group = DrawingGroup::new();

        // Add it to the drawing graph, like any other drawing.
        self.add_drawing(drawing_group.clone().upcast());

        // Save the previous `current_drawing_group` value. The stack is
        // allocated lazily because many uses have a depth of one.
        //
        // If this is the first call, the value of `current_drawing_group`
        // is `None` because `add_drawing` doesn't create a current group
        // for the first drawing. Having `None` on the stack is valid, and
        // simply denotes that this new drawing group is the first child in
        // the root drawing group. It is also possible for the first value
        // on the stack to be `Some`, which means that the root drawing
        // group has other children.
        let previous = self.current_drawing_group.take();
        self.previous_drawing_group_stack.get_or_insert_with(|| Vec::with_capacity(2)).push(previous);

        // Set this drawing group as the current one so that subsequent
        // drawings are added as its children until pop is called.
        self.current_drawing_group = Some(drawing_group.clone());

        drawing_group
    }

    fn add_new_geometry_drawing(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: Ref<Geometry>,
    ) {
        // Instantiate the geometry drawing.
        let geometry_drawing = GeometryDrawing::new();
        geometry_drawing.set_brush(brush.cloned());
        geometry_drawing.set_pen(pen.cloned());
        geometry_drawing.set_geometry(geometry);

        // Add it to the drawing graph.
        self.add_drawing(geometry_drawing.upcast());
    }

    fn add_drawing(&mut self, new_drawing: Ref<Drawing>) {
        match (&self.root_drawing, &self.current_drawing_group) {
            (None, Some(_)) => {
                panic!("When a DrawingGroup is set, it should be made the root if a root drawing didnt exist.");
            }
            (None, None) => {
                // If this is the first drawing being added, avoid creating
                // a drawing group and set this drawing as the root drawing.
                // This optimizes the common case where only a single child
                // exists in the root drawing group.
                self.root_drawing = Some(new_drawing);
            }
            (Some(root_drawing), None) => {
                // When the second drawing is added at the root level, set a
                // drawing group as the root and add both drawings to it.
                let current_drawing_group = DrawingGroup::new();

                // Add both children.
                let children = current_drawing_group.children();
                children.add(root_drawing.clone());
                children.add(new_drawing);

                // Set the new drawing group as the current.
                self.root_drawing = Some(current_drawing_group.clone().upcast());
                self.current_drawing_group = Some(current_drawing_group);
            }
            (Some(_), Some(current_drawing_group)) => {
                // If there already is a current drawing group, then simply
                // add the new drawing to it.
                current_drawing_group.children().add(new_drawing);
            }
        }
    }
}

impl IDrawingContextCore for DrawingGroupDrawingContext {
    fn draw_line_core(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        // Instantiate the geometry.
        let geometry = self.platform_render_interface.create_line_geometry(p1, p2);

        // Add the drawing to the drawing graph.
        self.add_new_geometry_drawing(None, Some(pen), PlatformGeometry::new(geometry).upcast());
    }

    fn draw_geometry_impl_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Arc<dyn IGeometryImpl>,
    ) {
        if brush.is_none() && pen.is_none() {
            return;
        }

        self.add_new_geometry_drawing(brush, pen, PlatformGeometry::new(geometry.clone()).upcast());
    }

    fn draw_rectangle_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
        // Instantiate the geometry.
        let geometry = self.platform_render_interface.create_rectangle_geometry(rrect.rect);

        // Add the drawing to the drawing graph.
        self.add_new_geometry_drawing(brush, pen, PlatformGeometry::new(geometry).upcast());
    }

    fn draw_ellipse_core(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        if brush.is_none() && pen.is_none() {
            return;
        }

        // Instantiate the geometry.
        let geometry = self.platform_render_interface.create_ellipse_geometry(rect);

        // Add the drawing to the drawing graph.
        self.add_new_geometry_drawing(brush, pen, PlatformGeometry::new(geometry).upcast());
    }

    fn draw_bitmap(&mut self, _source: &std::sync::Arc<crate::platform::SharedBitmapImpl>, _opacity: f64, _source_rect: Rect, _dest_rect: Rect) {
        panic!("Drawing a bitmap into a DrawingGroup is not implemented.");
    }

    fn custom(&mut self, _custom: &Rc<dyn ICustomDrawOperation>) {
        panic!("Custom draw operations are not supported when drawing into a DrawingGroup.");
    }

    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        let Some(foreground) = foreground else {
            return;
        };

        let glyph_run_drawing = GlyphRunDrawing::new();
        glyph_run_drawing.set_foreground(Some(foreground.clone()));
        glyph_run_drawing.set_glyph_run(Some(glyph_run.clone()));

        // Add the drawing to the drawing graph.
        self.add_drawing(glyph_run_drawing.upcast());
    }

    fn push_clip_core(&mut self, rect: Rect) {
        let drawing_group = self.push_new_drawing_group();

        drawing_group.set_clip_geometry(RectangleGeometry::with_rect(rect).upcast::<Geometry>());
    }

    fn push_rounded_clip_core(&mut self, _rect: RoundedRect) {
        panic!("Pushing a rounded clip into a DrawingGroup is not implemented.");
    }

    fn push_geometry_clip_core(&mut self, clip: &Ref<Geometry>) {
        let drawing_group = self.push_new_drawing_group();

        drawing_group.set_clip_geometry(clip.clone());
    }

    fn push_opacity_core(&mut self, opacity: f64) {
        let drawing_group = self.push_new_drawing_group();

        drawing_group.set_opacity(opacity);
    }

    fn push_opacity_mask_core(&mut self, mask: &Rc<dyn IBrush>, _bounds: Rect) {
        let drawing_group = self.push_new_drawing_group();

        drawing_group.set_opacity_mask(Some(mask.clone()));
    }

    fn push_transform_core(&mut self, matrix: Matrix) {
        // Instantiate a new drawing group and set it as the current one.
        let drawing_group = self.push_new_drawing_group();

        // Set the transform on the new drawing group.
        drawing_group.set_transform(MatrixTransform::with_matrix(matrix).upcast::<Transform>());
    }

    fn push_render_options_core(&mut self, render_options: RenderOptions) {
        // Instantiate a new drawing group and set it as the current one.
        let drawing_group = self.push_new_drawing_group();

        // Set the render options on the new drawing group.
        drawing_group.set_render_options(Some(render_options));
    }

    fn push_text_options_core(&mut self, text_options: TextOptions) {
        // Instantiate a new drawing group and set it as the current one.
        let drawing_group = self.push_new_drawing_group();

        // Set the text options on the new drawing group.
        drawing_group.set_text_options(Some(text_options));
    }

    fn push_effect_core(&mut self, effect: &Rc<dyn IEffect>, bounds: Rect) {
        // Instantiate a new drawing group and set it as the current one.
        let drawing_group = self.push_new_drawing_group();

        // Set the effect on the new drawing group.
        drawing_group.set_effect(Some(effect.clone()));
        drawing_group.set_effect_bounds(Some(bounds));
    }

    fn pop_clip_core(&mut self) {
        self.pop();
    }

    fn pop_geometry_clip_core(&mut self) {
        self.pop();
    }

    fn pop_opacity_core(&mut self) {
        self.pop();
    }

    fn pop_opacity_mask_core(&mut self) {
        self.pop();
    }

    fn pop_transform_core(&mut self) {
        self.pop();
    }

    fn pop_render_options_core(&mut self) {
        self.pop();
    }

    fn pop_text_options_core(&mut self) {
        self.pop();
    }

    fn pop_effect_core(&mut self) {
        self.pop();
    }

    fn dispose_core(&mut self) {
        // Dispose may be called multiple times without failing.
        if self.disposed {
            return;
        }

        // Match any outstanding push calls with a pop.
        let stack_count = self.previous_drawing_group_stack.as_ref().map_or(0, Vec::len);
        for _ in 0..stack_count {
            self.pop();
        }

        // Close with the root drawing group's children.
        let root_children = match &self.current_drawing_group {
            // If a root drawing group was created because multiple elements
            // exist at the root level, provide its children collection
            // directly.
            Some(current_drawing_group) => current_drawing_group.children(),
            // Create a new collection if no root drawing group was created
            // because the root level only contained a single child. The
            // collection is needed because opening a group always replaces
            // its children collection.
            None => {
                let root_children = DrawingCollection::new();
                if let Some(root_drawing) = &self.root_drawing {
                    root_children.add(root_drawing.clone());
                }
                root_children
            }
        };

        self.drawing_group.set_children(root_children);

        self.disposed = true;
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream tests of this class record through a
    // drawing context.
    use super::*;
    use crate::media::effects::BlurEffect;
    use crate::media::{Brushes, DrawingImage, GeometryDrawing, IImage, ImageDrawing, TranslateTransform};
    use crate::{PixelRect, Size};

    fn counter(drawing: &Drawing) -> Rc<Cell<i32>> {
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        drawing.invalidated(move || c.set(c.get() + 1));
        count
    }

    #[test]
    fn children_changes_and_child_changes_raise_invalidated() {
        let group = DrawingGroup::new();
        let count = counter(&group);

        let child = ImageDrawing::new();
        group.children().add(child.clone().upcast());
        assert_eq!(1, count.get());

        child.set_rect(Rect::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(2, count.get());
        assert_eq!(Rect::new(1.0, 2.0, 3.0, 4.0), group.get_bounds());

        group.children().remove_at(0);
        assert_eq!(3, count.get());
        child.set_rect(Rect::default());
        assert_eq!(3, count.get());
    }

    #[test]
    fn replaced_children_collection_is_tracked() {
        let group = DrawingGroup::new();
        let old = group.children();
        let child = ImageDrawing::new();
        let replacement = DrawingCollection::from_items([child.clone().upcast()]);
        let count = counter(&group);

        group.set_children(replacement.clone());
        assert_eq!(1, count.get());
        assert!(group.get_direct_value(DrawingGroup::children_property()).ptr_eq(&replacement));

        child.set_rect(Rect::new(0.0, 0.0, 5.0, 5.0));
        assert_eq!(2, count.get());

        old.add(ImageDrawing::new().upcast());
        assert_eq!(2, count.get());

        group.set_children(replacement);
        assert_eq!(2, count.get());
    }

    #[test]
    #[should_panic(expected = "Collection reset is not supported.")]
    fn resetting_the_children_is_not_supported() {
        let group = DrawingGroup::new();
        let children = group.children();
        children.set_reset_behavior(crate::media::ResetBehavior::Reset);
        children.add(ImageDrawing::new().upcast());
        children.clear();
    }

    #[test]
    fn nested_value_changes_raise_invalidated() {
        let group = DrawingGroup::new();
        let transform = TranslateTransform::new();
        let effect = BlurEffect::new();
        group.set_transform(transform.clone().upcast::<Transform>());
        group.set_effect(Some(effect.clone().into()));
        let count = counter(&group);

        transform.set_x(10.0);
        assert_eq!(1, count.get());
        effect.set_radius(1.0);
        assert_eq!(2, count.get());

        group.set_transform(None);
        group.set_effect(None);
        assert_eq!(4, count.get());
        transform.set_x(20.0);
        effect.set_radius(2.0);
        assert_eq!(4, count.get());

        // A brush is not baked into the recording: only its replacement
        // invalidates the drawing.
        let drawing = GeometryDrawing::new();
        let count = counter(&drawing);
        let brush = crate::media::SolidColorBrush::new();
        drawing.set_brush(Some(brush.clone().into()));
        assert_eq!(1, count.get());
        brush.set_opacity(0.5);
        assert_eq!(1, count.get());
        drawing.set_brush(Some(Brushes::red() as Rc<dyn IBrush>));
        assert_eq!(2, count.get());
    }

    #[test]
    fn bounds_apply_the_transform() {
        let group = DrawingGroup::new();
        let a = ImageDrawing::new();
        a.set_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = ImageDrawing::new();
        b.set_rect(Rect::new(20.0, 20.0, 10.0, 10.0));
        group.children().add_range([a.upcast(), b.upcast()]);
        assert_eq!(Rect::new(0.0, 0.0, 30.0, 30.0), group.get_bounds());

        let transform = TranslateTransform::new();
        transform.set_x(5.0);
        transform.set_y(7.0);
        group.set_transform(transform.upcast::<Transform>());
        assert_eq!(Rect::new(5.0, 7.0, 30.0, 30.0), group.get_bounds());
        assert_eq!(None, group.effect_bounds());
        assert_eq!(None, group.render_options());
    }

    #[test]
    fn drawing_image_follows_its_drawing() {
        let drawing = ImageDrawing::new();
        drawing.set_rect(Rect::new(0.0, 0.0, 16.0, 8.0));
        let image = DrawingImage::with_drawing(drawing.clone().upcast::<Drawing>());
        assert_eq!(Size::new(16.0, 8.0), image.size());

        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        image.invalidated(move || c.set(c.get() + 1));

        drawing.set_rect(Rect::new(0.0, 0.0, 4.0, 4.0));
        assert_eq!(1, count.get());
        assert_eq!(Size::new(4.0, 4.0), image.size());

        image.set_viewbox(Some(Rect::new(1.0, 1.0, 2.0, 3.0)));
        assert_eq!(2, count.get());
        assert_eq!(Size::new(2.0, 3.0), image.size());

        // A nested drawing image invalidates the drawing that draws it.
        let outer = ImageDrawing::new();
        let handle: Rc<dyn IImage> = image.clone().into();
        outer.set_image_source(Some(handle.clone()));
        let outer_count = counter(&outer);
        image.set_viewbox(None);
        assert_eq!(1, outer_count.get());
        assert!(handle.as_bitmap().is_none());
        assert!(handle.as_object().unwrap().is::<DrawingImage>());

        image.set_drawing(None);
        let before = count.get();
        drawing.set_rect(Rect::default());
        assert_eq!(before, count.get());
        assert_eq!(Size::default(), image.size());
        let _ = PixelRect::default();
    }

    // --- from upstream: DrawingGroupTests --------------------------------

    use crate::media::immutable::ImmutableSolidColorBrush;
    use crate::media::{Colors, PlatformDrawingContext};
    use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl, MockPlatformRenderInterface};

    fn solid(color: crate::media::Color) -> Rc<dyn IBrush> {
        Rc::new(ImmutableSolidColorBrush::new(color))
    }

    fn blur(radius: f64) -> Rc<dyn IEffect> {
        let effect = BlurEffect::new();
        effect.set_radius(radius);
        effect.into()
    }

    fn rectangle_drawing(brush: Rc<dyn IBrush>, rect: Rect) -> Ref<Drawing> {
        let drawing = GeometryDrawing::new();
        drawing.set_brush(Some(brush));
        drawing.set_geometry(RectangleGeometry::with_rect(rect).upcast::<Geometry>());
        drawing.upcast()
    }

    /// A drawing context core that captures the bounds of the pushed effect
    /// and opacity mask.
    #[derive(Default)]
    struct MockDrawingContext {
        effect: Option<Rc<dyn IEffect>>,
        bounds: Rect,
        opacity_mask_bounds: Rect,
    }

    impl IDrawingContextCore for MockDrawingContext {
        fn draw_line_core(&mut self, _: &Rc<dyn IPen>, _: Point, _: Point) {}
        fn draw_geometry_impl_core(&mut self, _: Option<&Rc<dyn IBrush>>, _: Option<&Rc<dyn IPen>>, _: &Arc<dyn IGeometryImpl>) {
        }
        fn draw_rectangle_core(&mut self, _: Option<&Rc<dyn IBrush>>, _: Option<&Rc<dyn IPen>>, _: RoundedRect, _: &BoxShadows) {
        }
        fn draw_ellipse_core(&mut self, _: Option<&Rc<dyn IBrush>>, _: Option<&Rc<dyn IPen>>, _: Rect) {}
        fn draw_bitmap(&mut self, _: &std::sync::Arc<crate::platform::SharedBitmapImpl>, _: f64, _: Rect, _: Rect) {}
        fn custom(&mut self, _: &Rc<dyn ICustomDrawOperation>) {}
        fn draw_glyph_run(&mut self, _: Option<&Rc<dyn IBrush>>, _: &Rc<GlyphRun>) {}
        fn push_clip_core(&mut self, _: Rect) {}
        fn push_rounded_clip_core(&mut self, _: RoundedRect) {}
        fn push_geometry_clip_core(&mut self, _: &Ref<Geometry>) {}
        fn push_opacity_core(&mut self, _: f64) {}
        fn push_opacity_mask_core(&mut self, _: &Rc<dyn IBrush>, bounds: Rect) {
            self.opacity_mask_bounds = bounds;
        }
        fn push_transform_core(&mut self, _: Matrix) {}
        fn push_render_options_core(&mut self, _: RenderOptions) {}
        fn push_text_options_core(&mut self, _: TextOptions) {}
        fn push_effect_core(&mut self, effect: &Rc<dyn IEffect>, bounds: Rect) {
            self.effect = Some(effect.clone());
            self.bounds = bounds;
        }
        fn pop_clip_core(&mut self) {}
        fn pop_geometry_clip_core(&mut self) {}
        fn pop_opacity_core(&mut self) {}
        fn pop_opacity_mask_core(&mut self) {}
        fn pop_transform_core(&mut self) {}
        fn pop_render_options_core(&mut self) {}
        fn pop_text_options_core(&mut self) {}
        fn pop_effect_core(&mut self) {}
        fn dispose_core(&mut self) {}
    }

    #[test]
    fn push_effect_should_store_provided_bounds() {
        let (scope, _) = MockPlatformRenderInterface::install();
        let group = DrawingGroup::new();
        let effect = blur(10.0);
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);

        {
            let mut context = group.open();
            let state = context.push_effect(&effect, bounds);
            context.draw_rectangle(
                Some(&solid(Colors::RED)),
                None,
                Rect::new(20.0, 20.0, 50.0, 50.0),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
            context.pop(state);
        }

        // Opening the group adds a child drawing group to the root group
        // when an effect is pushed.
        assert_eq!(1, group.children().len());
        let child_group = group.children().get(0).cast::<DrawingGroup>().expect("a drawing group");
        assert!(child_group.effect() == Some(effect));
        assert_eq!(Some(bounds), child_group.effect_bounds());
        scope.dispose();
    }

    #[test]
    fn drawing_with_effect_should_use_stored_bounds() {
        let (scope, _) = MockPlatformRenderInterface::install();
        let effect = blur(10.0);
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let group = DrawingGroup::new();
        group.set_effect(Some(effect.clone()));
        group.set_effect_bounds(Some(bounds));
        group.children().add(rectangle_drawing(solid(Colors::RED), Rect::new(20.0, 20.0, 50.0, 50.0)));

        let mut mock_context = MockDrawingContext::default();
        {
            let mut context = DrawingContext::new(&mut mock_context);
            group.draw(&mut context);
        }

        assert!(mock_context.effect == Some(effect));
        assert_eq!(bounds, mock_context.bounds);
        scope.dispose();
    }

    /// Regression test: the platform drawing context was passing the raw
    /// content bounds to the platform effect API without inflating them by
    /// the effect output padding, causing effects to be clipped.
    #[test]
    fn platform_drawing_context_push_effect_should_inflate_bounds_for_platform_api() {
        let log = DrawingLog::new();
        let mut effect_impl = MockDrawingContextImpl::new(log.clone());
        effect_impl.supports_effects = true;
        let mut core = PlatformDrawingContext::borrowed(&mut effect_impl);
        let mut context = DrawingContext::new(&mut core);

        let effect = blur(10.0);
        let content_bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let expected_inflated_bounds =
            content_bounds.inflate_thickness(EffectExtensions::get_effect_output_padding(Some(&*effect)));

        let state = context.push_effect(&effect, content_bounds);
        context.pop(state);
        context.dispose();

        // The platform API received the inflated (output) bounds, not the
        // raw content bounds.
        assert_eq!(1, log.count("PushEffect"));
        assert_eq!(log.entries()[0], format!("PushEffect {expected_inflated_bounds}"));
    }

    /// Regression test: drawing a group was passing uninflated local bounds
    /// to the opacity mask when an effect was also set, meaning the opacity
    /// mask didn't cover the effect output region.
    #[test]
    fn draw_core_with_effect_and_opacity_mask_should_use_effect_bounds_for_opacity_mask() {
        let (scope, _) = MockPlatformRenderInterface::install();
        let effect = blur(10.0);
        let content_bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let expected_effect_bounds =
            content_bounds.inflate_thickness(EffectExtensions::get_effect_output_padding(Some(&*effect)));

        let group = DrawingGroup::new();
        group.set_effect(Some(effect));
        group.set_effect_bounds(Some(content_bounds));
        group.set_opacity_mask(Some(solid(Colors::RED)));
        group.children().add(rectangle_drawing(solid(Colors::BLUE), Rect::new(20.0, 20.0, 50.0, 50.0)));

        let mut mock_context = MockDrawingContext::default();
        {
            let mut context = DrawingContext::new(&mut mock_context);
            group.draw(&mut context);
        }

        assert_eq!(expected_effect_bounds, mock_context.opacity_mask_bounds);
        scope.dispose();
    }

    #[test]
    fn invalidated_is_raised_when_child_is_added_or_removed() {
        let group = DrawingGroup::new();
        let child: Ref<Drawing> = GeometryDrawing::new().upcast();
        let count = counter(&group);

        group.children().add(child.clone());
        assert_eq!(1, count.get());

        group.children().remove(&child);
        assert_eq!(2, count.get());
    }

    #[test]
    fn invalidated_is_raised_when_child_changes() {
        let child = GeometryDrawing::new();
        let group = DrawingGroup::new();
        group.children().add(child.clone().upcast());

        let count = counter(&group);

        child.set_brush(Some(Brushes::red()));
        assert!(count.get() > 0);
    }

    #[test]
    fn removed_child_is_no_longer_tracked() {
        let child = GeometryDrawing::new();
        let group = DrawingGroup::new();
        let handle: Ref<Drawing> = child.clone().upcast();
        group.children().add(handle.clone());
        group.children().remove(&handle);

        let count = counter(&group);

        child.set_brush(Some(Brushes::red()));
        assert_eq!(0, count.get());
    }

    // --- not from upstream: recording into a group and drawing it ---------

    fn replay(drawing: &Drawing) -> Vec<String> {
        let log = DrawingLog::new();
        let mut platform_impl = MockDrawingContextImpl::new(log.clone());
        platform_impl.log_transforms = false;
        platform_impl.supports_effects = true;
        {
            let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
            let mut context = DrawingContext::new(&mut core);
            drawing.draw(&mut context);
        }
        log.entries()
    }

    #[test]
    fn open_with_a_single_drawing_makes_it_the_only_child() {
        let (scope, _) = MockPlatformRenderInterface::install();
        let group = DrawingGroup::new();
        let old_children = group.children();
        let count = counter(&group);
        {
            let mut context = group.open();
            context.draw_ellipse(Some(&solid(Colors::RED)), None, Rect::new(1.0, 2.0, 3.0, 4.0));
            // Nothing changes before the context is closed.
            assert_eq!(0, group.children().len());
        }
        assert!(!group.children().ptr_eq(&old_children));
        assert_eq!(1, group.children().len());
        assert_eq!(1, count.get());
        let child = group.children().get(0).cast::<GeometryDrawing>().expect("a geometry drawing");
        assert!(child.pen().is_none());
        assert_eq!(Rect::new(1.0, 2.0, 3.0, 4.0), group.get_bounds());

        // Opening again replaces the content; an empty recording clears it.
        group.open().dispose();
        assert_eq!(0, group.children().len());
        scope.dispose();
    }

    #[test]
    fn open_records_states_as_nested_groups_and_draws_back_the_same_content() {
        let (scope, _) = MockPlatformRenderInterface::install();
        let group = DrawingGroup::new();
        let pen: Rc<dyn IPen> = Rc::new(crate::media::immutable::ImmutablePen::with_brush(
                Some(Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE))),
                2.0,
            ));
        {
            let mut context = group.open();
            context.draw_line(&pen, Point::new(0.0, 0.0), Point::new(10.0, 10.0));
            let transform = context.push_transform(Matrix::create_translation(5.0, 5.0));
            let opacity = context.push_opacity(0.5);
            context.draw_rectangle(
                Some(&solid(Colors::RED)),
                None,
                Rect::new(0.0, 0.0, 4.0, 4.0),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
            context.pop(opacity);
            context.pop(transform);
            let clip = context.push_clip(Rect::new(0.0, 0.0, 20.0, 20.0));
            let options = context.push_render_options(RenderOptions::default());
            let geometry: Ref<Geometry> = RectangleGeometry::with_rect(Rect::new(1.0, 1.0, 2.0, 2.0)).upcast();
            let geometry_clip = context.push_geometry_clip(&geometry);
            let mask = context.push_opacity_mask(&solid(Colors::GREEN), Rect::new(0.0, 0.0, 9.0, 9.0));
            context.draw_geometry(Some(&solid(Colors::RED)), Some(&pen), &geometry);
            // The states still pushed are closed with the context.
            let _ = (clip, options, geometry_clip, mask);
        }

        // Root: the line, the transform group and the clip group.
        let children = group.children();
        assert_eq!(3, children.len());
        assert!(children.get(0).is::<GeometryDrawing>());
        let transform_group = children.get(1).cast::<DrawingGroup>().unwrap();
        assert_eq!(Matrix::create_translation(5.0, 5.0), transform_group.transform().unwrap().value());
        let opacity_group = transform_group.children().get(0).cast::<DrawingGroup>().unwrap();
        assert_eq!(0.5, opacity_group.opacity());
        assert!(opacity_group.children().get(0).is::<GeometryDrawing>());
        let clip_group = children.get(2).cast::<DrawingGroup>().unwrap();
        assert_eq!(Rect::new(0.0, 0.0, 20.0, 20.0), clip_group.clip_geometry().unwrap().bounds());
        let options_group = clip_group.children().get(0).cast::<DrawingGroup>().unwrap();
        assert_eq!(Some(RenderOptions::default()), options_group.render_options());

        assert_eq!(
            replay(&group),
            [
                "PushOpacity 1",
                "DrawGeometry none Blue@2 0, 0, 10, 10",
                "PushOpacity 1",
                "PushOpacity 0.5",
                "DrawGeometry Red none 0, 0, 4, 4",
                "PopOpacity",
                "PopOpacity",
                "PushOpacity 1",
                "PushGeometryClip 0, 0, 20, 20",
                "PushOpacity 1",
                "PushRenderOptions",
                "PushOpacity 1",
                "PushGeometryClip 1, 1, 2, 2",
                "PushOpacity 1",
                // The bounds of the masked content, including the stroke.
                "PushOpacityMask Green 0, 0, 4, 4",
                "DrawGeometry Red Blue@2 1, 1, 2, 2",
                "PopOpacityMask",
                "PopOpacity",
                "PopGeometryClip",
                "PopOpacity",
                "PopRenderOptions",
                "PopOpacity",
                "PopGeometryClip",
                "PopOpacity",
                "PopOpacity",
            ]
        );
        scope.dispose();
    }

    #[test]
    fn open_records_glyph_runs_and_text_options_and_draws_them_back() {
        use crate::media::text_formatting::testing::{utf16, TextTestScope};
        use crate::media::{TextHintingMode, Typeface};

        let _scope = TextTestScope::new();
        let glyph_typeface = Typeface::default_typeface().glyph_typeface();
        let glyphs: Vec<u16> =
            "ab".chars().map(|c| glyph_typeface.character_to_glyph_map().get_glyph(c as i32)).collect();
        let glyph_run = GlyphRun::from_glyph_indices(glyph_typeface, 12.0, utf16("ab"), &glyphs, None, 0);
        let text_options = TextOptions { text_hinting_mode: TextHintingMode::Strong, ..Default::default() };

        let group = DrawingGroup::new();
        {
            let mut context = group.open();
            let options = context.push_text_options(text_options);
            context.draw_glyph_run(Some(&solid(Colors::RED)), &glyph_run);
            // Without a foreground nothing is recorded.
            context.draw_glyph_run(None, &glyph_run);
            context.pop(options);
        }

        // The pushed state is the only root drawing: a group with the text
        // options whose only child is the glyph run drawing.
        let children = group.children();
        assert_eq!(1, children.len());
        let options_group = children.get(0).cast::<DrawingGroup>().expect("a group for the text options");
        assert_eq!(Some(text_options), options_group.text_options());
        assert_eq!(None, group.text_options());
        let group_children = options_group.children();
        assert_eq!(1, group_children.len());
        let drawing = group_children.get(0).cast::<GlyphRunDrawing>().expect("a glyph run drawing");
        assert!(Rc::ptr_eq(&glyph_run, &drawing.glyph_run().unwrap()));
        assert!(*drawing.foreground().unwrap() == *solid(Colors::RED));
        assert_eq!(glyph_run.bounds(), drawing.get_bounds());
        assert_eq!(glyph_run.bounds(), group.get_bounds());

        let glyph_bounds = glyph_run.platform_impl().bounds();
        assert_eq!(
            replay(&group),
            [
                "PushOpacity 1".to_string(),
                "PushOpacity 1".to_string(),
                "PushTextOptions Unspecified Strong Unspecified".to_string(),
                format!("DrawGlyphRun Red {glyph_bounds}"),
                "PopTextOptions".to_string(),
                "PopOpacity".to_string(),
                "PopOpacity".to_string(),
            ]
        );
    }

    #[test]
    fn glyph_run_drawing_without_a_glyph_run_draws_nothing() {
        let drawing = GlyphRunDrawing::new();
        drawing.set_foreground(Some(solid(Colors::RED)));
        assert_eq!(Rect::default(), drawing.get_bounds());
        assert!(replay(&drawing).is_empty());
    }

    #[test]
    fn drawing_a_group_pushes_its_effect_with_the_children_bounds() {
        let (scope, _) = MockPlatformRenderInterface::install();
        let effect = blur(10.0);
        let content = Rect::new(20.0, 20.0, 50.0, 50.0);
        let padding = EffectExtensions::get_effect_output_padding(Some(&*effect));
        let group = DrawingGroup::new();
        group.set_effect(Some(effect));
        group.children().add(rectangle_drawing(solid(Colors::RED), content));

        assert_eq!(
            replay(&group),
            [
                "PushOpacity 1".to_string(),
                format!("PushEffect {}", content.inflate_thickness(padding)),
                "DrawGeometry Red none 20, 20, 50, 50".to_string(),
                "PopEffect".to_string(),
                "PopOpacity".to_string(),
            ]
        );
        scope.dispose();
    }

    #[test]
    #[should_panic(expected = "Pushing a rounded clip into a DrawingGroup is not implemented.")]
    fn open_does_not_record_rounded_clips() {
        let (_scope, _) = MockPlatformRenderInterface::install();
        let group = DrawingGroup::new();
        let mut context = group.open();
        let _ = context.push_rounded_clip(RoundedRect::from_rect(Rect::new(0.0, 0.0, 1.0, 1.0)));
    }

    #[test]
    #[should_panic(expected = "Drawing a bitmap into a DrawingGroup is not implemented.")]
    fn open_does_not_record_bitmaps() {
        let (_scope, render_interface) = MockPlatformRenderInterface::install();
        let bitmap: std::sync::Arc<crate::platform::SharedBitmapImpl> = std::sync::Arc::new(crate::rendering::testing::MockDrawingContextLayerImpl::new(
            render_interface.log().clone(),
            crate::PixelSize::new(1, 1),
        ));
        let group = DrawingGroup::new();
        let mut context = group.open();
        context.draw_bitmap(&bitmap, 1.0, Rect::default(), Rect::default());
    }
}
