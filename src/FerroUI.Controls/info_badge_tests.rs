//! Tests of `InfoBadge` (WinUI; docs/porting/winui-controls.md, section 10.2), adapted from the
//! tests of #286. The control has no template of its own (the WinUI theme has it,
//! `src/FerroUI.Themes.WinUI/Controls/InfoBadge.xaml`); the looks of the display kinds are the
//! theme's, so the tests check the pseudo-classes and the template settings the control sets.

use crate::primitives::TemplatedControl;
use crate::templates::FuncControlTemplate;
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Border, FontIcon, FontIconSource, InfoBadge, PathIcon, PathIconSource, Symbol, SymbolIcon, SymbolIconSource};
use ferroui_base::data::BindingPriority;
use ferroui_base::media::Geometry;
use ferroui_base::{CornerRadius, Rect, Size};

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::styled_window())
}

fn layout(badge: &InfoBadge) {
    badge.measure(Size::new(1000.0, 1000.0));
    badge.arrange(Rect::from_size(badge.desired_size()));
}

/// The display kind the pseudo-classes of `badge` name.
fn display_kind(badge: &InfoBadge) -> Vec<&'static str> {
    [":dot", ":icon", ":font-icon", ":value"].into_iter().filter(|class| badge.classes().contains(class)).collect()
}

/// A template whose root has a fixed size.
fn sized_template(width: f64, height: f64) -> std::rc::Rc<FuncControlTemplate> {
    FuncControlTemplate::new(move |_, _| {
        let root = Border::new();
        root.set_width(width);
        root.set_height(height);
        root.upcast()
    })
}

#[test]
fn the_default_badge_is_a_dot_with_its_own_settings() {
    let _app = start();
    let first = InfoBadge::new();
    let second = InfoBadge::new();
    assert_eq!(-1, first.value());
    assert!(first.icon_source().is_none());
    assert_ne!(first.template_settings(), second.template_settings());
    assert!(first.template_settings().icon_element().is_none());
    assert!(InfoBadge::template_settings_property().setter().is_none());
    assert_eq!(vec![":dot"], display_kind(&first));
}

#[test]
fn a_number_takes_precedence_over_the_icon_and_minus_one_shows_the_icon_or_the_dot() {
    let _app = start();
    let badge = InfoBadge::new();
    let source = SymbolIconSource::new();
    source.set_symbol(Symbol::Setting);
    badge.set_icon_source(Some(source.upcast()));
    assert_eq!(vec![":icon"], display_kind(&badge));
    let icon = badge.template_settings().icon_element().expect("an icon").cast::<SymbolIcon>().expect("a symbol icon");
    assert_eq!(Symbol::Setting, icon.symbol());

    for value in [0, 10, i32::MAX] {
        badge.set_numeric_value(value);
        assert_eq!(vec![":value"], display_kind(&badge));
    }
    badge.set_numeric_value(-1);
    assert_eq!(vec![":icon"], display_kind(&badge));
    badge.set_numeric_value(10);
    badge.set_icon_source(None);
    assert_eq!(vec![":value"], display_kind(&badge));
    badge.set_numeric_value(-1);
    assert_eq!(vec![":dot"], display_kind(&badge));
}

#[test]
fn a_value_below_minus_one_is_coerced_to_minus_one() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.set_numeric_value(7);
    for invalid in [-2, -10, i32::MIN] {
        // WinUI throws; the port coerces and logs, so markup and bindings cannot end the
        // application.
        badge.set_numeric_value(invalid);
        assert_eq!(-1, badge.value());
        badge.set_numeric_value(7);
    }
}

#[test]
fn the_icon_sources_create_their_own_elements_and_a_font_icon_has_its_state() {
    let _app = start();
    let badge = InfoBadge::new();
    let font = FontIconSource::new();
    font.set_glyph("99+");
    badge.set_icon_source(Some(font.clone().upcast()));
    assert_eq!(vec![":font-icon"], display_kind(&badge));
    let icon = badge.template_settings().icon_element().expect("an icon").cast::<FontIcon>().expect("a font icon");
    assert_eq!("99+", icon.glyph());
    font.set_glyph("3");
    assert_eq!("3", icon.glyph());

    let path = PathIconSource::new();
    let geometry = Geometry::parse("M0,0 L10,0 L5,5 Z").expect("a geometry");
    path.set_data(Some(geometry.clone()));
    badge.set_icon_source(Some(path.upcast()));
    assert_eq!(vec![":icon"], display_kind(&badge));
    let icon = badge.template_settings().icon_element().expect("an icon").cast::<PathIcon>().expect("a path icon");
    assert_eq!(Some(geometry), icon.data());
}

