//! Native InfoBadge contracts derived from the pinned WinUI API tests and template.

use ferroui_base::data::BindingPriority;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::{Brushes, Geometry, IBrush};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{CornerRadius, FerroObject, Rect, Ref, Size, Thickness};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    FontIcon, FontIconSource, InfoBadge, InfoBadgeTemplateSettings, PathIconSource, Symbol,
    SymbolIcon, SymbolIconSource,
};
use ferroui_controls::{
    Border, Control, PathIcon, StackPanel, TextBlock, ThemeVariantScope, Viewbox, Window,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn part<T: ferroui_base::ObjectType>(badge: &InfoBadge, name: &str) -> Ref<T> {
    badge
        .get_template_descendants()
        .iter()
        .find(|element| element.name().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing InfoBadge part {name}"))
        .cast::<T>()
        .unwrap()
}

fn layout(badge: &InfoBadge) {
    badge.measure(Size::new(1000.0, 1000.0));
    badge.arrange(Rect::from_size(badge.desired_size()));
}

#[test]
fn default_badge_is_a_nonfocusable_dot_with_instance_owned_settings() {
    let _app = start();
    let first = InfoBadge::new();
    let second = InfoBadge::new();
    assert_eq!(first.value(), -1);
    assert!(first.icon_source().is_none());
    assert!(!first.focusable());
    assert_eq!(first.min_width(), 4.0);
    assert_eq!(first.min_height(), 4.0);
    assert_eq!(first.max_height(), 16.0);
    assert_ne!(first.template_settings(), second.template_settings());
    assert!(first.template_settings().icon_element().is_none());
    assert!(InfoBadge::template_settings_property().setter().is_none());
    layout(&first);
    assert!(!part::<TextBlock>(&first, "ValueTextBlock").is_visible());
    assert!(!part::<Viewbox>(&first, "IconPresenter").is_visible());
    assert_eq!(first.desired_size(), Size::new(4.0, 4.0));
    assert_eq!(
        first.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(2.0)
    );
}

#[test]
fn numeric_values_take_priority_over_icons_and_minus_one_restores_the_icon_or_dot() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.apply_template();
    let text = part::<TextBlock>(&badge, "ValueTextBlock");
    let presenter = part::<Viewbox>(&badge, "IconPresenter");
    let source = SymbolIconSource::new();
    source.set_symbol(Symbol::Setting);
    badge.set_icon_source(Some(source.upcast()));
    assert!(!text.is_visible());
    assert!(presenter.is_visible());
    assert_eq!(
        badge
            .template_settings()
            .icon_element()
            .unwrap()
            .cast::<SymbolIcon>()
            .unwrap()
            .symbol(),
        Symbol::Setting
    );
    for value in [0, 10, i32::MAX] {
        badge.set_numeric_value(value);
        assert!(text.is_visible());
        assert!(!presenter.is_visible());
        assert_eq!(text.text().as_deref(), Some(value.to_string().as_str()));
    }
    badge.set_numeric_value(-1);
    assert!(!text.is_visible());
    assert!(presenter.is_visible());
    badge.set_numeric_value(10);
    badge.set_icon_source(None);
    assert!(text.is_visible());
    assert!(!presenter.is_visible());
    badge.set_numeric_value(-1);
    assert!(!text.is_visible());
    assert!(!presenter.is_visible());
}

#[test]
fn values_below_minus_one_are_rejected_before_the_effective_value_changes() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.set_numeric_value(7);
    for invalid in [-2, -10, i32::MIN] {
        assert!(catch_unwind(AssertUnwindSafe(|| badge.set_numeric_value(invalid))).is_err());
        assert_eq!(badge.value(), 7);
        assert!(catch_unwind(AssertUnwindSafe(|| {
            FerroObject::set_value(&badge, InfoBadge::value_property(), invalid);
        }))
        .is_err());
        assert_eq!(badge.value(), 7);
    }
}

