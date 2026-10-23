//! Port of `Diagnostics/ViewModels/ControlLayoutViewModel.cs`.

use super::ViewModelBase;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::{
    ferro_markup_type, FerroObject, FerroPropertyChangedEventArgs, FerroPropertyRegistry, Ref, StyledProperty,
    Thickness, Visual,
};
use ferroui_controls::{Border, Decorator};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The layout of a control as the layout explorer shows it: the margin, the border and the
/// padding (which edit the control), the size and its constraints, and the alignments.
pub struct ControlLayoutViewModel {
    base: ViewModelBase,
    control: Ref<Visual>,
    border_thickness: RefCell<Thickness>,
    height: RefCell<f64>,
    height_constraint: RefCell<Option<String>>,
    horizontal_alignment: RefCell<HorizontalAlignment>,
    margin_thickness: RefCell<Thickness>,
    padding_thickness: RefCell<Thickness>,
    updating_from_control: Cell<bool>,
    vertical_alignment: RefCell<VerticalAlignment>,
    width: RefCell<f64>,
    width_constraint: RefCell<Option<String>>,
    has_padding: bool,
    has_border: bool,
}

impl PartialEq for ControlLayoutViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ControlLayoutViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

/// `Math.Round(value, 2)`: to two decimals, a midpoint to the even neighbour.
pub(crate) fn round2(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    (value * 100.0).round_ties_even() / 100.0
}

/// A number as the managed original formats it into text (`{0}`): the shortest form that
/// reads back, infinities as the symbol.
pub(crate) fn format_number(value: f64) -> String {
    if value == f64::INFINITY {
        "\u{221e}".to_string()
    } else if value == f64::NEG_INFINITY {
        "-\u{221e}".to_string()
    } else if value.is_nan() {
        "NaN".to_string()
    } else {
        format!("{value}")
    }
}

