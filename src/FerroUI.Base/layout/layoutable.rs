use super::{EffectiveViewportChangedEventArgs, ILayoutManager, ILayoutRoot, LayoutHelper, MinMax};
use crate::reactive::{Disposable, IDisposable};
use crate::styling::{Container, ContainerSizing};
use crate::utilities::{HandlerList, MathUtilities};
use crate::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, ObjectType, Point, Rect, Ref, Size, StyledElementImpl, StyledElementImplExt,
    StyledProperty, StyledPropertyOptions, Thickness, Upcast, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Defines how a control aligns itself horizontally in its parent control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HorizontalAlignment {
    /// The control stretches to fill the width of the parent control.
    #[default]
    Stretch,
    /// The control aligns itself to the left of the parent control.
    Left,
    /// The control centers itself in the parent control.
    Center,
    /// The control aligns itself to the right of the parent control.
    Right,
}

/// Defines how a control aligns itself vertically in its parent control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VerticalAlignment {
    /// The control stretches to fill the height of the parent control.
    #[default]
    Stretch,
    /// The control aligns itself to the top of the parent control.
    Top,
    /// The control centers itself within the parent control.
    Center,
    /// The control aligns itself to the bottom of the parent control.
    Bottom,
}

/// Implements layout-related functionality for a control.
#[repr(C)]
pub struct Layoutable {
    base: Visual,
    desired_size: Cell<Size>,
    is_measure_valid: Cell<bool>,
    is_arrange_valid: Cell<bool>,
    measuring: Cell<bool>,
    previous_measure: Cell<Option<Size>>,
    previous_arrange: Cell<Option<Rect>>,
    is_attaching_to_visual_tree: Cell<bool>,
    effective_viewport_changed: HandlerList<dyn Fn(&EffectiveViewportChangedEventArgs)>,
    layout_updated: HandlerList<dyn Fn()>,
    layout_updated_subscription: RefCell<Option<(Rc<dyn ILayoutManager>, u64)>>,
}

ferro_class! {
    Layoutable: Visual, virtuals LayoutableImpl: VisualImpl {
        /// Creates the visual children of the control, if necessary.
        fn apply_template(this);
        /// The default implementation of the control's measure pass.
        ///
        /// This calculates the desired size taking into account the control's
        /// margin and min/max/explicit size, and calls `measure_override` for
        /// the content. Usually `measure_override` is what you want to
        /// override.
        fn measure_core(this, available_size: Size) -> Size;
        /// Measures the control and its child elements as part of a layout
        /// pass. Returns the size the control would like to be.
        fn measure_override(this, available_size: Size) -> Size;
        /// The default implementation of the control's arrange pass.
        ///
        /// This calculates the final bounds taking into account the control's
        /// margin, alignment and size constraints, and calls
        /// `arrange_override` for the content. Usually `arrange_override` is
        /// what you want to override.
        fn arrange_core(this, final_rect: Rect);
        /// Positions child elements as part of a layout pass. Returns the
        /// size used.
        fn arrange_override(this, final_size: Size) -> Size;
        /// Called by `invalidate_measure`.
        fn on_measure_invalidated(this);
    }
}
crate::ferro_class_info!(Layoutable { new: Layoutable::new });

impl FerroObjectImpl for Layoutable {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Visual::is_visible_property().as_property() {
            this.set_desired_size(Size::default());

            // All changes to visibility cause the parent element to be notified.
            if let Some(parent) = this.visual_parent() {
                if let Some(parent) = parent.downcast_ref::<Layoutable>() {
                    parent.child_desired_size_changed(this);
                }
            }

