// This source file is adapted from the Windows Presentation Foundation
// project (https://github.com/dotnet/wpf/), MIT License, courtesy of The .NET
// Foundation.

use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::utilities::math_utilities::{max, min};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};

/// Defines the available docking modes for a control in a [`DockPanel`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Dock {
    /// The control is docked to the left of the panel.
    #[default]
    Left = 0,
    /// The control is docked to the bottom of the panel.
    Bottom = 1,
    /// The control is docked to the right of the panel.
    Right = 2,
    /// The control is docked to the top of the panel.
    Top = 3,
}

/// A panel which arranges its children at the top, bottom, left, right or
/// center.
#[repr(C)]
pub struct DockPanel {
    base: Panel,
}

ferro_class!(DockPanel: Panel);
ferroui_base::ferro_class_info!(DockPanel { new: DockPanel::new });
ferro_impl_classes!(
    DockPanel: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl FerroObjectImpl for DockPanel {}

impl LayoutableImpl for DockPanel {
    /// Updates the desired size of the panel. Called by parent elements; this
    /// is the first pass of layout.
    ///
    /// Children are measured based on their sizing properties and the dock
    /// they have. Each child is allowed to consume all of the space on the
    /// side on which it is docked; left/right docked children are granted all
    /// vertical space for their entire width, and top/bottom docked children
    /// are granted all horizontal space for their entire height.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut parent_width = 0.0;
        let mut parent_height = 0.0;
        let mut accumulated_width = 0.0;
        let mut accumulated_height = 0.0;

        let mut horizontal_spacing = false;
        let mut vertical_spacing = false;
        let children = this.children().snapshot();
        let last_child_fill = this.last_child_fill();
        let children_count = if last_child_fill { children.len().saturating_sub(1) } else { children.len() };

        for child in children.iter().take(children_count) {
            let child_constraint = Size::new(
                max(0.0, available_size.width - accumulated_width),
                max(0.0, available_size.height - accumulated_height),
            );

            child.measure(child_constraint);
            let child_desired_size = child.desired_size();

            // Now, we adjust:
            // 1. Size consumed by children (accumulated size). This will be
            //    used when computing subsequent children to determine how
            //    much space is remaining for them.
            // 2. Parent size implied by this child (parent size) when added
            //    to the current children (accumulated size). This is
            //    different from the size above in one respect: a `Dock::Left`
            //    child implies a height, but does not actually consume any
            //    height for subsequent children.
            // If we accumulate size in a given dimension, the next child (or
            // the end conditions after the child loop) will deal with
            // computing our minimum size (parent size) due to that
            // accumulation. Therefore, we only need to compute our minimum
            // size (parent size) in dimensions that this child does not
            // accumulate: width for top/bottom, height for left/right.
            match child.get_value(Self::dock_property()) {
                Dock::Left | Dock::Right => {
                    parent_height = max(parent_height, accumulated_height + child_desired_size.height);
                    if child.is_visible() {
                        accumulated_width += this.horizontal_spacing();
                        horizontal_spacing = true;
                    }
                    accumulated_width += child_desired_size.width;
                }
                Dock::Top | Dock::Bottom => {
                    parent_width = max(parent_width, accumulated_width + child_desired_size.width);
                    if child.is_visible() {
                        accumulated_height += this.vertical_spacing();
                        vertical_spacing = true;
                    }
                    accumulated_height += child_desired_size.height;
                }
            }
        }

        if last_child_fill && !children.is_empty() {
            let child = &children[children.len() - 1];
            let child_constraint = Size::new(
                max(0.0, available_size.width - accumulated_width),
                max(0.0, available_size.height - accumulated_height),
            );

            child.measure(child_constraint);
            let child_desired_size = child.desired_size();
            parent_height = max(parent_height, accumulated_height + child_desired_size.height);
            parent_width = max(parent_width, accumulated_width + child_desired_size.width);
            accumulated_height += child_desired_size.height;
            accumulated_width += child_desired_size.width;
        } else {
            if horizontal_spacing {
                accumulated_width -= this.horizontal_spacing();
            }
            if vertical_spacing {
                accumulated_height -= this.vertical_spacing();
            }
        }

        // Make sure the final accumulated size is reflected in parent size.
        parent_width = max(parent_width, accumulated_width);
        parent_height = max(parent_height, accumulated_height);
        Size::new(parent_width, parent_height)
    }

    /// The dock panel computes a position and final size for each of its
    /// children based upon their dock and sizing properties.
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        if children.is_empty() {
            return final_size;
        }

        let mut current_bounds = Rect::from_size(final_size);
        let last_child_fill = this.last_child_fill();
        let children_count = if last_child_fill { children.len() - 1 } else { children.len() };

        for child in children.iter().take(children_count) {
            if !child.is_visible() {
                continue;
            }

            let dock = child.get_value(Self::dock_property());
            match dock {
                Dock::Left => {
                    let mut width = min(child.desired_size().width, current_bounds.width);
                    child.arrange(current_bounds.with_width(width));
                    width += this.horizontal_spacing();
                    current_bounds = Rect::new(
                        current_bounds.x + width,
                        current_bounds.y,
                        max(0.0, current_bounds.width - width),
                        current_bounds.height,
                    );
                }
                Dock::Top => {
                    let mut height = min(child.desired_size().height, current_bounds.height);
                    child.arrange(current_bounds.with_height(height));
                    height += this.vertical_spacing();
                    current_bounds = Rect::new(
                        current_bounds.x,
                        current_bounds.y + height,
                        current_bounds.width,
                        max(0.0, current_bounds.height - height),
                    );
                }
                Dock::Right => {
                    let mut width = min(child.desired_size().width, current_bounds.width);
                    child.arrange(Rect::new(
                        current_bounds.x + current_bounds.width - width,
                        current_bounds.y,
                        width,
                        current_bounds.height,
                    ));
                    width += this.horizontal_spacing();
                    current_bounds = current_bounds.with_width(max(0.0, current_bounds.width - width));
                }
                Dock::Bottom => {
                    let mut height = min(child.desired_size().height, current_bounds.height);
                    child.arrange(Rect::new(
                        current_bounds.x,
                        current_bounds.y + current_bounds.height - height,
                        current_bounds.width,
                        height,
                    ));
                    height += this.vertical_spacing();
                    current_bounds = current_bounds.with_height(max(0.0, current_bounds.height - height));
                }
            }
        }

        if last_child_fill {
            let child = &children[children.len() - 1];
            child.arrange(Rect::new(current_bounds.x, current_bounds.y, current_bounds.width, current_bounds.height));
        }

        final_size
    }
}

