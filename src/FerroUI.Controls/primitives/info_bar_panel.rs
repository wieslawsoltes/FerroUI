// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../NOTICE.md.
// The pinned InfoBarPanel.cpp layout, expressed with native layout and property APIs.

use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property,
    instantiate, AttachedProperty, FerroObjectImpl, FerroProperty, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl,
};
use std::cell::Cell;

/// Lays out its children on one line when they fit in its width, and below each other otherwise
/// (`InfoBarPanel` of WinUI). One line: the children side by side, each moved by its
/// `HorizontalOrientationMargin` (the left margin of the first and the right margin of the last
/// left out), the last one given the rest of the width, the panel padded by
/// `HorizontalOrientationPadding`. Below each other: each child moved by its
/// `VerticalOrientationMargin` (the top margin of the first and the bottom margin of the last
/// left out), the panel padded by `VerticalOrientationPadding`. A child without a width or a
/// height (a collapsed one) is left out of both. One child, children wider than the panel, or a
/// child that would be taller on one line than the minimum height of the parent of the panel
/// (less the vertical margins of the panel) give the layout below each other.
#[repr(C)]
pub struct InfoBarPanel {
    base: Panel,
    is_vertical: Cell<bool>,
}

ferro_class!(InfoBarPanel: Panel);
ferro_class_info!(InfoBarPanel { new: InfoBarPanel::new, markup: { namespace: "FerroUI.Controls.Primitives" } });
ferro_impl_classes!(
    InfoBarPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_properties! { impl InfoBarPanel {
    ferro_property!(
        /// Defines the `HorizontalOrientationPadding` property.
        pub fn horizontal_orientation_padding_property() -> StyledProperty<Thickness> {
            FerroProperty::register::<InfoBarPanel, _>("HorizontalOrientationPadding", Thickness::default())
        }
    );

    ferro_property!(
        /// Defines the `VerticalOrientationPadding` property.
        pub fn vertical_orientation_padding_property() -> StyledProperty<Thickness> {
            FerroProperty::register::<InfoBarPanel, _>("VerticalOrientationPadding", Thickness::default())
        }
    );

    ferro_property!(
        /// Defines the `HorizontalOrientationMargin` attached property.
        pub fn horizontal_orientation_margin_property() -> AttachedProperty<Thickness> {
            FerroProperty::register_attached::<InfoBarPanel, Control, _>("HorizontalOrientationMargin", Thickness::default())
        }
    );

    ferro_property!(
        /// Defines the `VerticalOrientationMargin` attached property.
        pub fn vertical_orientation_margin_property() -> AttachedProperty<Thickness> {
            FerroProperty::register_attached::<InfoBarPanel, Control, _>("VerticalOrientationMargin", Thickness::default())
        }
    );
} }

impl LayoutableImpl for InfoBarPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let mut total_width = 0.0_f64;
        let mut total_height = 0.0_f64;
        let mut width_of_widest = 0.0_f64;
        let mut height_of_tallest = 0.0_f64;
        let mut height_of_tallest_in_horizontal = 0.0_f64;
        let mut n_items = 0_usize;

        // The minimum height of the parent less the vertical margins of the panel (WinUI: the
        // grid of the InfoBar template, InfoBarMinHeight).
        let margin = this.margin();
        let min_height = this
            .parent()
            .and_then(|parent| parent.cast::<Layoutable>())
            .map(|parent| parent.min_height() - (margin.top + margin.bottom))
            .unwrap_or(0.0);

        let children = this.children().snapshot();
        let child_count = children.len();
        for child in children.iter() {
            child.measure(available_size);
            let child_desired_size = child.desired_size();

            if child_desired_size.width != 0.0 && child_desired_size.height != 0.0 {
                // The width of all items if they were laid out horizontally, without the left
                // margin of the first and the right margin of the last child.
                let horizontal_margin = Self::get_horizontal_orientation_margin(child);
                total_width += child_desired_size.width
                    + if n_items > 0 {
                        horizontal_margin.left
                    } else {
                        0.0
                    }
                    + if n_items + 1 < child_count {
                        horizontal_margin.right
                    } else {
                        0.0
                    };

                // The height of all items if they were laid out vertically, without the top
                // margin of the first and the bottom margin of the last child.
                let vertical_margin = Self::get_vertical_orientation_margin(child);
                total_height += child_desired_size.height
                    + if n_items > 0 {
                        vertical_margin.top
                    } else {
                        0.0
                    }
                    + if n_items + 1 < child_count {
                        vertical_margin.bottom
                    } else {
                        0.0
                    };

                width_of_widest = width_of_widest.max(child_desired_size.width);
                height_of_tallest = height_of_tallest.max(child_desired_size.height);

                let child_height_in_horizontal =
                    child_desired_size.height + horizontal_margin.top + horizontal_margin.bottom;
                height_of_tallest_in_horizontal =
                    height_of_tallest_in_horizontal.max(child_height_in_horizontal);

                n_items += 1;
            }
        }

        // One item is laid out vertically (its margins work out better that way); so are items
        // wider than the panel, and items of which one is taller than the minimum height of the
        // parent on one line, although they may seem to fit because their text wraps.
        let is_vertical = n_items == 1
            || total_width > available_size.width
            || (min_height > 0.0 && height_of_tallest_in_horizontal > min_height);
        this.is_vertical.set(is_vertical);

        if is_vertical {
            let padding = this.vertical_orientation_padding();
            Size::new(
                width_of_widest + padding.left + padding.right,
                total_height + padding.top + padding.bottom,
            )
        } else {
            let padding = this.horizontal_orientation_padding();
            Size::new(
                total_width + padding.left + padding.right,
                height_of_tallest + padding.top + padding.bottom,
            )
        }
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();

        if this.is_vertical.get() {
            // The children below each other.
            let padding = this.vertical_orientation_padding();
            let mut vertical_offset = padding.top;
            let mut has_previous_element = false;

            for child in children.iter() {
                let desired_size = child.desired_size();
                if desired_size.width != 0.0 && desired_size.height != 0.0 {
                    let vertical_margin = Self::get_vertical_orientation_margin(child);

                    vertical_offset += if has_previous_element {
                        vertical_margin.top
                    } else {
                        0.0
                    };
                    child.arrange(Rect::new(
                        padding.left + vertical_margin.left,
                        vertical_offset,
                        desired_size.width,
                        desired_size.height,
                    ));
                    vertical_offset += desired_size.height + vertical_margin.bottom;

                    has_previous_element = true;
                }
            }
        } else {
            // The children side by side.
            let padding = this.horizontal_orientation_padding();
            let mut horizontal_offset = padding.left;
            let mut has_previous_element = false;

            let child_count = children.len();
            for (i, child) in children.iter().enumerate() {
                let desired_size = child.desired_size();
                if desired_size.width != 0.0 && desired_size.height != 0.0 {
                    let horizontal_margin = Self::get_horizontal_orientation_margin(child);

                    horizontal_offset += if has_previous_element {
                        horizontal_margin.left
                    } else {
                        0.0
                    };
                    let top = padding.top + horizontal_margin.top;
                    if i + 1 < child_count {
                        child.arrange(Rect::new(
                            horizontal_offset,
                            top,
                            desired_size.width,
                            desired_size.height,
                        ));
                    } else {
                        // The rest of the width to the last child.
                        child.arrange(Rect::new(
                            horizontal_offset,
                            top,
                            desired_size.width.max(final_size.width - horizontal_offset),
                            desired_size.height,
                        ));
                    }

                    horizontal_offset += desired_size.width + horizontal_margin.right;

                    has_previous_element = true;
                }
            }
        }

        final_size
    }
}

