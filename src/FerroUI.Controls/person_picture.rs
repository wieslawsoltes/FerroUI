//! WinUI's `PersonPicture` (`controls/dev/PersonPicture/PersonPicture.cpp`, `.h` and `.idl` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use crate::automation::peers::{AutomationPeer, PersonPictureAutomationPeer};
use crate::automation::AutomationProperties;
use crate::primitives::{PersonPictureTemplateSettings, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use crate::shapes::Ellipse;
use crate::utils::initials_generator;
use crate::{Contact, ControlImpl, ControlImplExt, FontIcon, SizeChangedEventArgs, TextBlock};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{IImageBrushSource, ImageBrush, Stretch};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

// The English strings of WinUI's resources (`Strings/en-us/Resources.resw`).
const GROUP_NAME: &str = "Group";
const PERSON_NAME: &str = "Person";

#[derive(Clone, Default)]
struct Parts {
    initials_text_block: Option<Ref<TextBlock>>,
    badge_number_text_block: Option<Ref<TextBlock>>,
    badge_glyph_icon: Option<Ref<FontIcon>>,
    badging_ellipse: Option<Ref<Ellipse>>,
    badging_background_ellipse: Option<Ref<Ellipse>>,
}

/// A picture of a person or a group (WinUI's `PersonPicture`): the profile picture, else the
/// initials of the display name or of the contact, else the glyph of a contact, in a circle,
/// with a badge (a number, a glyph or an image) at its top right.
///
/// The template is the control theme of the WinUI theme (`Controls/PersonPicture.xaml`, from
/// WinUI's `PersonPicture.xaml`), with WinUI's parts `InitialsTextBlock`, `BadgeNumberTextBlock`,
/// `BadgeGlyphIcon`, `BadgingEllipse` and `BadgingBackgroundEllipse`. The control sets WinUI's
/// visual states as pseudo-classes: `:photo`, `:nophotoorinitials` and `:group` (CommonStates;
/// Initials sets none), `:badge` and `:badgeimage` (BadgeStates). It keeps itself a circle: a
/// change of its size sets its width and height to the smaller of them, as WinUI's does. The
/// Fluent and Simple themes have no template for it (docs/porting/winui-controls.md, section 5.1).
///
/// Differences (docs/porting/winui-controls.md, section 10.10): the contact is FerroUI's
/// [`Contact`], whose pictures are images, so the picture of a contact is shown at once (WinUI
/// opens and decodes a stream reference asynchronously, at the size of the control); the
/// pictures are image brush sources (a bitmap), the type of the image brush that paints them.
#[repr(C)]
pub struct PersonPicture {
    base: TemplatedControl,
    parts: RefCell<Parts>,
    display_name_initials: RefCell<String>,
    contact_display_name_initials: RefCell<String>,
    contact_image_source: RefCell<Option<Rc<dyn IImageBrushSource>>>,
    badge_image_brush: Ref<ImageBrush>,
}

ferro_class!(PersonPicture: TemplatedControl);
ferroui_base::ferro_class_info!(PersonPicture {
    new: PersonPicture::new,
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [
            TemplatePart("InitialsTextBlock", type(Ref<TextBlock>)),
            TemplatePart("BadgeNumberTextBlock", type(Ref<TextBlock>)),
            TemplatePart("BadgeGlyphIcon", type(Ref<FontIcon>)),
            TemplatePart("BadgingEllipse", type(Ref<Ellipse>)),
            TemplatePart("BadgingBackgroundEllipse", type(Ref<Ellipse>)),
            PseudoClasses(":photo", ":nophotoorinitials", ":group", ":badge", ":badgeimage"),
        ],
    },
});
ferro_impl_classes!(PersonPicture: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

/// The states of the groups of the template (WinUI's visual states).
#[derive(Clone, Copy, PartialEq, Eq)]
enum CommonState {
    Photo,
    Initials,
    NoPhotoOrInitials,
    Group,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BadgeState {
    NoBadge,
    BadgeWithoutImageSource,
    BadgeWithImageSource,
}

impl FerroObjectImpl for PersonPicture {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::badge_number_property().as_property()
            || property == Self::badge_glyph_property().as_property()
            || property == Self::badge_image_source_property().as_property()
        {
            this.update_badge();
        } else if property == Self::badge_text_property().as_property() {
            this.update_automation_name();
        } else if property == Self::contact_property().as_property() {
            let old = change.get_old_value::<Option<Ref<Contact>>>().flatten();
            let new = change.get_new_value::<Option<Ref<Contact>>>();
            this.on_contact_changed(old, new);
        } else if property == Self::display_name_property().as_property() {
            // OnDisplayNameChanged
            *this.display_name_initials.borrow_mut() = initials_generator::initials_from_display_name(&this.display_name());
            this.update_if_ready();
        } else if property == Self::profile_picture_property().as_property()
            || property == Self::initials_property().as_property()
            || property == Self::is_group_property().as_property()
        {
            this.update_if_ready();
        }
        // "No additional action required for s_PreferSmallImageProperty"
    }
}

impl ControlImpl for PersonPicture {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        PersonPictureAutomationPeer::new(this).upcast()
    }

    fn on_size_changed(this: &Self, args: &SizeChangedEventArgs) {
        Self::parent_on_size_changed(this, args);
        let new_size = args.new_size();
        let previous_size = args.previous_size();
        let width_changed = new_size.width != previous_size.width;
        let height_changed = new_size.height != previous_size.height;
        let size = if width_changed && height_changed {
            // "Maintain circle by enforcing the new size on both Width and Height. To do so, we
            // will use the minimum value."
            new_size.width.min(new_size.height)
        } else if width_changed {
            new_size.width
        } else if height_changed {
            new_size.height
        } else {
            return;
        };
        this.set_height(size);
        this.set_width(size);

        // "Design guidelines have specified the font size to be 42% of the container", at least 1.
        let font_size = (this.width() * 0.42).max(1.0);
        let parts = this.parts.borrow().clone();
        if let Some(initials) = &parts.initials_text_block {
            initials.set_font_size(font_size);
        }
        if let (Some(badging), Some(background), Some(number), Some(glyph)) = (
            &parts.badging_ellipse,
            &parts.badging_background_ellipse,
            &parts.badge_number_text_block,
            &parts.badge_glyph_icon,
        ) {
            // "We want to keep badging plate to be about 50% of the control", and its text 60% of
            // the plate.
            let plate = new_size.width.min(new_size.height) * 0.5;
            for ellipse in [badging, background] {
                ellipse.set_height(plate);
                ellipse.set_width(plate);
            }
            number.set_font_size(Self::badge_number_font_size(plate, this.badge_number()));
            glyph.set_font_size((plate * 0.6).max(1.0));
        }
    }
}

