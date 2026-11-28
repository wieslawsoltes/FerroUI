//! WinUI's `BitmapIconSource` and `ImageIconSource`: shareable descriptions of a [`BitmapIcon`]
//! and an [`ImageIcon`]. A later change of a property is set on every icon a source created, by
//! the base class.

use crate::{BitmapIcon, ImageIcon};
use ferroui_base::media::IImage;
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    Ref, StyledProperty,
};
use ferroui_controls::{IconElement, IconSource, IconSourceImpl, IconSourceImplExt};
use std::rc::Rc;

/// The description of a [`BitmapIcon`] (WinUI's `BitmapIconSource`).
#[repr(C)]
pub struct BitmapIconSource {
    base: IconSource,
}

ferro_class!(BitmapIconSource: IconSource);
ferro_class_info!(BitmapIconSource { new: BitmapIconSource::new });
ferro_impl_classes!(BitmapIconSource: FerroObjectImpl);

impl IconSourceImpl for BitmapIconSource {
    fn create_icon_element_core(this: &Self) -> Option<Ref<IconElement>> {
        let icon = BitmapIcon::new();
        icon.set_uri_source(this.uri_source());
        icon.set_show_as_monochrome(this.show_as_monochrome());
        Some(icon.upcast())
    }

    fn get_icon_element_property_core(
        this: &Self,
        property: &'static FerroProperty,
    ) -> Option<&'static FerroProperty> {
        if property == Self::uri_source_property().as_property() {
            Some(BitmapIcon::uri_source_property().as_property())
        } else if property == Self::show_as_monochrome_property().as_property() {
            Some(BitmapIcon::show_as_monochrome_property().as_property())
        } else {
            Self::parent_get_icon_element_property_core(this, property)
        }
    }
}

ferro_properties! {
    impl BitmapIconSource {
        /// Defines the `UriSource` property.
        pub fn uri_source_property() -> StyledProperty<Option<Uri>> {
            FerroProperty::register::<BitmapIconSource, _>("UriSource", None)
        }

        /// Defines the `ShowAsMonochrome` property.
        pub fn show_as_monochrome_property() -> StyledProperty<bool> {
            FerroProperty::register::<BitmapIconSource, _>("ShowAsMonochrome", true)
        }
    }
}

impl BitmapIconSource {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconSource::construct() }
    }

    /// Creates a source without a URI.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The URI of the bitmap.
    pub fn uri_source(&self) -> Option<Uri> {
        self.get_value(Self::uri_source_property())
    }

    pub fn set_uri_source(&self, value: Option<Uri>) {
        self.set_value(Self::uri_source_property(), value);
    }

    /// Whether the bitmap is drawn in the single color of the foreground. The default is true.
    pub fn show_as_monochrome(&self) -> bool {
        self.get_value(Self::show_as_monochrome_property())
    }

    pub fn set_show_as_monochrome(&self, value: bool) {
        self.set_value(Self::show_as_monochrome_property(), value);
    }
}

/// The description of an [`ImageIcon`] (WinUI's `ImageIconSource`).
#[repr(C)]
pub struct ImageIconSource {
    base: IconSource,
}

ferro_class!(ImageIconSource: IconSource);
ferro_class_info!(ImageIconSource { new: ImageIconSource::new });
ferro_impl_classes!(ImageIconSource: FerroObjectImpl);

impl IconSourceImpl for ImageIconSource {
    fn create_icon_element_core(this: &Self) -> Option<Ref<IconElement>> {
        let icon = ImageIcon::new();
        icon.set_source(this.image_source());
        Some(icon.upcast())
    }

    fn get_icon_element_property_core(
        this: &Self,
        property: &'static FerroProperty,
    ) -> Option<&'static FerroProperty> {
        if property == Self::image_source_property().as_property() {
            Some(ImageIcon::source_property().as_property())
        } else {
            Self::parent_get_icon_element_property_core(this, property)
        }
    }
}

ferro_properties! {
    impl ImageIconSource {
        /// Defines the `ImageSource` property.
        pub fn image_source_property() -> StyledProperty<Option<Rc<dyn IImage>>> {
            FerroProperty::register::<ImageIconSource, _>("ImageSource", None)
        }
    }
}

impl ImageIconSource {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconSource::construct() }
    }

    /// Creates a source without an image.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The image of the icons.
    pub fn image_source(&self) -> Option<Rc<dyn IImage>> {
        self.get_value(Self::image_source_property())
    }

    pub fn set_image_source(&self, value: Option<Rc<dyn IImage>>) {
        self.set_value(Self::image_source_property(), value);
    }
}
