//! The contact a [`PersonPicture`](crate::PersonPicture) shows: the members of the Windows Runtime
//! `Windows.ApplicationModel.Contacts.Contact` that WinUI's `PersonPicture`
//! (`controls/dev/PersonPicture/PersonPicture.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`) reads.

use ferroui_base::media::IImageBrushSource;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// A contact (the Windows Runtime's `Contact`, the members a person picture reads): its
/// identifier, its names and its pictures.
///
/// The Windows contact store has no counterpart off Windows, so an application fills the contact
/// itself. The pictures are images (the Windows Runtime's are stream references, which the
/// picture opens and decodes asynchronously); the display name is the override, or the first
/// and last names joined by a space (the Windows Runtime composes it from every part of the
/// name).
#[repr(C)]
pub struct Contact {
    base: FerroObject,
}

ferro_class!(Contact: FerroObject);
ferro_class_info!(Contact {
    new: Contact::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(Contact: FerroObjectImpl);

ferro_properties! { impl Contact {
    ferro_property!(
        /// Defines the `Id` property: the identifier of the contact.
        pub fn id_property() -> StyledProperty<String> {
            FerroProperty::register::<Contact, _>("Id", String::new())
        }
    );

    ferro_property!(
        /// Defines the `FirstName` property.
        pub fn first_name_property() -> StyledProperty<String> {
            FerroProperty::register::<Contact, _>("FirstName", String::new())
        }
    );

    ferro_property!(
        /// Defines the `LastName` property.
        pub fn last_name_property() -> StyledProperty<String> {
            FerroProperty::register::<Contact, _>("LastName", String::new())
        }
    );

    ferro_property!(
        /// Defines the `DisplayNameOverride` property: the display name, when it is not the first
        /// and the last names.
        pub fn display_name_override_property() -> StyledProperty<String> {
            FerroProperty::register::<Contact, _>("DisplayNameOverride", String::new())
        }
    );

    ferro_property!(
        /// Defines the `SmallDisplayPicture` property.
        pub fn small_display_picture_property() -> StyledProperty<Option<Rc<dyn IImageBrushSource>>> {
            FerroProperty::register::<Contact, _>("SmallDisplayPicture", None)
        }
    );

    ferro_property!(
        /// Defines the `LargeDisplayPicture` property.
        pub fn large_display_picture_property() -> StyledProperty<Option<Rc<dyn IImageBrushSource>>> {
            FerroProperty::register::<Contact, _>("LargeDisplayPicture", None)
        }
    );

    ferro_property!(
        /// Defines the `SourceDisplayPicture` property.
        pub fn source_display_picture_property() -> StyledProperty<Option<Rc<dyn IImageBrushSource>>> {
            FerroProperty::register::<Contact, _>("SourceDisplayPicture", None)
        }
    );

    ferro_property!(
        /// Defines the `Thumbnail` property.
        pub fn thumbnail_property() -> StyledProperty<Option<Rc<dyn IImageBrushSource>>> {
            FerroProperty::register::<Contact, _>("Thumbnail", None)
        }
    );
} }

impl Contact {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates an empty contact.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the identifier of the contact.
    pub fn id(&self) -> String {
        self.get_value(Self::id_property())
    }

    /// Sets the identifier of the contact.
    pub fn set_id(&self, value: impl Into<String>) {
        self.set_value(Self::id_property(), value.into());
    }

    /// Gets the first name.
    pub fn first_name(&self) -> String {
        self.get_value(Self::first_name_property())
    }

    /// Sets the first name.
    pub fn set_first_name(&self, value: impl Into<String>) {
        self.set_value(Self::first_name_property(), value.into());
    }

    /// Gets the last name.
    pub fn last_name(&self) -> String {
        self.get_value(Self::last_name_property())
    }

    /// Sets the last name.
    pub fn set_last_name(&self, value: impl Into<String>) {
        self.set_value(Self::last_name_property(), value.into());
    }

    /// Gets the display name override.
    pub fn display_name_override(&self) -> String {
        self.get_value(Self::display_name_override_property())
    }

    /// Sets the display name override.
    pub fn set_display_name_override(&self, value: impl Into<String>) {
        self.set_value(Self::display_name_override_property(), value.into());
    }

    /// Gets the display name: the override, or the first and the last names joined by a space.
    pub fn display_name(&self) -> String {
        let display_name = self.display_name_override();
        if !display_name.is_empty() {
            return display_name;
        }
        [self.first_name(), self.last_name()].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ")
    }

    /// Gets the small display picture.
    pub fn small_display_picture(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.get_value(Self::small_display_picture_property())
    }

    /// Sets the small display picture.
    pub fn set_small_display_picture(&self, value: Option<Rc<dyn IImageBrushSource>>) {
        self.set_value(Self::small_display_picture_property(), value);
    }

    /// Gets the large display picture.
    pub fn large_display_picture(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.get_value(Self::large_display_picture_property())
    }

    /// Sets the large display picture.
    pub fn set_large_display_picture(&self, value: Option<Rc<dyn IImageBrushSource>>) {
        self.set_value(Self::large_display_picture_property(), value);
    }

    /// Gets the source display picture.
    pub fn source_display_picture(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.get_value(Self::source_display_picture_property())
    }

    /// Sets the source display picture.
    pub fn set_source_display_picture(&self, value: Option<Rc<dyn IImageBrushSource>>) {
        self.set_value(Self::source_display_picture_property(), value);
    }

    /// Gets the thumbnail.
    pub fn thumbnail(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.get_value(Self::thumbnail_property())
    }

    /// Sets the thumbnail.
    pub fn set_thumbnail(&self, value: Option<Rc<dyn IImageBrushSource>>) {
        self.set_value(Self::thumbnail_property(), value);
    }
}
