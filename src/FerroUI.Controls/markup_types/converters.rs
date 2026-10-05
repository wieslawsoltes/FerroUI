//! Markup metadata of the converters of this crate (`FerroUI.Controls.Converters`):
//! plain classes held through `Rc`, each assignable to the converter contract
//! it implements.

use crate::converters::*;
use ferroui_base::data::converters::{IMultiValueConverter, IValueConverter};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::input::KeyGesture;
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
    BorderGapMaskConverter,
    CornerRadiusFilterConverter,
    CornerRadiusToDoubleConverter,
    EnumToBoolConverter,
    MarginMultiplierConverter,
    MenuScrollingVisibilityConverter,
    PlatformKeyGestureConverter,
    StringFormatConverter,
    TreeViewItemIndentConverter
);

ferro_markup_type!(class BorderGapMaskConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<BorderGapMaskConverter>, Option<Rc<BorderGapMaskConverter>>],
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => || Rc::new(BorderGapMaskConverter::new())],
});

ferro_markup_type!(class CornerRadiusFilterConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<CornerRadiusFilterConverter>, Option<Rc<CornerRadiusFilterConverter>>],
    this: Rc<CornerRadiusFilterConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(CornerRadiusFilterConverter::new())],
    properties: [
        Filter: Corners {
            get: |c: &Rc<CornerRadiusFilterConverter>| c.filter(),
            set: |c: &Rc<CornerRadiusFilterConverter>, value: Corners| c.set_filter(value)
        },
        Scale: f64 {
            get: |c: &Rc<CornerRadiusFilterConverter>| c.scale(),
            set: |c: &Rc<CornerRadiusFilterConverter>, value: f64| c.set_scale(value)
        },
    ],
});

ferro_markup_type!(class CornerRadiusToDoubleConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<CornerRadiusToDoubleConverter>, Option<Rc<CornerRadiusToDoubleConverter>>],
    this: Rc<CornerRadiusToDoubleConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(CornerRadiusToDoubleConverter::new())],
    properties: [
        Corner: Corners {
            get: |c: &Rc<CornerRadiusToDoubleConverter>| c.corner(),
            set: |c: &Rc<CornerRadiusToDoubleConverter>, value: Corners| c.set_corner(value)
        },
    ],
});

ferro_markup_type!(class EnumToBoolConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<EnumToBoolConverter>, Option<Rc<EnumToBoolConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(EnumToBoolConverter::new())],
});

ferro_markup_type!(class MarginMultiplierConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<MarginMultiplierConverter>, Option<Rc<MarginMultiplierConverter>>],
    this: Rc<MarginMultiplierConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(MarginMultiplierConverter::new())],
    properties: [
        Indent: f64 {
            get: |c: &Rc<MarginMultiplierConverter>| c.indent(),
            set: |c: &Rc<MarginMultiplierConverter>, value: f64| c.set_indent(value)
        },
        Left: bool {
            get: |c: &Rc<MarginMultiplierConverter>| c.left(),
            set: |c: &Rc<MarginMultiplierConverter>, value: bool| c.set_left(value)
        },
        Top: bool {
            get: |c: &Rc<MarginMultiplierConverter>| c.top(),
            set: |c: &Rc<MarginMultiplierConverter>, value: bool| c.set_top(value)
        },
        Right: bool {
            get: |c: &Rc<MarginMultiplierConverter>| c.right(),
            set: |c: &Rc<MarginMultiplierConverter>, value: bool| c.set_right(value)
        },
        Bottom: bool {
            get: |c: &Rc<MarginMultiplierConverter>| c.bottom(),
            set: |c: &Rc<MarginMultiplierConverter>, value: bool| c.set_bottom(value)
        },
    ],
});

ferro_markup_type!(class MenuScrollingVisibilityConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<MenuScrollingVisibilityConverter>, Option<Rc<MenuScrollingVisibilityConverter>>],
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => || Rc::new(MenuScrollingVisibilityConverter::default())],
    fields: [Instance: Rc<MenuScrollingVisibilityConverter> => MenuScrollingVisibilityConverter::instance],
});

ferro_markup_type!(class PlatformKeyGestureConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<PlatformKeyGestureConverter>, Option<Rc<PlatformKeyGestureConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(PlatformKeyGestureConverter::new())],
    methods: [
        static fn ToPlatformString(KeyGesture) -> String =>
            |gesture: KeyGesture| PlatformKeyGestureConverter::to_platform_string(&gesture),
    ],
});

ferro_markup_type!(class StringFormatConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<StringFormatConverter>, Option<Rc<StringFormatConverter>>],
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => || Rc::new(StringFormatConverter::new())],
});

ferro_markup_type!(class TreeViewItemIndentConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<TreeViewItemIndentConverter>, Option<Rc<TreeViewItemIndentConverter>>],
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => || Rc::new(TreeViewItemIndentConverter::default())],
    fields: [Instance: Rc<TreeViewItemIndentConverter> => TreeViewItemIndentConverter::instance],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <BorderGapMaskConverter as MarkupTyped>::MARKUP,
    <CornerRadiusFilterConverter as MarkupTyped>::MARKUP,
    <CornerRadiusToDoubleConverter as MarkupTyped>::MARKUP,
    <EnumToBoolConverter as MarkupTyped>::MARKUP,
    <MarginMultiplierConverter as MarkupTyped>::MARKUP,
    <MenuScrollingVisibilityConverter as MarkupTyped>::MARKUP,
    <PlatformKeyGestureConverter as MarkupTyped>::MARKUP,
    <StringFormatConverter as MarkupTyped>::MARKUP,
    <TreeViewItemIndentConverter as MarkupTyped>::MARKUP,
];

/// Registers the nullable handles of the converters and their casts to the
/// converter contracts with the untyped value conversions of the current
/// thread.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<CornerRadiusFilterConverter>>();
    ValueTypes::register_cast::<Rc<CornerRadiusFilterConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<CornerRadiusToDoubleConverter>>();
    ValueTypes::register_cast::<Rc<CornerRadiusToDoubleConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<EnumToBoolConverter>>();
    ValueTypes::register_cast::<Rc<EnumToBoolConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<MarginMultiplierConverter>>();
    ValueTypes::register_cast::<Rc<MarginMultiplierConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<PlatformKeyGestureConverter>>();
    ValueTypes::register_cast::<Rc<PlatformKeyGestureConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<BorderGapMaskConverter>>();
    ValueTypes::register_cast::<Rc<BorderGapMaskConverter>, Rc<dyn IMultiValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<MenuScrollingVisibilityConverter>>();
    ValueTypes::register_cast::<Rc<MenuScrollingVisibilityConverter>, Rc<dyn IMultiValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<StringFormatConverter>>();
    ValueTypes::register_cast::<Rc<StringFormatConverter>, Rc<dyn IMultiValueConverter>>(|c| c.clone());
    ValueTypes::register_nullable::<Rc<TreeViewItemIndentConverter>>();
    ValueTypes::register_cast::<Rc<TreeViewItemIndentConverter>, Rc<dyn IMultiValueConverter>>(|c| c.clone());
}