#[test]
fn icon_source_types_keep_their_native_elements_margins_and_live_updates() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.apply_template();
    let presenter = part::<Viewbox>(&badge, "IconPresenter");
    let font = FontIconSource::new();
    font.set_glyph("99+");
    badge.set_icon_source(Some(font.clone().upcast()));
    let icon = badge
        .template_settings()
        .icon_element()
        .unwrap()
        .cast::<FontIcon>()
        .unwrap();
    assert_eq!(icon.glyph(), "99+");
    assert_eq!(presenter.margin(), Thickness::new(4.0, 0.0, 4.0, 2.0));
    font.set_glyph("3");
    assert_eq!(icon.glyph(), "3");
    let path = PathIconSource::new();
    let geometry = Geometry::parse("M0,0 L10,0 L5,5 Z").unwrap();
    path.set_data(Some(geometry.clone()));
    badge.set_icon_source(Some(path.upcast()));
    assert_eq!(presenter.margin(), Thickness::uniform(4.0));
    assert_eq!(
        badge
            .template_settings()
            .icon_element()
            .unwrap()
            .cast::<PathIcon>()
            .unwrap()
            .data(),
        Some(geometry)
    );
}

#[test]
fn reusable_icon_sources_never_share_parented_elements_between_badges() {
    let _app = start();
    let first = InfoBadge::new();
    let second = InfoBadge::new();
    let source = FontIconSource::new();
    source.set_glyph("A");
    first.set_icon_source(Some(source.clone().upcast()));
    second.set_icon_source(Some(source.clone().upcast()));
    // Native content presenters realize children once attached to a logical
    // tree. Measuring detached badges does not exercise parent ownership.
    let panel = StackPanel::new();
    panel.children().add(first.clone().upcast::<Control>());
    panel.children().add(second.clone().upcast::<Control>());
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.layout_manager().execute_layout_pass();
    let first_icon = first
        .template_settings()
        .icon_element()
        .unwrap()
        .cast::<FontIcon>()
        .unwrap();
    let second_icon = second
        .template_settings()
        .icon_element()
        .unwrap()
        .cast::<FontIcon>()
        .unwrap();
    assert_ne!(first_icon, second_icon);
    source.set_glyph("B");
    assert_eq!(first_icon.glyph(), "B");
    assert_eq!(second_icon.glyph(), "B");
    let first_presenter = part::<ContentPresenter>(&first, "IconContentPresenter");
    let second_presenter = part::<ContentPresenter>(&second, "IconContentPresenter");
    assert_eq!(first_presenter.child(), Some(first_icon.clone().upcast()));
    assert_eq!(second_presenter.child(), Some(second_icon.clone().upcast()));
    assert_eq!(first_icon.parent(), Some(first_presenter.clone().upcast()));
    assert_eq!(
        second_icon.parent(),
        Some(second_presenter.clone().upcast())
    );
    assert_eq!(
        first_icon.visual_parent(),
        Some(first_presenter.clone().upcast())
    );
    assert_eq!(
        second_icon.visual_parent(),
        Some(second_presenter.clone().upcast())
    );
    first.set_icon_source(Some(SymbolIconSource::new().upcast()));
    window.layout_manager().execute_layout_pass();
    assert!(first_icon.parent().is_none());
    assert!(first_icon.visual_parent().is_none());
    assert!(first_presenter.child().unwrap().is::<SymbolIcon>());
    assert_eq!(second_presenter.child(), Some(second_icon.clone().upcast()));
    source.set_glyph("C");
    assert_eq!(second_icon.glyph(), "C");
    assert_eq!(second_icon.visual_parent(), Some(second_presenter.upcast()));
    window.close();
}

#[test]
fn measurement_keeps_narrow_content_square_without_clamping_wider_numeric_content() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.set_template(Some(FuncControlTemplate::for_type::<InfoBadge>(|_, _| {
        let root = Border::new();
        root.set_width(3.0);
        root.set_height(12.0);
        root.upcast()
    })));
    layout(&badge);
    assert_eq!(badge.desired_size(), Size::new(12.0, 12.0));
    badge.set_template(Some(FuncControlTemplate::for_type::<InfoBadge>(|_, _| {
        let root = Border::new();
        root.set_width(30.0);
        root.set_height(8.0);
        root.upcast()
    })));
    layout(&badge);
    assert_eq!(badge.desired_size(), Size::new(30.0, 8.0));
}

