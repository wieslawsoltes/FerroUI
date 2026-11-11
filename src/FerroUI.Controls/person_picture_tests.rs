//! Tests of `PersonPicture` (WinUI; docs/porting/winui-controls.md, G2 of #418): the API tests of
//! WinUI (`controls/dev/PersonPicture/APITests/PersonPictureTests.cs`) and the behaviour of
//! `PersonPicture.cpp` (the badges, the contact, the circle).

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::AutomationProperties;
use crate::input_test_support::part;
use crate::shapes::Ellipse;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Contact, Control, FontIcon, Grid, PersonPicture, TextBlock, Window};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::IImageBrushSource;
use ferroui_base::{Ref, StyledElement};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn named<T: ferroui_base::ObjectType + ferroui_base::Upcast<StyledElement>>(element: Ref<T>, name: &str) -> Ref<T> {
    let styled: &StyledElement = (*element).upcast();
    styled.set_name(Some(name.to_string()));
    element
}

/// The parts of `Controls/PersonPicture.xaml`: the initials, and the badge with its number, its
/// glyph and its two ellipses.
fn person_picture_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let root = Grid::new();
        root.children().add(&named(TextBlock::new(), "InitialsTextBlock").register_in_name_scope(&**scope));
        let badge = Grid::new();
        for name in ["BadgingBackgroundEllipse", "BadgingEllipse"] {
            badge.children().add(&named(Ellipse::new(), name).register_in_name_scope(&**scope));
        }
        badge.children().add(&named(TextBlock::new(), "BadgeNumberTextBlock").register_in_name_scope(&**scope));
        badge.children().add(&named(FontIcon::new(), "BadgeGlyphIcon").register_in_name_scope(&**scope));
        root.children().add(&badge);
        root.upcast::<Control>()
    })
}

fn picture() -> Ref<PersonPicture> {
    let picture = PersonPicture::new();
    picture.set_template(Some(person_picture_template()));
    picture
}