            if change.get_new_value::<bool>() {
                // We only invalidate ourselves when visibility is changed to true.
                this.invalidate_measure();

                // If any descendant had its measure/arrange invalidated while
                // we were hidden, they will need to be registered with the
                // layout manager now that they are again effectively visible.
                if let Some(layout_root) = this.get_layout_root() {
                    let manager = layout_root.layout_manager();
                    if let Some(children) = this.visual_children_snapshot() {
                        for child in children.iter() {
                            if let Some(child) = child.downcast_ref::<Layoutable>() {
                                child.ancestor_became_visible(&*manager);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl StyledElementImpl for Layoutable {
    fn invalidate_styles(this: &Self, recurse: bool) {
        Self::parent_invalidate_styles(this, recurse);
        this.invalidate_measure();
    }

    fn on_control_theme_changed(this: &Self) {
        Self::parent_on_control_theme_changed(this);
        this.invalidate_measure();
    }

    fn on_templated_parent_control_theme_changed(this: &Self) {
        Self::parent_on_templated_parent_control_theme_changed(this);
        this.invalidate_measure();
    }
}

impl VisualImpl for Layoutable {
    fn on_attached_to_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.is_attaching_to_visual_tree.set(true);
        Self::parent_on_attached_to_visual_tree_core(this, e);
        this.is_attaching_to_visual_tree.set(false);

        if let Some(root) = this.get_layout_root() {
            let manager = root.layout_manager();
            if !this.layout_updated.is_empty() {
                this.subscribe_layout_updated(manager.clone());
            }
            if !this.effective_viewport_changed.is_empty() {
                manager.register_effective_viewport_listener(this);
            }
        }
    }

    fn on_detached_from_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        if let Some(root) = this.get_layout_root() {
            this.unsubscribe_layout_updated();
            if !this.effective_viewport_changed.is_empty() {
                root.layout_manager().unregister_effective_viewport_listener(this);
            }
        }
        Self::parent_on_detached_from_visual_tree_core(this, e);
    }

    fn on_visual_parent_changed(this: &Self, old_parent: Option<&Ref<Visual>>, new_parent: Option<&Ref<Visual>>) {
        LayoutHelper::invalidate_self_and_children_measure(this);
        Self::parent_on_visual_parent_changed(this, old_parent, new_parent);
    }
}

impl LayoutableImpl for Layoutable {
    fn apply_template(_this: &Self) {}

    fn measure_core(this: &Self, available_size: Size) -> Size {
        this.apply_styling();

        if !this.is_visible() {
            return Size::default();
        }

        let mut margin = this.margin();
        let use_layout_rounding = this.use_layout_rounding();
        let mut scale = 1.0;

        if use_layout_rounding {
            scale = LayoutHelper::get_layout_scale(this);
            margin = LayoutHelper::round_layout_thickness(margin, scale);
        }

        this.apply_template();

        let min_max = MinMax::new(this);
        let constrained_size = LayoutHelper::apply_min_max(min_max, available_size.deflate(margin));

        let mut container_sizing = ContainerSizing::Normal;
        if let Some(query_provider) = Container::get_query_provider(this) {
            let sizing = Container::get_sizing(this);
            if sizing != ContainerSizing::Normal {
                container_sizing = sizing;
                query_provider.set_size(constrained_size.width, constrained_size.height, container_sizing);
            }
        }

        let measured = this.measure_override(constrained_size);

        let mut width = MathUtilities::clamp(measured.width, min_max.min_width, min_max.max_width);
        let mut height = MathUtilities::clamp(measured.height, min_max.min_height, min_max.max_height);

        match container_sizing {
            ContainerSizing::Normal => {}
            ContainerSizing::Width => {
                width = if constrained_size.width.is_infinite() { width } else { constrained_size.width };
            }
            ContainerSizing::Height => {
                width = measured.width;
                height = if constrained_size.height.is_infinite() { height } else { constrained_size.height };
            }
            ContainerSizing::WidthAndHeight => {
                width = if constrained_size.width.is_infinite() { width } else { constrained_size.width };
                height = if constrained_size.height.is_infinite() { height } else { constrained_size.height };
            }
        }

        if use_layout_rounding {
            let rounded = LayoutHelper::round_layout_size_up(Size::new(width, height), scale);
            width = rounded.width;
            height = rounded.height;
        }

        width += margin.left + margin.right;
        height += margin.top + margin.bottom;

        if width > available_size.width {
            width = available_size.width;
        }
        if height > available_size.height {
            height = available_size.height;
        }
        if width < 0.0 {
            width = 0.0;
        }
        if height < 0.0 {
            height = 0.0;
        }

        Size::new(width, height)
    }

    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;

        if let Some(children) = this.visual_children_snapshot() {
            for visual in children.iter() {
                if let Some(layoutable) = visual.downcast_ref::<Layoutable>() {
                    layoutable.measure(available_size);
                    let child_size = layoutable.desired_size();
                    if child_size.width > width {
                        width = child_size.width;
                    }
                    if child_size.height > height {
                        height = child_size.height;
                    }
                }
            }
        }

        Size::new(width, height)
    }

    fn arrange_core(this: &Self, final_rect: Rect) {
        if !this.is_visible() {
            return;
        }

        let use_layout_rounding = this.use_layout_rounding();
        let scale = LayoutHelper::get_layout_scale(this);

        let mut margin = this.margin();
        let mut origin_x = final_rect.x + margin.left;
        let mut origin_y = final_rect.y + margin.top;

        // Margin has to be treated separately because the layout rounding
        // function is not linear: f(a + b) != f(a) + f(b). If the margin isn't
        // pre-rounded some sizes will be offset by 1 pixel in certain scales.
        if use_layout_rounding {
            margin = LayoutHelper::round_layout_thickness(margin, scale);
        }

        let available_width = (final_rect.width - margin.left - margin.right).max(0.0);
        let available_height = (final_rect.height - margin.top - margin.bottom).max(0.0);
        let mut available_size_minus_margins = Size::new(available_width, available_height);

        let horizontal_alignment = this.horizontal_alignment();
        let vertical_alignment = this.vertical_alignment();
        let mut size = available_size_minus_margins;
        let desired_size = this.desired_size();

        if horizontal_alignment != HorizontalAlignment::Stretch {
            size = size.with_width(size.width.min(desired_size.width - margin.left - margin.right));
        }
        if vertical_alignment != VerticalAlignment::Stretch {
            size = size.with_height(size.height.min(desired_size.height - margin.top - margin.bottom));
        }

        size = LayoutHelper::apply_min_max(MinMax::new(this), size);

        if use_layout_rounding {
            size = LayoutHelper::round_layout_size_up(size, scale);
            available_size_minus_margins = LayoutHelper::round_layout_size_up(available_size_minus_margins, scale);
        }

        size = this.arrange_override(size).constrain(size);

        match horizontal_alignment {
            HorizontalAlignment::Center | HorizontalAlignment::Stretch => {
                origin_x += (available_size_minus_margins.width - size.width) / 2.0;
            }
            HorizontalAlignment::Right => origin_x += available_size_minus_margins.width - size.width,
            HorizontalAlignment::Left => {}
        }

        match vertical_alignment {
            VerticalAlignment::Center | VerticalAlignment::Stretch => {
                origin_y += (available_size_minus_margins.height - size.height) / 2.0;
            }
            VerticalAlignment::Bottom => origin_y += available_size_minus_margins.height - size.height,
            VerticalAlignment::Top => {}
        }

        let mut origin = Point::new(origin_x, origin_y);
        if use_layout_rounding {
            origin = LayoutHelper::round_layout_point(origin, scale);
        }

        this.set_bounds(Rect::from_position_size(origin, size));
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let arrange_rect = Rect::from_size(final_size);
        if let Some(children) = this.visual_children_snapshot() {
            for visual in children.iter() {
                if let Some(layoutable) = visual.downcast_ref::<Layoutable>() {
                    layoutable.arrange(arrange_rect);
                }
            }
        }
        final_size
    }

    fn on_measure_invalidated(_this: &Self) {}
}

crate::ferro_properties! { impl Layoutable {
    ferro_property!(
        /// Defines the `DesiredSize` property.
        pub fn desired_size_property() -> DirectProperty<Layoutable, Size> {
            FerroProperty::register_direct::<Layoutable, _>("DesiredSize", |o| o.desired_size(), None, Size::default())
        }
    );

    ferro_property!(
        /// Defines the `Width` property.
        pub fn width_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Layoutable, _>(
                "Width",
                StyledPropertyOptions::new(f64::NAN).validate(Layoutable::validate_dimension),
            )
        }
    );

    ferro_property!(
        /// Defines the `Height` property.
        pub fn height_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Layoutable, _>(
                "Height",
                StyledPropertyOptions::new(f64::NAN).validate(Layoutable::validate_dimension),
            )
        }
    );

    ferro_property!(
        /// Defines the `MinWidth` property.
        pub fn min_width_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Layoutable, _>(
                "MinWidth",
                StyledPropertyOptions::new(0.0).validate(Layoutable::validate_minimum_dimension),
            )
        }
    );

    ferro_property!(
        /// Defines the `MaxWidth` property.
        pub fn max_width_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Layoutable, _>(
                "MaxWidth",
                StyledPropertyOptions::new(f64::INFINITY).validate(Layoutable::validate_maximum_dimension),
            )
        }
    );

