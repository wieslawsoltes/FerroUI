// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::internal::*;
use crate::ProportionalStackPanelSplitter;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    AttachedProperty, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl, Panel, PanelImpl};
use std::cell::Cell;
/// A panel arranging children horizontally or vertically using the source proportional sizing policy.
#[repr(C)]
pub struct ProportionalStackPanel {
    base: Panel,
    is_assigning_proportions: Cell<bool>,
}
ferro_class! {
    ProportionalStackPanel:Panel,virtuals ProportionalStackPanelImpl:PanelImpl {
        /// Assigns child proportions before measurement or arrangement.
        fn on_assign_proportions(this,size:Size,splitter_thickness:f64);
        /// Resolves the split-axis length after excluding visible splitter gaps.
        fn get_proportional_dimension(this,size:Size,splitter_thickness:f64)->f64;
        /// Resolves the finite constraint used by proportional measurement.
        fn get_measure_constraint(this,constraint:Size,splitter_thickness:f64)->Size;
        /// Resolves the individual pane's measure constraint.
        fn get_child_measure_constraint(this,child:Ref<Control>,constraint:Size)->Size;
        /// Resizes two panes through the native splitter input policy.
        fn resize_proportions(this,target:Ref<Control>,neighbor:Ref<Control>,drag_delta:f64,minimum_size:f64);
    }
}
ferro_class_info!(ProportionalStackPanel {
    new: ProportionalStackPanel::new
});
ferro_impl_classes!(ProportionalStackPanel:StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {
    impl ProportionalStackPanel {
        /// Identifies the split orientation, Vertical by default.
        pub fn orientation_property()->StyledProperty<Orientation> {FerroProperty::register::<ProportionalStackPanel,_>("Orientation",Orientation::Vertical)}
        /// Identifies the two-way Proportion attached property, initially NaN.
        pub fn proportion_property()->AttachedProperty<f64> {FerroProperty::register_attached_with::<ProportionalStackPanel,Control,_>("Proportion",StyledPropertyOptions::new(f64::NAN).default_binding_mode(BindingMode::TwoWay))}
        /// Identifies the two-way collapsed-state attached property.
        pub fn is_collapsed_property()->AttachedProperty<bool> {FerroProperty::register_attached_with::<ProportionalStackPanel,Control,_>("IsCollapsed",StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay))}
        /// Identifies the two-way stored collapsed proportion, initially NaN.
        pub fn collapsed_proportion_property()->AttachedProperty<f64> {FerroProperty::register_attached_with::<ProportionalStackPanel,Control,_>("CollapsedProportion",StyledPropertyOptions::new(f64::NAN).default_binding_mode(BindingMode::TwoWay))}
    }
}
impl ProportionalStackPanel {
    fn static_constructor() {
        Panel::affects_parent_measure::<Self>(&[Self::is_collapsed_property().as_property()]);
        Panel::affects_parent_arrange::<Self>(&[Self::is_collapsed_property().as_property()]);
        Self::proportion_property()
            .changed()
            .add_class_handler::<Control>(|sender, e| {
                let Some(parent) = sender
                    .visual_parent()
                    .and_then(|parent| parent.cast::<ProportionalStackPanel>())
                else {
                    return;
                };
                if parent.is_assigning_proportions.get() {
                    return;
                }
                let value = e.get_new_value::<f64>();
                if !Self::get_is_collapsed(sender) && !value.is_nan() {
                    Self::set_collapsed_proportion(sender, value);
                }
                parent.invalidate_measure();
                parent.invalidate_arrange();
            });
        Self::collapsed_proportion_property()
            .changed()
            .add_class_handler::<Control>(|sender, _| {
                let Some(parent) = sender
                    .visual_parent()
                    .and_then(|parent| parent.cast::<ProportionalStackPanel>())
                else {
                    return;
                };
                if parent.is_assigning_proportions.get() {
                    return;
                }
                parent.invalidate_measure();
                parent.invalidate_arrange();
            });
    }
    /// Initializes fields for this panel or a derived sizing-policy class.
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            is_assigning_proportions: Cell::new(false),
        }
    }
    /// Creates the panel using the framework's native class model.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the direction used to measure and arrange children.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    /// Sets the direction and invalidates measurement through the source property hook.
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value);
    }
    /// Gets the attached proportion.
    pub fn get_proportion(control: &FerroObject) -> f64 {
        control.get_value(Self::proportion_property())
    }
    /// Sets the current attached proportion, retaining any existing binding.
    pub fn set_proportion(control: &FerroObject, value: f64) {
        control.set_current_value(Self::proportion_property(), value);
    }
    /// Gets the attached collapsed state.
    pub fn get_is_collapsed(control: &FerroObject) -> bool {
        control.get_value(Self::is_collapsed_property())
    }
    /// Sets the attached collapsed state.
    pub fn set_is_collapsed(control: &FerroObject, value: bool) {
        control.set_value(Self::is_collapsed_property(), value);
    }
    /// Gets the attached stored proportion used on expansion.
    pub fn get_collapsed_proportion(control: &FerroObject) -> f64 {
        control.get_value(Self::collapsed_proportion_property())
    }
    /// Sets the attached stored proportion used on expansion.
    pub fn set_collapsed_proportion(control: &FerroObject, value: f64) {
        control.set_value(Self::collapsed_proportion_property(), value);
    }
    fn assign_proportions(&self, size: Size, splitter: f64) {
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        self.is_assigning_proportions.set(true);
        let _reset = Reset(&self.is_assigning_proportions);
        self.on_assign_proportions(size, splitter);
    }
}
impl FerroObjectImpl for ProportionalStackPanel {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::orientation_property().as_property() {
            let children = this.children();
            let mut index = 0;
            while index < children.count() {
                if let Some(splitter) =
                    ProportionalStackPanelSplitter::is_splitter(Some(&children.get(index)))
                {
                    splitter.update_visual_state();
                }
                index += 1;
            }
            this.invalidate_measure();
        }
    }
}
impl ProportionalStackPanelImpl for ProportionalStackPanel {
    fn on_assign_proportions(this: &Self, size: Size, splitter_thickness: f64) {
        ProportionManager::new(
            &this.children(),
            size,
            splitter_thickness,
            this.orientation(),
        )
        .assign_proportions();
    }
    fn get_proportional_dimension(this: &Self, size: Size, splitter_thickness: f64) -> f64 {
        ProportionUtils::get_relevant_dimension(size, this.orientation()) - splitter_thickness
    }
    fn get_measure_constraint(this: &Self, constraint: Size, _splitter_thickness: f64) -> Size {
        let horizontal = this.orientation() == Orientation::Horizontal;
        if constraint == Size::INFINITY
            || (horizontal && constraint.width.is_infinite())
            || (!horizontal && constraint.height.is_infinite())
        {
            panic!(
                "Proportional StackPanel cannot be inside a control that offers infinite space."
            );
        }
        constraint
    }
    fn get_child_measure_constraint(_this: &Self, _child: Ref<Control>, constraint: Size) -> Size {
        constraint
    }
    fn resize_proportions(
        this: &Self,
        target: Ref<Control>,
        neighbor: Ref<Control>,
        drag_delta: f64,
        minimum_size: f64,
    ) {
        ProportionResizer::new(this, &target, &neighbor, drag_delta, minimum_size).apply_resize();
    }
}
impl LayoutableImpl for ProportionalStackPanel {
    fn measure_override(this: &Self, constraint: Size) -> Size {
        let mut used_width = 0.0;
        let mut used_height = 0.0;
        let mut maximum_width = 0.0;
        let mut maximum_height = 0.0;
        let children = this.children();
        let splitter_thickness =
            SplitterCalculator::get_total_splitter_thickness(&children, Self::get_is_collapsed);
        let constraint = this.get_measure_constraint(constraint, splitter_thickness);
        this.assign_proportions(constraint, splitter_thickness);
        let dimension = this.get_proportional_dimension(constraint, splitter_thickness);
        let mut needs_next_splitter = false;
        let mut fractions = 0.0;
        let mut index = 0;
        while index < children.count() {
            let control = children.get(index);
            index += 1;
            let is_splitter = ProportionalStackPanelSplitter::is_splitter(Some(&control)).is_some();
            let remaining = Size::new(
                max(0.0, constraint.width - used_width - splitter_thickness),
                max(0.0, constraint.height - used_height - splitter_thickness),
            );
            let proportion = Self::get_proportion(&control);
            if !is_splitter && Self::get_is_collapsed(&control) {
                control.measure(Size::default());
                continue;
            }
            if !is_splitter {
                let size = if this.orientation() == Orientation::Horizontal {
                    let width = DimensionCalculator::calculate_dimension_with_constraints(
                        &control,
                        Orientation::Horizontal,
                        dimension,
                        proportion,
                        &mut fractions,
                    );
                    Size::new(width, constraint.height)
                } else {
                    let height = DimensionCalculator::calculate_dimension_with_constraints(
                        &control,
                        Orientation::Vertical,
                        dimension,
                        proportion,
                        &mut fractions,
                    );
                    Size::new(constraint.width, height)
                };
                control.measure(this.get_child_measure_constraint(control.clone(), size));
                needs_next_splitter = true;
            } else {
                if !needs_next_splitter {
                    control.measure(Size::default());
                    continue;
                }
                control.measure(remaining);
                needs_next_splitter = false;
            }
            let desired = control.desired_size();
            if this.orientation() == Orientation::Horizontal {
                maximum_height = max(maximum_height, used_height + desired.height);
                if is_splitter {
                    used_width += desired.width;
                } else {
                    used_width += DimensionCalculator::calculate_dimension_with_constraints(
                        &control,
                        Orientation::Horizontal,
                        dimension,
                        proportion,
                        &mut fractions,
                    );
                }
            } else {
                maximum_width = max(maximum_width, used_width + desired.width);
                if is_splitter {
                    used_height += desired.height;
                } else {
                    used_height += DimensionCalculator::calculate_dimension_with_constraints(
                        &control,
                        Orientation::Vertical,
                        dimension,
                        proportion,
                        &mut fractions,
                    );
                }
            }
        }
        Size::new(
            max(maximum_width, used_width),
            max(maximum_height, used_height),
        )
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        let mut left = 0.0;
        let mut top = 0.0;
        let children = this.children();
        let splitter =
            SplitterCalculator::get_total_splitter_thickness(&children, Self::get_is_collapsed);
        this.assign_proportions(size, splitter);
        let dimension = this.get_proportional_dimension(size, splitter);
        let mut needs_next_splitter = false;
        let mut fractions = 0.0;
        let mut index = 0;
        while index < children.count() {
            let control = children.get(index);
            let is_splitter = ProportionalStackPanelSplitter::is_splitter(Some(&control)).is_some();
            if !is_splitter && Self::get_is_collapsed(&control) {
                control.arrange(Rect::default());
                index += 1;
                continue;
            }
            if !is_splitter {
                needs_next_splitter = true;
            } else if !needs_next_splitter {
                control.arrange(Rect::default());
                index += 1;
                continue;
            }
            let mut remaining = Rect::new(
                left,
                top,
                max(0.0, size.width - left),
                max(0.0, size.height - top),
            );
            if index < children.count() {
                let desired = control.desired_size();
                let proportion = Self::get_proportion(&control);
                if this.orientation() == Orientation::Horizontal {
                    let width = if is_splitter {
                        desired.width
                    } else {
                        DimensionCalculator::calculate_dimension_with_constraints(
                            &control,
                            Orientation::Horizontal,
                            dimension,
                            proportion,
                            &mut fractions,
                        )
                    };
                    remaining = Rect::new(remaining.x, remaining.y, width, remaining.height);
                    left += width;
                } else {
                    let height = if is_splitter {
                        desired.height
                    } else {
                        DimensionCalculator::calculate_dimension_with_constraints(
                            &control,
                            Orientation::Vertical,
                            dimension,
                            proportion,
                            &mut fractions,
                        )
                    };
                    remaining = Rect::new(remaining.x, remaining.y, remaining.width, height);
                    top += height;
                }
            }
            control.arrange(remaining);
            index += 1;
        }
        size
    }
}
