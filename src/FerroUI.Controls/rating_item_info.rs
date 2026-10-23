// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from controls/dev/RatingControl/RatingControl.idl and RatingControl.cpp.

use ferroui_base::media::IImage;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// The extensible description of the images or font glyphs in a rating control.
#[repr(C)]
pub struct RatingItemInfo {
    base: FerroObject,
}
ferro_class!(RatingItemInfo: FerroObject);
ferro_class_info!(RatingItemInfo { new: RatingItemInfo::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(RatingItemInfo: FerroObjectImpl);

impl RatingItemInfo {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Glyphs for the six WinUI rating item states. Missing states use the upstream fallback chain.
#[repr(C)]
pub struct RatingItemFontInfo {
    base: RatingItemInfo,
}
ferro_class!(RatingItemFontInfo: RatingItemInfo);
ferro_class_info!(RatingItemFontInfo { new: RatingItemFontInfo::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(RatingItemFontInfo: FerroObjectImpl);

ferro_properties! { impl RatingItemFontInfo {
    pub fn glyph_property() -> StyledProperty<String> { FerroProperty::register::<RatingItemFontInfo, _>("Glyph", String::new()) }
    pub fn disabled_glyph_property() -> StyledProperty<String> { FerroProperty::register::<RatingItemFontInfo, _>("DisabledGlyph", String::new()) }
    pub fn pointer_over_glyph_property() -> StyledProperty<String> { FerroProperty::register::<RatingItemFontInfo, _>("PointerOverGlyph", String::new()) }
    pub fn pointer_over_placeholder_glyph_property() -> StyledProperty<String> { FerroProperty::register::<RatingItemFontInfo, _>("PointerOverPlaceholderGlyph", String::new()) }
    pub fn placeholder_glyph_property() -> StyledProperty<String> { FerroProperty::register::<RatingItemFontInfo, _>("PlaceholderGlyph", String::new()) }
    pub fn unset_glyph_property() -> StyledProperty<String> { FerroProperty::register::<RatingItemFontInfo, _>("UnsetGlyph", String::new()) }
} }

impl RatingItemFontInfo {
    pub fn construct() -> Self {
        Self {
            base: RatingItemInfo::construct(),
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
    pub fn disabled_glyph(&self) -> String {
        self.get_value(Self::disabled_glyph_property())
    }
    pub fn set_disabled_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::disabled_glyph_property(), value.into());
    }
    pub fn pointer_over_glyph(&self) -> String {
        self.get_value(Self::pointer_over_glyph_property())
    }
    pub fn set_pointer_over_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::pointer_over_glyph_property(), value.into());
    }
    pub fn pointer_over_placeholder_glyph(&self) -> String {
        self.get_value(Self::pointer_over_placeholder_glyph_property())
    }
    pub fn set_pointer_over_placeholder_glyph(&self, value: impl Into<String>) {
        self.set_value(
            Self::pointer_over_placeholder_glyph_property(),
            value.into(),
        );
    }
    pub fn placeholder_glyph(&self) -> String {
        self.get_value(Self::placeholder_glyph_property())
    }
    pub fn set_placeholder_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::placeholder_glyph_property(), value.into());
    }
    pub fn unset_glyph(&self) -> String {
        self.get_value(Self::unset_glyph_property())
    }
    pub fn set_unset_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::unset_glyph_property(), value.into());
    }

    pub(crate) fn glyph_for(&self, state: RatingItemState) -> String {
        let candidate = match state {
            RatingItemState::Disabled => self.disabled_glyph(),
            RatingItemState::PointerOver => self.pointer_over_glyph(),
            RatingItemState::PointerOverPlaceholder => self.pointer_over_placeholder_glyph(),
            RatingItemState::Placeholder => self.placeholder_glyph(),
            RatingItemState::Unset => self.unset_glyph(),
            RatingItemState::Set => return self.glyph(),
        };
        if !candidate.is_empty() {
            candidate
        } else if state == RatingItemState::PointerOverPlaceholder {
            self.glyph_for(RatingItemState::Placeholder)
        } else {
            self.glyph()
        }
    }
}

/// Native image sources for each rating state. Each item owns its own Image visual.
#[repr(C)]
pub struct RatingItemImageInfo {
    base: RatingItemInfo,
}
ferro_class!(RatingItemImageInfo: RatingItemInfo);
ferro_class_info!(RatingItemImageInfo { new: RatingItemImageInfo::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(RatingItemImageInfo: FerroObjectImpl);

ferro_properties! { impl RatingItemImageInfo {
    pub fn image_property() -> StyledProperty<Option<Rc<dyn IImage>>> { FerroProperty::register::<RatingItemImageInfo, _>("Image", None) }
    pub fn disabled_image_property() -> StyledProperty<Option<Rc<dyn IImage>>> { FerroProperty::register::<RatingItemImageInfo, _>("DisabledImage", None) }
    pub fn pointer_over_image_property() -> StyledProperty<Option<Rc<dyn IImage>>> { FerroProperty::register::<RatingItemImageInfo, _>("PointerOverImage", None) }
    pub fn pointer_over_placeholder_image_property() -> StyledProperty<Option<Rc<dyn IImage>>> { FerroProperty::register::<RatingItemImageInfo, _>("PointerOverPlaceholderImage", None) }
    pub fn placeholder_image_property() -> StyledProperty<Option<Rc<dyn IImage>>> { FerroProperty::register::<RatingItemImageInfo, _>("PlaceholderImage", None) }
    pub fn unset_image_property() -> StyledProperty<Option<Rc<dyn IImage>>> { FerroProperty::register::<RatingItemImageInfo, _>("UnsetImage", None) }
} }

impl RatingItemImageInfo {
    pub fn construct() -> Self {
        Self {
            base: RatingItemInfo::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn image(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::image_property())
    }
    pub fn set_image(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::image_property(), value);
    }
    pub fn disabled_image(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::disabled_image_property())
    }
    pub fn set_disabled_image(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::disabled_image_property(), value);
    }
    pub fn pointer_over_image(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::pointer_over_image_property())
    }
    pub fn set_pointer_over_image(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::pointer_over_image_property(), value);
    }
    pub fn pointer_over_placeholder_image(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::pointer_over_placeholder_image_property())
    }
    pub fn set_pointer_over_placeholder_image(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::pointer_over_placeholder_image_property(), value);
    }
    pub fn placeholder_image(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::placeholder_image_property())
    }
    pub fn set_placeholder_image(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::placeholder_image_property(), value);
    }
    pub fn unset_image(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::unset_image_property())
    }
    pub fn set_unset_image(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::unset_image_property(), value);
    }

    pub(crate) fn image_for(&self, state: RatingItemState) -> Option<Rc<dyn IImage>> {
        let candidate = match state {
            RatingItemState::Disabled => self.disabled_image(),
            RatingItemState::PointerOver => self.pointer_over_image(),
            RatingItemState::PointerOverPlaceholder => self.pointer_over_placeholder_image(),
            RatingItemState::Placeholder => self.placeholder_image(),
            RatingItemState::Unset => self.unset_image(),
            RatingItemState::Set => return self.image(),
        };
        candidate.or_else(|| {
            if state == RatingItemState::PointerOverPlaceholder {
                self.image_for(RatingItemState::Placeholder)
            } else {
                self.image()
            }
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RatingItemState {
    Set,
    Unset,
    Placeholder,
    PointerOver,
    PointerOverPlaceholder,
    Disabled,
}
