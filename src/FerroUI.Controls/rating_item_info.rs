//! WinUI's `RatingItemInfo`, `RatingItemFontInfo` and `RatingItemImageInfo`
//! (`controls/dev/RatingControl/RatingItemInfo.cpp`, `RatingItemFontInfo.cpp`,
//! `RatingItemImageInfo.cpp` and `RatingControl.idl` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), with the fallbacks of `RatingControl::GetAppropriateGlyph` and
//! `GetAppropriateImageSource`.

use ferroui_base::media::IImage;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// What the items of a [`RatingControl`](crate::RatingControl) show: glyphs of a font
/// ([`RatingItemFontInfo`]) or images ([`RatingItemImageInfo`]) (WinUI's `RatingItemInfo`).
#[repr(C)]
pub struct RatingItemInfo {
    base: FerroObject,
}
ferro_class!(RatingItemInfo: FerroObject);
ferro_class_info!(RatingItemInfo { new: RatingItemInfo::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(RatingItemInfo: FerroObjectImpl);

impl RatingItemInfo {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The glyphs of the items of a rating control in each state (WinUI's `RatingItemFontInfo`): an
/// empty glyph of a state is the glyph of the placeholder for the pointer over the placeholder,
/// and `Glyph` otherwise.
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
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
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

/// The images of the items of a rating control in each state (WinUI's `RatingItemImageInfo`),
/// with the fallbacks of the glyphs.
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
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
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

/// The state an item is drawn in (WinUI's `RatingControlStates`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RatingItemState {
    Set,
    Unset,
    Placeholder,
    PointerOver,
    PointerOverPlaceholder,
    Disabled,
}
