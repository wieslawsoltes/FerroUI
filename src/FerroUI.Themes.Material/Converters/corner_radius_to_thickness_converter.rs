//! Converts a corner radius to a thickness: the `CornerRadiusToThicknessConverter` of
//! microsoft-ui-xaml (`controls/dev/Common/CornerRadiusToThicknessConverter.idl`, `.h`, `.cpp`,
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0; Copyright (c) Microsoft Corporation,
//! licensed under the MIT License, see NOTICE.md of the crate), which the theme resources
//! `TopThicknessFilterConverter`, `BottomThicknessFilterConverter`, `LeftThicknessFilterConverter`,
//! `RightThicknessFilterConverter`, `TabViewLeftInsetCornerConverter` and
//! `TabViewRightInsetCornerConverter` of `CornerRadius_themeresources.xaml` are instances of.
//!
//! WinUI's class is a dependency object whose `ConversionKind` and `Multiplier` are dependency
//! properties; here it is a plain class with plain properties, as the converters of FerroUI's
//! control set (`ferroui_controls::converters::CornerRadiusFilterConverter`): the resources set
//! them once, and nothing binds them.

use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_enum, ferro_markup_type, BoxedValue, CornerRadius, FerroProperty, Thickness};
use std::cell::Cell;
use std::rc::Rc;

/// Which corners of a corner radius become which sides of the thickness
/// (`CornerRadiusToThicknessConverterKind` of WinUI, with its members in the order of the source).
/// The sides a kind does not name are 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CornerRadiusToThicknessConverterKind {
    /// Top from the top left corner, bottom from the bottom left corner.
    FilterTopAndBottomFromLeft,
    /// Top from the top right corner, bottom from the bottom right corner.
    FilterTopAndBottomFromRight,
    /// Left from the top left corner, right from the top right corner (the default of WinUI).
    #[default]
    FilterLeftAndRightFromTop,
    /// Left from the bottom left corner, right from the bottom right corner.
    FilterLeftAndRightFromBottom,
    /// Top from the top left corner.
    FilterTopFromTopLeft,
    /// Top from the top right corner.
    FilterTopFromTopRight,
    /// Right from the top right corner.
    FilterRightFromTopRight,
    /// Right from the bottom right corner.
    FilterRightFromBottomRight,
    /// Bottom from the bottom right corner.
    FilterBottomFromBottomRight,
    /// Bottom from the bottom left corner.
    FilterBottomFromBottomLeft,
    /// Left from the bottom left corner.
    FilterLeftFromBottomLeft,
    /// Left from the top left corner.
    FilterLeftFromTopLeft,
}

ferro_markup_enum!(
    CornerRadiusToThicknessConverterKind {
        FilterTopAndBottomFromLeft,
        FilterTopAndBottomFromRight,
        FilterLeftAndRightFromTop,
        FilterLeftAndRightFromBottom,
        FilterTopFromTopLeft,
        FilterTopFromTopRight,
        FilterRightFromTopRight,
        FilterRightFromBottomRight,
        FilterBottomFromBottomRight,
        FilterBottomFromBottomLeft,
        FilterLeftFromBottomLeft,
        FilterLeftFromTopLeft,
    },
    { namespace: "FerroUI.Themes.Material.Converters" }
);

/// Converts a corner radius to a thickness whose sides are corners of the radius, chosen by the
/// conversion kind and multiplied by the multiplier.
pub struct CornerRadiusToThicknessConverter {
    conversion_kind: Cell<CornerRadiusToThicknessConverterKind>,
    multiplier: Cell<f64>,
}

impl Default for CornerRadiusToThicknessConverter {
    fn default() -> Self {
        Self::new()
    }
}

// A converter is a reference type: an instance equals itself.
impl PartialEq for CornerRadiusToThicknessConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CornerRadiusToThicknessConverter {
    /// Creates a converter of the default kind (`FilterLeftAndRightFromTop`) with the multiplier 1.
    pub fn new() -> Self {
        Self { conversion_kind: Cell::new(CornerRadiusToThicknessConverterKind::default()), multiplier: Cell::new(1.0) }
    }

    /// Gets which corners become which sides of the thickness.
    pub fn conversion_kind(&self) -> CornerRadiusToThicknessConverterKind {
        self.conversion_kind.get()
    }

