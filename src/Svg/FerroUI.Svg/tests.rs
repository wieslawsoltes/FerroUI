use super::*;
use base64::Engine;
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadows, Colors};
use ferroui_base::platform::{IDrawingContextImpl, IRenderTarget, RenderTargetSceneInfo};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Size, Vector};
use std::io::Read;

fn rectangle(context: &mut dyn IDrawingContextImpl, color: ferroui_base::media::Color, rect: Rect) {
    context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(color)),
        None,
        RoundedRect::from(rect),
        &BoxShadows::default(),
    );
}

fn scene() -> SvgDocument {
    let mut document = SvgDocument::new(Size::new(100.0, 80.0)).unwrap();
    document
        .record_layer(
            SvgLayerOptions::new("base", "Base & chassis").with_tags(["structure"]),
            |context| {
                rectangle(context, Colors::RED, Rect::new(5.0, 5.0, 50.0, 40.0));
                context.push_clip(Rect::new(10.0, 10.0, 40.0, 30.0));
                context.draw_ellipse(
                    Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
                    None,
                    Rect::new(20.0, 20.0, 30.0, 30.0),
                );
                context.pop_clip();
            },
        )
        .unwrap();
    document
        .record_layer(
            SvgLayerOptions::new("glass", "Cover")
                .with_tags(["shell"])
                .with_depth(2.0),
            |context| {
                let pen = ImmutablePen::from_uint32(0xff102030, 2.0);
                context.draw_line(Some(&pen), Point::new(0.0, 0.0), Point::new(80.0, 60.0));
            },
        )
        .unwrap();
    document
}

#[test]
fn supported_layers_remain_vectors_with_stable_ids_and_self_contained_references() {
    let options = SvgExportOptions {
        fidelity: SvgFidelity::VectorOnly,
        ..Default::default()
    };
    let first = scene().export(&options).unwrap();
    let second = scene().export(&options).unwrap();
    assert_eq!(
        first.svg, second.svg,
        "Skia's process-global clip IDs must not leak into exports"
    );
    assert_eq!(first.report.vector_layers, 2);
    assert_eq!(first.report.raster_layers, 0);
    let xml = roxmltree::Document::parse(first.as_str()).unwrap();
    assert!(xml
        .descendants()
        .any(|n| n.attribute("id") == Some("ferro-base")));
    assert!(!xml.descendants().any(|n| n.has_tag_name("image")));
    let ids: std::collections::HashSet<_> = xml
        .descendants()
        .filter_map(|n| n.attribute("id"))
        .collect();
    for node in xml.descendants() {
        for attribute in node.attributes() {
            if let Some(id) = attribute
                .value()
                .strip_prefix("url(#")
                .and_then(|v| v.strip_suffix(')'))
            {
                assert!(ids.contains(id), "dangling resource reference {id}");
            }
        }
    }
}

#[test]
fn destination_dependent_blend_rasterizes_the_whole_island_and_strict_mode_refuses() {
    let mut document = SvgDocument::new(Size::new(20.0, 20.0)).unwrap();
    document
        .record_layer(SvgLayerOptions::new("blend", "Blend test"), |context| {
            rectangle(context, Colors::RED, Rect::new(0.0, 0.0, 20.0, 20.0));
            assert!(context.try_push_compositing_layer(
                BitmapBlendingMode::DestinationOut,
                1.0,
                Rect::new(0.0, 0.0, 20.0, 20.0)
            ));
            rectangle(context, Colors::BLUE, Rect::new(5.0, 5.0, 10.0, 10.0));
            context.pop_compositing_layer();
        })
        .unwrap();
    assert!(matches!(
        document.export(&SvgExportOptions {
            fidelity: SvgFidelity::VectorOnly,
            ..Default::default()
        }),
        Err(SvgError::VectorFidelity { .. })
    ));
    let output = document
        .export(&SvgExportOptions {
            raster_scale: 1.0,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(output.report.raster_layers, 1);
    let xml = roxmltree::Document::parse(output.as_str()).unwrap();
    let image = xml.descendants().find(|n| n.has_tag_name("image")).unwrap();
    let data = image
        .attribute(("http://www.w3.org/1999/xlink", "href"))
        .unwrap()
        .strip_prefix("data:image/png;base64,")
        .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .unwrap();
    let decoded = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&bytes)).unwrap();
    let raster = decoded
        .make_raster_image(None, skia_safe::image::CachingHint::Allow)
        .unwrap();
    let pixels = raster.peek_pixels().unwrap();
    assert_eq!(pixels.get_color((2, 2)), skia_safe::Color::RED);
    assert_eq!(
        pixels.get_color((10, 10)).a(),
        0,
        "destination-out must see earlier content"
    );
}