fn show(picture: &PersonPicture) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(picture.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

fn name(picture: &PersonPicture) -> String {
    AutomationProperties::get_name(picture).unwrap_or_default()
}

fn has(picture: &PersonPicture, pseudo_class: &str) -> bool {
    picture.pseudo_classes().contains(pseudo_class)
}

/// An image of the tests: a source without a platform bitmap (the mock render platform makes none).
struct TestImage;

impl IImageBrushSource for TestImage {
    fn bitmap(&self) -> Option<&ferroui_base::utilities::RefCounted<ferroui_base::platform::SharedBitmapImpl>> {
        None
    }
}

fn image() -> Rc<dyn IImageBrushSource> {
    Rc::new(TestImage)
}

#[test]
fn the_defaults_are_those_of_winui_and_the_properties_can_be_set() {
    // VerifyDefaultsAndBasicSetting
    let _app = start();
    let picture = PersonPicture::new();
    assert_eq!("", picture.badge_glyph());
    assert_eq!(0, picture.badge_number());
    assert!(!picture.is_group());
    assert!(!picture.prefer_small_image());
    assert!(picture.profile_picture().is_none());
    assert!(picture.contact().is_none());
    assert_eq!("", picture.display_name());
    assert_eq!("", picture.initials());

    picture.set_badge_glyph("\u{E765}");
    assert_eq!("\u{E765}", picture.badge_glyph());
    picture.set_badge_number(10);
    assert_eq!(10, picture.badge_number());
    picture.set_is_group(true);
    assert!(picture.is_group());
    picture.set_prefer_small_image(true);
    assert!(picture.prefer_small_image());
    picture.set_display_name("Some Name");
    assert_eq!("Some Name", picture.display_name());
    picture.set_initials("MS");
    assert_eq!("MS", picture.initials());
    let contact = Contact::new();
    contact.set_first_name("FirstName");
    picture.set_contact(Some(contact));
    assert_eq!("FirstName", picture.contact().unwrap().first_name());
    picture.set_profile_picture(Some(image()));
    assert!(picture.profile_picture().is_some());
}

#[test]
fn the_automation_name_states_the_person_and_the_badge() {
    // VerifyAutomationName
    let _app = start();
    let picture = PersonPicture::new();
    picture.set_initials("AB");
    assert_eq!("AB", name(&picture));
    picture.set_display_name("Jane Smith");
    assert_eq!("Jane Smith", name(&picture));
    let contact = Contact::new();
    contact.set_first_name("John");
    contact.set_last_name("Doe");
    picture.set_contact(Some(contact));
    assert_eq!("John Doe", name(&picture));
    picture.set_is_group(true);
    assert_eq!("Group", name(&picture));
    picture.set_is_group(false);
    picture.set_badge_glyph("\u{E765}");
    assert_eq!("John Doe, icon", name(&picture));
    picture.set_badge_text("Skype");
    assert_eq!("John Doe, Skype", name(&picture));
    picture.set_badge_text("");
    picture.set_badge_number(5);
    assert_eq!("John Doe, 5 items", name(&picture));
    picture.set_badge_text("direct reports");
    assert_eq!("John Doe, 5 direct reports", name(&picture));
    picture.set_badge_text("");
    picture.set_badge_number(1);
    assert_eq!("John Doe, 1 item", name(&picture));
    // Nothing set: "Person", once the template is applied.
    let fresh = self::picture();
    let _window = show(&fresh);
    assert_eq!("Person", name(&fresh));
}

#[test]
fn the_states_follow_the_photo_the_initials_and_the_group() {
    // VerifyVSMStatesForPhotosAndInitials (the texts and fonts of the states are the theme's;
    // the theme tests check them).
    let _app = start();
    let picture = picture();
    let _window = show(&picture);
    assert!(has(&picture, ":nophotoorinitials"));
    picture.set_is_group(true);
    assert!(has(&picture, ":group") && !has(&picture, ":nophotoorinitials"));
    picture.set_is_group(false);
    picture.set_initials("JS");
    assert!(!has(&picture, ":group") && !has(&picture, ":nophotoorinitials") && !has(&picture, ":photo"));
    assert_eq!("JS", picture.template_settings().actual_initials());
    picture.set_initials("");
    assert!(has(&picture, ":nophotoorinitials"));
    picture.set_display_name("jane doe");
    assert_eq!("JD", picture.template_settings().actual_initials());
    picture.set_profile_picture(Some(image()));
    assert!(has(&picture, ":photo"));
    let brush = picture.template_settings().actual_image_brush().expect("the brush of the photo");
    assert!(brush.source().is_some());
    picture.set_profile_picture(None);
    assert!(picture.template_settings().actual_image_brush().is_none());
    assert!(!has(&picture, ":photo"));
}

#[test]
fn the_badge_shows_a_number_a_glyph_or_an_image() {
    let _app = start();
    let picture = picture();
    let _window = show(&picture);
    let number = part::<TextBlock>(&picture, "BadgeNumberTextBlock");
    let glyph = part::<FontIcon>(&picture, "BadgeGlyphIcon");
    assert!(!has(&picture, ":badge"));
    picture.set_badge_number(7);
    assert!(has(&picture, ":badge"));
    assert_eq!(Some("7"), number.text().as_deref());
    picture.set_badge_number(150);
    assert_eq!(Some("99+"), number.text().as_deref());
    picture.set_badge_number(0);
    assert!(!has(&picture, ":badge"));
    assert_eq!(Some(""), number.text().as_deref());
    picture.set_badge_glyph("\u{E765}");
    assert!(has(&picture, ":badge"));
    assert_eq!("\u{E765}", glyph.glyph());
    picture.set_badge_number(120);
    assert_eq!(Some("99+"), number.text().as_deref());
    assert!(glyph.glyph().is_empty());
    picture.set_badge_number(0);
    assert_eq!("\u{E765}", glyph.glyph());
    assert_eq!(Some(""), number.text().as_deref());
    // An image wins over the number and the glyph.
    picture.set_badge_image_source(Some(image()));
    assert!(has(&picture, ":badgeimage") && !has(&picture, ":badge"));
    assert!(glyph.glyph().is_empty());
    let ellipse = part::<Ellipse>(&picture, "BadgingEllipse");
    assert!(ellipse.fill().is_some());
}

#[test]
fn a_contact_gives_its_initials_and_its_preferred_picture() {
    let _app = start();
    let picture = picture();
    let _window = show(&picture);
    let contact = Contact::new();
    contact.set_first_name("ada");
    contact.set_last_name("lovelace");
    picture.set_contact(Some(contact.clone()));
    assert_eq!("AL", picture.template_settings().actual_initials());
    assert!(!has(&picture, ":photo"));
    // The display name comes before the contact.
    picture.set_display_name("Grace Hopper");
    assert_eq!("GH", picture.template_settings().actual_initials());

    let small = image();
    let large = image();
    let contact = Contact::new();
    contact.set_id("1");
    contact.set_small_display_picture(Some(small.clone()));
    contact.set_large_display_picture(Some(large.clone()));
    picture.set_contact(Some(contact));
    assert!(has(&picture, ":photo"));
    let shown = picture.template_settings().actual_image_brush().unwrap().source().unwrap();
    assert!(Rc::ptr_eq(&shown, &large));
    // PreferSmallImage applies when the contact is set.
    picture.set_prefer_small_image(true);
    let contact = Contact::new();
    contact.set_id("2");
    contact.set_small_display_picture(Some(small.clone()));
    contact.set_large_display_picture(Some(large));
    picture.set_contact(Some(contact));
    let shown = picture.template_settings().actual_image_brush().unwrap().source().unwrap();
    assert!(Rc::ptr_eq(&shown, &small));
    // The profile picture comes before the contact's.
    let profile = image();
    picture.set_profile_picture(Some(profile.clone()));
    let shown = picture.template_settings().actual_image_brush().unwrap().source().unwrap();
    assert!(Rc::ptr_eq(&shown, &profile));
    picture.set_profile_picture(None);
    picture.set_contact(None);
    assert!(!has(&picture, ":photo"));
}

#[test]
fn the_picture_keeps_itself_a_circle_and_scales_its_text() {
    let _app = start();
    let picture = picture();
    picture.set_width(100.0);
    picture.set_height(60.0);
    let window = show(&picture);
    window.layout_manager().execute_layout_pass();
    assert_eq!(60.0, picture.width());
    assert_eq!(60.0, picture.height());
    let initials = part::<TextBlock>(&picture, "InitialsTextBlock");
    assert!((initials.font_size() - 60.0 * 0.42).abs() < 1e-9);
    let ellipse = part::<Ellipse>(&picture, "BadgingEllipse");
    assert_eq!(30.0, ellipse.width());
    let number = part::<TextBlock>(&picture, "BadgeNumberTextBlock");
    assert!((number.font_size() - 18.0).abs() < 1e-9);
}

#[test]
fn a_tiny_picture_does_not_crash() {
    // VerifySmallWidthAndHeightDoNotCrash: the font size stays at least 1.
    let _app = start();
    let picture = picture();
    let window = show(&picture);
    picture.set_width(0.4);
    picture.set_height(0.4);
    window.layout_manager().execute_layout_pass();
    let initials = part::<TextBlock>(&picture, "InitialsTextBlock");
    assert_eq!(1.0, initials.font_size());
}

#[test]
fn the_peer_is_text() {
    let _app = start();
    let picture = picture();
    picture.set_display_name("Jane Doe");
    let _window = show(&picture);
    let peer = ControlAutomationPeer::create_peer_for_element(&picture);
    assert_eq!(AutomationControlType::Text, peer.get_automation_control_type());
    assert_eq!("PersonPicture", peer.get_class_name());
    assert_eq!("Jane Doe", peer.get_name());
}
