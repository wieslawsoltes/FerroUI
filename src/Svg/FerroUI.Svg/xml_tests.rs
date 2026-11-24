use super::*;

fn svg(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100" height="80">{body}</svg>"#
    )
}

fn optimized(body: &str) -> OptimizedSvg {
    optimize(&svg(body), &XmlOptions::default()).unwrap()
}

#[test]
fn references_are_tokenized_and_semantic_ids_survive() {
    let result = optimized(
        r##"<defs><linearGradient id="a"><stop stop-color="#fff"/></linearGradient><linearGradient id="ab"><stop stop-color="#123"/></linearGradient></defs><g id="ferro-layer"><path fill="url( '#ab' )" stroke="url(#a)" d="M0 0L1 1"/><use xlink:href="#ferro-layer-shape"/><rect id="ferro-layer-shape" fill="#abc"/></g>"##,
    );
    let document = Document::parse(&result.svg).unwrap();
    let path = document
        .descendants()
        .find(|node| node.has_tag_name("path"))
        .unwrap();
    assert_eq!(path.attribute("fill"), Some("url(#svg-1)"));
    assert_eq!(path.attribute("stroke"), Some("url(#svg-0)"));
    assert!(result.svg.contains("id=\"ferro-layer\""));
    assert!(result.svg.contains("xlink:href=\"#ferro-layer-shape\""));
    assert!(result.svg.contains("fill=\"#abc\""));
    assert!(result.svg.contains("stop-color=\"#fff\""));
}

#[test]
fn skia_generation_ids_do_not_affect_output() {
    let first = optimized(
        r#"<clipPath id="cl_345"><rect width="10" height="10"/></clipPath><g clip-path="url(#cl_345)"><rect width="5" height="5"/></g>"#,
    );
    let second = optimized(
        r#"<clipPath id="cl_999"><rect width="10" height="10"/></clipPath><g clip-path="url(#cl_999)"><rect width="5" height="5"/></g>"#,
    );
    assert_eq!(first.svg, second.svg);
}

