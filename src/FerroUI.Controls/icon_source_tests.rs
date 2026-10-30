//! Tests of the icon sources (WinUI's `IconSource` family; docs/porting/winui-controls.md, the
//! icons): each element is new, the later changes of the source reach every element that is
//! alive, and neither keeps the other alive.

use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    ContentControl, Control, FontIcon, FontIconSource, IconSource, PathIcon, PathIconSource, Symbol, SymbolIcon,
    SymbolIconSource,
};
use ferroui_base::media::{Brushes, FontFamily, FontStyle, FontWeight, Geometry, IBrush};
use ferroui_base::{instantiate, Ref};
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn font_icon(source: &FontIconSource) -> Ref<FontIcon> {
    source.create_icon_element().expect("an element").cast::<FontIcon>().unwrap()
}

#[test]
fn the_abstract_source_creates_no_element() {
    let _app = start();
    // WinUI's IconSource::CreateIconElementCoreImpl is E_NOTIMPL: the base has no element.
    let source: Ref<IconSource> = instantiate(IconSource::construct());
    assert!(source.create_icon_element().is_none());
    source.set_foreground(Some(Brushes::red() as Rc<dyn IBrush>));
}

#[test]
fn font_source_creates_independent_elements_and_updates_every_live_property() {
    let _app = start();
    let source = FontIconSource::new();
    source.set_glyph("AB");
    let first = font_icon(&source);
    let second = font_icon(&source);
    assert_ne!(first, second);
    assert_eq!(first.glyph(), "AB");
    assert_eq!(first.font_size(), 20.0);
    // No font family on the source: the icon keeps its own.
    assert_eq!(None, source.font_family());
    assert_eq!(FontIcon::new().font_family(), first.font_family());
    source.set_glyph("C");
    source.set_font_family(FontFamily::new("Custom Icon Family"));
    source.set_font_size(32.0);
    source.set_font_style(FontStyle::Italic);
    source.set_font_weight(FontWeight::Bold);
    source.set_mirrored_when_right_to_left(true);
    source.set_foreground(Some(Brushes::red() as Rc<dyn IBrush>));
    for icon in [&first, &second] {
        assert_eq!(icon.glyph(), "C");
        assert_eq!(icon.font_family().name(), "Custom Icon Family");
        assert_eq!(icon.font_size(), 32.0);
        assert_eq!(icon.font_style(), FontStyle::Italic);
        assert_eq!(icon.font_weight(), FontWeight::Bold);
        assert!(icon.mirrored_when_right_to_left());
        assert_eq!(icon.foreground(), Some(Brushes::red() as Rc<dyn IBrush>));
    }
}

#[test]
fn unset_foreground_inherits_until_the_source_supplies_an_explicit_value() {
    let _app = start();
    let source = FontIconSource::new();
    let icon = source.create_icon_element().unwrap();
    let parent = ContentControl::new();
    parent.set_foreground(Some(Brushes::blue() as Rc<dyn IBrush>));
    parent.set_content(Some(Control::boxed(icon.clone())));
    assert_eq!(icon.foreground(), Some(Brushes::blue() as Rc<dyn IBrush>));
    source.set_foreground(Some(Brushes::red() as Rc<dyn IBrush>));
    assert_eq!(icon.foreground(), Some(Brushes::red() as Rc<dyn IBrush>));
    parent.set_foreground(Some(Brushes::green() as Rc<dyn IBrush>));
    assert_eq!(icon.foreground(), Some(Brushes::red() as Rc<dyn IBrush>));
    source.set_foreground(None);
    assert_eq!(icon.foreground(), None);
    let later = source.create_icon_element().unwrap();
    parent.set_content(Some(Control::boxed(later.clone())));
    assert_eq!(later.foreground(), Some(Brushes::green() as Rc<dyn IBrush>));
}

#[test]
fn source_changes_do_not_overwrite_unrelated_element_local_values() {
    let _app = start();
    let source = FontIconSource::new();
    let icon = font_icon(&source);
    icon.set_font_size(48.0);
    icon.set_foreground(Some(Brushes::blue() as Rc<dyn IBrush>));
    source.set_glyph("new glyph");
    assert_eq!(icon.font_size(), 48.0);
    assert_eq!(icon.foreground(), Some(Brushes::blue() as Rc<dyn IBrush>));
    source.set_font_size(25.0);
    assert_eq!(icon.font_size(), 25.0);
    assert_eq!(icon.foreground(), Some(Brushes::blue() as Rc<dyn IBrush>));
}

