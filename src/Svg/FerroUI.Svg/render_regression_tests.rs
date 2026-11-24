use super::*;
use crate::{SvgExplodedView, SvgLayerFilter, SvgLayerOptions, SvgRenderTarget, SvgView};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Colors};
use ferroui_base::platform::{IDrawingContextImpl, IRenderTarget, RenderTargetSceneInfo};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{PixelSize, RoundedRect, Size};

fn rectangle(context: &mut dyn IDrawingContextImpl, bounds: Rect) {
    context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from(bounds),
        &BoxShadows::default(),
    );
}

fn simple_document() -> SvgDocument {
    let mut document = SvgDocument::new(Size::new(40.0, 20.0)).unwrap();
    document
        .record_layer(SvgLayerOptions::new("part", "A part"), |context| {
            rectangle(context, Rect::new(0.0, 0.0, 20.0, 10.0));
        })
        .unwrap();
    document
}

#[test]
fn repeated_raster_payloads_are_interned_across_transformed_layers() {
    let mut document = simple_document();
    document
        .record_layer(
            SvgLayerOptions::new("copy", "A copy")
                .with_transform(Matrix::create_translation(5.0, 0.0)),
            |context| {
                rectangle(context, Rect::new(0.0, 0.0, 20.0, 10.0));
            },
        )
        .unwrap();
    let output = document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(output.report.raster_layers, 2);
    assert_eq!(output.report.embedded_images, 1);
    assert_eq!(output.report.definitions_deduplicated, 1);
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let uses = xml
        .descendants()
        .filter(|node| node.has_tag_name("use"))
        .collect::<Vec<_>>();
    assert_eq!(uses.len(), 2);
    assert_eq!(
        uses[0].attribute(("http://www.w3.org/1999/xlink", "href")),
        uses[1].attribute(("http://www.w3.org/1999/xlink", "href"))
    );
}

#[test]
fn inspection_respects_stroke_alpha_and_the_selected_layers() {
    let mut document = simple_document();
    document
        .record_layer(
            SvgLayerOptions::new("labels", "Unchanged annotations"),
            |context| {
                rectangle(context, Rect::new(2.0, 2.0, 10.0, 5.0));
            },
        )
        .unwrap();
    let output = document
        .export(&SvgExportOptions {
            view: SvgView {
                inspection: SvgInspection::Wireframe {
                    stroke: Color::from_argb(64, 10, 20, 30),
                    stroke_width: 1.0,
                },
                inspection_layers: SvgLayerFilter::id("part"),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let part = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("ferro-part"))
        .unwrap();
    assert!(part.descendants().any(|node| {
        node.attribute("stroke") == Some("#0a141e")
            && node
                .attribute("stroke-opacity")
                .and_then(|a| a.parse::<f64>().ok())
                == Some(64.0 / 255.0)
    }));
    let annotations = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("ferro-labels"))
        .unwrap();
    assert!(!annotations
        .descendants()
        .any(|node| node.attribute("stroke") == Some("#0a141e")));
}

#[test]
fn raster_wireframe_emits_only_bounds_and_keeps_unselected_pixels() {
    let mut document = simple_document();
    document
        .record_layer(
            SvgLayerOptions::new("labels", "Unchanged annotations"),
            |context| {
                rectangle(context, Rect::new(2.0, 2.0, 10.0, 5.0));
            },
        )
        .unwrap();
    let output = document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            view: SvgView {
                inspection: SvgInspection::wireframe(),
                inspection_layers: SvgLayerFilter::id("part"),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let part = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("ferro-part"))
        .unwrap();
    assert!(!part
        .descendants()
        .any(|node| node.has_tag_name("use") || node.has_tag_name("image")));
    assert!(part
        .descendants()
        .any(|node| node.has_tag_name("rect") && node.attribute("fill") == Some("none")));
    assert_eq!(output.report.embedded_images, 1);
}

#[test]
fn label_and_metadata_whitespace_round_trips() {
    let mut options = SvgLayerOptions::new("part", "A\nB\tC\rD");
    options
        .metadata
        .insert("description".into(), "one\ntwo\tthree\rfour".into());
    let mut document = SvgDocument::new(Size::new(10.0, 10.0)).unwrap();
    document.record_layer(options, |_| {}).unwrap();
    let output = document.export(&SvgExportOptions::default()).unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let layer = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("ferro-part"))
        .unwrap();
    assert_eq!(layer.attribute("data-name"), Some("A\nB\tC\rD"));
    assert_eq!(
        layer.attribute("data-meta-description"),
        Some("one\ntwo\tthree\rfour")
    );
}