    ferro_property!(
        /// Defines the `MinHeight` property.
        pub fn min_height_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Layoutable, _>(
                "MinHeight",
                StyledPropertyOptions::new(0.0).validate(Layoutable::validate_minimum_dimension),
            )
        }
    );

    ferro_property!(
        /// Defines the `MaxHeight` property.
        pub fn max_height_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<Layoutable, _>(
                "MaxHeight",
                StyledPropertyOptions::new(f64::INFINITY).validate(Layoutable::validate_maximum_dimension),
            )
        }
    );

    ferro_property!(
        /// Defines the `Margin` property.
        pub fn margin_property() -> StyledProperty<Thickness> {
            FerroProperty::register_with::<Layoutable, _>(
                "Margin",
                StyledPropertyOptions::new(Thickness::default()).validate(Layoutable::validate_thickness),
            )
        }
    );

    ferro_property!(
        /// Defines the `HorizontalAlignment` property.
        pub fn horizontal_alignment_property() -> StyledProperty<HorizontalAlignment> {
            FerroProperty::register::<Layoutable, _>("HorizontalAlignment", HorizontalAlignment::Stretch)
        }
    );

    ferro_property!(
        /// Defines the `VerticalAlignment` property.
        pub fn vertical_alignment_property() -> StyledProperty<VerticalAlignment> {
            FerroProperty::register::<Layoutable, _>("VerticalAlignment", VerticalAlignment::Stretch)
        }
    );

    ferro_property!(
        /// Defines the `UseLayoutRounding` property.
        pub fn use_layout_rounding_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<Layoutable, _>(
                "UseLayoutRounding",
                StyledPropertyOptions::new(true).inherits(true),
            )
        }
    );
} }