#[test]
fn preserve_ids_cannot_collide_with_generated_ids() {
    let result = optimize(
        &svg(r#"<g id="ferro-0"/><g id="old"/><g id="ferro-1"/>"#),
        &XmlOptions {
            id_prefix: "ferro".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(result.svg.contains("id=\"ferro-2\""));
    let document = Document::parse(&result.svg).unwrap();
    let ids = document
        .descendants()
        .filter_map(|n| n.attribute("id"))
        .collect::<HashSet<_>>();
    assert_eq!(ids.len(), 3);
}

#[test]
fn identical_resources_share_one_definition_without_losing_references() {
    let result = optimized(
        r##"<defs><linearGradient id="first" x1="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><linearGradient id="second" x1="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><image id="im1" width="1" height="1" xlink:href="data:image/png;base64,AAAA"/><image id="im2" width="1" height="1" xlink:href="data:image/png;base64,AAAA"/></defs><rect fill="url(#first)"/><rect fill="url(#second)"/><use xlink:href="#im1"/><use xlink:href="#im2"/>"##,
    );
    assert_eq!(result.stats.definitions_deduplicated, 2);
    assert_eq!(result.stats.image_elements, 1);
    let document = Document::parse(&result.svg).unwrap();
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.has_tag_name("linearGradient"))
            .count(),
        1
    );
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.has_tag_name("image"))
            .count(),
        1
    );
    let references = document
        .descendants()
        .filter(|n| n.has_tag_name("use"))
        .filter_map(|n| n.attribute((XLINK_NS, "href")))
        .collect::<Vec<_>>();
    assert_eq!(references[0], references[1]);
    assert_eq!(result.stats.input_bytes, svg(r##"<defs><linearGradient id="first" x1="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><linearGradient id="second" x1="0"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><image id="im1" width="1" height="1" xlink:href="data:image/png;base64,AAAA"/><image id="im2" width="1" height="1" xlink:href="data:image/png;base64,AAAA"/></defs><rect fill="url(#first)"/><rect fill="url(#second)"/><use xlink:href="#im1"/><use xlink:href="#im2"/>"##).len());
    assert_eq!(result.stats.output_bytes, result.svg.len());
}

#[test]
fn resource_inheritance_and_semantic_definition_identity_are_preserved() {
    let result = optimized(
        r#"<g color="red"><defs><linearGradient id="a"><stop stop-color="currentColor"/></linearGradient></defs></g><g color="blue"><defs><linearGradient id="b"><stop stop-color="currentColor"/></linearGradient></defs></g><defs><clipPath id="ferro-clip-a"><rect width="10"/></clipPath><clipPath id="ferro-clip-b"><rect width="10"/></clipPath></defs>"#,
    );
    assert_eq!(result.stats.definitions_deduplicated, 0);
    assert!(result.svg.contains("ferro-clip-a") && result.svg.contains("ferro-clip-b"));
}

#[test]
fn identically_named_designer_groups_do_not_prevent_safe_resource_sharing() {
    let result = optimized(
        r#"<g id="ferro-one" data-layer="one"><clipPath id="a"><rect width="10"/></clipPath><rect clip-path="url(#a)"/></g><g id="ferro-two" data-layer="two"><clipPath id="b"><rect width="10"/></clipPath><rect clip-path="url(#b)"/></g>"#,
    );
    assert_eq!(result.stats.definitions_deduplicated, 1);
    assert!(result.svg.contains("ferro-one") && result.svg.contains("ferro-two"));
}

#[test]
fn descendants_with_independent_ids_are_not_removed_by_interning() {
    let result = optimized(
        r##"<defs><clipPath id="a"><rect id="child-a" width="10"/></clipPath><clipPath id="b"><rect id="child-b" width="10"/></clipPath></defs><use xlink:href="#child-b"/>"##,
    );
    assert_eq!(result.stats.definitions_deduplicated, 0);
    assert!(result.svg.contains("id=\"svg-3\""));
}

#[test]
fn pretty_printing_preserves_unicode_and_every_text_position() {
    let input = svg(
        r#"<title>A &amp; B</title><g><text xml:space="preserve" x="1 2 3">  Zażółć <tspan>世界</tspan> &#9; &amp; &lt;🦀&gt;  </text></g>"#,
    );
    let compact = optimize(&input, &XmlOptions::default()).unwrap();
    let pretty = optimize(
        &input,
        &XmlOptions {
            pretty: true,
            ..Default::default()
        },
    )
    .unwrap();
    fn text(svg: &str) -> String {
        let document = Document::parse(svg).unwrap();
        document
            .descendants()
            .find(|n| n.has_tag_name("text"))
            .unwrap()
            .descendants()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect()
    }
    assert_eq!(text(&input), text(&compact.svg));
    assert_eq!(text(&input), text(&pretty.svg));
    assert!(pretty.svg.contains("\n  <g>\n"));
    assert!(pretty.svg.contains("Zażółć <tspan>世界</tspan> \t"));
}

#[test]
fn attribute_newlines_round_trip_without_xml_normalization_loss() {
    let result = optimized(r#"<g data-label="one&#10;two&#9;three&#13;four&quot;five"/>"#);
    let document = Document::parse(&result.svg).unwrap();
    assert_eq!(
        document
            .descendants()
            .find(|n| n.has_tag_name("g"))
            .unwrap()
            .attribute("data-label"),
        Some("one\ntwo\tthree\rfour\"five")
    );
}

#[test]
fn fragments_are_collision_free_and_keep_namespace_bindings() {
    let input = r##"<s:svg xmlns:s="http://www.w3.org/2000/svg" xmlns:l="http://www.w3.org/1999/xlink" xmlns:ink="http://www.inkscape.org/namespaces/inkscape" width="20"><s:defs><s:path id="p" d="M0 0L1 1"/></s:defs><s:g ink:label="Łącznik"><s:use l:href="#p"/></s:g></s:svg>"##;
    let first = fragment(input, "first", &InspectionStyle::Normal).unwrap();
    let second = fragment(input, "second", &InspectionStyle::Normal).unwrap();
    let joined = svg(&format!("{first}{second}"));
    let result = optimize(&joined, &XmlOptions::default()).unwrap();
    let document = Document::parse(&result.svg).unwrap();
    let uses = document
        .descendants()
        .filter(|n| n.has_tag_name((SVG_NS, "use")))
        .collect::<Vec<_>>();
    assert_eq!(uses.len(), 2);
    assert_ne!(
        uses[0].attribute((XLINK_NS, "href")),
        uses[1].attribute((XLINK_NS, "href"))
    );
    assert!(result.svg.contains("Łącznik"));
}

#[test]
fn fragment_preserves_root_presentation_in_a_group() {
    let result = fragment(r#"<svg xmlns="http://www.w3.org/2000/svg" fill="red" opacity="0.5" width="20" height="10"><rect width="10"/></svg>"#, "part", &InspectionStyle::Normal).unwrap();
    let wrapped = svg(&result);
    let document = Document::parse(&wrapped).unwrap();
    let group = document
        .descendants()
        .find(|n| n.has_tag_name("g"))
        .unwrap();
    assert_eq!(group.attribute("fill"), Some("red"));
    assert_eq!(group.attribute("opacity"), Some("0.5"));
    assert!(group.attribute("width").is_none());
}

#[test]
fn rounding_is_explicit_and_does_not_rewrite_paths_or_transforms() {
    let input = svg(
        r#"<path d="M.123456 -.333333L1e-7 0" transform="matrix(1 .000001 0 1 .123456 0)" stroke-width="1.23456"/><text x="1.23456 -.0000001 .5 1.5.5" font-size="12.3456px">x</text>"#,
    );
    let exact = optimize(&input, &XmlOptions::default()).unwrap();
    assert!(exact.svg.contains("stroke-width=\"1.23456\""));
    let rounded = optimize(
        &input,
        &XmlOptions {
            precision: Some(2),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(rounded.svg.contains("d=\"M.123456 -.333333L1e-7 0\""));
    assert!(rounded.svg.contains("matrix(1 .000001 0 1 .123456 0)"));
    assert!(rounded.svg.contains("stroke-width=\"1.23\""));
    assert!(rounded.svg.contains("font-size=\"12.35px\""));
    assert!(rounded.svg.contains("x=\"1.23 0 0.5 1.5 0.5\""));
}

#[test]
fn xray_does_not_corrupt_clips_masks_or_gradient_definitions() {
    let input = svg(
        r#"<defs><clipPath id="clip"><rect width="10" fill="black"/></clipPath><mask id="mask"><rect width="10" fill="white"/></mask><linearGradient id="paint"><stop stop-color="red"/></linearGradient></defs><rect clip-path="url(#clip)" mask="url(#mask)" fill="url(#paint)" width="10"/>"#,
    );
    let result = fragment(
        &input,
        "layer",
        &InspectionStyle::XRay {
            fill_opacity: 0.2,
            stroke: "#0af".into(),
            stroke_width: 0.75,
            stroke_opacity: 1.0,
        },
    )
    .unwrap();
    let wrapped = svg(&result);
    let document = Document::parse(&wrapped).unwrap();
    let rectangles = document
        .descendants()
        .filter(|n| n.has_tag_name("rect"))
        .collect::<Vec<_>>();
    assert_eq!(rectangles[0].attribute("fill"), Some("black"));
    assert!(rectangles[0].attribute("stroke").is_none());
    assert_eq!(rectangles[1].attribute("fill"), Some("white"));
    assert!(rectangles[1].attribute("fill-opacity").is_none());
    assert_eq!(rectangles[2].attribute("fill-opacity"), Some("0.2"));
    assert_eq!(rectangles[2].attribute("stroke"), Some("#0af"));
    assert_eq!(rectangles[2].attribute("fill"), Some("url(#layer-2)"));
}

#[test]
fn wireframe_draws_bitmap_bounds_and_retains_the_source_resource() {
    let input = svg(
        r##"<defs><image id="im" width="20" height="30" xlink:href="data:image/png;base64,AAAA"/></defs><use xlink:href="#im" transform="translate(10 20)"/><image x="2" y="3" width="4" height="5" xlink:href="data:image/png;base64,AAAA"/>"##,
    );
    let result = fragment(
        &input,
        "layer",
        &InspectionStyle::Wireframe {
            stroke: "blue".into(),
            stroke_width: 1.0,
            stroke_opacity: 1.0,
        },
    )
    .unwrap();
    let wrapped = svg(&result);
    let document = Document::parse(&wrapped).unwrap();
    assert_eq!(
        document
            .descendants()
            .filter(|n| n.has_tag_name("image"))
            .count(),
        1
    );
    let rectangles = document
        .descendants()
        .filter(|n| n.has_tag_name("rect"))
        .collect::<Vec<_>>();
    assert_eq!(rectangles.len(), 2);
    assert_eq!(rectangles[0].attribute("width"), Some("20"));
    assert_eq!(
        rectangles[0].attribute("transform"),
        Some("translate(10 20)")
    );
    assert_eq!(rectangles[0].attribute("fill"), Some("none"));
    assert_eq!(rectangles[1].attribute("x"), Some("2"));
    assert!(rectangles
        .iter()
        .all(|n| n.attribute((XLINK_NS, "href")).is_none()));
}

#[test]
fn unsafe_and_unresolved_documents_are_rejected() {
    for body in [
        r#"<script>alert(1)</script>"#,
        r#"<foreignObject/>"#,
        r#"<animate attributeName="x"/>"#,
        r#"<rect onclick="x"/>"#,
        r#"<image href="https://example.com/image.png"/>"#,
        r#"<image href="data:image/svg+xml;base64,AAAA"/>"#,
        r#"<rect fill="url(https://example.com/paint)"/>"#,
        r#"<style>rect{fill:red}</style>"#,
        r#"<rect style="fill:u\72l(#a)"/>"#,
        r##"<use xlink:href="#missing"/>"##,
        r#"<rect filter="url(#missing)"/>"#,
        r#"<g id="a"/><g id="a"/>"#,
        r#"<g xml:base="https://example.com"/>"#,
    ] {
        assert!(
            optimize(&svg(body), &XmlOptions::default()).is_err(),
            "accepted {body}"
        );
    }
    assert!(optimize(
        r#"<!DOCTYPE svg [<!ENTITY e "x">]><svg xmlns="http://www.w3.org/2000/svg">&e;</svg>"#,
        &XmlOptions::default()
    )
    .is_err());
    assert!(optimize(
        &format!("<?xml-stylesheet href=\"evil.css\"?>{}", svg("")),
        &XmlOptions::default()
    )
    .is_err());
}

#[test]
fn invalid_options_are_errors() {
    for prefix in ["", "0prefix", "a b", "a\"", "a:b"] {
        assert!(fragment(&svg(""), prefix, &InspectionStyle::Normal).is_err());
    }
    assert!(optimize(
        &svg(""),
        &XmlOptions {
            precision: Some(13),
            ..Default::default()
        }
    )
    .is_err());
    for width in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(fragment(
            &svg(""),
            "a",
            &InspectionStyle::Wireframe {
                stroke: "blue".into(),
                stroke_width: width,
                stroke_opacity: 1.0,
            }
        )
        .is_err());
    }
    assert!(fragment(
        &svg(""),
        "a",
        &InspectionStyle::XRay {
            fill_opacity: 2.0,
            stroke: "blue".into(),
            stroke_width: 1.0,
            stroke_opacity: 1.0,
        }
    )
    .is_err());
}

#[test]
fn disabled_deduplication_preserves_repeated_definitions() {
    let result = optimize(
        &svg(r#"<clipPath id="a"><rect/></clipPath><clipPath id="b"><rect/></clipPath>"#),
        &XmlOptions {
            deduplicate: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.stats.definitions_deduplicated, 0);
    assert_eq!(
        Document::parse(&result.svg)
            .unwrap()
            .descendants()
            .filter(|n| n.has_tag_name("clipPath"))
            .count(),
        2
    );
}

#[test]
fn accessibility_references_follow_id_reassignment() {
    let result = optimized(
        r#"<title id="title">Example</title><desc id="description">Description</desc><g aria-labelledby="title description" aria-describedby="description"/>"#,
    );
    assert!(result.svg.contains("aria-labelledby=\"svg-0 svg-1\""));
    assert!(result.svg.contains("aria-describedby=\"svg-1\""));
}

#[test]
fn optimization_is_idempotent_after_resource_interning() {
    let options = XmlOptions::default();
    let first = optimize(&svg(r#"<clipPath id="a"><rect/></clipPath><clipPath id="b"><rect/></clipPath><g id="last" clip-path="url(#b)"/>"#), &options).unwrap();
    let second = optimize(&first.svg, &options).unwrap();
    assert_eq!(first.svg, second.svg);
    assert_eq!(first.stats.definitions_deduplicated, 1);
    assert_eq!(second.stats.definitions_deduplicated, 0);
}

#[test]
fn inspection_overrides_inline_paint_without_destroying_other_style() {
    let input = svg(
        r#"<path style="fill:red;stroke:green!important;stroke-width:9;filter:none" d="M0 0L10 10"/>"#,
    );
    let result = fragment(
        &input,
        "layer",
        &InspectionStyle::Wireframe {
            stroke: "blue".into(),
            stroke_width: 1.0,
            stroke_opacity: 1.0,
        },
    )
    .unwrap();
    let wrapped = svg(&result);
    let document = Document::parse(&wrapped).unwrap();
    let path = document
        .descendants()
        .find(|n| n.has_tag_name("path"))
        .unwrap();
    assert_eq!(path.attribute("fill"), Some("none"));
    assert_eq!(path.attribute("stroke"), Some("blue"));
    assert_eq!(path.attribute("style"), Some("filter:none"));
}
