//! The canvas and native editor project the same UTF-8 document runs into
//! FerroUI's public UTF-16 text-layout contract. No backend-specific glyph or
//! text APIs are used here.
use super::*;
use ferrodesign_core::TextDirection;
use ferroui_base::media::text_formatting::{TextParagraphStyle, TextParagraphStyles};
use ferroui_base::media::text_formatting::unicode::{BidiClass, Codepoint};
use ferroui_base::media::{FontFeature, FontFeatureCollection};
use ferroui_controls::presenters::TextStyleOverrides;
use ferroui_controls::TextBox;

pub(crate) fn flow_direction(node: &Node) -> FlowDirection {
    resolve_direction(node.text_direction, &node.text)
}

fn resolve_direction(direction: Option<TextDirection>, text: &str) -> FlowDirection {
    match direction {
        None | Some(TextDirection::LeftToRight) => FlowDirection::LeftToRight,
        Some(TextDirection::RightToLeft) => FlowDirection::RightToLeft,
        Some(TextDirection::Auto) => text.chars().find_map(|ch| match Codepoint::new(ch as u32).bi_di_class() {
            BidiClass::LeftToRight => Some(FlowDirection::LeftToRight),
            BidiClass::RightToLeft | BidiClass::ArabicLetter => Some(FlowDirection::RightToLeft),
            _ => None,
        }).unwrap_or(FlowDirection::LeftToRight),
    }
}

/// Independent paragraph ranges use the same public formatter as the native
/// TextBox. Auto resolves the first strong character separately per paragraph,
/// including paragraphs without explicit overrides and final empty paragraphs.
pub(crate) fn paragraph_styles(node: &Node) -> Option<TextParagraphStyles> {
    if node.text_paragraphs.is_empty() && node.text_direction != Some(TextDirection::Auto) { return None; }
    let mut styles = Vec::new();
    let mut cursor = 0;
    let mut utf16 = 0;
    for range in ferrodesign_core::text_paragraph_ranges(&node.text) {
        while cursor < node.text_paragraphs.len() && node.text_paragraphs[cursor].start < range.start { cursor += 1; }
        let style = node.text_paragraphs.get(cursor).filter(|style| style.start == range.start && style.end == range.end);
        let direction = style.and_then(|style| style.direction).or(node.text_direction);
        let start = utf16;
        let length = node.text[range.clone()].encode_utf16().count() as i32;
        utf16 += length;
        styles.push(ValueSpan::new(start, length, TextParagraphStyle {
            text_alignment: style.and_then(|style| style.alignment).map(|value| match value {
                TextAlign::Left => TextAlignment::Left, TextAlign::Center => TextAlignment::Center,
                TextAlign::Right => TextAlignment::Right, TextAlign::Justify => TextAlignment::Justify,
            }),
            flow_direction: Some(resolve_direction(direction, &node.text[range])),
            line_height: style.and_then(|style| style.line_height).map(|value| font_size(node.font_size) * value),
            letter_spacing: style.and_then(|style| style.letter_spacing),
            space_before: style.and_then(|style| style.space_before).unwrap_or(0.0),
            space_after: style.and_then(|style| style.space_after).unwrap_or(0.0),
            indent: style.and_then(|style| style.indent).unwrap_or(0.0),
        }));
    }
    Some(Rc::from(styles))
}

pub(crate) fn font_features(node: &Node) -> Option<FontFeatureCollection> {
    (!node.font_features.is_empty()).then(|| FontFeatureCollection::from_items(node.font_features.iter().map(|feature| FontFeature {
        tag: feature.tag.clone(),
        // FerroUI's public feature uses an i32 bit pattern. Its HarfBuzz shaper
        // passes this through as u32, retaining the full OpenType parameter.
        value: feature.value as i32,
        start: 0,
        end: -1,
    })))
}