impl Layoutable {
    fn static_constructor() {
        Self::affects_measure::<Layoutable>(&[
            Self::width_property().as_property(),
            Self::height_property().as_property(),
            Self::min_width_property().as_property(),
            Self::max_width_property().as_property(),
            Self::min_height_property().as_property(),
            Self::max_height_property().as_property(),
            Self::margin_property().as_property(),
            Self::horizontal_alignment_property().as_property(),
            Self::vertical_alignment_property().as_property(),
        ]);
    }

    fn validate_dimension(value: &f64) -> bool {
        value.is_nan() || Self::validate_minimum_dimension(value)
    }

    fn validate_minimum_dimension(value: &f64) -> bool {
        *value != f64::INFINITY && Self::validate_maximum_dimension(value)
    }

    fn validate_maximum_dimension(value: &f64) -> bool {
        *value >= 0.0
    }

    fn validate_thickness(value: &Thickness) -> bool {
        value.left.is_finite() && value.top.is_finite() && value.right.is_finite() && value.bottom.is_finite()
    }

    /// Creates the class data; see [`crate::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Visual::construct(),
            desired_size: Cell::new(Size::default()),
            is_measure_valid: Cell::new(false),
            is_arrange_valid: Cell::new(false),
            measuring: Cell::new(false),
            previous_measure: Cell::new(None),
            previous_arrange: Cell::new(None),
            is_attaching_to_visual_tree: Cell::new(false),
            effective_viewport_changed: HandlerList::new(),
            layout_updated: HandlerList::new(),
            layout_updated_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // --- events -----------------------------------------------------------

