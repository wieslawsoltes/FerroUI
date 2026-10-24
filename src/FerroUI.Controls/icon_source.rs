//! WinUI's icon sources (`IconSource_Partial.cpp`, `FontIconSource_Partial.cpp`,
//! `SymbolIconSource_Partial.cpp` and `PathIconSource_Partial.cpp` of `dxaml/xcp/dxaml/lib`,
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{FontIcon, Symbol, SymbolIcon};
use ferroui_base::data::core::ValueTypes;
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

/// The base class of the descriptions of an icon that can be shared (WinUI's `IconSource`): each
/// [`create_icon_element`](Self::create_icon_element) creates a new icon element, and a later
/// change of a property of the source is set on every element it created that is still alive.
///
/// The source holds its elements weakly and does not subscribe to them, so neither keeps the
/// other alive. This class is abstract: it creates no element ([`create_icon_element`] is `None`,
/// as WinUI's base returns no element); the derived sources create theirs.
///
/// [`create_icon_element`]: Self::create_icon_element
#[repr(C)]
pub struct IconSource {
    base: FerroObject,
    elements: RefCell<Vec<WeakRef<IconElement>>>,
}

ferro_class! {
    IconSource: FerroObject, virtuals IconSourceImpl: FerroObjectImpl {
        /// Creates the icon element of the source with the values of its properties
        /// (`CreateIconElementCore`); `None` for the abstract base.
        fn create_icon_element_core(this) -> Option<Ref<IconElement>>;
        /// The property of the icon elements that a property of the source is set on
        /// (`GetIconElementPropertyCore`); `None` for a property that is not.
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
            // A handler of an element may change the source again: the value is read again for
            // each element, so the later elements get the latest value, not the one of this
            // notification.
            let value = this.get_value_untyped(change.property());
            if target.property_type() == change.property().property_type() {
                element.set_value_untyped(target, &*value, BindingPriority::LocalValue);
            } else {
                // A source property of a nullable type for a property of the element that is not
                // (FontIconSource.FontFamily): no value leaves the element's own.
                match ValueTypes::normalize(value) {
                    Some(value) => element.set_value_untyped(target, value.as_any(), BindingPriority::LocalValue),
                    None => element.clear_value_untyped(target),
                }
            }
        }
    }
}