pub(crate) fn range_styles(node: &Node, brush: Rc<dyn IBrush>) -> Option<TextStyleOverrides> {
    if node.text.is_empty() { return None; }
    let features = font_features(node);
    let properties = |run: Option<&ferrodesign_core::TextRun>| -> Rc<dyn TextRunProperties> {
        Rc::new(GenericTextRunProperties::with_all(
            typeface(run.and_then(|run| run.font_family.as_deref()).unwrap_or(&node.font_family),
                run.and_then(|run| run.font_weight).unwrap_or(node.font_weight),
                run.and_then(|run| run.italic).unwrap_or(node.italic)),
            font_size(run.and_then(|run| run.font_size).unwrap_or(node.font_size)),
            decorations(run.and_then(|run| run.decoration).unwrap_or(node.text_decoration)),
            Some(run.and_then(|run| run.color).map(|color| solid_brush(color, brush.opacity())).unwrap_or_else(|| brush.clone())),
            None, BaselineAlignment::Baseline, Some(CultureInfo::invariant_culture()), features.clone(),
        ))
    };
    let defaults = properties(None);
    let mut ranges: Vec<_> = node.text_runs.iter().filter(|run| run.start < run.end
        && run.end <= node.text.len() && node.text.is_char_boundary(run.start)
        && node.text.is_char_boundary(run.end)).collect();
    ranges.sort_by_key(|run| run.start);
    let mut result = Vec::with_capacity(ranges.len() * 2 + 1);
    let mut cursor = 0;
    for run in ranges {
        if run.end <= cursor { continue; }
        if run.start > cursor {
            let start = byte_to_utf16(&node.text, cursor);
            result.push(ValueSpan::new(start, byte_to_utf16(&node.text, run.start) - start, defaults.clone()));
        }
        let start = byte_to_utf16(&node.text, run.start.max(cursor));
        result.push(ValueSpan::new(start, byte_to_utf16(&node.text, run.end) - start, properties(Some(run))));
        cursor = run.end;
    }
    if cursor < node.text.len() {
        let start = byte_to_utf16(&node.text, cursor);
        result.push(ValueSpan::new(start, byte_to_utf16(&node.text, node.text.len()) - start, defaults));
    }
    Some(Rc::from(result))
}

/// A native TextBox has one foreground paint. Preserve the first visible fill
/// (including gradients/images and fill opacity), and let range colors override
/// it. Its selection and IME overlays keep these fonts and advances intact.
pub(crate) fn editor_brush(node: &Node) -> Rc<dyn IBrush> {
    node.fills.iter().find(|fill| fill.visible).map(|fill| {
        let (brush, _) = make_brush(&fill.paint, fill.opacity, &mut HashMap::new());
        let brush: Rc<dyn IBrush> = brush;
        brush
    }).unwrap_or_else(|| solid_brush(Color::TRANSPARENT, 1.0))
}

/// Authored text keeps its shaped contours and fractional baseline in document
/// coordinates. Device hinting and baseline snapping would move its ink as the
/// view changes and disagree with its scalable stroke, shadow and flatten paths.
pub(crate) fn design_text_options() -> TextOptions {
    TextOptions {
        text_hinting_mode: TextHintingMode::None,
        baseline_pixel_alignment: BaselinePixelAlignment::Unaligned,
        ..TextOptions::default()
    }
}

pub(crate) fn configure_text_box(field: &TextBox, node: &Node) {
    TextOptions::set_text_options(field, design_text_options());
    // The inline editor shares the document's fractional layout bounds before
    // the host applies its world/camera transform. This inherits into its
    // template, preserving wrapping and placement while entering edit mode.
    field.set_use_layout_rounding(false);
    field.set_font_family(FontFamily::new(&node.font_family));
    field.set_font_size(font_size(node.font_size));
    field.set_font_weight(FontWeight(node.font_weight.clamp(1, 1000) as i32));
    field.set_font_style(if node.italic { FontStyle::Italic } else { FontStyle::Normal });
    field.set_font_features(font_features(node));
    field.set_flow_direction(flow_direction(node));
    field.set_line_height(font_size(node.font_size) * finite(node.line_height).clamp(0.01, 100.0));
    field.set_letter_spacing(finite(node.letter_spacing));
    field.set_text_alignment(match node.text_align {
        TextAlign::Left => TextAlignment::Left, TextAlign::Center => TextAlignment::Center,
        TextAlign::Right => TextAlignment::Right, TextAlign::Justify => TextAlignment::Justify,
    });
    field.set_text_wrapping(if node.text_auto_resize == TextAutoResize::WidthAndHeight { TextWrapping::NoWrap } else { TextWrapping::Wrap });
    let brush = editor_brush(node);
    field.set_foreground(Some(brush.clone()));
    field.set_text_style_overrides(range_styles(node, brush));
    field.set_text_paragraph_styles(paragraph_styles(node));
}