#[test]
fn outlined_exports_also_outline_generated_exploded_labels() {
    let output = simple_document()
        .export(&SvgExportOptions {
            view: SvgView {
                exploded: Some(SvgExplodedView::default()),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    assert!(!xml.descendants().any(|node| node.has_tag_name("text")));
    let label = xml
        .descendants()
        .find(|node| node.attribute("data-role") == Some("layer-label"))
        .unwrap();
    assert!(label.descendants().any(|node| node.has_tag_name("path")
        && node.attribute("d").is_some_and(|path| !path.is_empty())));
}

#[test]
fn exploded_annotations_are_painted_after_all_scene_layers() {
    let mut document = SvgDocument::new(Size::new(40.0, 20.0)).unwrap();
    for (id, depth) in [("back", 1.0), ("front", 2.0)] {
        document
            .record_layer(SvgLayerOptions::new(id, id).with_depth(depth), |context| {
                rectangle(context, Rect::new(0.0, 0.0, 20.0, 10.0))
            })
            .unwrap();
    }
    let output = document
        .export(&SvgExportOptions {
            text: SvgTextMode::Editable,
            view: SvgView {
                exploded: Some(SvgExplodedView::default()),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let last_layer_end = xml
        .descendants()
        .find(|node| node.attribute("id") == Some("ferro-front"))
        .unwrap()
        .range()
        .end;
    let annotations = xml
        .descendants()
        .filter(|node| {
            matches!(
                node.attribute("data-role"),
                Some("layer-label" | "explosion-guide")
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(annotations.len(), 4);
    assert!(annotations
        .iter()
        .all(|node| node.range().start >= last_layer_end));
    let last_guide_end = annotations
        .iter()
        .filter(|node| node.attribute("data-role") == Some("explosion-guide"))
        .map(|node| node.range().end)
        .max()
        .unwrap();
    assert!(annotations
        .iter()
        .filter(|node| node.attribute("data-role") == Some("layer-label"))
        .all(|node| node.range().start >= last_guide_end));
}

#[test]
fn composed_transform_and_bounds_overflow_are_errors() {
    let mut document = SvgDocument::new(Size::new(2.0, 2.0)).unwrap();
    document
        .record_layer(
            SvgLayerOptions::new("part", "Part")
                .with_depth(1.0)
                .with_transform(Matrix::create_translation(f64::MAX, 0.0)),
            |_| {},
        )
        .unwrap();
    assert!(matches!(
        document.export(&SvgExportOptions {
            view: SvgView {
                exploded: Some(SvgExplodedView {
                    step: Vector::new(f64::MAX, 0.0),
                    show_labels: false,
                    show_connectors: false
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
        Err(SvgError::InvalidOptions(_))
    ));
    let mut scaled = SvgDocument::new(Size::new(2.0, 2.0)).unwrap();
    scaled
        .record_layer(
            SvgLayerOptions::new("part", "Part")
                .with_transform(Matrix::create_scale(f64::MAX, 1.0)),
            |_| {},
        )
        .unwrap();
    assert!(matches!(
        scaled.export(&SvgExportOptions::default()),
        Err(SvgError::InvalidOptions(_))
    ));
}

#[test]
fn raster_mapping_rejects_nonfinite_native_scale_and_translation_before_replay() {
    for bounds in [
        Rect::new(0.0, 0.0, 1e-50, 1.0),
        Rect::new(0.0, 0.0, 1.0, 1e-50),
        Rect::new(f64::from(f32::MAX) * 0.75, 0.0, 1.0, 1.0),
        Rect::new(0.0, -f64::from(f32::MAX) * 0.75, 1.0, 1.0),
    ] {
        let mut document = SvgDocument::new(Size::new(2.0, 2.0)).unwrap();
        document
            .record_layer(
                SvgLayerOptions::new("part", "Part").with_bounds(bounds),
                |_| {},
            )
            .unwrap();
        let options = SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            raster_scale: 2.0,
            ..Default::default()
        };
        // The surface is at most 2x2; this is a matrix failure, not a pixel-budget failure.
        let (width, height) = raster_size(bounds, &options).unwrap();
        assert!(width <= 2 && height <= 2);
        assert!(
            matches!(document.export(&options), Err(SvgError::InvalidOptions(message)) if message.contains("finite Skia device transform")),
            "accepted {bounds:?}"
        );
    }
}

#[test]
fn target_records_device_pixels_and_converts_to_logical_coordinates_once() {
    let target = SvgRenderTarget::new();
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(40, 20),
        2.0,
        CompositionTransparencyLevel::None,
    );
    let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
    context.set_transform(Matrix::create_scale(2.0, 2.0));
    rectangle(&mut *context, Rect::new(0.0, 0.0, 10.0, 10.0));
    context.dispose();
    let document = target.take_document().unwrap();
    assert_eq!(document.size(), Size::new(20.0, 10.0));
    assert_eq!(
        document.layers()[0].bounds(),
        Rect::new(0.0, 0.0, 40.0, 20.0)
    );
    assert_eq!(
        document.layers()[0].options().transform,
        Matrix::create_scale(0.5, 0.5)
    );
    let png = raster_layer(
        &document.layers()[0],
        &SvgExportOptions {
            raster_scale: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    let image = sk::Image::from_encoded(sk::Data::new_copy(&png))
        .unwrap()
        .make_raster_image(None, sk::image::CachingHint::Allow)
        .unwrap();
    let pixels = image.peek_pixels().unwrap();
    assert_eq!((pixels.width(), pixels.height()), (40, 20));
    assert_eq!(pixels.get_color((15, 10)), sk::Color::RED);
    assert_eq!(pixels.get_color((30, 10)).a(), 0);
    let output = document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::VectorOnly,
            ..Default::default()
        })
        .unwrap();
    assert!(output.svg.contains("matrix(0.5 0 0 0.5 0 0)"));
}

#[test]
fn target_accepts_pixel_sized_compositor_layer_blits_and_preserves_rounded_logical_size() {
    let target = SvgRenderTarget::new();
    let mut scene = RenderTargetSceneInfo::new(
        PixelSize::new(40, 20),
        2.0,
        CompositionTransparencyLevel::None,
    );
    scene.logical_size = Size::new(19.8, 9.9);
    let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
    let layer = context.create_layer(PixelSize::new(40, 20));
    let mut layer_context = layer.create_drawing_context();
    rectangle(&mut *layer_context, Rect::new(0.0, 0.0, 20.0, 20.0));
    layer_context.dispose();
    context.clear(Colors::TRANSPARENT);
    context.set_transform(Matrix::IDENTITY);
    layer.blit(&mut *context);
    context.dispose();
    let document = target.take_document().unwrap();
    assert_eq!(document.size(), Size::new(19.8, 9.9));
    assert_eq!(
        document.layers()[0].options().transform,
        Matrix::create_scale(0.5, 0.5)
    );
    let output = document
        .export(&SvgExportOptions {
            raster_scale: 1.0,
            ..Default::default()
        })
        .unwrap();
    assert!(output.svg.contains("viewBox=\"0 0 19.8 9.9\""));
    assert_eq!(output.report.raster_layers, 1);
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    let image = xml
        .descendants()
        .find(|node| node.has_tag_name("image"))
        .unwrap();
    assert_eq!(image.attribute("width"), Some("40"));
    assert_eq!(image.attribute("height"), Some("20"));
}

#[test]
fn target_clip_survives_exporting_a_larger_viewport() {
    let target = SvgRenderTarget::new();
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(40, 20),
        1.0,
        CompositionTransparencyLevel::None,
    );
    let (mut context, _) = target.try_create_drawing_context(&scene).unwrap();
    rectangle(&mut *context, Rect::new(30.0, 10.0, 50.0, 50.0));
    context.dispose();
    let output = target
        .export_last(&SvgExportOptions {
            view_box: Some(Rect::new(0.0, 0.0, 100.0, 100.0)),
            fidelity: SvgFidelity::VectorOnly,
            ..Default::default()
        })
        .unwrap()
        .unwrap();
    let xml = roxmltree::Document::parse(&output.svg).unwrap();
    assert!(xml
        .descendants()
        .filter(|node| node.has_tag_name("clipPath"))
        .any(|clip| clip.children().any(|node| node.has_tag_name("rect")
            && node.attribute("width") == Some("40")
            && node.attribute("height") == Some("20"))));
}

#[test]
fn target_rejects_invalid_device_scaling_before_recording() {
    let target = SvgRenderTarget::new();
    for scaling in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::from_bits(1)] {
        let scene = RenderTargetSceneInfo::new(
            PixelSize::new(40, 20),
            scaling,
            CompositionTransparencyLevel::None,
        );
        assert!(target.try_create_drawing_context(&scene).is_err());
    }
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(0, 20),
        1.0,
        CompositionTransparencyLevel::None,
    );
    assert!(target.try_create_drawing_context(&scene).is_err());
}

#[test]
fn completion_exports_finalized_frames_and_never_publishes_disposed_frames() {
    let calls = Rc::new(std::cell::Cell::new(0));
    let recorded = calls.clone();
    let target = SvgRenderTarget::with_completion(move |document| {
        let output = document.export(&SvgExportOptions::default()).unwrap();
        assert_eq!(output.report.layers, 1);
        recorded.set(recorded.get() + 1);
    });
    assert!(target.prefers_full_frame_rendering());
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(40, 20),
        2.0,
        CompositionTransparencyLevel::None,
    );
    let (context, _) = target.try_create_drawing_context(&scene).unwrap();
    drop(context);
    assert_eq!(calls.get(), 1);
    assert_eq!(target.frame_count(), 1);
    let (context, _) = target.try_create_drawing_context(&scene).unwrap();
    target.dispose();
    drop(context);
    assert_eq!(calls.get(), 1);
    assert_eq!(target.frame_count(), 1);
}