    /// Sets which corners become which sides of the thickness.
    pub fn set_conversion_kind(&self, value: CornerRadiusToThicknessConverterKind) {
        self.conversion_kind.set(value)
    }

    /// Gets the factor every side of the thickness is multiplied by.
    pub fn multiplier(&self) -> f64 {
        self.multiplier.get()
    }

    /// Sets the factor every side of the thickness is multiplied by.
    pub fn set_multiplier(&self, value: f64) {
        self.multiplier.set(value)
    }

    /// The thickness of a corner radius for a conversion kind and a multiplier (the `Convert`
    /// overload of WinUI that takes the radius, the kind and the multiplier).
    pub fn convert_radius(
        radius: CornerRadius,
        kind: CornerRadiusToThicknessConverterKind,
        multiplier: f64,
    ) -> Thickness {
        use CornerRadiusToThicknessConverterKind as Kind;

        match kind {
            Kind::FilterLeftAndRightFromTop => {
                Thickness::new(radius.top_left * multiplier, 0.0, radius.top_right * multiplier, 0.0)
            }
            Kind::FilterLeftAndRightFromBottom => {
                Thickness::new(radius.bottom_left * multiplier, 0.0, radius.bottom_right * multiplier, 0.0)
            }
            Kind::FilterTopAndBottomFromLeft => {
                Thickness::new(0.0, radius.top_left * multiplier, 0.0, radius.bottom_left * multiplier)
            }
            Kind::FilterTopAndBottomFromRight => {
                Thickness::new(0.0, radius.top_right * multiplier, 0.0, radius.bottom_right * multiplier)
            }
            Kind::FilterTopFromTopLeft => Thickness::new(0.0, radius.top_left * multiplier, 0.0, 0.0),
            Kind::FilterTopFromTopRight => Thickness::new(0.0, radius.top_right * multiplier, 0.0, 0.0),
            Kind::FilterRightFromTopRight => Thickness::new(0.0, 0.0, radius.top_right * multiplier, 0.0),
            Kind::FilterRightFromBottomRight => Thickness::new(0.0, 0.0, radius.bottom_right * multiplier, 0.0),
            Kind::FilterBottomFromBottomRight => Thickness::new(0.0, 0.0, 0.0, radius.bottom_right * multiplier),
            Kind::FilterBottomFromBottomLeft => Thickness::new(0.0, 0.0, 0.0, radius.bottom_left * multiplier),
            Kind::FilterLeftFromBottomLeft => Thickness::new(radius.bottom_left * multiplier, 0.0, 0.0, 0.0),
            Kind::FilterLeftFromTopLeft => Thickness::new(radius.top_left * multiplier, 0.0, 0.0, 0.0),
        }
    }
}

impl IValueConverter for CornerRadiusToThicknessConverter {
    /// The thickness of a corner radius; a value that is not a corner radius converts to the
    /// unset value (WinUI fails to unbox it).
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(radius) = cast_value::<CornerRadius>(value) else {
            return Ok(Some(FerroProperty::unset_value()));
        };

        Ok(Some(Rc::new(Self::convert_radius(radius, self.conversion_kind.get(), self.multiplier.get()))))
    }

    /// Fails: the conversion has no inverse (`E_NOTIMPL` in WinUI).
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}

ferro_markup_type!(class CornerRadiusToThicknessConverter {
    namespace: "FerroUI.Themes.Material.Converters",
    handles: [Rc<CornerRadiusToThicknessConverter>, Option<Rc<CornerRadiusToThicknessConverter>>],
    this: Rc<CornerRadiusToThicknessConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(CornerRadiusToThicknessConverter::new())],
    properties: [
        ConversionKind: CornerRadiusToThicknessConverterKind {
            get: |c: &Rc<CornerRadiusToThicknessConverter>| c.conversion_kind(),
            set: |c: &Rc<CornerRadiusToThicknessConverter>, value: CornerRadiusToThicknessConverterKind| c.set_conversion_kind(value)
        },
        Multiplier: f64 {
            get: |c: &Rc<CornerRadiusToThicknessConverter>| c.multiplier(),
            set: |c: &Rc<CornerRadiusToThicknessConverter>, value: f64| c.set_multiplier(value)
        },
    ],
});