impl IconSourceImpl for IconSource {
    // IconSource::CreateIconElementCoreImpl: "an abstract base type", E_NOTIMPL.
    fn create_icon_element_core(_this: &Self) -> Option<Ref<IconElement>> {
        None
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
    /// Defines the `Foreground` property: the foreground of the icons. Unset (`None`, the
    /// default), an icon keeps the foreground it inherits.
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

    /// Creates an icon element with the values of the source (`IconSource::CreateIconElement`):
    /// the foreground only when the source has one, so that an icon without inherits its
    /// foreground. `None` for a source that creates no element.
    pub fn create_icon_element(&self) -> Option<Ref<IconElement>> {
        let element = self.create_icon_element_core()?;
        if let Some(foreground) = self.foreground() {
            element.set_foreground(Some(foreground));
        }
        let mut elements = self.elements.borrow_mut();
        elements.retain(|element| element.upgrade().is_some());
        elements.push(element.downgrade());
        Some(element)
    }
}

/// The description of a [`FontIcon`] (WinUI's `FontIconSource`).
///
/// Missing (docs/porting/winui-controls.md, FontIcon): `IsTextScaleFactorEnabled`.
#[repr(C)]
pub struct FontIconSource {
    base: IconSource,
}
ferro_class!(FontIconSource: IconSource);
ferro_class_info!(FontIconSource { new: FontIconSource::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(FontIconSource: FerroObjectImpl);

impl IconSourceImpl for FontIconSource {
    // FontIconSource::CreateIconElementCoreImpl: the font family only when the source has one.
    fn create_icon_element_core(this: &Self) -> Option<Ref<IconElement>> {
        let icon = FontIcon::new();
        icon.set_glyph(this.glyph());
        icon.set_font_size(this.font_size());
        icon.set_font_weight(this.font_weight());
        icon.set_font_style(this.font_style());
        icon.set_mirrored_when_right_to_left(this.mirrored_when_right_to_left());
        if let Some(font_family) = this.font_family() {
            icon.set_font_family(font_family);
        }
        Some(icon.upcast())
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
    /// Defines the `Glyph` property.
    pub fn glyph_property() -> StyledProperty<String> {
        FerroProperty::register::<FontIconSource, _>("Glyph", String::new())
    }
    /// Defines the `FontFamily` property: `None` by default, which leaves the icon its own (the
    /// theme's `SymbolThemeFontFamily`).
    pub fn font_family_property() -> StyledProperty<Option<FontFamily>> {
        FerroProperty::register::<FontIconSource, _>("FontFamily", None)
    }
    /// Defines the `FontSize` property: 20 by default, as `FontIcon`.
    pub fn font_size_property() -> StyledProperty<f64> {
        FerroProperty::register_with::<FontIconSource, _>(
            "FontSize",
            StyledPropertyOptions::new(20.0_f64).validate(|size| size.is_finite() && *size > 0.0),
        )
    }
    /// Defines the `FontStyle` property.
    pub fn font_style_property() -> StyledProperty<FontStyle> {
        FerroProperty::register::<FontIconSource, _>("FontStyle", FontStyle::Normal)
    }
    /// Defines the `FontWeight` property.
    pub fn font_weight_property() -> StyledProperty<FontWeight> {
        FerroProperty::register::<FontIconSource, _>("FontWeight", FontWeight::Normal)
    }
    /// Defines the `MirroredWhenRightToLeft` property.
    pub fn mirrored_when_right_to_left_property() -> StyledProperty<bool> {
        FerroProperty::register::<FontIconSource, _>("MirroredWhenRightToLeft", false)
    }
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
    pub fn font_family(&self) -> Option<FontFamily> {
        self.get_value(Self::font_family_property())
    }
    pub fn set_font_family(&self, value: impl Into<Option<FontFamily>>) {
        self.set_value(Self::font_family_property(), value.into());
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

/// The description of a [`SymbolIcon`] (WinUI's `SymbolIconSource`).
#[repr(C)]
pub struct SymbolIconSource {
    base: IconSource,
}
ferro_class!(SymbolIconSource: IconSource);
ferro_class_info!(SymbolIconSource { new: SymbolIconSource::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SymbolIconSource: FerroObjectImpl);
impl IconSourceImpl for SymbolIconSource {
    fn create_icon_element_core(this: &Self) -> Option<Ref<IconElement>> {
        Some(SymbolIcon::with_symbol(this.symbol()).upcast())
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
    /// Defines the `Symbol` property: `Emoji` by default, as `SymbolIcon`.
    pub fn symbol_property() -> StyledProperty<Symbol> {
        FerroProperty::register::<SymbolIconSource, _>("Symbol", Symbol::Emoji)
    }
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

/// The description of a [`PathIcon`] (WinUI's `PathIconSource`): the icons are the path icons of
/// FerroUI.
#[repr(C)]
pub struct PathIconSource {
    base: IconSource,
}
ferro_class!(PathIconSource: IconSource);
ferro_class_info!(PathIconSource { new: PathIconSource::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(PathIconSource: FerroObjectImpl);
impl IconSourceImpl for PathIconSource {
    fn create_icon_element_core(this: &Self) -> Option<Ref<IconElement>> {
        let icon = PathIcon::new();
        icon.set_data(this.data());
        Some(icon.upcast())
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
    /// Defines the `Data` property.
    pub fn data_property() -> StyledProperty<Option<Ref<Geometry>>> {
        FerroProperty::register::<PathIconSource, _>("Data", None)
    }
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
