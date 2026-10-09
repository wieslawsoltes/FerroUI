use super::{
    Flex, FlexAlignContent, FlexAlignItems, FlexBasisKind, FlexDirection, FlexJustifyContent, FlexWrap,
};
use crate::{Control, ControlImpl, Panel, PanelImpl, PanelImplExt};
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::utilities::math_utilities::max as math_max;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroProperty, Point, Rect,
    Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// A panel that arranges child controls using CSS FlexBox principles.
/// It organizes child items in one or more lines along a main-axis (either
/// row or column) and provides advanced control over their sizing and
/// layout.
///
/// See the CSS FlexBox specification: <https://www.w3.org/TR/css-flexbox-1>
#[repr(C)]
pub struct FlexPanel {
    base: Panel,
    state: FlexLayoutState,
    child_visibility_subscriptions: RefCell<HashMap<Ref<Control>, Rc<dyn IDisposable>>>,
    visible_children: RefCell<Option<Rc<[Ref<Control>]>>>,
}

ferro_class!(FlexPanel: Panel);
ferroui_base::ferro_class_info!(FlexPanel { new: FlexPanel::new });
ferro_impl_classes!(
    FlexPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferroui_base::ferro_impl_classes!(FlexPanel: FerroObjectImpl);

ferroui_base::ferro_properties! {
    impl FlexPanel {
        /// Defines the `Direction` property.
        pub fn direction_property() -> StyledProperty<FlexDirection> {
            FerroProperty::register::<FlexPanel, _>("Direction", FlexDirection::Row)
        }

        /// Defines the `JustifyContent` property.
        pub fn justify_content_property() -> StyledProperty<FlexJustifyContent> {
            FerroProperty::register::<FlexPanel, _>("JustifyContent", FlexJustifyContent::FlexStart)
        }

        /// Defines the `AlignItems` property.
        pub fn align_items_property() -> StyledProperty<FlexAlignItems> {
            FerroProperty::register::<FlexPanel, _>("AlignItems", FlexAlignItems::Stretch)
        }

        /// Defines the `AlignContent` property.
        pub fn align_content_property() -> StyledProperty<FlexAlignContent> {
            FerroProperty::register::<FlexPanel, _>("AlignContent", FlexAlignContent::Stretch)
        }

        /// Defines the `Wrap` property.
        pub fn wrap_property() -> StyledProperty<FlexWrap> {
            FerroProperty::register::<FlexPanel, _>("Wrap", FlexWrap::NoWrap)
        }

        /// Defines the `ColumnSpacing` property.
        pub fn column_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<FlexPanel, _>("ColumnSpacing", 0.0)
        }

        /// Defines the `RowSpacing` property.
        pub fn row_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<FlexPanel, _>("RowSpacing", 0.0)
        }
    }
}

