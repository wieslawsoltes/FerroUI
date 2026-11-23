use crate::document::Item;
use crate::{SvgDocument, SvgError, SvgLimits};
use ferroui_base::media::PathCommand;
use ferroui_base::{Point, Rect};

fn svg(body: &str) -> Vec<u8> {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100">{body}</svg>"#)
        .into_bytes()
}

fn shapes(items: &[Item]) -> usize {
    items
        .iter()
        .map(|item| match item {
            Item::Shape(_) => 1,
            Item::Group(group) => shapes(&group.children),
        })
        .sum()
}

fn first_shape(items: &[Item]) -> Option<&crate::document::Shape> {
    items.iter().find_map(|item| match item {
        Item::Shape(shape) => Some(shape),
        Item::Group(group) => first_shape(&group.children),
    })
}

fn close(left: Rect, right: Rect) -> bool {
    [(left.x, right.x), (left.y, right.y), (left.width, right.width), (left.height, right.height)]
        .iter()
        .all(|(a, b)| (a - b).abs() < 1e-6)
}

#[test]
fn the_size_comes_from_width_and_height_then_the_view_box() {
    assert_eq!(SvgDocument::parse(&svg("")).unwrap().size().width, 100.0);
    let sized = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2in" height="48" viewBox="0 0 10 10"/>"#;
    let document = SvgDocument::parse(sized).unwrap();
    assert_eq!((document.size().width, document.size().height), (192.0, 48.0));
    // Centred and scaled to fit, as the default preserveAspectRatio asks.
    let m = document.root.transform;
    assert_eq!((m.m11, m.m22, m.m31, m.m32), (4.8, 4.8, 72.0, 0.0));
}

