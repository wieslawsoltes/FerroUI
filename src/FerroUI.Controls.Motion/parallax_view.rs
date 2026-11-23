//! `ParallaxView` of WinUI (`controls/dev/ParallaxView`).
//!
//! The scroll source is a [`ScrollViewer`], or the first one inside the source element (the
//! scroll viewer of a list). It has no overpan, so the underpan and overpan terms of the source's
//! formulas are zero.

use crate::parallax_expressions::{self as expressions, Axis, HORIZONTAL, VERTICAL};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::animations::ExpressionAnimation;
use ferroui_base::rendering::composition::{CompositionPropertySet, CompositionVisual, ElementComposition, ICompositionObjectAnimations};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, Vector3D, Visual, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::hosting::ElementCompositionPreview;
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::{Control, ControlImpl, Decorator, ScrollViewer};
use std::cell::RefCell;
use std::rc::Rc;

/// How the source start and end offsets of a [`ParallaxView`] are read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ParallaxSourceOffsetKind {
    /// The offsets are positions in the scrolling content.
    Absolute = 0,
    /// The offsets are added to the range in which the view is on screen.
    #[default]
    Relative = 1,
}

ferroui_base::ferro_markup_enum!(ParallaxSourceOffsetKind { Absolute, Relative }, { namespace: "FerroUI.Controls" });

/// The composition objects of one axis.
#[derive(Default)]
struct AxisAnimations {
    parallax: Option<Rc<ExpressionAnimation>>,
    is_started: bool,
    /// What the running animation was started with: the same values do not restart it.
    last: Option<(String, [f32; 8])>,
}

#[derive(Default)]
struct State {
    target: Option<Rc<CompositionVisual>>,
    axes: [AxisAnimations; 2],
    scroller: Option<WeakRef<ScrollViewer>>,
    source_set: Option<Rc<CompositionPropertySet>>,
    scroller_changed: Option<Rc<dyn IDisposable>>,
    /// The handler that connects again when a source without a scroll viewer yet is loaded.
    source_loaded: Option<(WeakRef<Control>, RoutedEventHandlerToken)>,
}

/// A container that moves its child at a different speed than a scrolling source.
#[repr(C)]
pub struct ParallaxView {
    base: Decorator,
    state: RefCell<State>,
}

ferro_class!(ParallaxView: Decorator);
ferroui_base::ferro_class_info!(ParallaxView {
    new: ParallaxView::new,
    markup: { namespace: "FerroUI.Controls", content: Child },
});
ferro_impl_classes!(ParallaxView: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

ferro_properties! {
    impl ParallaxView {
        /// Defines the `Source` property: the element that scrolls.
        pub fn source_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<ParallaxView, _>("Source", None)
        }
        /// Defines the `HorizontalShift` property.
        pub fn horizontal_shift_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("HorizontalShift", 0.0)
        }
        /// Defines the `VerticalShift` property.
        pub fn vertical_shift_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("VerticalShift", 0.0)
        }
        /// Defines the `HorizontalSourceStartOffset` property.
        pub fn horizontal_source_start_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("HorizontalSourceStartOffset", 0.0)
        }
        /// Defines the `HorizontalSourceEndOffset` property.
        pub fn horizontal_source_end_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("HorizontalSourceEndOffset", 0.0)
        }
        /// Defines the `VerticalSourceStartOffset` property.
        pub fn vertical_source_start_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("VerticalSourceStartOffset", 0.0)
        }
        /// Defines the `VerticalSourceEndOffset` property.
        pub fn vertical_source_end_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("VerticalSourceEndOffset", 0.0)
        }
        /// Defines the `HorizontalSourceOffsetKind` property.
        pub fn horizontal_source_offset_kind_property() -> StyledProperty<ParallaxSourceOffsetKind> {
            FerroProperty::register::<ParallaxView, _>("HorizontalSourceOffsetKind", ParallaxSourceOffsetKind::Relative)
        }
        /// Defines the `VerticalSourceOffsetKind` property.
        pub fn vertical_source_offset_kind_property() -> StyledProperty<ParallaxSourceOffsetKind> {
            FerroProperty::register::<ParallaxView, _>("VerticalSourceOffsetKind", ParallaxSourceOffsetKind::Relative)
        }
        /// Defines the `IsHorizontalShiftClamped` property.
        pub fn is_horizontal_shift_clamped_property() -> StyledProperty<bool> {
            FerroProperty::register::<ParallaxView, _>("IsHorizontalShiftClamped", true)
        }
        /// Defines the `IsVerticalShiftClamped` property.
        pub fn is_vertical_shift_clamped_property() -> StyledProperty<bool> {
            FerroProperty::register::<ParallaxView, _>("IsVerticalShiftClamped", true)
        }
        /// Defines the `MaxHorizontalShiftRatio` property.
        pub fn max_horizontal_shift_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("MaxHorizontalShiftRatio", 1.0)
        }
        /// Defines the `MaxVerticalShiftRatio` property.
        pub fn max_vertical_shift_ratio_property() -> StyledProperty<f64> {
            FerroProperty::register::<ParallaxView, _>("MaxVerticalShiftRatio", 1.0)
        }
    }
}