impl FlexPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<FlexPanel>(&[
            Self::direction_property().as_property(),
            Self::justify_content_property().as_property(),
            Self::wrap_property().as_property(),
            Self::column_spacing_property().as_property(),
            Self::row_spacing_property().as_property(),
        ]);

        Layoutable::affects_arrange::<FlexPanel>(&[
            Self::align_items_property().as_property(),
            Self::align_content_property().as_property(),
        ]);

        Panel::affects_parent_measure::<FlexPanel>(&[
            Layoutable::horizontal_alignment_property().as_property(),
            Layoutable::vertical_alignment_property().as_property(),
            Flex::order_property().as_property(),
            Flex::basis_property().as_property(),
            Flex::shrink_property().as_property(),
            Flex::grow_property().as_property(),
        ]);

        Panel::affects_parent_arrange::<FlexPanel>(&[Flex::align_self_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            state: FlexLayoutState::default(),
            child_visibility_subscriptions: RefCell::new(HashMap::new()),
            visible_children: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The direction of the panel's main-axis, determining the orientation
    /// in which child controls are laid out.
    ///
    /// The default value is [`FlexDirection::Row`]. Equivalent to the CSS
    /// flex-direction property.
    pub fn direction(&self) -> FlexDirection {
        self.get_value(Self::direction_property())
    }

    pub fn set_direction(&self, value: FlexDirection) {
        self.set_value(Self::direction_property(), value)
    }

    /// The main-axis alignment of child items inside a line of the panel.
    /// Typically used to distribute extra free space leftover after flexible
    /// lengths and margins have been resolved.
    ///
    /// The default value is [`FlexJustifyContent::FlexStart`]. Equivalent to
    /// the CSS justify-content property.
    pub fn justify_content(&self) -> FlexJustifyContent {
        self.get_value(Self::justify_content_property())
    }

    pub fn set_justify_content(&self, value: FlexJustifyContent) {
        self.set_value(Self::justify_content_property(), value)
    }

    /// The cross-axis alignment of all child items inside a line of the
    /// panel. Similar to `justify_content`, but in the perpendicular
    /// direction.
    ///
    /// The default value is [`FlexAlignItems::Stretch`]. Equivalent to the
    /// CSS align-items property.
    pub fn align_items(&self) -> FlexAlignItems {
        self.get_value(Self::align_items_property())
    }

    pub fn set_align_items(&self, value: FlexAlignItems) {
        self.set_value(Self::align_items_property(), value)
    }

    /// The cross-axis alignment of lines in the panel when there is extra
    /// space. Similar to `align_items`, but for entire lines. The `wrap`
    /// property set to [`FlexWrap::Wrap`] mode allows controls to be
    /// arranged on multiple lines.
    ///
    /// The default value is [`FlexAlignContent::Stretch`]. Equivalent to the
    /// CSS align-content property.
    pub fn align_content(&self) -> FlexAlignContent {
        self.get_value(Self::align_content_property())
    }

    pub fn set_align_content(&self, value: FlexAlignContent) {
        self.set_value(Self::align_content_property(), value)
    }

    /// The wrap mode, controlling whether the panel is single-line or
    /// multi-line. Additionally, it determines the cross-axis stacking
    /// direction for new lines.
    ///
    /// The default value is [`FlexWrap::NoWrap`]. Equivalent to the CSS
    /// flex-wrap property.
    pub fn wrap(&self) -> FlexWrap {
        self.get_value(Self::wrap_property())
    }

    pub fn set_wrap(&self, value: FlexWrap) {
        self.set_value(Self::wrap_property(), value)
    }

    /// The minimum horizontal spacing between child items or lines,
    /// depending on the main-axis direction of the panel.
    ///
    /// The default value is 0. Similar to the CSS column-gap property.
    pub fn column_spacing(&self) -> f64 {
        self.get_value(Self::column_spacing_property())
    }

    pub fn set_column_spacing(&self, value: f64) {
        self.set_value(Self::column_spacing_property(), value)
    }

    /// The minimum vertical spacing between child items or lines, depending
    /// on the main-axis direction of the panel.
    ///
    /// The default value is 0. Similar to the CSS row-gap property.
    pub fn row_spacing(&self) -> f64 {
        self.get_value(Self::row_spacing_property())
    }

    pub fn set_row_spacing(&self, value: f64) {
        self.set_value(Self::row_spacing_property(), value)
    }

    /// The visible children in layout order, when there are any.
    fn non_empty_visible_children(&self) -> Option<Rc<[Ref<Control>]>> {
        self.visible_children.borrow().clone().filter(|children| !children.is_empty())
    }

    fn measure_child(element: &Control, max: Uv, is_column: bool) -> Uv {
        let basis = Flex::get_basis(element);
        let flex_constraint = match basis.kind() {
            FlexBasisKind::Auto => max.u,
            FlexBasisKind::Absolute => basis.value(),
            FlexBasisKind::Relative => max.u * basis.value() / 100.0,
        };
        element.measure(Uv::to_size(max.with_u(flex_constraint), is_column));

        let size = Uv::from_size(element.desired_size(), is_column);

        let flex_length = match basis.kind() {
            FlexBasisKind::Auto => size.u,
            FlexBasisKind::Absolute | FlexBasisKind::Relative => math_max(size.u, flex_constraint),
        };
        let size = size.with_u(flex_length);

        Flex::set_base_length(element, flex_length);
        Flex::set_current_length(element, flex_length);
        size
    }

    fn determine_align_content(
        current_align_content: FlexAlignContent,
        free_v: f64,
        lines_count: usize,
    ) -> FlexAlignContent {
        // Determine AlignContent based on available space and line count
        match current_align_content {
            // If there's free vertical space, handle distribution based on the content alignment
            FlexAlignContent::Stretch if free_v > 0.0 => FlexAlignContent::Stretch,
            FlexAlignContent::SpaceBetween if free_v > 0.0 && lines_count > 1 => FlexAlignContent::SpaceBetween,
            FlexAlignContent::SpaceAround if free_v > 0.0 && lines_count > 0 => FlexAlignContent::SpaceAround,
            FlexAlignContent::SpaceEvenly if free_v > 0.0 && lines_count > 0 => FlexAlignContent::SpaceEvenly,

            // Default alignments when there's no free space or not enough lines
            FlexAlignContent::Stretch => FlexAlignContent::FlexStart,
            FlexAlignContent::SpaceBetween => FlexAlignContent::FlexStart,
            FlexAlignContent::SpaceAround => FlexAlignContent::Center,
            FlexAlignContent::SpaceEvenly => FlexAlignContent::Center,
            FlexAlignContent::FlexStart | FlexAlignContent::Center | FlexAlignContent::FlexEnd => {
                current_align_content
            }
        }
    }

    fn get_cross_axis_pos_and_spacing(
        align_content: FlexAlignContent,
        spacing: Uv,
        free_v: f64,
        lines_count: usize,
    ) -> (f64, f64) {
        let lines = lines_count as f64;
        match align_content {
            FlexAlignContent::FlexStart => (0.0, spacing.v),
            FlexAlignContent::FlexEnd => (free_v, spacing.v),
            FlexAlignContent::Center => (free_v / 2.0, spacing.v),
            FlexAlignContent::Stretch => (0.0, spacing.v),

            FlexAlignContent::SpaceBetween if lines_count > 1 => (0.0, spacing.v + free_v / (lines - 1.0)),
            FlexAlignContent::SpaceBetween => (0.0, spacing.v),

            FlexAlignContent::SpaceAround if lines_count > 0 => (free_v / lines / 2.0, spacing.v + free_v / lines),
            FlexAlignContent::SpaceAround => (free_v / 2.0, spacing.v),

            FlexAlignContent::SpaceEvenly => (free_v / (lines + 1.0), spacing.v + free_v / (lines + 1.0)),
        }
    }

    fn get_main_axis_pos_and_spacing(
        justify_content: FlexJustifyContent,
        line: &FlexLine,
        spacing: Uv,
        remaining_free_u: f64,
        items_count: usize,
    ) -> (f64, f64) {
        if line.grow > 0.0 {
            return (0.0, spacing.u);
        }

        let items = items_count as f64;
        match justify_content {
            FlexJustifyContent::FlexStart => (0.0, spacing.u),
            FlexJustifyContent::FlexEnd => (remaining_free_u, spacing.u),
            FlexJustifyContent::Center => (remaining_free_u / 2.0, spacing.u),

            FlexJustifyContent::SpaceBetween if items_count > 1 => {
                (0.0, spacing.u + remaining_free_u / (items - 1.0))
            }
            FlexJustifyContent::SpaceBetween => (0.0, spacing.u),

            FlexJustifyContent::SpaceAround if items_count > 0 => {
                (remaining_free_u / items / 2.0, spacing.u + remaining_free_u / items)
            }
            FlexJustifyContent::SpaceAround => (remaining_free_u / 2.0, spacing.u),

            FlexJustifyContent::SpaceEvenly if items_count > 0 => {
                (remaining_free_u / (items + 1.0), spacing.u + remaining_free_u / (items + 1.0))
            }
            FlexJustifyContent::SpaceEvenly => (remaining_free_u / 2.0, spacing.u),
        }
    }

    /// Returns the number of items of the line and its free main-axis space.
    fn get_line_measure_u(line: &FlexLine, panel_size_u: f64, spacing_u: f64) -> (usize, f64) {
        let items_count = line.count();
        let total_spacing_u = (items_count as f64 - 1.0) * spacing_u;
        let total_u = line.u + total_spacing_u;
        let free_u = panel_size_u - total_u;
        (items_count, free_u)
    }

    /// Returns the sum of the flex factors that apply to the line, its number
    /// of auto margins and the free space that remains to be distributed.
    fn get_line_mult_info(line: &FlexLine, free_u: f64) -> (f64, i32, f64) {
        let line_mult = if free_u < 0.0 {
            line.shrink
        } else if free_u > 0.0 {
            line.grow
        } else {
            0.0
        };
        // https://www.w3.org/TR/css-flexbox-1/#remaining-free-space
        // Sum of flex factors less than 1 reduces remaining free space to be distributed.
        if line_mult > 0.0 && line_mult < 1.0 {
            (line_mult, line.auto_margins, free_u * line_mult)
        } else {
            (line_mult, line.auto_margins, free_u)
        }
    }

    fn get_item_mult(element: &Layoutable, free_u: f64) -> f64 {
        if free_u < 0.0 {
            Flex::get_shrink(element)
        } else if free_u > 0.0 {
            Flex::get_grow(element)
        } else {
            0.0
        }
    }

    fn get_item_auto_margins(element: &Layoutable, is_column: bool) -> i32 {
        if is_column {
            match element.vertical_alignment() {
                VerticalAlignment::Stretch => 0,
                VerticalAlignment::Top | VerticalAlignment::Bottom => 1,
                VerticalAlignment::Center => 2,
            }
        } else {
            match element.horizontal_alignment() {
                HorizontalAlignment::Stretch => 0,
                HorizontalAlignment::Left | HorizontalAlignment::Right => 1,
                HorizontalAlignment::Center => 2,
            }
        }
    }

    fn on_child_visibility_changed(&self, _is_visible: bool) {
        self.update_visible_children();
    }

    fn update_visible_children(&self) {
        let mut visible_children: Vec<Ref<Control>> =
            self.children().snapshot().iter().filter(|child| child.is_visible()).cloned().collect();
        // A stable sort: items with the same order keep their source order.
        visible_children.sort_by_key(|child| Flex::get_order(child));
        *self.visible_children.borrow_mut() = Some(visible_children.into());

        self.invalidate_measure();
    }
}

impl LayoutableImpl for FlexPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let Some(children) = this.non_empty_visible_children() else {
            this.state.lines.borrow_mut().clear();
            this.state.measure_count.set(this.state.measure_count.get().wrapping_add(1));
            return Size::default();
        };

        let direction = this.direction();
        let is_column = matches!(direction, FlexDirection::Column | FlexDirection::ColumnReverse);
        let wrap = this.wrap();

        let max = Uv::from_size(available_size, is_column);
        let spacing = Uv::from_width_height(this.column_spacing(), this.row_spacing(), is_column);

        let mut line_data = LineData::default();
        let (mut child_index, mut first_child_index, mut item_index) = (0usize, 0usize, 0usize);

        // The lines are taken out of the state while children are measured:
        // no borrow is held across their layout code.
        let mut lines = std::mem::take(&mut *this.state.lines.borrow_mut());
        lines.clear();

        let mut panel_size_u = 0.0;
        let max_selector = |l: &FlexLine| l.u + (l.count() as f64 - 1.0) * spacing.u;

        for element in children.iter() {
            let size = Self::measure_child(element, max, is_column);

            if wrap != FlexWrap::NoWrap
                && MathUtilities::greater_than_or_close(line_data.u + size.u + item_index as f64 * spacing.u, max.u)
            {
                // As upstream, a first item that does not fit closes an empty
                // line (its last index is one before its first).
                let line = FlexLine::new(first_child_index as isize, child_index as isize - 1, line_data);
                panel_size_u = math_max(panel_size_u, max_selector(&line));
                lines.push(line);
                line_data = LineData::default();
                first_child_index = child_index;
                item_index = 0;
            }

            line_data.u += size.u;
            line_data.v = math_max(line_data.v, size.v);
            line_data.shrink += Flex::get_shrink(element);
            line_data.grow += Flex::get_grow(element);
            line_data.auto_margins += Self::get_item_auto_margins(element, is_column);
            item_index += 1;
            child_index += 1;
        }

        if item_index != 0 {
            let line =
                FlexLine::new(first_child_index as isize, (first_child_index + item_index) as isize - 1, line_data);
            panel_size_u = math_max(panel_size_u, max_selector(&line));
            lines.push(line);
        }

        let total_spacing_v = (lines.len() as f64 - 1.0) * spacing.v;

        // Resizing along main axis using grow and shrink factors can affect cross axis, so remeasure affected items and lines.
        for flex_line in lines.iter_mut() {
            let (_items_count, free_u) = Self::get_line_measure_u(flex_line, max.u, spacing.u);
            let (line_mult, _auto_margins, remaining_free_u) = Self::get_line_mult_info(flex_line, free_u);

            if line_mult != 0.0 && remaining_free_u != 0.0 {
                let mut max_v: f64 = 0.0;
                for element in &children[flex_line.range()] {
                    let base_length = Flex::get_base_length(element);
                    let mult = Self::get_item_mult(element, free_u);
                    let length = math_max(0.0, base_length + remaining_free_u * mult / line_mult);
                    element.measure(Uv::to_size(max.with_u(length), is_column));

                    max_v = math_max(Uv::from_size(element.desired_size(), is_column).v, max_v);
                }

                flex_line.v = max_v;
            }
        }

        if wrap == FlexWrap::WrapReverse {
            lines.reverse();
        }

        let total_line_v: f64 = lines.iter().map(|l| l.v).sum();
        let panel_size =
            if lines.is_empty() { Uv::default() } else { Uv::new(panel_size_u, total_line_v + total_spacing_v) };

        *this.state.lines.borrow_mut() = lines;
        this.state.measure_count.set(this.state.measure_count.get().wrapping_add(1));

        Uv::to_size(panel_size, is_column)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let Some(children) = this.non_empty_visible_children() else {
            return final_size;
        };
        if this.state.lines.borrow().is_empty() {
            return final_size;
        }

        let direction = this.direction();
        let is_column = matches!(direction, FlexDirection::Column | FlexDirection::ColumnReverse);
        let is_reverse = matches!(direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse);

        let panel_size = Uv::from_size(final_size, is_column);
        let spacing = Uv::from_width_height(this.column_spacing(), this.row_spacing(), is_column);
        let justify_content = this.justify_content();
        let align_items = this.align_items();

        // The lines are taken out of the state while children are arranged:
        // no borrow is held across their layout code.
        let lines = std::mem::take(&mut *this.state.lines.borrow_mut());
        let measure_count = this.state.measure_count.get();

        let lines_count = lines.len();
        let total_line_v: f64 = lines.iter().map(|s| s.v).sum();
        let total_spacing_v = (lines_count as f64 - 1.0) * spacing.v;
        let total_v = total_line_v + total_spacing_v;
        let free_v = panel_size.v - total_v;

        let align_content = Self::determine_align_content(this.align_content(), free_v, lines_count);

        let (mut v, spacing_v) = Self::get_cross_axis_pos_and_spacing(align_content, spacing, free_v, lines_count);

        let scale_v = if align_content == FlexAlignContent::Stretch && total_line_v != 0.0 {
            (panel_size.v - total_spacing_v) / total_line_v
        } else {
            1.0
        };

        for line in &lines {
            let line_children = &children[line.range()];
            let line_v = scale_v * line.v;
            let (items_count, free_u) = Self::get_line_measure_u(line, panel_size.u, spacing.u);
            let (line_mult, line_auto_margins, mut remaining_free_u) = Self::get_line_mult_info(line, free_u);

            let mut current_free_u = remaining_free_u;
            if line_mult != 0.0 && remaining_free_u != 0.0 {
                for element in line_children {
                    let base_length = Flex::get_base_length(element);
                    let mult = Self::get_item_mult(element, free_u);
                    if mult != 0.0 {
                        let length = math_max(0.0, base_length + remaining_free_u * mult / line_mult);
                        Flex::set_current_length(element, length);
                        current_free_u -= length - base_length;
                    }
                }
            }
            remaining_free_u = current_free_u;

            if line_auto_margins != 0 && remaining_free_u != 0.0 {
                for element in line_children {
                    let base_length = Flex::get_current_length(element);
                    let auto_margins = Self::get_item_auto_margins(element, is_column);
                    if auto_margins != 0 {
                        let length = math_max(
                            0.0,
                            base_length + remaining_free_u * auto_margins as f64 / line_auto_margins as f64,
                        );
                        Flex::set_current_length(element, length);
                        current_free_u -= length - base_length;
                    }
                }
            }
            remaining_free_u = current_free_u;

            let (mut u, spacing_u) =
                Self::get_main_axis_pos_and_spacing(justify_content, line, spacing, remaining_free_u, items_count);

            for element in line_children {
                let size = Uv::from_size(element.desired_size(), is_column).with_u(Flex::get_current_length(element));
                let align = Flex::get_align_self(element).unwrap_or(align_items);

                let position_v = match align {
                    FlexAlignItems::FlexStart => v,
                    FlexAlignItems::FlexEnd => v + line_v - size.v,
                    FlexAlignItems::Center => v + (line_v - size.v) / 2.0,
                    FlexAlignItems::Stretch => v,
                };

                let size = size.with_v(if align == FlexAlignItems::Stretch { line_v } else { size.v });
                let position = Uv::new(if is_reverse { panel_size.u - size.u - u } else { u }, position_v);
                let origin = Uv::to_point(position, is_column);
                let extent = Uv::to_size(size, is_column);
                element.arrange(Rect::new(origin.x, origin.y, extent.width, extent.height));

                u += size.u + spacing_u;
            }

            v += line_v + spacing_v;
        }

        // A measure pass that ran while the children were arranged has
        // stored new lines; they win.
        if this.state.measure_count.get() == measure_count {
            *this.state.lines.borrow_mut() = lines;
        }

        final_size
    }
}