#[test]
fn path_data_accepts_the_compact_forms() {
    let document = SvgDocument::parse(&svg(r#"<path d="M10-5.5.5.5l1,1h2v2a1 1 0 00.5.5zm1 1"/>"#)).unwrap();
    let shape = first_shape(&document.root.children).unwrap();
    assert!(matches!(shape.commands[1], PathCommand::BeginFigure(point, true) if point == Point::new(10.0, -5.5)));
    // The numbers after the move are a line, the packed arc flags are read one by one.
    assert!(matches!(shape.commands[2], PathCommand::LineTo(point, true) if point == Point::new(0.5, 0.5)));
    assert!(shape.commands.iter().any(|command| matches!(command, PathCommand::CubicBezierTo(..))));
    assert!(shape.commands.iter().any(|command| matches!(command, PathCommand::EndFigure(true))));
}

#[test]
fn a_path_with_an_error_is_drawn_up_to_the_error() {
    let document = SvgDocument::parse(&svg(r#"<path d="M 1 1 L 5 5 L oops 7"/>"#)).unwrap();
    let shape = first_shape(&document.root.children).unwrap();
    assert_eq!(shape.commands.iter().filter(|command| matches!(command, PathCommand::LineTo(..))).count(), 1);
}

#[test]
fn the_bounds_of_a_curve_are_its_extent_and_not_its_control_points() {
    let document = SvgDocument::parse(&svg(r#"<path d="M 0 0 C 0 40 40 40 40 0"/>"#)).unwrap();
    let shape = first_shape(&document.root.children).unwrap();
    assert!(close(shape.bounds, Rect::new(0.0, 0.0, 40.0, 30.0)), "{:?}", shape.bounds);
}

#[test]
fn an_arc_of_a_circle_reaches_the_circle() {
    let document = SvgDocument::parse(&svg(r#"<circle cx="50" cy="40" r="10"/>"#)).unwrap();
    let shape = first_shape(&document.root.children).unwrap();
    assert!(close(shape.bounds, Rect::new(40.0, 30.0, 20.0, 20.0)), "{:?}", shape.bounds);
}

#[test]
fn transforms_compose_from_the_last_to_the_first() {
    let list = crate::transform::parse_transform("translate(10,20) scale(2)").unwrap();
    assert_eq!(list.transform(Point::new(1.0, 1.0)), Point::new(12.0, 22.0));
    let turned = crate::transform::parse_transform("rotate(90 10 10)").unwrap().transform(Point::new(20.0, 10.0));
    assert!((turned.x - 10.0).abs() < 1e-9 && (turned.y - 20.0).abs() < 1e-9);
    assert!(crate::transform::parse_transform("scale(1").is_none());
    assert!(crate::transform::parse_transform("unknown(1)").is_none());
}

#[test]
fn css_hex_colours_have_their_alpha_last() {
    let color = crate::color::parse_color("#11223344").unwrap();
    assert_eq!((color.a, color.r, color.g, color.b), (0x44, 0x11, 0x22, 0x33));
    assert_eq!(crate::color::parse_color("#f00").unwrap().r, 255);
    assert!(crate::color::parse_color("red").is_some());
    assert!(crate::color::parse_color("#xyz").is_none());
    assert!(crate::color::parse_color("#é0").is_none());
}

#[test]
fn hidden_and_unknown_elements_draw_nothing() {
    let body = r#"<rect width="9" height="9" display="none"/><rect width="9" height="9" visibility="hidden"/>
        <script>alert(1)</script><foreignObject/><image href="http://example.invalid/a.png"/><text>x</text>"#;
    assert_eq!(shapes(&SvgDocument::parse(&svg(body)).unwrap().root.children), 0);
}

#[test]
fn use_expands_local_references_and_refuses_cycles() {
    let body = r##"<defs><rect id="r" width="5" height="5"/></defs><use href="#r" x="1"/><use xlink:href="#r" y="1"/>
        <use href="http://example.invalid/x.svg#r"/><g id="a"><use href="#a"/></g>"##;
    assert_eq!(shapes(&SvgDocument::parse(&svg(body)).unwrap().root.children), 2);
    // A use inside the element it names draws nothing of it a second time.
    let inside = r##"<g id="a"><rect width="1" height="1"/><use href="#a"/></g>"##;
    assert_eq!(shapes(&SvgDocument::parse(&svg(inside)).unwrap().root.children), 1);
    let cycle = r##"<g id="a"><use href="#b"/></g><g id="b"><use href="#a"/><rect width="1" height="1"/></g>"##;
    assert!(SvgDocument::parse(&svg(cycle)).is_ok());
}

#[test]
fn gradients_inherit_through_href_and_survive_a_cycle() {
    let body = r##"<defs><linearGradient id="base"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient>
        <linearGradient id="child" href="#base" x2="0" y2="1"/>
        <linearGradient id="x" href="#y"/><linearGradient id="y" href="#x"/></defs>
        <rect width="9" height="9" fill="url(#child)"/><rect width="9" height="9" fill="url(#x) green"/>"##;
    let document = SvgDocument::parse(&svg(body)).unwrap();
    let shape = first_shape(&document.root.children).unwrap();
    assert!(matches!(shape.fill, Some(crate::document::Paint::Gradient(..))));
    let Item::Shape(second) = &document.root.children[1] else { panic!("a shape") };
    assert!(matches!(second.fill, Some(crate::document::Paint::Solid(..))));
}

#[test]
fn malformed_input_is_an_error_and_never_a_panic() {
    for input in [
        &b""[..],
        b"<",
        b"not xml",
        b"<html/>",
        b"\xff\xfe<svg/>",
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="10"/>"#,
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="1e9" height="10"/>"#,
    ] {
        assert!(SvgDocument::parse(input).is_err(), "{:?}", String::from_utf8_lossy(input));
    }
    // Values that are wrong leave the element to its defaults.
    for body in [
        r#"<rect width="abc" height="-1"/>"#,
        r#"<path d="" fill="url("/>"#,
        r#"<path d="M 1e400 0 L 1 1"/>"#,
        r#"<circle r="NaN"/>"#,
        r#"<g transform="matrix(1 2)"><polygon points="1"/></g>"#,
        r##"<rect width="1" height="1" clip-path="url(#missing)" fill="url(#missing)"/>"##,
        r#"<path d="M 0 0 A 0 0 0 0 0 5 5 A 1e308 1e308 0 1 1 0 0"/>"#,
    ] {
        assert!(SvgDocument::parse(&svg(body)).is_ok(), "{body}");
    }
}

#[test]
fn a_document_type_definition_is_refused() {
    let laughs = br#"<!DOCTYPE svg [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;">]><svg xmlns="http://www.w3.org/2000/svg"><text>&b;</text></svg>"#;
    assert!(matches!(SvgDocument::parse(laughs), Err(SvgError::InvalidXml(_))));
}

#[test]
fn the_limits_stop_a_document_made_to_exhaust_the_decoder() {
    let limits = SvgLimits { max_bytes: 16, ..SvgLimits::default() };
    assert!(matches!(SvgDocument::parse_with_limits(&svg(""), &limits), Err(SvgError::TooLarge { .. })));

    // Each level uses the one below ten times: 10^7 elements from a few hundred bytes.
    let mut body = String::from(r#"<defs><rect id="l0" width="1" height="1"/>"#);
    for level in 1..=7 {
        let below = level - 1;
        body.push_str(&format!(r#"<g id="l{level}">"#));
        for _ in 0..10 {
            body.push_str(&format!(r##"<use href="#l{below}"/>"##));
        }
        body.push_str("</g>");
    }
    body.push_str(r##"</defs><use href="#l7"/>"##);
    assert_eq!(SvgDocument::parse(&svg(&body)).err(), Some(SvgError::LimitExceeded("elements")));

    let deep = "<g>".repeat(100) + &"</g>".repeat(100);
    assert_eq!(SvgDocument::parse(&svg(&deep)).err(), Some(SvgError::LimitExceeded("depth")));

    let limits = SvgLimits { max_path_segments: 3, ..SvgLimits::default() };
    let long = svg(r#"<path d="M0 0L1 1L2 2L3 3L4 4"/>"#);
    assert_eq!(SvgDocument::parse_with_limits(&long, &limits).err(), Some(SvgError::LimitExceeded("path segments")));
}

#[test]
fn a_clip_path_without_content_hides_the_element_and_a_missing_one_does_not() {
    let hidden = r##"<defs><clipPath id="c"/></defs><rect width="9" height="9" clip-path="url(#c)"/>"##;
    assert_eq!(shapes(&SvgDocument::parse(&svg(hidden)).unwrap().root.children), 0);
    let shown = r##"<rect width="9" height="9" clip-path="url(#nothing)"/>"##;
    assert_eq!(shapes(&SvgDocument::parse(&svg(shown)).unwrap().root.children), 1);
    let clipped = r##"<defs><clipPath id="c"><circle r="4"/></clipPath></defs><rect width="9" height="9" clip-path="url(#c)"/>"##;
    let document = SvgDocument::parse(&svg(clipped)).unwrap();
    let Item::Group(group) = &document.root.children[0] else { panic!("a group") };
    assert!(group.clip.is_some());
    assert_eq!(crate::document::bounds_of(&group.children).map(|bounds| bounds.width), Some(9.0));
}

#[test]
fn the_sample_of_the_gallery_decodes_completely() {
    let bytes = include_bytes!("../../samples/WinUIGallery/Assets/SampleMedia/MirrorPCConsent.svg");
    let document = SvgDocument::parse(bytes).unwrap();
    assert_eq!((document.size().width, document.size().height), (328.0, 164.0));
    // The 106 paths and the rectangle of the clip, which is not drawn.
    assert_eq!(shapes(&document.root.children), 106);
}