macro_rules! accessors {
    ($($get:ident, $set:ident, $property:ident: $type:ty;)*) => {
        impl ParallaxView {
            $(
                #[doc = concat!("The value of `", stringify!($property), "`.")]
                pub fn $get(&self) -> $type {
                    self.get_value(Self::$property())
                }

                #[doc = concat!("Sets the value of `", stringify!($property), "`.")]
                pub fn $set(&self, value: $type) {
                    self.set_value(Self::$property(), value);
                }
            )*
        }
    };
}

accessors! {
    source, set_source, source_property: Option<Ref<Control>>;
    horizontal_shift, set_horizontal_shift, horizontal_shift_property: f64;
    vertical_shift, set_vertical_shift, vertical_shift_property: f64;
    horizontal_source_start_offset, set_horizontal_source_start_offset, horizontal_source_start_offset_property: f64;
    horizontal_source_end_offset, set_horizontal_source_end_offset, horizontal_source_end_offset_property: f64;
    vertical_source_start_offset, set_vertical_source_start_offset, vertical_source_start_offset_property: f64;
    vertical_source_end_offset, set_vertical_source_end_offset, vertical_source_end_offset_property: f64;
    horizontal_source_offset_kind, set_horizontal_source_offset_kind, horizontal_source_offset_kind_property: ParallaxSourceOffsetKind;
    vertical_source_offset_kind, set_vertical_source_offset_kind, vertical_source_offset_kind_property: ParallaxSourceOffsetKind;
    is_horizontal_shift_clamped, set_is_horizontal_shift_clamped, is_horizontal_shift_clamped_property: bool;
    is_vertical_shift_clamped, set_is_vertical_shift_clamped, is_vertical_shift_clamped_property: bool;
    max_horizontal_shift_ratio, set_max_horizontal_shift_ratio, max_horizontal_shift_ratio_property: f64;
    max_vertical_shift_ratio, set_max_vertical_shift_ratio, max_vertical_shift_ratio_property: f64;
}

