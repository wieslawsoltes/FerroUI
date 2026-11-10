//! The plain text layouts a font manager keeps (a FerroUI addition,
//! docs/porting/desktop-performance.md, section 9.29).

use std::rc::Rc;

use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::text_formatting::testing::{
    line_text, paragraph_properties, paragraph_properties_with, run_properties, SingleBufferTextSource,
    TextTestScope,
};
use crate::media::text_formatting::text_layout_cache::{IDLE_CAPACITY, IDLE_TEXT_CAPACITY, MAX_TEXT_LENGTH};
use crate::media::text_formatting::{
    GenericTextRunProperties, ITextSource, TextLayout, TextParagraphProperties, TextRunProperties,
};
use crate::media::{
    BaselineAlignment, Brushes, Colors, FlowDirection, FontFeatureCollection, FontManager, IBrush, TextAlignment, TextDecorationCollection, TextTrimming, TextWrapping, Typeface,
};

fn layout_with(
    text: &str,
    properties: &Rc<dyn TextParagraphProperties>,
    text_trimming: &Rc<dyn TextTrimming>,
    max_width: f64,
    max_height: f64,
    max_lines: i32,
) -> Rc<TextLayout> {
    let run_properties = properties.default_text_run_properties().clone();

    TextLayout::shared_plain(
        text,
        || -> Rc<dyn ITextSource> { Rc::new(SingleBufferTextSource::new(text, run_properties)) },
        properties,
        text_trimming,
        max_width,
        max_height,
        max_lines,
        None,
    )
}

fn layout(text: &str, properties: &Rc<dyn TextParagraphProperties>, max_width: f64) -> Rc<TextLayout> {
    layout_with(text, properties, &<dyn TextTrimming>::none(), max_width, f64::INFINITY, 0)
}

fn plain(font_rendering_em_size: f64) -> Rc<dyn TextParagraphProperties> {
    paragraph_properties(&run_properties(font_rendering_em_size), TextWrapping::NoWrap)
}

fn with_brush(brush: Option<Rc<dyn IBrush>>) -> Rc<dyn TextParagraphProperties> {
    let run_properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        12.0,
        None,
        brush,
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ));

    paragraph_properties(&run_properties, TextWrapping::NoWrap)
}

fn kept() -> usize {
    FontManager::current().text_layouts().len()
}

#[test]
fn the_same_text_with_the_same_properties_is_one_layout() {
    let _scope = TextTestScope::new();

    let first = layout("12.34", &plain(12.0), f64::INFINITY);
    // Other property objects with the same values.
    let second = layout("12.34", &plain(12.0), f64::INFINITY);

    assert!(Rc::ptr_eq(&first, &second));
    assert!(first.is_shared());
    assert_eq!(1, kept());
    assert_eq!("12.34", line_text(&*first.text_lines()[0]));
}

#[test]
fn what_a_layout_is_made_of_tells_layouts_apart() {
    let _scope = TextTestScope::new();

    let properties = plain(12.0);
    let none = <dyn TextTrimming>::none();
    let reference = layout("12.34", &properties, 200.0);

    let others = [
        layout("12.35", &properties, 200.0),
        layout("12.34", &plain(13.0), 200.0),
        layout("12.34", &properties, 201.0),
        layout("12.34", &paragraph_properties(&run_properties(12.0), TextWrapping::Wrap), 200.0),
        layout(
            "12.34",
            &paragraph_properties_with(
                &run_properties(12.0),
                TextWrapping::NoWrap,
                TextAlignment::Right,
                FlowDirection::LeftToRight,
            ),
            200.0,
        ),
        layout(
            "12.34",
            &paragraph_properties_with(
                &run_properties(12.0),
                TextWrapping::NoWrap,
                TextAlignment::Left,
                FlowDirection::RightToLeft,
            ),
            200.0,
        ),
        layout_with("12.34", &properties, &<dyn TextTrimming>::character_ellipsis(), 200.0, f64::INFINITY, 0),
        layout_with("12.34", &properties, &none, 200.0, 100.0, 0),
        layout_with("12.34", &properties, &none, 200.0, f64::INFINITY, 1),
    ];

    for (index, other) in others.iter().enumerate() {
        assert!(!Rc::ptr_eq(&reference, other), "layout {index} is the reference");
        assert!(other.is_shared());
    }
    assert_eq!(1 + others.len(), kept());

    // And each is found again.
    assert!(Rc::ptr_eq(&reference, &layout("12.34", &properties, 200.0)));
    assert!(Rc::ptr_eq(&others[2], &layout("12.34", &plain(12.0), 201.0)));
}

#[test]
fn a_layout_is_shared_for_the_same_brush_object_only() {
    let _scope = TextTestScope::new();

    let brush: Rc<dyn IBrush> = Brushes::black();
    let first = layout("abc", &with_brush(Some(brush.clone())), f64::INFINITY);
    let second = layout("abc", &with_brush(Some(brush)), f64::INFINITY);
    assert!(Rc::ptr_eq(&first, &second));

    // Brushes are told apart as objects: one of the same colour that is another object can
    // get another colour later, and the layout draws with the brush it was made with.
    let red: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    let other_red: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    let with_red = layout("abc", &with_brush(Some(red)), f64::INFINITY);
    let with_other_red = layout("abc", &with_brush(Some(other_red)), f64::INFINITY);
    assert!(!Rc::ptr_eq(&with_red, &with_other_red));
    assert!(!Rc::ptr_eq(&first, &with_red));

    let without = layout("abc", &with_brush(None), f64::INFINITY);
    assert!(!Rc::ptr_eq(&first, &without));
}