    /// Occurs when the element's effective viewport changes.
    pub fn effective_viewport_changed(
        &self,
        handler: impl Fn(&EffectiveViewportChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        if self.effective_viewport_changed.is_empty() && !self.is_attaching_to_visual_tree.get() {
            if let Some(root) = self.get_layout_root() {
                root.layout_manager().register_effective_viewport_listener(self);
            }
        }
        let token = self.effective_viewport_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.effective_viewport_changed.remove(token);
                if this.effective_viewport_changed.is_empty() {
                    if let Some(root) = this.get_layout_root() {
                        root.layout_manager().unregister_effective_viewport_listener(&this);
                    }
                }
            }
        })
    }

    /// Occurs when a layout pass completes for the control.
    pub fn layout_updated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        if self.layout_updated.is_empty() && !self.is_attaching_to_visual_tree.get() {
            if let Some(root) = self.get_layout_root() {
                self.subscribe_layout_updated(root.layout_manager());
            }
        }
        let token = self.layout_updated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.layout_updated.remove(token);
                if this.layout_updated.is_empty() {
                    this.unsubscribe_layout_updated();
                }
            }
        })
    }

    fn subscribe_layout_updated(&self, manager: Rc<dyn ILayoutManager>) {
        if self.layout_updated_subscription.borrow().is_some() {
            return;
        }
        let weak = self.to_ref().downgrade();
        let token = manager.add_layout_updated(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                if !this.layout_updated.is_empty() {
                    for (_, handler) in this.layout_updated.snapshot().iter() {
                        handler();
                    }
                }
            }
        }));
        *self.layout_updated_subscription.borrow_mut() = Some((manager, token));
    }

    fn unsubscribe_layout_updated(&self) {
        if let Some((manager, token)) = self.layout_updated_subscription.take() {
            manager.remove_layout_updated(token);
        }
    }

    pub(crate) fn raise_effective_viewport_changed(&self, e: &EffectiveViewportChangedEventArgs) {
        if !self.effective_viewport_changed.is_empty() {
            for (_, handler) in self.effective_viewport_changed.snapshot().iter() {
                handler(e);
            }
        }
    }

    // --- properties -------------------------------------------------------

    /// Executes a layout pass.
    ///
    /// You should not usually need to call this method explicitly: the layout
    /// manager schedules layout passes itself.
    pub fn update_layout(&self) {
        if let Some(manager) = self.get_layout_manager() {
            manager.execute_layout_pass();
        }
    }

    /// The layout root of the tree the control is attached to.
    pub fn get_layout_root(&self) -> Option<Rc<dyn ILayoutRoot>> {
        self.presentation_source().map(|s| s.layout_root())
    }

    /// The layout manager of the tree the control is attached to.
    pub fn get_layout_manager(&self) -> Option<Rc<dyn ILayoutManager>> {
        self.get_layout_root().map(|r| r.layout_manager())
    }

    /// The width of the element.
    pub fn width(&self) -> f64 {
        self.get_value(Self::width_property())
    }

    pub fn set_width(&self, value: f64) {
        self.set_value(Self::width_property(), value)
    }

    /// The height of the element.
    pub fn height(&self) -> f64 {
        self.get_value(Self::height_property())
    }

    pub fn set_height(&self, value: f64) {
        self.set_value(Self::height_property(), value)
    }

    /// The minimum width of the element.
    pub fn min_width(&self) -> f64 {
        self.get_value(Self::min_width_property())
    }

    pub fn set_min_width(&self, value: f64) {
        self.set_value(Self::min_width_property(), value)
    }

    /// The maximum width of the element.
    pub fn max_width(&self) -> f64 {
        self.get_value(Self::max_width_property())
    }

    pub fn set_max_width(&self, value: f64) {
        self.set_value(Self::max_width_property(), value)
    }

    /// The minimum height of the element.
    pub fn min_height(&self) -> f64 {
        self.get_value(Self::min_height_property())
    }

    pub fn set_min_height(&self, value: f64) {
        self.set_value(Self::min_height_property(), value)
    }

    /// The maximum height of the element.
    pub fn max_height(&self) -> f64 {
        self.get_value(Self::max_height_property())
    }

    pub fn set_max_height(&self, value: f64) {
        self.set_value(Self::max_height_property(), value)
    }

    /// The margin around the element.
    pub fn margin(&self) -> Thickness {
        self.get_value(Self::margin_property())
    }

    pub fn set_margin(&self, value: Thickness) {
        self.set_value(Self::margin_property(), value)
    }

    /// The element's preferred horizontal alignment in its parent.
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_alignment_property())
    }

    pub fn set_horizontal_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_alignment_property(), value)
    }

    /// The element's preferred vertical alignment in its parent.
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_alignment_property())
    }

    pub fn set_vertical_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_alignment_property(), value)
    }

    /// The size that this element computed during the measure pass of the
    /// layout process.
    #[inline]
    pub fn desired_size(&self) -> Size {
        self.desired_size.get()
    }

    fn set_desired_size(&self, value: Size) {
        self.set_and_raise_cell(Self::desired_size_property(), &self.desired_size, value);
    }

    /// Whether the control's layout measure is valid.
    #[inline]
    pub fn is_measure_valid(&self) -> bool {
        self.is_measure_valid.get()
    }

    /// Whether the control's layout arrange is valid.
    #[inline]
    pub fn is_arrange_valid(&self) -> bool {
        self.is_arrange_valid.get()
    }

    /// Whether the control's layout should be snapped to pixel boundaries.
    pub fn use_layout_rounding(&self) -> bool {
        self.get_value(Self::use_layout_rounding_property())
    }

    pub fn set_use_layout_rounding(&self, value: bool) {
        self.set_value(Self::use_layout_rounding_property(), value)
    }

    /// The available size passed in the previous layout pass, if any.
    pub fn previous_measure(&self) -> Option<Size> {
        self.previous_measure.get()
    }

    /// The rect passed in the previous layout pass, if any.
    pub fn previous_arrange(&self) -> Option<Rect> {
        self.previous_arrange.get()
    }

    // --- layout -----------------------------------------------------------

    /// Carries out a measure of the control.
    pub fn measure(&self, available_size: Size) {
        if available_size.width.is_nan() || available_size.height.is_nan() {
            panic!("Cannot call Measure using a size with NaN values.");
        }

        if !self.is_measure_valid.get() || self.previous_measure.get() != Some(available_size) {
            let previous_desired_size = self.desired_size();

            self.is_measure_valid.set(true);

            self.measuring.set(true);
            let guard = MeasuringGuard(&self.measuring);
            let desired_size = self.measure_core(available_size);
            drop(guard);

            if Self::is_invalid_size(desired_size) {
                panic!("Invalid size returned for Measure.");
            }

            self.set_desired_size(desired_size);
            self.previous_measure.set(Some(available_size));

            if self.desired_size() != previous_desired_size {
                if let Some(parent) = self.visual_parent() {
                    if let Some(parent) = parent.downcast_ref::<Layoutable>() {
                        parent.child_desired_size_changed(self);
                    }
                }
            }
        }
    }

    /// Arranges the control and its children.
    pub fn arrange(&self, rect: Rect) {
        if Self::is_invalid_rect(rect) {
            panic!("Invalid Arrange rectangle.");
        }

        if !self.is_measure_valid.get() {
            self.measure(self.previous_measure.get().unwrap_or(rect.size()));
        }

        if !self.is_arrange_valid.get() || self.previous_arrange.get() != Some(rect) {
            self.is_arrange_valid.set(true);
            self.arrange_core(rect);
            self.previous_arrange.set(Some(rect));
        }
    }

    /// Invalidates the measurement of the control and queues a new layout
    /// pass.
    pub fn invalidate_measure(&self) {
        if self.is_measure_valid.get() {
            self.is_measure_valid.set(false);
            self.is_arrange_valid.set(false);

            if self.is_attached_to_visual_tree() {
                if let Some(manager) = self.get_layout_manager() {
                    manager.invalidate_measure(self);
                }
                self.invalidate_visual();
            }
            self.on_measure_invalidated();
        }
    }

    /// Invalidates the arrangement of the control and queues a new layout
    /// pass.
    pub fn invalidate_arrange(&self) {
        if self.is_arrange_valid.get() {
            self.is_arrange_valid.set(false);
            if let Some(manager) = self.get_layout_manager() {
                manager.invalidate_arrange(self);
            }
            self.invalidate_visual();
        }
    }

    /// Called by a child when its desired size changes.
    pub(crate) fn child_desired_size_changed(&self, _control: &Layoutable) {
        if !self.measuring.get() {
            self.invalidate_measure();
        }
    }

    /// Marks a property as affecting the control's measurement: after a
    /// change to any of the properties on an object of class `T`,
    /// `invalidate_measure` is called on it.
    pub fn affects_measure<T: ObjectType + Upcast<Layoutable>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                if let Some(sender) = e.sender().downcast_ref::<T>() {
                    let layoutable: &Layoutable = sender.upcast();
                    layoutable.invalidate_measure();
                }
            });
        }
    }

    /// Marks a property as affecting the control's arrangement: after a
    /// change to any of the properties on an object of class `T`,
    /// `invalidate_arrange` is called on it.
    pub fn affects_arrange<T: ObjectType + Upcast<Layoutable>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                if let Some(sender) = e.sender().downcast_ref::<T>() {
                    let layoutable: &Layoutable = sender.upcast();
                    layoutable.invalidate_arrange();
                }
            });
        }
    }

    fn ancestor_became_visible(&self, layout_manager: &dyn ILayoutManager) {
        if !self.is_visible() {
            return;
        }

        if !self.is_measure_valid.get() {
            layout_manager.invalidate_measure(self);
            self.invalidate_visual();
        } else if !self.is_arrange_valid.get() {
            layout_manager.invalidate_arrange(self);
            self.invalidate_visual();
        }

        if let Some(children) = self.visual_children_snapshot() {
            for child in children.iter() {
                if let Some(child) = child.downcast_ref::<Layoutable>() {
                    child.ancestor_became_visible(layout_manager);
                }
            }
        }
    }

    fn is_invalid_rect(rect: Rect) -> bool {
        MathUtilities::is_negative_or_non_finite(rect.width)
            || MathUtilities::is_negative_or_non_finite(rect.height)
            || !MathUtilities::is_finite(rect.x)
            || !MathUtilities::is_finite(rect.y)
    }

    fn is_invalid_size(size: Size) -> bool {
        MathUtilities::is_negative_or_non_finite(size.width) || MathUtilities::is_negative_or_non_finite(size.height)
    }
}

/// Resets the "measuring" flag even if `measure_core` unwinds.
struct MeasuringGuard<'a>(&'a Cell<bool>);

impl Drop for MeasuringGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
