// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Based on the pinned WinUI IconSource API and *_Partial.cpp implementations.

use super::{FontIcon, Symbol, SymbolIcon};
use crate::primitives::TemplatedControl;
use crate::{IconElement, PathIcon};
use ferroui_base::data::BindingPriority;
use ferroui_base::media::{FontFamily, FontStyle, FontWeight, Geometry, IBrush};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledProperty, StyledPropertyOptions, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A reusable icon description. Every created element is independent, and
/// subsequent source-property changes propagate to all surviving elements.
///
/// The source keeps weak element references and does not subscribe to them, so
/// removing an element or dropping a source creates no retention cycle.
/// This base class is abstract: concrete sources override element creation.
#[repr(C)]
pub struct IconSource {
    base: FerroObject,
    elements: RefCell<Vec<WeakRef<IconElement>>>,
}

ferro_class! {
    IconSource: FerroObject, virtuals IconSourceImpl: FerroObjectImpl {
        /// Creates the concrete native icon and copies its initial properties.
        fn create_icon_element_core(this) -> Ref<IconElement>;
        /// Maps a source property to the property updated on each native icon.
        fn get_icon_element_property_core(this, property: &'static FerroProperty) -> Option<&'static FerroProperty>;
    }
}
ferro_class_info!(IconSource { markup: { namespace: "FerroUI.Controls" } });

impl FerroObjectImpl for IconSource {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let Some(target) = this.get_icon_element_property_core(change.property()) else {
            return;
        };
        // Do not keep the collection borrowed while a target raises callbacks.
        let elements: Vec<_> = this
            .elements
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .collect();
        this.elements
            .borrow_mut()
            .retain(|element| element.upgrade().is_some());
        for element in elements {
            // A previous target can synchronously change this source again.
            // Re-read the effective value so the remaining targets cannot be
            // overwritten by an obsolete outer notification. An unrelated
            // reentrant source change does not interrupt this property's fan-out.
            let value = this.get_value_untyped(change.property());
            element.set_value_untyped(target, &*value, BindingPriority::LocalValue);
        }
    }
}

impl IconSourceImpl for IconSource {
    fn create_icon_element_core(_this: &Self) -> Ref<IconElement> {
        panic!("IconSource is abstract; a concrete source must create its native IconElement")
    }

    fn get_icon_element_property_core(
        _this: &Self,
        property: &'static FerroProperty,
    ) -> Option<&'static FerroProperty> {
        (property == Self::foreground_property().as_property())
            .then(|| TemplatedControl::foreground_property().as_property())
    }
}

ferro_properties! { impl IconSource {
    /// A source foreground. An unset source leaves a new icon's inherited foreground intact.
    pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<IconSource, _>("Foreground", None)
    }
} }

impl IconSource {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            elements: RefCell::new(Vec::new()),
        }
    }

    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }
    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value);
    }

    pub fn create_icon_element(&self) -> Ref<IconElement> {
        let element = self.create_icon_element_core();
        if let Some(foreground) = self.foreground() {
            element.set_foreground(Some(foreground));
        }
        let mut elements = self.elements.borrow_mut();
        elements.retain(|element| element.upgrade().is_some());
        elements.push(element.downgrade());
        element
    }
}