impl TemplatedControlImpl for PersonPicture {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        let parts = Parts {
            initials_text_block: scope.find_as::<TextBlock>("InitialsTextBlock"),
            badge_number_text_block: scope.find_as::<TextBlock>("BadgeNumberTextBlock"),
            badge_glyph_icon: scope.find_as::<FontIcon>("BadgeGlyphIcon"),
            badging_ellipse: scope.find_as::<Ellipse>("BadgingEllipse"),
            badging_background_ellipse: scope.find_as::<Ellipse>("BadgingBackgroundEllipse"),
        };
        // The image brush of the badge (BadgeImageBrush, the Fill of BadgingEllipse in the state
        // BadgeWithImageSource) belongs to the control; BadgingEllipse is transparent until that
        // state shows it.
        if let Some(ellipse) = &parts.badging_ellipse {
            ellipse.set_fill(Some(this.badge_image_brush.clone().into()));
        }
        *this.parts.borrow_mut() = parts;
        this.update_badge();
        this.update_if_ready();
    }
}

ferroui_base::ferro_properties! { impl PersonPicture {
    ferro_property!(
        /// Defines the `BadgeNumber` property: the number of the badge, none at 0 (the default);
        /// more than 99 shows "99+".
        pub fn badge_number_property() -> StyledProperty<i32> {
            FerroProperty::register::<PersonPicture, _>("BadgeNumber", 0)
        }
    );

    ferro_property!(
        /// Defines the `BadgeGlyph` property: a glyph of the symbol font for the badge.
        pub fn badge_glyph_property() -> StyledProperty<String> {
            FerroProperty::register::<PersonPicture, _>("BadgeGlyph", String::new())
        }
    );

    ferro_property!(
        /// Defines the `BadgeImageSource` property: an image for the badge.
        pub fn badge_image_source_property() -> StyledProperty<Option<Rc<dyn IImageBrushSource>>> {
            FerroProperty::register::<PersonPicture, _>("BadgeImageSource", None)
        }
    );

    ferro_property!(
        /// Defines the `BadgeText` property: what the badge stands for in the automation name
        /// ("5 messages" instead of "5 items").
        pub fn badge_text_property() -> StyledProperty<String> {
            FerroProperty::register::<PersonPicture, _>("BadgeText", String::new())
        }
    );

    ferro_property!(
        /// Defines the `IsGroup` property: whether the picture stands for a group.
        pub fn is_group_property() -> StyledProperty<bool> {
            FerroProperty::register::<PersonPicture, _>("IsGroup", false)
        }
    );

    ferro_property!(
        /// Defines the `Contact` property: the contact whose picture and initials are shown.
        pub fn contact_property() -> StyledProperty<Option<Ref<Contact>>> {
            FerroProperty::register::<PersonPicture, _>("Contact", None)
        }
    );

    ferro_property!(
        /// Defines the `DisplayName` property: the name the initials are made from.
        pub fn display_name_property() -> StyledProperty<String> {
            FerroProperty::register::<PersonPicture, _>("DisplayName", String::new())
        }
    );

    ferro_property!(
        /// Defines the `Initials` property: the initials, before those of the display name.
        pub fn initials_property() -> StyledProperty<String> {
            FerroProperty::register::<PersonPicture, _>("Initials", String::new())
        }
    );

    ferro_property!(
        /// Defines the `PreferSmallImage` property: whether the small picture of the contact is
        /// shown before the large one.
        pub fn prefer_small_image_property() -> StyledProperty<bool> {
            FerroProperty::register::<PersonPicture, _>("PreferSmallImage", false)
        }
    );

    ferro_property!(
        /// Defines the `ProfilePicture` property: the picture, before the one of the contact.
        pub fn profile_picture_property() -> StyledProperty<Option<Rc<dyn IImageBrushSource>>> {
            FerroProperty::register::<PersonPicture, _>("ProfilePicture", None)
        }
    );

    ferro_property!(
        /// Defines the `TemplateSettings` property: the initials and the brush the template binds to.
        pub fn template_settings_property() -> StyledProperty<Option<Ref<PersonPictureTemplateSettings>>> {
            FerroProperty::register::<PersonPicture, _>("TemplateSettings", None)
        }
    );
} }

