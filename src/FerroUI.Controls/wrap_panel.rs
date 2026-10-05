// This source file is adapted from the Windows Presentation Foundation
// project (https://github.com/dotnet/wpf/), MIT License, courtesy of The .NET
// Foundation.

use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::{INavigableContainer, InputElement, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl, Orientation};
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};

/// Defines how items are aligned and spaced in a [`WrapPanel`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WrapPanelItemsAlignment {
    /// Items are laid out so the first one in each column/row touches the
    /// top/left of the panel.
    #[default]
    Start = 0,
    /// Items are laid out so that each column/row is centred vertically/
    /// horizontally within the panel.
    Center = 1,
    /// Items are laid out so the last one in each column/row touches the
    /// bottom/right of the panel.
    End = 2,
    /// Items are laid out with equal spacing between them within each
    /// column/row.
    Justify = 3,
    /// Items are stretched evenly to fill the entire height/width of each
    /// column/row (the last line excluded).
    Stretch = 4,
    /// Items are stretched evenly to fill the entire height/width of each
    /// column/row (the last line included).
    StretchAll = 5,
}

/// Positions child elements in sequential position from left to right,
/// breaking content to the next line at the edge of the containing box.
/// Subsequent ordering happens sequentially from top to bottom or from right
/// to left, depending on the value of the `Orientation` property.
#[repr(C)]
pub struct WrapPanel {
    base: Panel,
}

ferro_class!(WrapPanel: Panel);
ferroui_base::ferro_class_info!(WrapPanel { new: WrapPanel::new });
ferro_impl_classes!(
    WrapPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl FerroObjectImpl for WrapPanel {}

impl LayoutableImpl for WrapPanel {
    fn measure_override(this: &Self, constraint: Size) -> Size {
        let item_width = this.item_width();
        let item_height = this.item_height();
        let item_spacing = this.item_spacing();
        let line_spacing = this.line_spacing();
        let orientation = this.orientation();
        let children = this.children().snapshot();
        let mut cur_line_size = UVSize::new(orientation);
        let mut panel_size = UVSize::new(orientation);
        let uv_constraint = UVSize::with_size(orientation, constraint.width, constraint.height);
        let item_width_set = !item_width.is_nan();
        let item_height_set = !item_height.is_nan();
        let mut item_exists = false;
        let mut line_exists = false;
        let mut items_alignment = this.items_alignment();
        let use_layout_rounding = this.use_layout_rounding();
        // If we have infinite space on the U axis, we always use Start
        // alignment to avoid strange behavior
        if uv_constraint.u == f64::INFINITY {
            items_alignment = WrapPanelItemsAlignment::Start;
        }
        // Justify/StretchAll need to measure with the full constraint on the
        // U axis
        if matches!(items_alignment, WrapPanelItemsAlignment::Justify | WrapPanelItemsAlignment::StretchAll) {
            panel_size.u = uv_constraint.u;
        }

        let child_constraint = Size::new(
            if item_width_set { item_width } else { constraint.width },
            if item_height_set { item_height } else { constraint.height },
        );

        for child in children.iter() {
            // Flow passes its own constraint to children
            child.measure(child_constraint);

            let child_size = UVSize::with_size(
                orientation,
                if item_width_set { item_width } else { child.desired_size().width },
                if item_height_set { item_height } else { child.desired_size().height },
            );

            let next_spacing = if item_exists && child.is_visible() { item_spacing } else { 0.0 };
            if greater_than(use_layout_rounding, cur_line_size.u + child_size.u + next_spacing, uv_constraint.u) {
                // Need to switch to another line
                panel_size.u = max(cur_line_size.u, panel_size.u);
                panel_size.v += cur_line_size.v + (if line_exists { line_spacing } else { 0.0 });
                cur_line_size = child_size;

                item_exists = child.is_visible();
                line_exists = true;
            } else {
                // Continue to accumulate a line
                cur_line_size.u += child_size.u + next_spacing;
                cur_line_size.v = max(child_size.v, cur_line_size.v);

                item_exists |= child.is_visible(); // keep true
            }
        }

        // Stretch needs to measure with the full constraint on the U axis if
        // there are multiple lines
        if line_exists && items_alignment == WrapPanelItemsAlignment::Stretch {
            panel_size.u = uv_constraint.u;
        }

        // The last line size, if any should be added
        panel_size.u = max(cur_line_size.u, panel_size.u);
        panel_size.v += cur_line_size.v + (if line_exists { line_spacing } else { 0.0 });

        Size::new(panel_size.width(), panel_size.height())
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let item_width = this.item_width();
        let item_height = this.item_height();
        let item_spacing = this.item_spacing();
        let line_spacing = this.line_spacing();
        let orientation = this.orientation();
        let is_horizontal = orientation == Orientation::Horizontal;
        let children = this.children().snapshot();
        let mut first_in_line = 0;
        let mut accumulated_v = 0.0;
        let mut cur_line_size = UVSize::new(orientation);
        let uv_final_size = UVSize::with_size(orientation, final_size.width, final_size.height);
        let item_width_set = !item_width.is_nan();
        let item_height_set = !item_height.is_nan();
        let mut item_exists = false;
        let mut line_exists = false;
        let mut items_alignment = this.items_alignment();
        let use_layout_rounding = this.use_layout_rounding();
        // If we have infinite space on the U axis, we always use Start
        // alignment to avoid strange behavior
        if uv_final_size.u == f64::INFINITY {
            items_alignment = WrapPanelItemsAlignment::Start;
        }

        let line = LineArrangement {
            children: &children,
            is_horizontal,
            use_item_u: if is_horizontal { item_width_set } else { item_height_set },
            item_u: if is_horizontal { item_width } else { item_height },
            item_spacing,
            final_u: uv_final_size.u,
            items_alignment,
        };

        for (i, child) in children.iter().enumerate() {
            let child_size = UVSize::with_size(
                orientation,
                if item_width_set { item_width } else { child.desired_size().width },
                if item_height_set { item_height } else { child.desired_size().height },
            );

            let next_spacing = if item_exists && child.is_visible() { item_spacing } else { 0.0 };
            if greater_than(use_layout_rounding, cur_line_size.u + child_size.u + next_spacing, uv_final_size.u) {
                // Need to switch to another line
                accumulated_v += if line_exists { line_spacing } else { 0.0 }; // add spacing to arrange line first
                line.arrange(accumulated_v, cur_line_size.v, first_in_line, i);
                accumulated_v += cur_line_size.v; // add the height of the line just arranged
                cur_line_size = child_size;

                first_in_line = i;

                item_exists = child.is_visible();
                line_exists = true;
            } else {
                // Continue to accumulate a line
                cur_line_size.u += child_size.u + next_spacing;
                cur_line_size.v = max(child_size.v, cur_line_size.v);

                item_exists |= child.is_visible(); // keep true
            }
        }

        // Arrange the last line, if any
        if first_in_line < children.len() {
            accumulated_v += if line_exists { line_spacing } else { 0.0 }; // add spacing to arrange line first
            line.arrange(accumulated_v, cur_line_size.v, first_in_line, children.len());
        }

        final_size
    }
}

/// The state shared by the arrangement of every line of a [`WrapPanel`].
struct LineArrangement<'a> {
    children: &'a [Ref<Control>],
    is_horizontal: bool,
    use_item_u: bool,
    item_u: f64,
    item_spacing: f64,
    final_u: f64,
    items_alignment: WrapPanelItemsAlignment,
}