#[test]
fn automatic_radius_tracks_height_while_local_radius_overrides_and_clear_restores_it() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.set_height(12.0);
    layout(&badge);
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(6.0)
    );
    badge.set_corner_radius(CornerRadius::new(1.0, 2.0, 3.0, 4.0));
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::new(1.0, 2.0, 3.0, 4.0)
    );
    badge.set_height(16.0);
    layout(&badge);
    assert_eq!(
        part::<Border>(&badge, "RootGrid").corner_radius(),
        CornerRadius::new(1.0, 2.0, 3.0, 4.0)
    );
    badge.clear_value(TemplatedControl::corner_radius_property());
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(8.0)
    );
    let _style = badge.set_value_with_priority(
        TemplatedControl::corner_radius_property(),
        CornerRadius::uniform(3.0),
        BindingPriority::Style,
    );
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(8.0)
    );
}

#[test]
fn local_corner_radius_beneath_animation_preserves_native_clear_value_semantics() {
    let _app = start();
    let badge = InfoBadge::new();
    let property = TemplatedControl::corner_radius_property();
    badge.set_height(12.0);
    badge.set_corner_radius(CornerRadius::uniform(3.0));
    layout(&badge);
    let animation = badge
        .set_value_with_priority(
            property,
            CornerRadius::uniform(5.0),
            BindingPriority::Animation,
        )
        .unwrap();
    badge.set_height(16.0);
    layout(&badge);
    assert!(badge.has_local_value(property));
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(5.0)
    );
    badge.clear_value(property);
    assert!(badge.has_local_value(property));
    assert!(badge.is_animating(property));
    badge.set_height(14.0);
    layout(&badge);
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(5.0)
    );
    animation.dispose();
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(3.0)
    );
    badge.clear_value(property);
    assert!(!badge.has_local_value(property));
    assert_eq!(
        badge.template_settings().info_badge_corner_radius(),
        CornerRadius::uniform(7.0)
    );
}

#[test]
fn settings_properties_update_actual_template_parts_and_remain_independent() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.set_icon_source(Some(FontIconSource::new().upcast()));
    badge.apply_template();
    let custom = SymbolIcon::with_symbol(Symbol::Accept);
    badge
        .template_settings()
        .set_icon_element(Some(custom.clone().upcast()));
    let content = part::<ContentPresenter>(&badge, "IconContentPresenter");
    assert_eq!(
        Control::from_boxed(&content.content().unwrap()),
        Some(custom.upcast())
    );
    badge
        .template_settings()
        .set_info_badge_corner_radius(CornerRadius::uniform(5.0));
    assert_eq!(
        part::<Border>(&badge, "RootGrid").corner_radius(),
        CornerRadius::uniform(5.0)
    );
    let independent = InfoBadgeTemplateSettings::new();
    assert!(independent.icon_element().is_none());
    assert_eq!(
        independent.info_badge_corner_radius(),
        CornerRadius::default()
    );
}

