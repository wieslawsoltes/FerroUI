// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.
//! Native properties for the source font-icon snapshot inputs.
use super::IconSource;
use ferroui_base::media::{FontFamily, FontStyle, FontWeight};
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledProperty, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};

#[repr(C)]
pub struct FontIconSource {
    base: IconSource,
}
ferro_class!(FontIconSource:IconSource);
ferro_impl_classes!(FontIconSource:FerroObjectImpl);
ferro_class_info!(FontIconSource {new:FontIconSource::new,markup:{namespace:"FerroRibbon.Compatibility",attributes:[UsableDuringInitialization(false)]}});
ferro_properties! {impl FontIconSource {
    pub fn glyph_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Glyph",Some(String::new()))}
    pub fn font_family_property()->StyledProperty<Option<FontFamily>> {FerroProperty::register::<Self,_>("FontFamily",Some(FontFamily::new("Segoe Fluent Icons,Segoe MDL2 Assets")))}
    pub fn font_size_property()->StyledProperty<f64> {FerroProperty::register::<Self,_>("FontSize",20.)}
    pub fn font_style_property()->StyledProperty<FontStyle> {FerroProperty::register::<Self,_>("FontStyle",FontStyle::Normal)}
    pub fn font_weight_property()->StyledProperty<FontWeight> {FerroProperty::register::<Self,_>("FontWeight",FontWeight::Normal)}
    pub fn is_text_scale_factor_enabled_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsTextScaleFactorEnabled",true)}
    pub fn mirrored_when_right_to_left_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("MirroredWhenRightToLeft",false)}
}}
macro_rules! property {
    ($get:ident,$set:ident,$prop:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$prop())
        }
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$prop(), value)
        }
    };
}
impl FontIconSource {
    pub fn construct() -> Self {
        Self {
            base: IconSource::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    property!(glyph, set_glyph, glyph_property, Option<String>);
    property!(
        font_family,
        set_font_family,
        font_family_property,
        Option<FontFamily>
    );
    property!(font_size, set_font_size, font_size_property, f64);
    property!(font_style, set_font_style, font_style_property, FontStyle);
    property!(
        font_weight,
        set_font_weight,
        font_weight_property,
        FontWeight
    );
    property!(
        is_text_scale_factor_enabled,
        set_is_text_scale_factor_enabled,
        is_text_scale_factor_enabled_property,
        bool
    );
    property!(
        mirrored_when_right_to_left,
        set_mirrored_when_right_to_left,
        mirrored_when_right_to_left_property,
        bool
    );
}