impl LineArrangement<'_> {
    fn child_u(&self, child: &Control) -> f64 {
        if self.use_item_u {
            self.item_u
        } else if self.is_horizontal {
            child.desired_size().width
        } else {
            child.desired_size().height
        }
    }

    fn arrange(&self, accumulated_v: f64, line_v: f64, start: usize, end_excluded: usize) {
        let children = self.children;
        let mut u = 0.0;
        let mut spacing = self.item_spacing;
        // Count of spacings between items
        let mut spacing_count = -1;
        let mut total_u = 0.0;
        let mut stretch_ratio = 1.0;
        let mut line_items_alignment = self.items_alignment;
        if self.items_alignment == WrapPanelItemsAlignment::Stretch && end_excluded == children.len() {
            // Don't stretch the last line
            line_items_alignment = WrapPanelItemsAlignment::Start;
        }

        if line_items_alignment != WrapPanelItemsAlignment::Start {
            for child in &children[start..end_excluded] {
                total_u += self.child_u(child);
                if child.is_visible() {
                    spacing_count += 1;
                }
            }
        }

        match line_items_alignment {
            WrapPanelItemsAlignment::Start => {}
            WrapPanelItemsAlignment::Center => {
                total_u += spacing * spacing_count as f64;
                u = (self.final_u - total_u) / 2.0;
            }
            WrapPanelItemsAlignment::End => {
                total_u += spacing * spacing_count as f64;
                u = self.final_u - total_u;
            }
            WrapPanelItemsAlignment::Justify => {
                let total_spacing = max(self.final_u - total_u, 0.0);
                if spacing_count > 0 {
                    spacing = total_spacing / spacing_count as f64;
                }
            }
            WrapPanelItemsAlignment::Stretch | WrapPanelItemsAlignment::StretchAll => {
                if !MathUtilities::is_zero(total_u) {
                    let final_u_without_spacing = max(self.final_u - spacing * spacing_count as f64, 0.0);
                    stretch_ratio = final_u_without_spacing / total_u;
                }
            }
        }

        for child in &children[start..end_excluded] {
            let layout_slot_u = self.child_u(child) * stretch_ratio;
            child.arrange(if self.is_horizontal {
                Rect::new(u, accumulated_v, layout_slot_u, line_v)
            } else {
                Rect::new(accumulated_v, u, line_v, layout_slot_u)
            });
            u += layout_slot_u + (if child.is_visible() { spacing } else { 0.0 });
        }
    }
}