#[test]
fn badges_that_share_an_icon_source_have_their_own_icons() {
    let _app = start();
    let first = InfoBadge::new();
    let second = InfoBadge::new();
    let source = FontIconSource::new();
    source.set_glyph("A");
    first.set_icon_source(Some(source.clone().upcast()));
    second.set_icon_source(Some(source.clone().upcast()));
    let first_icon = first.template_settings().icon_element().expect("an icon");
    let second_icon = second.template_settings().icon_element().expect("an icon");
    assert_ne!(first_icon, second_icon);
    source.set_glyph("B");
    assert_eq!("B", first_icon.cast::<FontIcon>().expect("a font icon").glyph());
    assert_eq!("B", second_icon.cast::<FontIcon>().expect("a font icon").glyph());
}

#[test]
fn a_narrow_badge_is_square_and_a_wide_one_keeps_its_width() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.set_template(Some(sized_template(3.0, 12.0)));
    layout(&badge);
    assert_eq!(Size::new(12.0, 12.0), badge.desired_size());
    badge.set_template(Some(sized_template(30.0, 8.0)));
    layout(&badge);
    assert_eq!(Size::new(30.0, 8.0), badge.desired_size());
}

#[test]
fn the_radius_is_half_the_height_unless_the_corner_radius_is_local() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.set_height(12.0);
    layout(&badge);
    assert_eq!(CornerRadius::uniform(6.0), badge.template_settings().info_badge_corner_radius());

    // A local corner radius is the radius.
    badge.set_corner_radius(CornerRadius::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(CornerRadius::new(1.0, 2.0, 3.0, 4.0), badge.template_settings().info_badge_corner_radius());
    badge.set_height(16.0);
    layout(&badge);
    assert_eq!(CornerRadius::new(1.0, 2.0, 3.0, 4.0), badge.template_settings().info_badge_corner_radius());

    // Cleared: half the height again.
    badge.clear_value(TemplatedControl::corner_radius_property());
    assert_eq!(CornerRadius::uniform(8.0), badge.template_settings().info_badge_corner_radius());

    // A corner radius of a style is not local.
    let style = badge.set_value_with_priority(
        TemplatedControl::corner_radius_property(),
        CornerRadius::uniform(3.0),
        BindingPriority::Style,
    );
    assert_eq!(CornerRadius::uniform(8.0), badge.template_settings().info_badge_corner_radius());
    badge.set_height(14.0);
    layout(&badge);
    assert_eq!(CornerRadius::uniform(7.0), badge.template_settings().info_badge_corner_radius());
    if let Some(style) = style {
        style.dispose();
    }
}

#[test]
fn an_animation_of_the_corner_radius_keeps_a_local_one_local() {
    let _app = start();
    let badge = InfoBadge::new();
    let property = TemplatedControl::corner_radius_property();
    badge.set_height(12.0);
    badge.set_corner_radius(CornerRadius::uniform(3.0));
    layout(&badge);
    let animation = badge
        .set_value_with_priority(property, CornerRadius::uniform(5.0), BindingPriority::Animation)
        .expect("an animation");
    // The animated value is the effective corner radius, and the local one stays local.
    assert_eq!(CornerRadius::uniform(5.0), badge.template_settings().info_badge_corner_radius());
    animation.dispose();
    assert_eq!(CornerRadius::uniform(3.0), badge.template_settings().info_badge_corner_radius());
    badge.clear_value(property);
    assert_eq!(CornerRadius::uniform(6.0), badge.template_settings().info_badge_corner_radius());
}

#[test]
fn the_settings_of_a_dropped_badge_do_not_keep_it() {
    let _app = start();
    let badge = InfoBadge::new();
    let settings = badge.template_settings();
    let weak = badge.downgrade();
    drop(badge);
    assert!(weak.upgrade().is_none());
    settings.set_info_badge_corner_radius(CornerRadius::uniform(2.0));
    assert!(weak.upgrade().is_none());
}