impl PersonPicture {
    fn static_constructor() {
        // The badge of WinUI's template is 4 units above and to the right of the picture
        // (PersonPictureBadgeGridMargin), and a control of WinUI does not clip to its bounds.
        Visual::clip_to_bounds_property().override_default_value::<PersonPicture>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        let badge_image_brush = ImageBrush::new();
        badge_image_brush.set_stretch(Stretch::UniformToFill);
        Self {
            base: TemplatedControl::construct(),
            parts: RefCell::new(Parts::default()),
            display_name_initials: RefCell::new(String::new()),
            contact_display_name_initials: RefCell::new(String::new()),
            contact_image_source: RefCell::new(None),
            badge_image_brush,
        }
    }

    /// Creates a picture without a person.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_value(Self::template_settings_property(), Some(PersonPictureTemplateSettings::new()));
        this
    }

    /// Gets the number of the badge.
    pub fn badge_number(&self) -> i32 {
        self.get_value(Self::badge_number_property())
    }

    /// Sets the number of the badge.
    pub fn set_badge_number(&self, value: i32) {
        self.set_value(Self::badge_number_property(), value);
    }

    /// Gets the glyph of the badge.
    pub fn badge_glyph(&self) -> String {
        self.get_value(Self::badge_glyph_property())
    }

    /// Sets the glyph of the badge.
    pub fn set_badge_glyph(&self, value: impl Into<String>) {
        self.set_value(Self::badge_glyph_property(), value.into());
    }

    /// Gets the image of the badge.
    pub fn badge_image_source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.get_value(Self::badge_image_source_property())
    }

    /// Sets the image of the badge.
    pub fn set_badge_image_source(&self, value: Option<Rc<dyn IImageBrushSource>>) {
        self.set_value(Self::badge_image_source_property(), value);
    }

    /// Gets what the badge stands for in the automation name.
    pub fn badge_text(&self) -> String {
        self.get_value(Self::badge_text_property())
    }

    /// Sets what the badge stands for in the automation name.
    pub fn set_badge_text(&self, value: impl Into<String>) {
        self.set_value(Self::badge_text_property(), value.into());
    }

    /// Gets whether the picture stands for a group.
    pub fn is_group(&self) -> bool {
        self.get_value(Self::is_group_property())
    }

    /// Sets whether the picture stands for a group.
    pub fn set_is_group(&self, value: bool) {
        self.set_value(Self::is_group_property(), value);
    }

    /// Gets the contact.
    pub fn contact(&self) -> Option<Ref<Contact>> {
        self.get_value(Self::contact_property())
    }

    /// Sets the contact.
    pub fn set_contact(&self, value: Option<Ref<Contact>>) {
        self.set_value(Self::contact_property(), value);
    }

    /// Gets the display name.
    pub fn display_name(&self) -> String {
        self.get_value(Self::display_name_property())
    }

    /// Sets the display name.
    pub fn set_display_name(&self, value: impl Into<String>) {
        self.set_value(Self::display_name_property(), value.into());
    }

    /// Gets the initials.
    pub fn initials(&self) -> String {
        self.get_value(Self::initials_property())
    }

    /// Sets the initials.
    pub fn set_initials(&self, value: impl Into<String>) {
        self.set_value(Self::initials_property(), value.into());
    }

    /// Gets whether the small picture of the contact is shown before the large one.
    pub fn prefer_small_image(&self) -> bool {
        self.get_value(Self::prefer_small_image_property())
    }

    /// Sets whether the small picture of the contact is shown before the large one.
    pub fn set_prefer_small_image(&self, value: bool) {
        self.set_value(Self::prefer_small_image_property(), value);
    }

    /// Gets the profile picture.
    pub fn profile_picture(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.get_value(Self::profile_picture_property())
    }

    /// Sets the profile picture.
    pub fn set_profile_picture(&self, value: Option<Rc<dyn IImageBrushSource>>) {
        self.set_value(Self::profile_picture_property(), value);
    }

    /// Gets the initials and the brush the template binds to.
    pub fn template_settings(&self) -> Ref<PersonPictureTemplateSettings> {
        self.get_value(Self::template_settings_property()).expect("The picture creates its template settings.")
    }

    fn go_to_common_state(&self, state: CommonState) {
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(":photo", state == CommonState::Photo);
        pseudo_classes.set(":nophotoorinitials", state == CommonState::NoPhotoOrInitials);
        pseudo_classes.set(":group", state == CommonState::Group);
    }

    fn go_to_badge_state(&self, state: BadgeState) {
        let pseudo_classes = self.pseudo_classes();
        pseudo_classes.set(":badge", state == BadgeState::BadgeWithoutImageSource);
        pseudo_classes.set(":badgeimage", state == BadgeState::BadgeWithImageSource);
    }

    fn get_initials(&self) -> String {
        let initials = self.initials();
        if !initials.is_empty() {
            return initials;
        }
        let display_name_initials = self.display_name_initials.borrow();
        if !display_name_initials.is_empty() {
            return display_name_initials.clone();
        }
        self.contact_display_name_initials.borrow().clone()
    }

    fn get_image_source(&self) -> Option<Rc<dyn IImageBrushSource>> {
        self.profile_picture().or_else(|| self.contact_image_source.borrow().clone())
    }

    fn update_if_ready(&self) {
        let initials = self.get_initials();
        let image_source = self.get_image_source();
        let settings = self.template_settings();
        settings.set_actual_initials(initials.clone());
        if let Some(image_source) = &image_source {
            let brush = settings.actual_image_brush().unwrap_or_else(|| {
                let brush = ImageBrush::new();
                brush.set_stretch(Stretch::UniformToFill);
                settings.set_actual_image_brush(Some(brush.clone()));
                brush
            });
            brush.set_source(Some(image_source.clone()));
        } else {
            settings.set_actual_image_brush(None);
        }

        // "If the control is converted to 'Group-mode', we'll clear individual-specific
        // information. When IsGroup evaluates to false, we will restore state."
        if self.is_group() {
            self.go_to_common_state(CommonState::Group);
        } else if image_source.is_some() {
            self.go_to_common_state(CommonState::Photo);
        } else if !initials.is_empty() {
            self.go_to_common_state(CommonState::Initials);
        } else {
            self.go_to_common_state(CommonState::NoPhotoOrInitials);
        }
        self.update_automation_name();
    }

    fn badge_number_font_size(plate: f64, value: i32) -> f64 {
        // Leave enough room for all three characters of "99+" in the circle.
        let scale = if value > 99 { 0.42 } else { 0.6 };
        (plate * scale).max(1.0)
    }

    fn update_badge(&self) {
        let parts = self.parts.borrow().clone();
        let image = self.badge_image_source();
        let number = self.badge_number();
        let number_text = match (image.is_some(), number) {
            (false, 1..=99) => number.to_string(),
            (false, 100..) => "99+".to_owned(),
            _ => String::new(),
        };
        let glyph = if image.is_none() && number == 0 { self.badge_glyph() } else { String::new() };
        let state = if image.is_some() {
            BadgeState::BadgeWithImageSource
        } else if !number_text.is_empty() || !glyph.is_empty() {
            BadgeState::BadgeWithoutImageSource
        } else {
            BadgeState::NoBadge
        };
        self.badge_image_brush.set_source(image);
        if let Some(text) = parts.badge_number_text_block {
            // The plate has its size from the first size change on; before that the size of
            // the number is the one of the template.
            let plate = parts.badging_ellipse.as_ref().map(|plate| plate.width()).filter(|width| width.is_finite());
            if let Some(plate) = plate {
                text.set_font_size(Self::badge_number_font_size(plate, number));
            }
            text.set_text(Some(&number_text));
        }
        if let Some(icon) = parts.badge_glyph_icon {
            icon.set_glyph(glyph);
        }
        self.go_to_badge_state(state);
        self.update_automation_name();
    }

    /// The automation name: the name of the person, and what the badge shows (WinUI's
    /// `UpdateAutomationName`): "Jane Doe, 5 items", "Jane Doe, icon", or with `BadgeText`
    /// "Jane Doe, 5 messages" and "Jane Doe, Skype".
    fn update_automation_name(&self) {
        let contact_name = if self.is_group() {
            GROUP_NAME.to_string()
        } else if let Some(name) = self.contact().map(|contact| contact.display_name()).filter(|name| !name.is_empty()) {
            name
        } else if !self.display_name().is_empty() {
            self.display_name()
        } else if !self.initials().is_empty() {
            self.initials()
        } else {
            PERSON_NAME.to_string()
        };
        let badge_text = self.badge_text();
        let badge_number = self.badge_number();
        let automation_name = if badge_number > 0 {
            if !badge_text.is_empty() {
                format!("{contact_name}, {badge_number} {badge_text}")
            } else {
                // GetLocalizedPluralBadgeItemStringResource: every plural form of the English
                // resources is "items".
                let items = if badge_number == 1 { "item" } else { "items" };
                format!("{contact_name}, {badge_number} {items}")
            }
        } else if !self.badge_glyph().is_empty() || self.badge_image_source().is_some() {
            if !badge_text.is_empty() {
                format!("{contact_name}, {badge_text}")
            } else {
                format!("{contact_name}, icon")
            }
        } else {
            contact_name
        };
        AutomationProperties::set_name(self, Some(&automation_name));
    }

    fn on_contact_changed(&self, old: Option<Ref<Contact>>, new: Option<Ref<Contact>>) {
        let mut is_new_contact = true;
        if let (Some(old), Some(new)) = (&old, &new) {
            // "If both contact IDs are null, it will be treated as a new contact."
            if !old.id().is_empty() || !new.id().is_empty() {
                is_new_contact = old.id() != new.id();
            }
        }
        self.update_control_for_contact(is_new_contact);
    }

    fn update_control_for_contact(&self, is_new_contact: bool) {
        let Some(contact) = self.contact() else {
            // "Explicitly setting to empty/nullptr ensures the bound XAML is correctly updated."
            self.contact_display_name_initials.borrow_mut().clear();
            *self.contact_image_source.borrow_mut() = None;
            self.update_if_ready();
            return;
        };
        *self.contact_display_name_initials.borrow_mut() = initials_generator::initials_from_contact_object(Some(&contact));
        // "Order of preference (but all work): Large, Small, Source, Thumbnail"
        let picture = if self.prefer_small_image() && contact.small_display_picture().is_some() {
            contact.small_display_picture()
        } else {
            contact
                .large_display_picture()
                .or_else(|| contact.small_display_picture())
                .or_else(|| contact.source_display_picture())
                .or_else(|| contact.thumbnail())
        };
        if picture.is_none() || is_new_contact {
            // "Prevent the case where context of a contact changes, but we show an old person
            // while the new one is loaded."
            *self.contact_image_source.borrow_mut() = None;
        }
        if picture.is_some() {
            *self.contact_image_source.borrow_mut() = picture;
        }
        self.update_if_ready();
    }
}