#[test]
fn resource_changes_and_explicit_brushes_update_the_rendered_parts() {
    let _app = start();
    let badge = InfoBadge::new();
    let scope = ThemeVariantScope::new();
    scope.set_requested_theme_variant(Some(ThemeVariant::light()));
    scope.set_child(Some(badge.clone().upcast()));
    badge.set_numeric_value(1);
    badge.resources().add(
        "InfoBadgeBackground",
        Some(Rc::new(Brushes::red() as Rc<dyn IBrush>)),
    );
    badge.resources().add(
        "InfoBadgeForeground",
        Some(Rc::new(Brushes::blue() as Rc<dyn IBrush>)),
    );
    badge
        .resources()
        .add("InfoBadgeValueFontSize", Some(Rc::new(13.0_f64)));
    badge.apply_template();
    let root = part::<Border>(&badge, "RootGrid");
    let text = part::<TextBlock>(&badge, "ValueTextBlock");
    assert_eq!(root.background(), Some(Brushes::red() as Rc<dyn IBrush>));
    assert_eq!(text.foreground(), Some(Brushes::blue() as Rc<dyn IBrush>));
    assert_eq!(text.font_size(), 13.0);
    badge.resources().set(
        "InfoBadgeBackground",
        Some(Rc::new(Brushes::green() as Rc<dyn IBrush>)),
    );
    assert_eq!(root.background(), Some(Brushes::green() as Rc<dyn IBrush>));
    badge.set_background(Some(Brushes::black() as Rc<dyn IBrush>));
    badge.set_foreground(Some(Brushes::white() as Rc<dyn IBrush>));
    scope.set_requested_theme_variant(Some(ThemeVariant::dark()));
    assert_eq!(badge.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(root.background(), Some(Brushes::black() as Rc<dyn IBrush>));
    assert_eq!(text.foreground(), Some(Brushes::white() as Rc<dyn IBrush>));
    let background_animation = badge
        .set_value_with_priority(
            TemplatedControl::background_property(),
            Some(Brushes::red() as Rc<dyn IBrush>),
            BindingPriority::Animation,
        )
        .unwrap();
    let foreground_animation = badge
        .set_value_with_priority(
            TemplatedControl::foreground_property(),
            Some(Brushes::blue() as Rc<dyn IBrush>),
            BindingPriority::Animation,
        )
        .unwrap();
    assert_eq!(root.background(), Some(Brushes::red() as Rc<dyn IBrush>));
    assert_eq!(text.foreground(), Some(Brushes::blue() as Rc<dyn IBrush>));
    badge.set_foreground(Some(Brushes::green() as Rc<dyn IBrush>));
    assert_eq!(text.foreground(), Some(Brushes::blue() as Rc<dyn IBrush>));
    background_animation.dispose();
    foreground_animation.dispose();
    assert_eq!(root.background(), Some(Brushes::black() as Rc<dyn IBrush>));
    assert_eq!(text.foreground(), Some(Brushes::green() as Rc<dyn IBrush>));
    badge.set_padding(Thickness::uniform(2.0));
    assert_eq!(root.padding(), Thickness::uniform(2.0));
}

#[test]
fn replacing_the_template_releases_old_content_and_only_updates_current_parts() {
    let _app = start();
    let badge = InfoBadge::new();
    let source = FontIconSource::new();
    badge.set_icon_source(Some(source.clone().upcast()));
    layout(&badge);
    let original_template = badge.template();
    let old_content = part::<ContentPresenter>(&badge, "IconContentPresenter");
    let old_text = part::<TextBlock>(&badge, "ValueTextBlock");
    badge.set_template(Some(FuncControlTemplate::for_type::<InfoBadge>(|_, _| {
        Border::new().upcast()
    })));
    badge.apply_template();
    assert!(old_content.content().is_none());
    badge.set_template(original_template);
    layout(&badge);
    badge.set_numeric_value(42);
    assert_eq!(old_text.text().as_deref(), Some("-1"));
    assert_eq!(
        part::<TextBlock>(&badge, "ValueTextBlock")
            .text()
            .as_deref(),
        Some("42")
    );
}

#[test]
fn reentrant_part_notifications_finish_with_the_latest_display_kind() {
    let _app = start();
    let badge = InfoBadge::new();
    badge.apply_template();
    let text = part::<TextBlock>(&badge, "ValueTextBlock");
    let weak = badge.downgrade();
    text.property_changed(move |change| {
        if change.property() == TextBlock::text_property().as_property()
            && change.get_new_value::<Option<String>>().as_deref() == Some("10")
        {
            let badge = weak.upgrade().unwrap();
            badge.set_numeric_value(-1);
            badge.set_icon_source(Some(SymbolIconSource::new().upcast()));
        }
    });
    badge.set_numeric_value(10);
    assert_eq!(badge.value(), -1);
    assert!(!text.is_visible());
    assert!(part::<Viewbox>(&badge, "IconPresenter").is_visible());
    assert_eq!(text.text().as_deref(), Some("-1"));
}

#[test]
fn settings_and_resource_subscriptions_do_not_keep_a_detached_badge_alive() {
    let _app = start();
    let badge = InfoBadge::new();
    let settings = badge.template_settings();
    let resources = badge.resources();
    let weak = badge.downgrade();
    drop(badge);
    assert!(weak.upgrade().is_none());
    settings.set_info_badge_corner_radius(CornerRadius::uniform(2.0));
    resources.add(
        "InfoBadgeBackground",
        Some(Rc::new(Brushes::green() as Rc<dyn IBrush>)),
    );
    assert!(weak.upgrade().is_none());
}