fn greater_than(use_layout_rounding: bool, value1: f64, value2: f64) -> bool {
    if use_layout_rounding {
        value1 > value2 && value1 - value2 > LayoutHelper::LAYOUT_EPSILON
    } else {
        MathUtilities::greater_than(value1, value2)
    }
}

/// A size expressed along (`u`) and across (`v`) the panel's orientation.
#[derive(Clone, Copy)]
struct UVSize {
    u: f64,
    v: f64,
    orientation: Orientation,
}

impl UVSize {
    fn with_size(orientation: Orientation, width: f64, height: f64) -> Self {
        if orientation == Orientation::Horizontal {
            Self { u: width, v: height, orientation }
        } else {
            Self { u: height, v: width, orientation }
        }
    }

    fn new(orientation: Orientation) -> Self {
        Self { u: 0.0, v: 0.0, orientation }
    }

    fn width(&self) -> f64 {
        if self.orientation == Orientation::Horizontal {
            self.u
        } else {
            self.v
        }
    }

    fn height(&self) -> f64 {
        if self.orientation == Orientation::Horizontal {
            self.v
        } else {
            self.u
        }
    }
}

ferroui_base::ferro_properties! { impl WrapPanel {
    ferro_property!(
        /// Defines the `ItemSpacing` property.
        pub fn item_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<WrapPanel, _>("ItemSpacing", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `LineSpacing` property.
        pub fn line_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<WrapPanel, _>("LineSpacing", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property.
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<WrapPanel, _>("Orientation", Orientation::Horizontal)
        }
    );

    ferro_property!(
        /// Defines the `ItemsAlignment` property.
        pub fn items_alignment_property() -> StyledProperty<WrapPanelItemsAlignment> {
            FerroProperty::register::<WrapPanel, _>("ItemsAlignment", WrapPanelItemsAlignment::Start)
        }
    );

    ferro_property!(
        /// Defines the `ItemWidth` property.
        pub fn item_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<WrapPanel, _>("ItemWidth", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `ItemHeight` property.
        pub fn item_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<WrapPanel, _>("ItemHeight", f64::NAN)
        }
    );
} }

impl WrapPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<WrapPanel>(&[
            Self::item_spacing_property().as_property(),
            Self::line_spacing_property().as_property(),
            Self::orientation_property().as_property(),
            Self::item_width_property().as_property(),
            Self::item_height_property().as_property(),
        ]);
        Layoutable::affects_arrange::<WrapPanel>(&[Self::items_alignment_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The spacing between items.
    pub fn item_spacing(&self) -> f64 {
        self.get_value(Self::item_spacing_property())
    }

    pub fn set_item_spacing(&self, value: f64) {
        self.set_value(Self::item_spacing_property(), value)
    }

    /// The spacing between lines.
    pub fn line_spacing(&self) -> f64 {
        self.get_value(Self::line_spacing_property())
    }

    pub fn set_line_spacing(&self, value: f64) {
        self.set_value(Self::line_spacing_property(), value)
    }

    /// The orientation in which child controls will be layed out.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// The alignment of items in the panel.
    pub fn items_alignment(&self) -> WrapPanelItemsAlignment {
        self.get_value(Self::items_alignment_property())
    }

    pub fn set_items_alignment(&self, value: WrapPanelItemsAlignment) {
        self.set_value(Self::items_alignment_property(), value)
    }

    /// The width of all items in the panel.
    pub fn item_width(&self) -> f64 {
        self.get_value(Self::item_width_property())
    }

    pub fn set_item_width(&self, value: f64) {
        self.set_value(Self::item_width_property(), value)
    }

    /// The height of all items in the panel.
    pub fn item_height(&self) -> f64 {
        self.get_value(Self::item_height_property())
    }

    pub fn set_item_height(&self, value: f64) {
        self.set_value(Self::item_height_property(), value)
    }
}

impl INavigableContainer for WrapPanel {
    /// Gets the next control in the specified direction.
    fn get_control(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        _wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let children = self.children();
        let horiz = self.orientation() == Orientation::Horizontal;
        let mut index: i64 = from
            .and_then(|from| from.cast::<Control>())
            .and_then(|from| children.index_of(&from))
            .map_or(-1, |index| index as i64);

        match direction {
            NavigationDirection::First => index = 0,
            NavigationDirection::Last => index = children.count() as i64 - 1,
            NavigationDirection::Next => index += 1,
            NavigationDirection::Previous => index -= 1,
            NavigationDirection::Left => index = if horiz { index - 1 } else { -1 },
            NavigationDirection::Right => index = if horiz { index + 1 } else { -1 },
            NavigationDirection::Up => index = if horiz { -1 } else { index - 1 },
            NavigationDirection::Down => index = if horiz { -1 } else { index + 1 },
            _ => {}
        }

        if index >= 0 && (index as usize) < children.count() {
            return Some(children.get(index as usize).upcast());
        }

        None
    }
}