impl FerroObjectImpl for ParallaxView {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::source_property().as_property() || property == Decorator::child_property().as_property() {
            this.connect();
        } else if property == Visual::bounds_property().as_property() {
            // The size and the place of the view are part of the offsets.
            this.update_axis(&HORIZONTAL);
            this.update_axis(&VERTICAL);
        } else if property == Self::horizontal_shift_property().as_property() {
            this.invalidate_measure();
            this.update_axis(&HORIZONTAL);
        } else if property == Self::vertical_shift_property().as_property() {
            this.invalidate_measure();
            this.update_axis(&VERTICAL);
        } else if property == Self::is_horizontal_shift_clamped_property().as_property()
            || property == Self::horizontal_source_offset_kind_property().as_property()
            || property == Self::horizontal_source_start_offset_property().as_property()
            || property == Self::horizontal_source_end_offset_property().as_property()
            || property == Self::max_horizontal_shift_ratio_property().as_property()
        {
            this.update_axis(&HORIZONTAL);
        } else if property == Self::is_vertical_shift_clamped_property().as_property()
            || property == Self::vertical_source_offset_kind_property().as_property()
            || property == Self::vertical_source_start_offset_property().as_property()
            || property == Self::vertical_source_end_offset_property().as_property()
            || property == Self::max_vertical_shift_ratio_property().as_property()
        {
            this.update_axis(&VERTICAL);
        }
    }
}