/// A live font-glyph source. OS text-scale integration is not exposed until supported.
#[repr(C)]
pub struct FontIconSource {
    base: IconSource,
}
ferro_class!(FontIconSource: IconSource);
ferro_class_info!(FontIconSource { new: FontIconSource::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(FontIconSource: FerroObjectImpl);

impl IconSourceImpl for FontIconSource {
    fn create_icon_element_core(this: &Self) -> Ref<IconElement> {
        let icon = FontIcon::new();
        icon.set_glyph(this.glyph());
        icon.set_font_family(this.font_family());
        icon.set_font_size(this.font_size());
        icon.set_font_style(this.font_style());
        icon.set_font_weight(this.font_weight());
        icon.set_mirrored_when_right_to_left(this.mirrored_when_right_to_left());
        icon.upcast()
    }

    fn get_icon_element_property_core(
        this: &Self,
        property: &'static FerroProperty,
    ) -> Option<&'static FerroProperty> {
        let pairs = [
            (
                Self::glyph_property().as_property(),
                FontIcon::glyph_property().as_property(),
            ),
            (
                Self::font_family_property().as_property(),
                TemplatedControl::font_family_property().as_property(),
            ),
            (
                Self::font_size_property().as_property(),
                TemplatedControl::font_size_property().as_property(),
            ),
            (
                Self::font_style_property().as_property(),
                TemplatedControl::font_style_property().as_property(),
            ),
            (
                Self::font_weight_property().as_property(),
                TemplatedControl::font_weight_property().as_property(),
            ),
            (
                Self::mirrored_when_right_to_left_property().as_property(),
                FontIcon::mirrored_when_right_to_left_property().as_property(),
            ),
        ];
        pairs
            .iter()
            .find_map(|(source, target)| (*source == property).then_some(*target))
            .or_else(|| Self::parent_get_icon_element_property_core(this, property))
    }
}

ferro_properties! { impl FontIconSource {
    pub fn glyph_property() -> StyledProperty<String> { FerroProperty::register::<FontIconSource, _>("Glyph", String::new()) }
    pub fn font_family_property() -> StyledProperty<FontFamily> { FerroProperty::register::<FontIconSource, _>("FontFamily", FontFamily::new("Segoe Fluent Icons")) }
    pub fn font_size_property() -> StyledProperty<f64> { FerroProperty::register_with::<FontIconSource, _>("FontSize", StyledPropertyOptions::new(20.0_f64).validate(|size| size.is_finite() && *size > 0.0)) }
    pub fn font_style_property() -> StyledProperty<FontStyle> { FerroProperty::register::<FontIconSource, _>("FontStyle", FontStyle::Normal) }
    pub fn font_weight_property() -> StyledProperty<FontWeight> { FerroProperty::register::<FontIconSource, _>("FontWeight", FontWeight::Normal) }
    pub fn mirrored_when_right_to_left_property() -> StyledProperty<bool> { FerroProperty::register::<FontIconSource, _>("MirroredWhenRightToLeft", false) }
} }

impl FontIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn glyph(&self) -> String {
        self.get_value(Self::glyph_property())
    }
    pub fn set_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::glyph_property(), value.into());
    }
    pub fn font_family(&self) -> FontFamily {
        self.get_value(Self::font_family_property())
    }
    pub fn set_font_family(&self, value: FontFamily) {
        self.set_value(Self::font_family_property(), value);
    }
    pub fn font_size(&self) -> f64 {
        self.get_value(Self::font_size_property())
    }
    pub fn set_font_size(&self, value: f64) {
        self.set_value(Self::font_size_property(), value);
    }
    pub fn font_style(&self) -> FontStyle {
        self.get_value(Self::font_style_property())
    }
    pub fn set_font_style(&self, value: FontStyle) {
        self.set_value(Self::font_style_property(), value);
    }
    pub fn font_weight(&self) -> FontWeight {
        self.get_value(Self::font_weight_property())
    }
    pub fn set_font_weight(&self, value: FontWeight) {
        self.set_value(Self::font_weight_property(), value);
    }
    pub fn mirrored_when_right_to_left(&self) -> bool {
        self.get_value(Self::mirrored_when_right_to_left_property())
    }
    pub fn set_mirrored_when_right_to_left(&self, value: bool) {
        self.set_value(Self::mirrored_when_right_to_left_property(), value);
    }
}

/// A reusable predefined symbol whose changes update every created icon.
#[repr(C)]
pub struct SymbolIconSource {
    base: IconSource,
}
ferro_class!(SymbolIconSource: IconSource);
ferro_class_info!(SymbolIconSource { new: SymbolIconSource::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SymbolIconSource: FerroObjectImpl);
impl IconSourceImpl for SymbolIconSource {
    fn create_icon_element_core(this: &Self) -> Ref<IconElement> {
        SymbolIcon::with_symbol(this.symbol()).upcast()
    }
    fn get_icon_element_property_core(
        this: &Self,
        property: &'static FerroProperty,
    ) -> Option<&'static FerroProperty> {
        if property == Self::symbol_property().as_property() {
            Some(SymbolIcon::symbol_property().as_property())
        } else {
            Self::parent_get_icon_element_property_core(this, property)
        }
    }
}
ferro_properties! { impl SymbolIconSource {
    pub fn symbol_property() -> StyledProperty<Symbol> { FerroProperty::register::<SymbolIconSource, _>("Symbol", Symbol::Emoji) }
} }
impl SymbolIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn symbol(&self) -> Symbol {
        self.get_value(Self::symbol_property())
    }
    pub fn set_symbol(&self, value: Symbol) {
        self.set_value(Self::symbol_property(), value);
    }
}

/// A live source for the framework's existing native PathIcon.
#[repr(C)]
pub struct PathIconSource {
    base: IconSource,
}
ferro_class!(PathIconSource: IconSource);
ferro_class_info!(PathIconSource { new: PathIconSource::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(PathIconSource: FerroObjectImpl);
impl IconSourceImpl for PathIconSource {
    fn create_icon_element_core(this: &Self) -> Ref<IconElement> {
        let icon = PathIcon::new();
        icon.set_data(this.data());
        icon.upcast()
    }
    fn get_icon_element_property_core(
        this: &Self,
        property: &'static FerroProperty,
    ) -> Option<&'static FerroProperty> {
        if property == Self::data_property().as_property() {
            Some(PathIcon::data_property().as_property())
        } else {
            Self::parent_get_icon_element_property_core(this, property)
        }
    }
}
ferro_properties! { impl PathIconSource {
    pub fn data_property() -> StyledProperty<Option<Ref<Geometry>>> { FerroProperty::register::<PathIconSource, _>("Data", None) }
} }
impl PathIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn data(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::data_property())
    }
    pub fn set_data(&self, value: Option<Ref<Geometry>>) {
        self.set_value(Self::data_property(), value);
    }
}