#[test]
fn a_layout_made_of_what_can_change_is_not_shared() {
    let _scope = TextTestScope::new();

    let decorated: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        12.0,
        Some(TextDecorationCollection::new()),
        None,
        None,
        BaselineAlignment::Baseline,
        None,
        None,
    ));
    let featured: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        12.0,
        None,
        None,
        None,
        BaselineAlignment::Baseline,
        None,
        Some(FontFeatureCollection::parse("kern=0").unwrap()),
    ));

    for run_properties in [decorated, featured] {
        let properties = paragraph_properties(&run_properties, TextWrapping::NoWrap);
        let first = layout("abc", &properties, f64::INFINITY);
        let second = layout("abc", &properties, f64::INFINITY);

        assert!(!first.is_shared());
        assert!(!Rc::ptr_eq(&first, &second));
    }

    let long = "a".repeat(MAX_TEXT_LENGTH + 1);
    let properties = plain(12.0);
    assert!(!layout(&long, &properties, f64::INFINITY).is_shared());
    assert!(layout(&long[..MAX_TEXT_LENGTH], &properties, f64::INFINITY).is_shared());

    assert_eq!(1, kept());
}

#[test]
fn disposing_a_shared_layout_leaves_it_to_its_other_holders() {
    let _scope = TextTestScope::new();

    let properties = plain(12.0);
    let first = layout("abc def", &properties, f64::INFINITY);
    let width = first.width();

    // What a text block does with the layout it lets go.
    first.dispose();
    drop(first);

    let second = layout("abc def", &properties, f64::INFINITY);
    assert_eq!(1, kept());
    assert_eq!("abc def", line_text(&*second.text_lines()[0]));
    assert_eq!(width, second.width());

    // A hit test reads the shaped runs, which a released layout no longer has.
    let hit = second.hit_test_text_position(4);
    assert_eq!(crate::media::text_formatting::testing::advance(12.0) * 4.0, hit.x);
}

#[test]
fn the_layouts_nothing_holds_are_let_go_from_the_oldest() {
    let _scope = TextTestScope::new();

    let properties = plain(12.0);
    let held = layout("held", &properties, f64::INFINITY);

    let count = IDLE_CAPACITY + IDLE_CAPACITY / 2;
    for index in 0..count {
        drop(layout(&index.to_string(), &properties, f64::INFINITY));
    }

    let cache_len = kept();
    assert!(cache_len < count, "{cache_len} of {count} layouts are kept");
    assert!(
        FontManager::current().text_layouts().idle_len() <= IDLE_CAPACITY + IDLE_CAPACITY / 4,
        "{} idle layouts are kept",
        FontManager::current().text_layouts().idle_len()
    );

    // The one that is held and the ones used last are still there.
    assert!(Rc::ptr_eq(&held, &layout("held", &properties, f64::INFINITY)));
    let before = kept();
    drop(layout(&(count - 1).to_string(), &properties, f64::INFINITY));
    assert_eq!(before, kept());
    // The first one is not.
    drop(layout("0", &properties, f64::INFINITY));
    assert_eq!(before + 1, kept());
}

#[test]
fn long_texts_nothing_holds_are_let_go_by_their_length() {
    let _scope = TextTestScope::new();

    let properties = plain(12.0);
    let held = layout("held", &properties, f64::INFINITY);

    // Far fewer layouts than the cache keeps by number, and four times the text it keeps.
    let count = 4 * IDLE_TEXT_CAPACITY / MAX_TEXT_LENGTH;
    assert!(count < IDLE_CAPACITY);
    for index in 0..count {
        let text = format!("{index:0width$}", width = MAX_TEXT_LENGTH);
        drop(layout(&text, &properties, f64::INFINITY));
    }

    let idle = FontManager::current().text_layouts().idle_len();
    let limit = (IDLE_TEXT_CAPACITY + IDLE_TEXT_CAPACITY / 4) / MAX_TEXT_LENGTH;
    assert!(idle <= limit, "{idle} idle layouts of {MAX_TEXT_LENGTH} bytes are kept, at most {limit} fit");
    assert!(idle > 0);

    // The one that is held and the one used last are still there.
    assert!(Rc::ptr_eq(&held, &layout("held", &properties, f64::INFINITY)));
    let before = kept();
    drop(layout(&format!("{:0width$}", count - 1, width = MAX_TEXT_LENGTH), &properties, f64::INFINITY));
    assert_eq!(before, kept());
}

#[test]
fn a_change_of_the_fonts_lets_every_layout_go() {
    let _scope = TextTestScope::new();

    let properties = plain(12.0);
    let before = layout("abc", &properties, f64::INFINITY);
    drop(layout("def", &properties, f64::INFINITY));
    assert_eq!(2, kept());

    FontManager::current().invalidate_glyph_typefaces();

    let after = layout("abc", &properties, f64::INFINITY);
    assert!(!Rc::ptr_eq(&before, &after));
    assert_eq!(1, kept());

    // The one that is still held is whole.
    assert_eq!("abc", line_text(&*before.text_lines()[0]));
    assert_eq!(before.width(), after.width());
}