impl VisualImpl for ParallaxView {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.connect();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.disconnect();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl LayoutableImpl for ParallaxView {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut desired = Size::default();
        if let Some(child) = this.child() {
            // The shift is part of what the child may fill.
            child.measure(Size::new(
                available_size.width + this.horizontal_shift().abs(),
                available_size.height + this.vertical_shift().abs(),
            ));
            desired = child.desired_size();
        }
        Size::new(
            if available_size.width.is_infinite() { desired.width } else { available_size.width },
            if available_size.height.is_infinite() { desired.height } else { available_size.height },
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(child) = this.child() {
            let rect = child_rect(&child, final_size, this.horizontal_shift(), this.vertical_shift());
            child.arrange(rect);
        }
        final_size
    }
}

/// The rectangle of the child: at least the view and the shift, stretched in proportion on the
/// other axis, and placed by its alignments as a border places its child.
fn child_rect(child: &Control, final_size: Size, horizontal_shift: f64, vertical_shift: f64) -> Rect {
    let desired = child.desired_size();
    let (mut width, mut height) = (desired.width, desired.height);
    if horizontal_shift != 0.0 && width < final_size.width + horizontal_shift.abs() {
        let ratio = if width > 0.0 { (final_size.width + horizontal_shift.abs()) / width } else { 0.0 };
        width = final_size.width + horizontal_shift.abs();
        if ratio != 0.0 && child.height().is_nan() && child.vertical_alignment() == VerticalAlignment::Stretch {
            height *= ratio;
        }
    }
    if vertical_shift != 0.0 && height < final_size.height + vertical_shift.abs() {
        let ratio = if height > 0.0 { (final_size.height + vertical_shift.abs()) / height } else { 0.0 };
        height = final_size.height + vertical_shift.abs();
        if ratio != 0.0 && child.width().is_nan() && child.horizontal_alignment() == HorizontalAlignment::Stretch {
            width *= ratio;
        }
    }
    let x = match child.horizontal_alignment() {
        HorizontalAlignment::Center => (final_size.width - width) / 2.0,
        HorizontalAlignment::Right => final_size.width - width,
        HorizontalAlignment::Stretch if width < final_size.width => (final_size.width - width) / 2.0,
        _ => 0.0,
    };
    let y = match child.vertical_alignment() {
        VerticalAlignment::Center => (final_size.height - height) / 2.0,
        VerticalAlignment::Bottom => final_size.height - height,
        VerticalAlignment::Stretch if height < final_size.height => (final_size.height - height) / 2.0,
        _ => 0.0,
    };
    Rect::new(x, y, width, height)
}

impl ParallaxView {
    fn static_constructor() {
        Layoutable::affects_arrange::<ParallaxView>(&[
            Self::horizontal_shift_property().as_property(),
            Self::vertical_shift_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Decorator::construct(), state: RefCell::new(State::default()) }
    }

    /// Creates a view without a child or a source.
    pub fn new() -> Ref<Self> {
        let view = instantiate(Self::construct());
        // The child reaches beyond the view by the shift.
        view.set_clip_to_bounds(true);
        view
    }

    /// `RefreshAutomaticHorizontalOffsets`: reads the position of the view in the scrolling
    /// content again, after it has moved there.
    pub fn refresh_automatic_horizontal_offsets(&self) {
        if self.horizontal_source_offset_kind() == ParallaxSourceOffsetKind::Relative {
            self.update_axis(&HORIZONTAL);
        }
    }

    /// `RefreshAutomaticVerticalOffsets`: as
    /// [`refresh_automatic_horizontal_offsets`](Self::refresh_automatic_horizontal_offsets).
    pub fn refresh_automatic_vertical_offsets(&self) {
        if self.vertical_source_offset_kind() == ParallaxSourceOffsetKind::Relative {
            self.update_axis(&VERTICAL);
        }
    }

    fn shift(&self, axis: &Axis) -> f64 {
        if axis.orientation == Orientation::Horizontal { self.horizontal_shift() } else { self.vertical_shift() }
    }

    fn scroller(&self) -> Option<Ref<ScrollViewer>> {
        self.state.borrow().scroller.as_ref().and_then(WeakRef::upgrade)
    }

    /// Finds the scroll viewer of the source and the visual of the child, and starts the motion.
    fn connect(&self) {
        self.disconnect();
        if !self.is_attached_to_visual_tree() {
            return;
        }
        let Some(source) = self.source() else { return };
        let Some(child) = self.child() else { return };
        let scroller = source.clone().cast::<ScrollViewer>().or_else(|| source.find_descendant_of_type::<ScrollViewer>(false));
        let Some(scroller) = scroller else {
            // A list has its scroll viewer once its template is applied, which is done by the
            // time it is loaded.
            let weak = self.to_ref().downgrade();
            let subscription = source.loaded(move |_, _| {
                if let Some(view) = weak.upgrade() {
                    view.connect();
                }
            });
            self.state.borrow_mut().source_loaded = Some((source.downgrade(), subscription));
            return;
        };
        let Some(target) = ElementComposition::get_element_visual(&child) else { return };
        let Some(source_set) = ElementCompositionPreview::get_scroll_viewer_manipulation_property_set(&scroller) else {
            return;
        };
        ElementCompositionPreview::set_is_translation_enabled(&child, true);
        let weak = self.to_ref().downgrade();
        let changed = scroller.property_changed(move |change| {
            if change.property() == ScrollViewer::extent_property().as_property()
                || change.property() == ScrollViewer::viewport_property().as_property()
            {
                if let Some(view) = weak.upgrade() {
                    view.update_axis(&HORIZONTAL);
                    view.update_axis(&VERTICAL);
                }
            }
        });
        {
            let mut state = self.state.borrow_mut();
            state.target = Some(target);
            state.scroller = Some(scroller.downgrade());
            state.source_set = Some(source_set);
            state.scroller_changed = Some(changed);
        }
        self.update_axis(&HORIZONTAL);
        self.update_axis(&VERTICAL);
    }

    /// Stops the motion and puts the child back.
    fn disconnect(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(subscription) = state.scroller_changed.take() {
            subscription.dispose();
        }
        if let Some((source, token)) = state.source_loaded.take() {
            if let Some(source) = source.upgrade() {
                source.remove_handler(Control::loaded_event(), token);
            }
        }
        if let Some(target) = state.target.take() {
            for axis in [&HORIZONTAL, &VERTICAL] {
                if state.axes[axis.index].is_started {
                    target.stop_animation(axis.target);
                }
            }
            target.set_translation(Vector3D::default());
        }
        state.axes = Default::default();
        state.scroller = None;
        state.source_set = None;
    }

    /// What the formulas read of the scroll viewer: is the view inside it, its offset in the
    /// unzoomed content, the viewport and the unzoomed content.
    fn measures(&self, axis: &Axis) -> Option<(bool, f32, f32, f32)> {
        let scroller = self.scroller()?;
        let is_horizontal = axis.orientation == Orientation::Horizontal;
        let of = |size: Size| if is_horizontal { size.width } else { size.height };
        let zoom = f64::from(scroller.zoom_factor()).max(f64::MIN_POSITIVE);
        let is_inside = scroller.is_visual_ancestor_of(self);
        let offset = if is_inside {
            // The place in the scrolling content, which scrolling does not change.
            let content = self
                .get_visual_ancestors()
                .find(|visual| visual.get_visual_parent().is_some_and(|parent| parent.cast::<ScrollContentPresenter>().is_some()));
            let point = content.and_then(|content| self.translate_point(Point::new(0.0, 0.0), &content)).unwrap_or_default();
            if is_horizontal { point.x } else { point.y }
        } else {
            0.0
        };
        Some((is_inside, offset as f32, of(scroller.viewport()) as f32, (of(scroller.extent()) / zoom) as f32))
    }

    /// The expression that moves the child along `axis`, or none for a shift of zero. It is
    /// started again only when its text or one of its values has changed.
    fn update_axis(&self, axis: &Axis) {
        let shift = self.shift(axis) as f32;
        if shift == 0.0 {
            let mut state = self.state.borrow_mut();
            let Some(target) = state.target.clone() else { return };
            let animations = &mut state.axes[axis.index];
            if animations.is_started {
                target.stop_animation(axis.target);
                animations.is_started = false;
                animations.last = None;
                let mut translation = target.translation();
                if axis.orientation == Orientation::Horizontal {
                    translation.x = 0.0;
                } else {
                    translation.y = 0.0;
                }
                target.set_translation(translation);
            }
            return;
        }
        let Some((is_inside, view_offset, viewport, content)) = self.measures(axis) else { return };
        let (start, end, kind, view_size, is_clamped, ratio) = if axis.orientation == Orientation::Horizontal {
            (
                self.horizontal_source_start_offset(),
                self.horizontal_source_end_offset(),
                self.horizontal_source_offset_kind(),
                self.bounds().width,
                self.is_horizontal_shift_clamped(),
                self.max_horizontal_shift_ratio(),
            )
        } else {
            (
                self.vertical_source_start_offset(),
                self.vertical_source_end_offset(),
                self.vertical_source_offset_kind(),
                self.bounds().height,
                self.is_vertical_shift_clamped(),
                self.max_vertical_shift_ratio(),
            )
        };
        let is_relative = kind == ParallaxSourceOffsetKind::Relative;
        let (start, end) = (start as f32, end as f32);
        let text = expressions::parallax(
            axis,
            is_clamped,
            shift,
            &expressions::start_offset(is_relative, is_inside, start),
            &expressions::end_offset(is_relative, is_inside, end, content, viewport),
        );
        let values = [start, end, view_offset, view_size as f32, viewport, content, ratio.max(0.0) as f32, shift];

        let mut state = self.state.borrow_mut();
        let (Some(target), Some(source)) = (state.target.clone(), state.source_set.clone()) else {
            return;
        };
        let animations = &mut state.axes[axis.index];
        if animations.is_started && animations.last.as_ref().is_some_and(|(last, old)| *last == text && *old == values) {
            return;
        }
        let compositor = target.compositor().clone();
        let animation = animations.parallax.get_or_insert_with(|| compositor.create_expression_animation()).clone();
        animation.set_expression(Some(text.clone()));
        animation.set_reference_parameter("source", source);
        for (name, value) in [
            ("startOffset", start),
            ("endOffset", end),
            ("parallaxViewOffset", view_offset),
            ("parallaxViewSize", view_size as f32),
            ("viewportSize", viewport),
            ("contentSize", content),
            ("maxUnderpanOffset", 0.0),
            ("maxOverpanOffset", 0.0),
            ("maxRatio", ratio.max(0.0) as f32),
            ("shift", shift),
        ] {
            animation.set_scalar_parameter(name, value);
        }
        if animations.is_started {
            target.stop_animation(axis.target);
        }
        target.start_animation(axis.target, &*animation);
        animations.is_started = true;
        animations.last = Some((text, values));
    }
}
