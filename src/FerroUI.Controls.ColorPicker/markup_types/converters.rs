//! Markup metadata of the converters of this crate (`FerroUI.Controls.Converters`
//! and `FerroUI.Controls.Primitives.Converters`): plain classes held through
//! `Rc`, each assignable to the converter contract.

use crate::converters::{
    ColorToDisplayNameConverter, ColorToHexConverter, DoNothingForNullConverter, ToBrushConverter, ToColorConverter,
};
use crate::primitives::converters::{AccentColorConverter, ContrastBrushConverter};
use crate::AlphaComponentPosition;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::media::HsvColor;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

// The converters are reference types: an instance equals itself.
macro_rules! identity_eq {
    ($($type_:ty),*) => {
        $(impl PartialEq for $type_ {
            fn eq(&self, other: &Self) -> bool {
                std::ptr::eq(self, other)
            }
        })*
    };
}
identity_eq!(
    AccentColorConverter,
    ColorToDisplayNameConverter,
    ColorToHexConverter,
    ContrastBrushConverter,
    DoNothingForNullConverter,
    ToBrushConverter,
    ToColorConverter
);

// FerroUI.Controls.Converters

ferro_markup_type!(class ColorToDisplayNameConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<ColorToDisplayNameConverter>, Option<Rc<ColorToDisplayNameConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(ColorToDisplayNameConverter::new())],
});

ferro_markup_type!(class ColorToHexConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<ColorToHexConverter>, Option<Rc<ColorToHexConverter>>],
    this: Rc<ColorToHexConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(ColorToHexConverter::new())],
    properties: [
        IsAlphaVisible: bool {
            get: |c: &Rc<ColorToHexConverter>| c.is_alpha_visible(),
            set: |c: &Rc<ColorToHexConverter>, value: bool| c.set_is_alpha_visible(value)
        },
        AlphaPosition: AlphaComponentPosition {
            get: |c: &Rc<ColorToHexConverter>| c.alpha_position(),
            set: |c: &Rc<ColorToHexConverter>, value: AlphaComponentPosition| c.set_alpha_position(value)
        },
    ],
});

ferro_markup_type!(class DoNothingForNullConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<DoNothingForNullConverter>, Option<Rc<DoNothingForNullConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(DoNothingForNullConverter::new())],
});

ferro_markup_type!(class ToBrushConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<ToBrushConverter>, Option<Rc<ToBrushConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(ToBrushConverter::new())],
});

ferro_markup_type!(class ToColorConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<ToColorConverter>, Option<Rc<ToColorConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(ToColorConverter::new())],
});

// FerroUI.Controls.Primitives.Converters

ferro_markup_type!(class AccentColorConverter {
    namespace: "FerroUI.Controls.Primitives.Converters",
    handles: [Rc<AccentColorConverter>, Option<Rc<AccentColorConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(AccentColorConverter::new())],
    methods: [
        static fn GetAccent(HsvColor, i32) -> HsvColor => AccentColorConverter::get_accent,
    ],
    fields: [ValueDelta: f64 => || AccentColorConverter::VALUE_DELTA],
});

ferro_markup_type!(class ContrastBrushConverter {
    namespace: "FerroUI.Controls.Primitives.Converters",
    handles: [Rc<ContrastBrushConverter>, Option<Rc<ContrastBrushConverter>>],
    this: Rc<ContrastBrushConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(ContrastBrushConverter::new())],
    properties: [
        AlphaThreshold: u8 {
            get: |c: &Rc<ContrastBrushConverter>| c.alpha_threshold(),
            set: |c: &Rc<ContrastBrushConverter>, value: u8| c.set_alpha_threshold(value)
        },
    ],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <ColorToDisplayNameConverter as MarkupTyped>::MARKUP,
    <ColorToHexConverter as MarkupTyped>::MARKUP,
    <DoNothingForNullConverter as MarkupTyped>::MARKUP,
    <ToBrushConverter as MarkupTyped>::MARKUP,
    <ToColorConverter as MarkupTyped>::MARKUP,
    <AccentColorConverter as MarkupTyped>::MARKUP,
    <ContrastBrushConverter as MarkupTyped>::MARKUP,
];

macro_rules! register_converters {
    ($($type_:ty),*) => {
        $(
            ValueTypes::register_nullable::<Rc<$type_>>();
            ValueTypes::register_cast::<Rc<$type_>, Rc<dyn IValueConverter>>(|c| c.clone());
        )*
    };
}

/// Registers the nullable handles of the converters and their casts to the
/// converter contract with the untyped value conversions of the current
/// thread.
pub(super) fn register_value_types() {
    register_converters!(
        ColorToDisplayNameConverter,
        ColorToHexConverter,
        DoNothingForNullConverter,
        ToBrushConverter,
        ToColorConverter,
        AccentColorConverter,
        ContrastBrushConverter
    );
}