impl InfoBarPanel {
    fn static_constructor() {
        Panel::affects_parent_measure::<InfoBarPanel>(&[
            Self::horizontal_orientation_margin_property().as_property(),
            Self::vertical_orientation_margin_property().as_property(),
        ]);
        Layoutable::affects_measure::<InfoBarPanel>(&[
            Self::horizontal_orientation_padding_property().as_property(),
            Self::vertical_orientation_padding_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            is_vertical: Cell::new(false),
        }
    }

    /// Creates an InfoBar panel.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The padding of the panel when its children are on one line.
    pub fn horizontal_orientation_padding(&self) -> Thickness {
        self.get_value(Self::horizontal_orientation_padding_property())
    }

    pub fn set_horizontal_orientation_padding(&self, value: Thickness) {
        self.set_value(Self::horizontal_orientation_padding_property(), value)
    }

    /// The padding of the panel when its children are below each other.
    pub fn vertical_orientation_padding(&self) -> Thickness {
        self.get_value(Self::vertical_orientation_padding_property())
    }

    pub fn set_vertical_orientation_padding(&self, value: Thickness) {
        self.set_value(Self::vertical_orientation_padding_property(), value)
    }

    /// Gets the margin of `control` when the children of the panel are on one line.
    pub fn get_horizontal_orientation_margin(control: &Control) -> Thickness {
        control.get_value(Self::horizontal_orientation_margin_property())
    }

    /// Sets the margin of `control` when the children of the panel are on one line.
    pub fn set_horizontal_orientation_margin(control: &Control, value: Thickness) {
        control.set_value(Self::horizontal_orientation_margin_property(), value)
    }

    /// Gets the margin of `control` when the children of the panel are below each other.
    pub fn get_vertical_orientation_margin(control: &Control) -> Thickness {
        control.get_value(Self::vertical_orientation_margin_property())
    }

    /// Sets the margin of `control` when the children of the panel are below each other.
    pub fn set_vertical_orientation_margin(control: &Control, value: Thickness) {
        control.set_value(Self::vertical_orientation_margin_property(), value)
    }
}
