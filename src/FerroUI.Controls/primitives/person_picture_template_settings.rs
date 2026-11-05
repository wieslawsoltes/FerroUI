//! WinUI's `PersonPictureTemplateSettings` (`controls/dev/PersonPicture/PersonPictureTemplateSettings.h`
//! and `PersonPicture.idl` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use ferroui_base::media::ImageBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};

/// The values the template of a [`PersonPicture`](crate::PersonPicture) binds to: the initials it
/// shows and the brush of its picture. Each picture has its own; the picture sets them.
#[repr(C)]
pub struct PersonPictureTemplateSettings {
    base: FerroObject,
}

ferro_class!(PersonPictureTemplateSettings: FerroObject);
ferro_class_info!(PersonPictureTemplateSettings {
    new: PersonPictureTemplateSettings::new,
    markup: { namespace: "FerroUI.Controls.Primitives" },
});
ferro_impl_classes!(PersonPictureTemplateSettings: FerroObjectImpl);

ferro_properties! { impl PersonPictureTemplateSettings {
    ferro_property!(
        /// Defines the `ActualInitials` property: the initials the picture shows.
        pub fn actual_initials_property() -> StyledProperty<String> {
            FerroProperty::register::<PersonPictureTemplateSettings, _>("ActualInitials", String::new())
        }
    );

    ferro_property!(
        /// Defines the `ActualImageBrush` property: the brush of the picture, `None` without one.
        pub fn actual_image_brush_property() -> StyledProperty<Option<Ref<ImageBrush>>> {
            FerroProperty::register::<PersonPictureTemplateSettings, _>("ActualImageBrush", None)
        }
    );
} }

impl PersonPictureTemplateSettings {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates the settings of a picture.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the initials the picture shows.
    pub fn actual_initials(&self) -> String {
        self.get_value(Self::actual_initials_property())
    }

    pub(crate) fn set_actual_initials(&self, value: String) {
        self.set_value(Self::actual_initials_property(), value);
    }

    /// Gets the brush of the picture.
    pub fn actual_image_brush(&self) -> Option<Ref<ImageBrush>> {
        self.get_value(Self::actual_image_brush_property())
    }

    pub(crate) fn set_actual_image_brush(&self, value: Option<Ref<ImageBrush>>) {
        self.set_value(Self::actual_image_brush_property(), value);
    }
}