pub(crate) fn refresh_text_box_ranges(field: &TextBox, node: &Node) {
    let brush = field.foreground().unwrap_or_else(|| editor_brush(node));
    field.set_text_style_overrides(range_styles(node, brush));
    field.set_text_paragraph_styles(paragraph_styles(node));
    field.set_flow_direction(flow_direction(node));
}

pub(crate) fn inline_geometry(node: &Node, brush: Rc<dyn IBrush>) -> (UiPoint, f64, f64) {
    let text = make_text_layout(node, brush);
    let width = if node.text_auto_resize == TextAutoResize::WidthAndHeight {
        text.layout.width_including_trailing_whitespace().max(20.0)
    } else { node.width.max(0.001) };
    (text.origin, width, text.layout.height().max(font_size(node.font_size) * node.line_height))
}

/// Read-only browser diagnostics from the actual native presenter. Keep the
/// bounded run sample out of ordinary TextBox probes and the rendering path.
pub(crate) fn text_box_state(field: &TextBox) -> serde_json::Value {
    use ferroui_controls::presenters::TextPresenter;
    use serde_json::json;
    let Some(presenter) = field.find_descendant_of_type::<TextPresenter>(false) else { return serde_json::Value::Null; };
    let layout = presenter.text_layout();
    let mut runs = Vec::new();
    let mut count = 0;
    for line in layout.text_lines() {
        for run in line.text_runs().iter() {
            if run.text().is_empty() { continue; }
            count += 1;
            if runs.len() >= 64 { continue; }
            let Some(properties) = run.properties() else { continue; };
            let color = properties.foreground_brush().and_then(|brush| brush.as_solid_color_brush()).map(|brush| {
                let color = brush.color();
                json!([color.r, color.g, color.b, color.a])
            });
            let features: Vec<_> = properties.font_features().into_iter().flat_map(|features| features.iter())
                .map(|feature| json!({"tag":feature.tag,"value":feature.value as u32})).collect();
            runs.push(json!({"text":run.text().to_string_lossy(),"utf16Length":run.text().len(),
                "fontFamily":properties.typeface().font_family().to_string(),
                "fontSize":properties.font_rendering_em_size(),"fontWeight":properties.typeface().weight().0,
                "italic":properties.typeface().style() == FontStyle::Italic,
                "decorationCount":properties.text_decorations().map_or(0, |value| value.len()),
                "features":features,"color":color}));
        }
    }
    let start = field.selection_start().min(field.selection_end());
    let length = (field.selection_end() - field.selection_start()).abs();
    let rect = |rect: ferroui_base::Rect| json!({"x":rect.x,"y":rect.y,"width":rect.width,"height":rect.height});
    let selected: Vec<_> = layout.hit_test_text_range(start, length).into_iter().map(rect).collect();
    let lines: Vec<_> = layout.text_lines().iter().enumerate().take(64).map(|(index, line)| {
        let origin = layout.line_origin(index);
        json!({"start":line.first_text_source_index(),"length":line.length(),
            "x":origin.x,"y":origin.y,"height":line.height(),
            "direction":format!("{:?}",layout.line_flow_direction(index))})
    }).collect();
    json!({"direction":format!("{:?}",presenter.flow_direction()),"runs":runs,"runCount":count,
        "styleCount":field.text_style_overrides().as_ref().map_or(0, |styles| styles.len()),
        "paragraphStyleCount":field.text_paragraph_styles().as_ref().map_or(0, |styles| styles.len()),"lines":lines,
        "layoutWidth":layout.width_including_trailing_whitespace(),"layoutHeight":layout.height(),
        "baseline":layout.baseline(),"lineCount":layout.text_lines().len(),
        "caretIndex":field.caret_index(),"selectionStart":field.selection_start(),"selectionEnd":field.selection_end(),
        "caret":rect(presenter.cursor_rectangle()),"selectionRects":selected,"preedit":presenter.preedit_text()})
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests {
    use super::*;
    use ferrodesign_core::{FontFeatureSetting, TextRun};
    use ferroui_base::media::imaging::RenderTargetBitmap;
    use ferroui_base::media::FontManager;
    use ferroui_base::platform::{IAssetLoader, StandardAssetLoader};
    use ferroui_base::{FerroLocator, PixelRect, PixelSize, Size};
    use ferroui_controls::presenters::TextPresenter;

    fn fonts() -> Rc<dyn ferroui_base::reactive::IDisposable> {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable().bind::<dyn IAssetLoader>().to_constant(Rc::new(StandardAssetLoader::new(None)));
        ferroui_vello::VelloPlatform::initialize();
        ferroui_harfbuzz::HarfBuzzPlatform::initialize();
        FontManager::current().add_font_collection(Rc::new(ferroui_fonts_inter::InterFontCollection::new()));
        scope
    }

    fn text_node(value: &str) -> Node {
        let mut node = Node::new(NodeKind::Text, "Typography", 0.0, 0.0, 500.0, 120.0);
        node.text = value.into();
        node.font_family = "fonts:Inter#Inter".into();
        node.font_size = 24.0;
        node.line_height = 1.4;
        node.fills = vec![ferrodesign_core::Fill::solid(Color::BLACK)];
        node
    }

    fn presenter(node: &Node) -> Ref<TextPresenter> {
        let field = TextBox::new();
        configure_text_box(&field, node);
        let presenter = TextPresenter::new();
        // This isolated presenter fixture has no TextBox template ancestor.
        // Copy the inherited visual policy along with its typography inputs.
        TextOptions::set_text_options(&presenter, TextOptions::get_text_options(&field));
        presenter.set_use_layout_rounding(field.use_layout_rounding());
        presenter.set_text(Some(&node.text));
        presenter.set_font_family(field.font_family());
        presenter.set_font_size(field.font_size());
        presenter.set_font_weight(field.font_weight());
        presenter.set_font_style(field.font_style());
        presenter.set_foreground(field.foreground());
        presenter.set_font_features(field.font_features());
        presenter.set_text_style_overrides(field.text_style_overrides());
        presenter.set_text_paragraph_styles(field.text_paragraph_styles());
        presenter.set_flow_direction(field.flow_direction());
        presenter.set_line_height(field.line_height());
        presenter.set_letter_spacing(field.letter_spacing());
        presenter.set_text_alignment(field.text_alignment());
        presenter.set_text_wrapping(field.text_wrapping());
        presenter.measure(Size::new(node.width, f64::INFINITY));
        presenter.arrange(UiRect::new(0.0, 0.0, node.width, presenter.desired_size().height));
        presenter
    }

    fn pixels(paint: impl FnOnce(&mut DrawingContext)) -> Vec<u8> {
        let bitmap = RenderTargetBitmap::new(PixelSize::new(512, 160));
        paint(&mut bitmap.create_drawing_context());
        let mut pixels = vec![0; 512 * 160 * 4];
        bitmap.copy_pixels(PixelRect::new(0, 0, 512, 160), &mut pixels, 512 * 4);
        pixels
    }

    #[test]
    fn typography_native_presenter_and_canvas_have_identical_styled_unicode_pixels_and_hit_geometry() {
        let scope = fonts();
        let mut node = text_node("AV Ω🦀ffi");
        node.letter_spacing = 0.375;
        node.text_decoration = TextDecoration::Underline;
        node.font_features = core_features("kern=0, liga=0");
        node.text_runs.push(TextRun {
            start: 3, end: "AV Ω🦀".len(), font_family: None, font_size: Some(36.0),
            font_weight: Some(700), color: Some(Color::rgb(196, 20, 66)), italic: Some(true),
            decoration: Some(TextDecoration::Strikethrough),
        });
        let canvas = make_text_layout(&node, editor_brush(&node));
        let presenter = presenter(&node);
        let options = TextOptions::get_text_options(&presenter);
        assert_eq!(options.text_hinting_mode, TextHintingMode::None);
        assert_eq!(options.baseline_pixel_alignment, BaselinePixelAlignment::Unaligned);
        assert!(!presenter.use_layout_rounding());
        let native = presenter.text_layout();
        assert_eq!(canvas.layout.width_including_trailing_whitespace(), native.width_including_trailing_whitespace());
        assert_eq!(canvas.layout.height(), native.height());
        for (start, length) in [(0, 2), (3, 1), (4, 2), (6, 3)] {
            assert_eq!(canvas.layout.hit_test_text_range(start, length), native.hit_test_text_range(start, length));
        }
        let mut document = Document::new("Native typography parity");
        document.insert_node(1, node).unwrap();
        let editor = Editor::new(document);
        let canvas_pixels = pixels(|context| {
            let stats = ScenePainter::default().render_editor(context, &editor,
                &editor.document.pages[0].children, Rect::new(0.0, 0.0, 512.0, 160.0));
            assert_eq!(stats.invalid_nodes, 0);
            assert_eq!(stats.unsupported_effects, 0);
        });
        let native_pixels = pixels(|context| {
            // ImmediateRenderer applies visual TextOptions just as the live
            // compositor does; calling presenter.render directly bypasses it.
            ferroui_base::rendering::ImmediateRenderer::render_clipped(context,
                &presenter.clone().upcast(), UiRect::new(0.0, 0.0, 512.0, 160.0));
        });
        assert!(canvas_pixels.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 500);
        assert_eq!(canvas_pixels, native_pixels);
        scope.dispose();
    }

    fn core_features(value: &str) -> Vec<FontFeatureSetting> { ferrodesign_core::parse_font_features(value).unwrap() }

    #[test]
    fn typography_feature_changes_shape_real_inter_advances_and_invalidate_the_retained_visual() {
        let scope = fonts();
        let mut node = text_node("AVAVAV");
        node.font_size = 40.0;
        node.font_features = core_features("kern=1");
        let with_kerning = make_text_layout(&node, editor_brush(&node));
        let before = visual_fingerprint(&node, 0, false, 0);
        node.font_features = core_features("kern=0");
        let without_kerning = make_text_layout(&node, editor_brush(&node));
        assert!(without_kerning.layout.width() - with_kerning.layout.width() > 0.1,
            "OpenType settings must change real HarfBuzz advances, not only the document");
        assert_ne!(before, visual_fingerprint(&node, 0, false, 0));
        let native = presenter(&node).text_layout();
        assert_eq!(without_kerning.layout.width(), native.width());
        scope.dispose();
    }

    #[test]
    fn typography_auto_direction_uses_unicode_strong_classes_and_explicit_direction_wins() {
        let mut node = text_node(" 123 🙂 שלום / Latin");
        assert_eq!(flow_direction(&node), FlowDirection::LeftToRight, "legacy default remains LTR");
        node.text_direction = Some(TextDirection::Auto);
        assert_eq!(flow_direction(&node), FlowDirection::RightToLeft);
        node.text = "42 العربية".into();
        assert_eq!(flow_direction(&node), FlowDirection::RightToLeft);
        node.text = "👩🏽‍💻 東京 العربية".into();
        assert_eq!(flow_direction(&node), FlowDirection::LeftToRight);
        node.text_direction = Some(TextDirection::RightToLeft);
        assert_eq!(flow_direction(&node), FlowDirection::RightToLeft);
        node.text = " 123 🙂".into();
        node.text_direction = Some(TextDirection::Auto);
        assert_eq!(flow_direction(&node), FlowDirection::LeftToRight);
    }

    #[test]
    fn typography_inline_baseline_and_height_follow_actual_rich_layout_metrics() {
        let scope = fonts();
        let mut node = text_node("Mixed size");
        node.text_vertical_align = TextVerticalAlign::Center;
        node.text_runs.push(TextRun { start: 0, end: 5, font_family: None, font_size: Some(44.0),
            font_weight: None, color: None, italic: None, decoration: None });
        let canvas = make_text_layout(&node, editor_brush(&node));
        let (origin, width, height) = inline_geometry(&node, editor_brush(&node));
        assert_eq!(origin, canvas.origin);
        assert_eq!(origin.y, (node.height - canvas.layout.height()) / 2.0);
        assert_eq!(width, node.width);
        assert!(height >= canvas.layout.height());
        scope.dispose();
    }

    #[test]
    fn typography_auto_direction_resolves_each_paragraph_and_preserves_utf16_boundaries() {
        let mut node = text_node("A🦀\r\nשלום\n");
        node.text_direction = Some(TextDirection::Auto);
        let styles = paragraph_styles(&node).unwrap();
        assert_eq!(styles.iter().map(|style| (style.start(), style.length())).collect::<Vec<_>>(), vec![(0,5),(5,5),(10,0)]);
        assert_eq!(styles.iter().map(|style| style.value().flow_direction.unwrap()).collect::<Vec<_>>(),
            vec![FlowDirection::LeftToRight, FlowDirection::RightToLeft, FlowDirection::LeftToRight]);
        node.apply_paragraph_style("A🦀\r\n".len().."A🦀\r\nשלום\n".len(), &ferrodesign_core::ParagraphStylePatch {
            direction: ferrodesign_core::TextStyleChange::Set(TextDirection::LeftToRight), ..Default::default()
        }).unwrap();
        assert_eq!(paragraph_styles(&node).unwrap()[1].value().flow_direction, Some(FlowDirection::LeftToRight));
    }

    #[test]
    fn typography_paragraph_native_and_canvas_pixels_geometry_and_cache_identity_agree() {
        let scope = fonts();
        let mut node = text_node("A🦀 B\nשלום\nLast");
        node.font_size = 20.0;
        node.text_direction = Some(TextDirection::Auto);
        node.width = 360.0;
        node.height = 150.0;
        let fingerprint = visual_fingerprint(&node, 0, false, 0);
        let ranges = ferrodesign_core::text_paragraph_ranges(&node.text);
        node.apply_paragraph_style(ranges[0].clone(), &ferrodesign_core::ParagraphStylePatch {
            indent: ferrodesign_core::TextStyleChange::Set(12.25),
            space_before: ferrodesign_core::TextStyleChange::Set(3.5),
            space_after: ferrodesign_core::TextStyleChange::Set(5.75),
            letter_spacing: ferrodesign_core::TextStyleChange::Set(0.375), ..Default::default()
        }).unwrap();
        node.apply_paragraph_style(ranges[1].clone(), &ferrodesign_core::ParagraphStylePatch {
            alignment: ferrodesign_core::TextStyleChange::Set(TextAlign::Right),
            indent: ferrodesign_core::TextStyleChange::Set(10.5),
            line_height: ferrodesign_core::TextStyleChange::Set(1.8),
            space_before: ferrodesign_core::TextStyleChange::Set(7.25), ..Default::default()
        }).unwrap();
        assert_ne!(fingerprint, visual_fingerprint(&node, 0, false, 0), "Paragraph edits invalidate retained text visuals");
        let canvas = make_text_layout(&node, editor_brush(&node));
        let presenter = presenter(&node);
        let native = presenter.text_layout();
        assert_eq!(canvas.layout.height(), native.height());
        assert_eq!(native.text_lines().len(), 3);
        assert_eq!(native.line_origin(0), UiPoint::new(12.25, 3.5));
        assert_eq!(native.line_flow_direction(1), FlowDirection::RightToLeft);
        for index in 0..3 {
            assert_eq!(canvas.layout.line_origin(index), native.line_origin(index));
            let line = &native.text_lines()[index];
            let start = line.first_text_source_index();
            assert_eq!(canvas.layout.hit_test_text_range(start, line.length()), native.hit_test_text_range(start, line.length()));
            presenter.move_caret_to_text_position(start, false);
            assert_eq!(presenter.cursor_rectangle().y, native.line_origin(index).y);
        }
        presenter.hide_caret();
        let mut document = Document::new("Native paragraph pixel parity");
        document.insert_node(1, node).unwrap();
        let editor = Editor::new(document);
        let canvas_pixels = pixels(|context| {
            let stats = ScenePainter::default().render_editor(context, &editor,
                &editor.document.pages[0].children, Rect::new(0.0, 0.0, 512.0, 160.0));
            assert_eq!(stats.invalid_nodes, 0);
        });
        let native_pixels = pixels(|context| ferroui_base::rendering::ImmediateRenderer::render_clipped(context,
            &presenter.clone().upcast(), UiRect::new(0.0, 0.0, 512.0, 160.0)));
        assert!(canvas_pixels.chunks_exact(4).filter(|pixel| pixel[3] > 0).count() > 100);
        assert_eq!(canvas_pixels, native_pixels);
        scope.dispose();
    }
}
