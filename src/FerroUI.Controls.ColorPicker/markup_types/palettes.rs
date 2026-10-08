//! Markup metadata of the colour palettes (`FerroUI.Controls`), their
//! contract, the colour helper (`FerroUI.Controls.Primitives`) and the list
//! of palette colours `ColorView.PaletteColors` holds.

use crate::color_palettes::*;
use crate::primitives::ColorHelper;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::media::Color;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

// The palettes are reference types: an instance equals itself.
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
    FlatColorPalette,
    FlatHalfColorPalette,
    FluentColorPalette,
    MaterialColorPalette,
    MaterialHalfColorPalette,
    SixteenColorPalette
);

ferro_markup_type!(interface dyn IColorPalette as "IColorPalette" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn IColorPalette>, Option<Rc<dyn IColorPalette>>],
    this: Rc<dyn IColorPalette>,
    properties: [
        ColorCount: i32 { get: |p: &Rc<dyn IColorPalette>| p.color_count() },
        ShadeCount: i32 { get: |p: &Rc<dyn IColorPalette>| p.shade_count() },
    ],
    methods: [
        fn GetColor(i32, i32) -> Color => |p: &Rc<dyn IColorPalette>, color_index: i32, shade_index: i32| {
            p.get_color(color_index, shade_index)
        },
    ],
});

macro_rules! palette {
    ($type_:ident) => {
        ferro_markup_type!(class $type_ {
            namespace: "FerroUI.Controls",
            handles: [Rc<$type_>, Option<Rc<$type_>>],
            this: Rc<$type_>,
            interfaces: [Rc<dyn IColorPalette>],
            constructors: [() => || Rc::new($type_::new())],
            properties: [
                ColorCount: i32 { get: |p: &Rc<$type_>| p.color_count() },
                ShadeCount: i32 { get: |p: &Rc<$type_>| p.shade_count() },
            ],
            methods: [
                fn GetColor(i32, i32) -> Color => |p: &Rc<$type_>, color_index: i32, shade_index: i32| {
                    p.get_color(color_index, shade_index)
                },
            ],
        });
    };
}

palette!(FlatColorPalette);
palette!(FlatHalfColorPalette);
palette!(FluentColorPalette);
palette!(MaterialColorPalette);
palette!(MaterialHalfColorPalette);
palette!(SixteenColorPalette);

ferro_markup_type!(static ColorHelper {
    namespace: "FerroUI.Controls.Primitives",
    methods: [
        static fn GetRelativeLuminance(Color) -> f64 => ColorHelper::get_relative_luminance,
        static fn ToDisplayName(Color) -> String => ColorHelper::to_display_name,
    ],
    static_properties: [
        ToDisplayNameExists: bool { get: ColorHelper::to_display_name_exists },
    ],
});

// The list of colors `ColorView.PaletteColors` holds, delivered to the items source of the
// palette list box of the control themes.
ferroui_controls::ferro_markup_list!(pub(super) PaletteColorList: Color);

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <dyn IColorPalette as MarkupTyped>::MARKUP,
    <FlatColorPalette as MarkupTyped>::MARKUP,
    <FlatHalfColorPalette as MarkupTyped>::MARKUP,
    <FluentColorPalette as MarkupTyped>::MARKUP,
    <MaterialColorPalette as MarkupTyped>::MARKUP,
    <MaterialHalfColorPalette as MarkupTyped>::MARKUP,
    <SixteenColorPalette as MarkupTyped>::MARKUP,
    <ColorHelper as MarkupTyped>::MARKUP,
];

macro_rules! register_palettes {
    ($($type_:ty),*) => {
        $(
            ValueTypes::register_nullable::<Rc<$type_>>();
            ValueTypes::register_cast::<Rc<$type_>, Rc<dyn IColorPalette>>(|p| p.clone());
        )*
    };
}

/// Registers the nullable handles of the palettes and their casts to the
/// palette contract.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<dyn IColorPalette>>();
    register_palettes!(
        FlatColorPalette,
        FlatHalfColorPalette,
        FluentColorPalette,
        MaterialColorPalette,
        MaterialHalfColorPalette,
        SixteenColorPalette
    );
}