impl ControlLayoutViewModel {
    pub fn new(control: &Visual) -> Rc<Self> {
        let registry = FerroPropertyRegistry::instance();
        let has_padding = registry.is_registered_for(control, Decorator::padding_property().as_property());
        let has_border = registry.is_registered_for(control, Border::border_thickness_property().as_property());

        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            control: control.to_ref(),
            border_thickness: RefCell::new(Thickness::default()),
            height: RefCell::new(0.0),
            height_constraint: RefCell::new(None),
            horizontal_alignment: RefCell::new(HorizontalAlignment::Stretch),
            margin_thickness: RefCell::new(Thickness::default()),
            padding_thickness: RefCell::new(Thickness::default()),
            updating_from_control: Cell::new(false),
            vertical_alignment: RefCell::new(VerticalAlignment::Stretch),
            width: RefCell::new(0.0),
            width_constraint: RefCell::new(None),
            has_padding,
            has_border,
        });

        // `control is FerroObject ao`: a visual always is one.
        {
            let ao: &FerroObject = &this.control;

            // `try { .. } finally { _updatingFromControl = false; }`: nothing in between can fail.
            this.updating_from_control.set(true);
            this.set_margin_thickness(ao.get_value(Layoutable::margin_property()));

            if this.has_padding {
                this.set_padding_thickness(ao.get_value(Decorator::padding_property()));
            }

            if this.has_border {
                this.set_border_thickness(ao.get_value(Border::border_thickness_property()));
            }

            this.set_horizontal_alignment(ao.get_value(Layoutable::horizontal_alignment_property()));
            this.set_vertical_alignment(ao.get_value(Layoutable::vertical_alignment_property()));
            this.updating_from_control.set(false);
        }

        this.update_size();
        this.update_size_constraints();

        this
    }

    pub fn margin_thickness(&self) -> Thickness {
        *self.margin_thickness.borrow()
    }

    pub fn set_margin_thickness(&self, value: Thickness) {
        self.raise_and_set_if_changed(&self.margin_thickness, value, "MarginThickness");
    }

    pub fn border_thickness(&self) -> Thickness {
        *self.border_thickness.borrow()
    }

    pub fn set_border_thickness(&self, value: Thickness) {
        self.raise_and_set_if_changed(&self.border_thickness, value, "BorderThickness");
    }

    pub fn padding_thickness(&self) -> Thickness {
        *self.padding_thickness.borrow()
    }

    pub fn set_padding_thickness(&self, value: Thickness) {
        self.raise_and_set_if_changed(&self.padding_thickness, value, "PaddingThickness");
    }

    pub fn width(&self) -> f64 {
        *self.width.borrow()
    }

    fn set_width(&self, value: f64) {
        self.raise_and_set_if_changed(&self.width, value, "Width");
    }

    pub fn height(&self) -> f64 {
        *self.height.borrow()
    }

    fn set_height(&self, value: f64) {
        self.raise_and_set_if_changed(&self.height, value, "Height");
    }

    pub fn width_constraint(&self) -> Option<String> {
        self.width_constraint.borrow().clone()
    }

    fn set_width_constraint(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.width_constraint, value, "WidthConstraint");
    }

    pub fn height_constraint(&self) -> Option<String> {
        self.height_constraint.borrow().clone()
    }

    fn set_height_constraint(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.height_constraint, value, "HeightConstraint");
    }

    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        *self.horizontal_alignment.borrow()
    }

    pub fn set_horizontal_alignment(&self, value: HorizontalAlignment) {
        self.raise_and_set_if_changed(&self.horizontal_alignment, value, "HorizontalAlignment");
    }

    pub fn vertical_alignment(&self) -> VerticalAlignment {
        *self.vertical_alignment.borrow()
    }

    pub fn set_vertical_alignment(&self, value: VerticalAlignment) {
        self.raise_and_set_if_changed(&self.vertical_alignment, value, "VerticalAlignment");
    }

    pub fn has_padding(&self) -> bool {
        self.has_padding
    }

    pub fn has_border(&self) -> bool {
        self.has_border
    }

    /// `RaiseAndSetIfChanged` of the base class, through this class's `RaisePropertyChanged`,
    /// which runs the override of `OnPropertyChanged` first.
    fn raise_and_set_if_changed<T: PartialEq>(&self, field: &RefCell<T>, value: T, property_name: &str) -> bool {
        if *field.borrow() == value {
            return false;
        }
        *field.borrow_mut() = value;
        self.raise_property_changed(property_name);
        true
    }

    /// `RaisePropertyChanged`: the override of `OnPropertyChanged`, then the handlers.
    fn raise_property_changed(&self, property_name: &str) {
        self.on_property_changed(property_name);
        self.base.raise_property_changed(property_name);
    }

    fn update_size_constraints(&self) {
        let ao: &FerroObject = &self.control;

        let create_constraint_info =
            |min_property: &'static StyledProperty<f64>, max_property: &'static StyledProperty<f64>| -> Option<String> {
                let has_min = ao.is_set(min_property.as_property());
                let has_max = ao.is_set(max_property.as_property());

                if has_min || has_max {
                    let mut builder = String::new();

                    if has_min {
                        let min_value = ao.get_value(min_property);
                        builder.push_str(&format!("Min: {}", format_number(round2(min_value))));
                        // `AppendLine()`.
                        builder.push('\n');
                    }

                    if has_max {
                        let max_value = ao.get_value(max_property);
                        builder.push_str(&format!("Max: {}", format_number(round2(max_value))));
                    }

                    return Some(builder);
                }

                None
            };

        self.set_width_constraint(create_constraint_info(
            Layoutable::min_width_property(),
            Layoutable::max_width_property(),
        ));
        self.set_height_constraint(create_constraint_info(
            Layoutable::min_height_property(),
            Layoutable::max_height_property(),
        ));
    }

    /// The override of `OnPropertyChanged`: an edit of the view model sets the property of the
    /// control, unless the change comes from the control.
    fn on_property_changed(&self, property_name: &str) {
        if self.updating_from_control.get() {
            return;
        }

        let ao: &FerroObject = &self.control;

        if property_name == "MarginThickness" {
            ao.set_value(Layoutable::margin_property(), self.margin_thickness());
        } else if self.has_padding && property_name == "PaddingThickness" {
            ao.set_value(Decorator::padding_property(), self.padding_thickness());
        } else if self.has_border && property_name == "BorderThickness" {
            ao.set_value(Border::border_thickness_property(), self.border_thickness());
        } else if property_name == "HorizontalAlignment" {
            ao.set_value(Layoutable::horizontal_alignment_property(), self.horizontal_alignment());
        } else if property_name == "VerticalAlignment" {
            ao.set_value(Layoutable::vertical_alignment_property(), self.vertical_alignment());
        }
    }

    /// Follows a change of a property of the control.
    pub fn control_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        // `try { .. } finally { _updatingFromControl = false; }`: nothing in between can fail.
        self.updating_from_control.set(true);

        let property = e.property();

        if property == Visual::bounds_property().as_property() {
            self.update_size();
        } else {
            let ao: &FerroObject = &self.control;

            if property == Layoutable::margin_property().as_property() {
                self.set_margin_thickness(ao.get_value(Layoutable::margin_property()));
            } else if property == Decorator::padding_property().as_property() {
                self.set_padding_thickness(ao.get_value(Decorator::padding_property()));
            } else if property == Border::border_thickness_property().as_property() {
                self.set_border_thickness(ao.get_value(Border::border_thickness_property()));
            } else if property == Layoutable::min_width_property().as_property()
                || property == Layoutable::max_width_property().as_property()
                || property == Layoutable::min_height_property().as_property()
                || property == Layoutable::max_height_property().as_property()
            {
                self.update_size_constraints();
            } else if property == Layoutable::horizontal_alignment_property().as_property() {
                self.set_horizontal_alignment(ao.get_value(Layoutable::horizontal_alignment_property()));
            } else if property == Layoutable::vertical_alignment_property().as_property() {
                self.set_vertical_alignment(ao.get_value(Layoutable::vertical_alignment_property()));
            }
        }

        self.updating_from_control.set(false);
    }

    fn update_size(&self) {
        let size = self.control.bounds();

        self.set_width(round2(size.width));
        self.set_height(round2(size.height));
    }
}