#[test]
fn reentrant_element_callbacks_preserve_the_latest_source_value_for_every_icon() {
    let _app = start();
    let source = FontIconSource::new();
    let first = font_icon(&source);
    let second = font_icon(&source);
    let weak = source.downgrade();
    first.property_changed(move |change| {
        if change.property() == FontIcon::glyph_property().as_property() {
            let source = weak.upgrade().unwrap();
            match change.get_new_value::<String>().as_str() {
                "outer" => source.set_glyph("redirected"),
                "unrelated" => source.set_font_size(64.0),
                _ => {}
            }
        }
    });
    source.set_glyph("outer");
    assert_eq!(source.glyph(), "redirected");
    assert_eq!(first.glyph(), "redirected");
    assert_eq!(second.glyph(), "redirected");
    source.set_glyph("unrelated");
    assert_eq!(first.glyph(), "unrelated");
    assert_eq!(second.glyph(), "unrelated");
    assert_eq!(first.font_size(), 64.0);
    assert_eq!(second.font_size(), 64.0);
}

#[test]
fn source_element_lifetimes_are_independent_and_no_element_subscription_is_retained() {
    let _app = start();
    let source = FontIconSource::new();
    let first = source.create_icon_element().unwrap();
    let first_weak = first.downgrade();
    assert_eq!(first.property_changed_subscriber_count(), 0);
    drop(first);
    assert!(first_weak.upgrade().is_none());
    source.set_glyph("prune the expired weak element");
    let second = font_icon(&source);
    let source_weak = source.downgrade();
    drop(source);
    assert!(source_weak.upgrade().is_none());
    assert_eq!(second.glyph(), "prune the expired weak element");
}

#[test]
fn symbol_source_uses_exact_symbol_values_and_live_brushes() {
    let _app = start();
    let source = SymbolIconSource::new();
    let first = source.create_icon_element().unwrap().cast::<SymbolIcon>().unwrap();
    let second = source.create_icon_element().unwrap().cast::<SymbolIcon>().unwrap();
    assert_eq!(first.symbol(), Symbol::Emoji);
    for symbol in Symbol::ALL {
        source.set_symbol(*symbol);
        assert_eq!(first.symbol(), *symbol);
        assert_eq!(second.symbol(), *symbol);
    }
    source.set_foreground(Some(Brushes::green() as Rc<dyn IBrush>));
    assert_eq!(first.foreground(), Some(Brushes::green() as Rc<dyn IBrush>));
    assert_eq!(
        second.foreground(),
        Some(Brushes::green() as Rc<dyn IBrush>)
    );
}

#[test]
fn path_source_shares_native_geometry_and_updates_surviving_elements() {
    let _app = start();
    let source = PathIconSource::new();
    let triangle = Geometry::parse("M0,0 L10,0 L5,10 Z").unwrap();
    source.set_data(Some(triangle.clone()));
    let icon = source.create_icon_element().unwrap().cast::<PathIcon>().unwrap();
    assert_eq!(icon.data(), Some(triangle));
    let rectangle = Geometry::parse("M0,0 L20,0 L20,20 L0,20 Z").unwrap();
    source.set_data(Some(rectangle.clone()));
    assert_eq!(icon.data(), Some(rectangle));
    source.set_data(None);
    assert!(icon.data().is_none());
}

#[test]
fn a_font_family_set_and_cleared_on_the_source_is_set_and_cleared_on_the_icons() {
    let _app = start();
    let source = FontIconSource::new();
    let icon = font_icon(&source);
    let own = icon.font_family();
    source.set_font_family(FontFamily::new("Custom Icon Family"));
    assert_eq!("Custom Icon Family", icon.font_family().name());
    source.set_font_family(None);
    assert_eq!(own, icon.font_family());
    let created = {
        source.set_font_family(FontFamily::new("Custom Icon Family"));
        font_icon(&source)
    };
    assert_eq!("Custom Icon Family", created.font_family().name());
}