#[test]
fn cutaway_explosion_and_selection_do_not_rerecord_or_mutate_the_document() {
    let document = scene();
    let normal = document.export(&SvgExportOptions::default()).unwrap().svg;
    let options = SvgExportOptions {
        view: SvgView {
            exploded: Some(SvgExplodedView {
                step: Vector::new(10.0, -5.0),
                show_labels: false,
                show_connectors: false,
            }),
            cutaway: Some(SvgCutaway {
                rect: Rect::new(10.0, 10.0, 15.0, 15.0),
                layers: SvgLayerFilter::tag("shell"),
                mode: SvgCutawayMode::RemoveInside,
                show_boundary: true,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let output = document.export(&options).unwrap();
    assert!(output.svg.contains("clip-rule=\"evenodd\""));
    assert!(output.svg.contains("matrix(1 0 0 1 20 -10)"));
    assert_eq!(
        normal,
        document.export(&SvgExportOptions::default()).unwrap().svg
    );
    let only_shell = document
        .export(&SvgExportOptions {
            view: SvgView {
                include: SvgLayerFilter::tag("shell"),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    assert_eq!(only_shell.report.layers, 1);
    assert!(!only_shell.svg.contains("ferro-base"));
}

#[test]
fn svgz_is_deterministic_and_round_trips() {
    let output = scene().export(&SvgExportOptions::compact()).unwrap();
    let (mut first, mut second) = (Vec::new(), Vec::new());
    output.write_svgz(&mut first, 6).unwrap();
    output.write_svgz(&mut second, 6).unwrap();
    assert_eq!(first, second);
    let mut decoded = String::new();
    flate2::read::GzDecoder::new(first.as_slice())
        .read_to_string(&mut decoded)
        .unwrap();
    assert_eq!(decoded, output.svg);
}

#[test]
fn finite_dimensions_and_raster_budgets_are_checked_before_allocating() {
    assert!(SvgDocument::new(Size::new(f64::NAN, 10.0)).is_err());
    assert!(SvgDocument::new(Size::new(0.0, 10.0)).is_err());
    let mut oversized = SvgDocument::new(Size::new(10.0, 10.0)).unwrap();
    let limit = f64::from(f32::MAX);
    // Finite, representable endpoints do not imply a representable width.
    assert!(oversized
        .record_layer(
            SvgLayerOptions::new("wide", "Wide").with_bounds(Rect::new(
                -limit,
                0.0,
                2.0 * limit,
                10.0
            )),
            |_| panic!("invalid recording bounds must be rejected before the callback"),
        )
        .is_err());
    let document = scene();
    let options = SvgExportOptions {
        fidelity: SvgFidelity::Raster,
        max_raster_pixels: 10,
        ..Default::default()
    };
    assert!(matches!(
        document.export(&options),
        Err(SvgError::RasterLimit { .. })
    ));
    assert!(document
        .export(&SvgExportOptions {
            raster_scale: f64::INFINITY,
            ..Default::default()
        })
        .is_err());
    assert!(document
        .export(&SvgExportOptions {
            precision: Some(13),
            ..Default::default()
        })
        .is_err());
}

#[test]
fn bounds_with_nonzero_origin_preserve_absolute_transform_semantics() {
    let mut document = SvgDocument::new(Size::new(80.0, 80.0)).unwrap();
    document
        .record_layer(
            SvgLayerOptions::new("offset", "Offset")
                .with_bounds(Rect::new(-10.0, 20.0, 30.0, 30.0)),
            |context| {
                context.set_transform(Matrix::IDENTITY);
                rectangle(context, Colors::RED, Rect::new(-10.0, 20.0, 15.0, 15.0));
            },
        )
        .unwrap();
    let png = crate::render::raster_layer(
        &document.layers[0],
        &SvgExportOptions {
            raster_scale: 1.0,
            ..Default::default()
        },
    )
    .unwrap();
    let decoded = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&png)).unwrap();
    let raster = decoded
        .make_raster_image(None, skia_safe::image::CachingHint::Allow)
        .unwrap();
    let pixels = raster.peek_pixels().unwrap();
    assert_eq!(pixels.get_color((2, 2)), skia_safe::Color::RED);
    assert_eq!(pixels.get_color((25, 25)).a(), 0);
    let vector = document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::VectorOnly,
            ..Default::default()
        })
        .unwrap();
    assert!(vector.svg.contains("translate(-10 20)"));
}

#[test]
fn duplicate_ids_invalid_metadata_and_unbalanced_callbacks_are_errors() {
    let mut document = scene();
    assert!(document
        .record_layer(SvgLayerOptions::new("base", "duplicate"), |_| {})
        .is_err());
    assert!(document
        .record_layer(SvgLayerOptions::new("bad id", "invalid"), |_| {})
        .is_err());
    assert!(document
        .record_layer(SvgLayerOptions::new("internal-section", "reserved"), |_| {})
        .is_err());
    assert!(document
        .record_layer(SvgLayerOptions::new("unbalanced", "bad state"), |c| c
            .push_opacity(0.5, None))
        .is_err());
    assert_eq!(document.layers().len(), 2);
}

#[test]
fn target_finalizes_frames_on_dispose_or_drop_and_does_not_retain_pixels() {
    let target = SvgRenderTarget::new();
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(40, 20),
        2.0,
        CompositionTransparencyLevel::None,
    );
    assert!(target
        .export_last(&SvgExportOptions::default())
        .unwrap()
        .is_none());
    let (mut context, properties) = target.try_create_drawing_context(&scene).unwrap();
    assert!(!properties.previous_frame_is_retained);
    assert!(target.try_create_drawing_context(&scene).is_err());
    rectangle(&mut *context, Colors::RED, Rect::new(0.0, 0.0, 10.0, 10.0));
    context.dispose();
    drop(context);
    assert_eq!(target.frame_count(), 1);
    let output = target
        .export_last(&SvgExportOptions::default())
        .unwrap()
        .unwrap();
    assert!(output.svg.contains("viewBox=\"0 0 20 10\""));
    let (context, _) = target.try_create_drawing_context(&scene).unwrap();
    drop(context);
    assert_eq!(target.frame_count(), 2);
    assert_eq!(
        target
            .export_last(&SvgExportOptions::default())
            .unwrap()
            .unwrap()
            .report
            .draw_calls,
        0
    );
    target.dispose();
    assert!(target.try_create_drawing_context(&scene).is_err());
}