ferro_markup_type!(class ControlLayoutViewModel {
    this: Rc<ControlLayoutViewModel>,
    handles: [ControlLayoutViewModel, Rc<ControlLayoutViewModel>, Option<Rc<ControlLayoutViewModel>>],
    properties: [
        MarginThickness: Thickness {
            get: ControlLayoutViewModel::margin_thickness,
            set: ControlLayoutViewModel::set_margin_thickness
        },
        BorderThickness: Thickness {
            get: ControlLayoutViewModel::border_thickness,
            set: ControlLayoutViewModel::set_border_thickness
        },
        PaddingThickness: Thickness {
            get: ControlLayoutViewModel::padding_thickness,
            set: ControlLayoutViewModel::set_padding_thickness
        },
        Width: f64 { get: ControlLayoutViewModel::width },
        Height: f64 { get: ControlLayoutViewModel::height },
        WidthConstraint: Option<String> { get: ControlLayoutViewModel::width_constraint },
        HeightConstraint: Option<String> { get: ControlLayoutViewModel::height_constraint },
        HorizontalAlignment: HorizontalAlignment {
            get: ControlLayoutViewModel::horizontal_alignment,
            set: ControlLayoutViewModel::set_horizontal_alignment
        },
        VerticalAlignment: VerticalAlignment {
            get: ControlLayoutViewModel::vertical_alignment,
            set: ControlLayoutViewModel::set_vertical_alignment
        },
        HasPadding: bool { get: ControlLayoutViewModel::has_padding },
        HasBorder: bool { get: ControlLayoutViewModel::has_border },
    ],
    notify_property_changed: ControlLayoutViewModel,
});