impl PanelImpl for FlexPanel {
    fn children_changed(this: &Self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        Self::parent_children_changed(this, e);

        for old in e.old_items {
            let disposable = this.child_visibility_subscriptions.borrow_mut().remove(old);
            if let Some(disposable) = disposable {
                disposable.dispose();
            }
        }

        for new_item in e.new_items {
            let weak = this.to_ref().downgrade();
            let object: &FerroObject = new_item;
            let subscription = FerroObjectExtensions::get_observable(object, Visual::is_visible_property())
                .subscribe_fn(move |is_visible| {
                    if let Some(this) = weak.upgrade() {
                        this.on_child_visibility_changed(is_visible);
                    }
                });
            let previous = this.child_visibility_subscriptions.borrow_mut().insert(new_item.clone(), subscription);
            if previous.is_some() {
                panic!("An item with the same key has already been added.");
            }
        }

        this.update_visible_children();
    }
}

#[derive(Default)]
struct FlexLayoutState {
    lines: RefCell<Vec<FlexLine>>,
    /// Counts the measure passes, so that an arrange pass can tell whether
    /// the lines it works on are still the current ones.
    measure_count: Cell<u64>,
}

#[derive(Clone, Copy, Default)]
struct LineData {
    u: f64,
    v: f64,
    shrink: f64,
    grow: f64,
    auto_margins: i32,
}

