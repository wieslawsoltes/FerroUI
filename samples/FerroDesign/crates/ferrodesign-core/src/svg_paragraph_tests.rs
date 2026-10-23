use super::*;

#[test]
fn svg_editable_paragraphs_preserve_hard_breaks_independent_metrics_direction_and_rich_runs() {
    let mut node = Node::new(NodeKind::Text, "Paragraph SVG", 0.0, 0.0, 200.0, 150.0)
        .with_text("A🦀\r\nשלום\u{2029}", 20.0, Color::BLACK);
    node.line_height = 1.2;
    let ranges = crate::text_paragraph_ranges(&node.text);
    node.apply_paragraph_style(ranges[0].clone(), &ParagraphStylePatch {
        alignment: TextStyleChange::Set(TextAlign::Center), direction: TextStyleChange::Set(TextDirection::LeftToRight),
        line_height: TextStyleChange::Set(1.5), letter_spacing: TextStyleChange::Set(0.5),
        space_before: TextStyleChange::Set(3.0), space_after: TextStyleChange::Set(7.0), indent: TextStyleChange::Set(10.0),
    }).unwrap();
    node.apply_paragraph_style(ranges[1].clone(), &ParagraphStylePatch {
        alignment: TextStyleChange::Set(TextAlign::Right), direction: TextStyleChange::Set(TextDirection::RightToLeft),
        line_height: TextStyleChange::Set(2.0), space_before: TextStyleChange::Set(5.0),
        space_after: TextStyleChange::Set(11.0), indent: TextStyleChange::Set(20.0), ..Default::default()
    }).unwrap();
    node.apply_text_style(1..5, &TextStylePatch { font_size: TextStyleChange::Set(28.0), ..Default::default() }).unwrap();
    let source = text_shape(&node, "fill=\"black\"");
    let xml = roxmltree::Document::parse(&source).unwrap();
    let paragraphs: Vec<_> = xml.root_element().children().filter(|child| child.is_element()).collect();
    assert_eq!(paragraphs.len(), 3);
    assert_eq!(paragraphs[0].attribute("x"), Some("105"));
    assert_eq!(paragraphs[0].attribute("y"), Some("23"));
    assert_eq!(paragraphs[0].attribute("text-anchor"), Some("middle"));
    assert_eq!(paragraphs[0].attribute("letter-spacing"), Some("0.5"));
    assert_eq!(paragraphs[0].attribute("data-paragraph-end"), Some("7"));
    assert_eq!(paragraphs[1].attribute("x"), Some("180"));
    assert_eq!(paragraphs[1].attribute("y"), Some("65"));
    assert_eq!(paragraphs[1].attribute("text-anchor"), Some("start"));
    assert_eq!(paragraphs[1].attribute("direction"), Some("rtl"));
    assert_eq!(paragraphs[1].attribute("data-line-height"), Some("40"));
    assert_eq!(paragraphs[2].attribute("y"), Some("116"));
    let restored: TextParagraph = serde_json::from_str(paragraphs[1].attribute("data-ferrodesign-paragraph").unwrap()).unwrap();
    assert_eq!(restored, node.text_paragraphs[1]);
    assert!(source.contains("font-size=\"28\""));
    assert!(source.contains('🦀'));
    assert!(!source.contains('\r'));
    assert!(!source.contains('\u{2029}'));
}

#[test]
fn svg_auto_paragraph_direction_uses_plaintext_bidi_and_reports_native_layout_limits() {
    let mut document = Document::new("Paragraph SVG warning");
    let mut node = Node::new(NodeKind::Text, "Auto label", 0.0, 0.0, 200.0, 120.0)
        .with_text("Latin\nשלום\n", 20.0, Color::BLACK);
    node.text_direction = Some(TextDirection::Auto);
    node.apply_paragraph_style(0..0, &ParagraphStylePatch {
        indent: TextStyleChange::Set(12.0), ..Default::default()
    }).unwrap();
    let id = document.insert_node(document.pages[0].id, node).unwrap();
    let report = document.export_svg_report(document.pages[0].id, &[id]);
    assert_eq!(report.source.matches("unicode-bidi=\"plaintext\"").count(), 3);
    assert_eq!(report.source.matches("data-paragraph-direction=\"auto\"").count(), 3);
    assert!(report.warnings.iter().any(|warning| warning.contains("native wrapping")
        && warning.contains("Auto-direction logical indentation")));
}