ferroui_base::ferro_properties! { impl DockPanel {
    ferro_property!(
        /// Defines the `Dock` attached property.
        pub fn dock_property() -> AttachedProperty<Dock> {
            FerroProperty::register_attached::<DockPanel, Control, _>("Dock", Dock::Left)
        }
    );

    ferro_property!(
        /// Defines the `LastChildFill` property.
        pub fn last_child_fill_property() -> StyledProperty<bool> {
            FerroProperty::register::<DockPanel, _>("LastChildFill", true)
        }
    );

    ferro_property!(
        /// Defines the `HorizontalSpacing` property.
        pub fn horizontal_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<DockPanel, _>("HorizontalSpacing", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `VerticalSpacing` property.
        pub fn vertical_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<DockPanel, _>("VerticalSpacing", 0.0)
        }
    );
} }

impl DockPanel {
    fn static_constructor() {
        Panel::affects_parent_measure::<DockPanel>(&[Self::dock_property().as_property()]);
        Layoutable::affects_measure::<DockPanel>(&[
            Self::last_child_fill_property().as_property(),
            Self::horizontal_spacing_property().as_property(),
            Self::vertical_spacing_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the value of the `Dock` attached property on the specified
    /// control.
    pub fn get_dock(control: &Control) -> Dock {
        control.get_value(Self::dock_property())
    }

    /// Sets the value of the `Dock` attached property on the specified
    /// control.
    pub fn set_dock(control: &Control, value: Dock) {
        control.set_value(Self::dock_property(), value)
    }

    /// Whether the last child of the panel fills the remaining space in the
    /// panel.
    pub fn last_child_fill(&self) -> bool {
        self.get_value(Self::last_child_fill_property())
    }

    pub fn set_last_child_fill(&self, value: bool) {
        self.set_value(Self::last_child_fill_property(), value)
    }

    /// The horizontal distance between the child objects.
    pub fn horizontal_spacing(&self) -> f64 {
        self.get_value(Self::horizontal_spacing_property())
    }

    pub fn set_horizontal_spacing(&self, value: f64) {
        self.set_value(Self::horizontal_spacing_property(), value)
    }

    /// The vertical distance between the child objects.
    pub fn vertical_spacing(&self) -> f64 {
        self.get_value(Self::vertical_spacing_property())
    }

    pub fn set_vertical_spacing(&self, value: f64) {
        self.set_value(Self::vertical_spacing_property(), value)
    }
}