#[derive(Clone, Copy)]
struct FlexLine {
    /// First item index.
    first: isize,
    /// Last item index.
    last: isize,
    /// Sum of main sizes of items.
    u: f64,
    /// Max of cross sizes of items.
    v: f64,
    /// Sum of shrink factors of flexible items.
    shrink: f64,
    /// Sum of grow factors of flexible items.
    grow: f64,
    /// Number of "auto margins" along main axis.
    auto_margins: i32,
}

impl FlexLine {
    fn new(first: isize, last: isize, l: LineData) -> Self {
        Self { first, last, u: l.u, v: l.v, shrink: l.shrink, grow: l.grow, auto_margins: l.auto_margins }
    }

    /// Number of items.
    fn count(&self) -> usize {
        (self.last - self.first + 1).max(0) as usize
    }

    /// The indexes of the items of the line.
    fn range(&self) -> std::ops::Range<usize> {
        let first = self.first.max(0) as usize;
        first..first + self.count()
    }
}

#[derive(Clone, Copy, Default)]
struct Uv {
    u: f64,
    v: f64,
}

impl Uv {
    fn new(u: f64, v: f64) -> Self {
        Self { u, v }
    }

    fn from_width_height(width: f64, height: f64, swap: bool) -> Self {
        Self::new(if swap { height } else { width }, if swap { width } else { height })
    }

    fn from_size(size: Size, swap: bool) -> Self {
        Self::from_width_height(size.width, size.height, swap)
    }

    fn to_point(uv: Uv, swap: bool) -> Point {
        Point::new(if swap { uv.v } else { uv.u }, if swap { uv.u } else { uv.v })
    }

    fn to_size(uv: Uv, swap: bool) -> Size {
        Size::new(if swap { uv.v } else { uv.u }, if swap { uv.u } else { uv.v })
    }

    fn with_u(self, u: f64) -> Self {
        Self::new(u, self.v)
    }

    fn with_v(self, v: f64) -> Self {
        Self::new(self.u, v)
    }
}
