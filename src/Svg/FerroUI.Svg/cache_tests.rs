use crate::*;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadows, Color, Colors};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Point, Rect, Size, Vector};
use std::rc::Rc;

fn add_layer(document: &mut SvgDocument, id: &str) -> SvgLayerId {
    document
        .record_layer(
            SvgLayerOptions::new(id, format!("Layer {id}"))
                .with_bounds(Rect::new(8.0, 4.0, 48.0, 36.0))
                .with_depth(2.0),
            |context| {
                let brush = ImmutableSolidColorBrush::new(Colors::RED);
                let pen = ImmutablePen::with_brush(Some(Rc::new(brush.clone())), 1.5);
                context.push_clip(Rect::new(10.0, 6.0, 28.0, 24.0));
                context.draw_rectangle(
                    Some(&brush),
                    None,
                    Rect::new(10.125, 6.25, 24.5, 16.75).into(),
                    &BoxShadows::default(),
                );
                context.draw_line(Some(&pen), Point::new(12.0, 8.0), Point::new(34.0, 28.0));
                context.pop_clip();
            },
        )
        .unwrap()
}

fn document() -> SvgDocument {
    let mut document = SvgDocument::new(Size::new(128.0, 80.0)).unwrap();
    add_layer(&mut document, "shape");
    document
}

fn options() -> SvgExportOptions {
    SvgExportOptions {
        fidelity: SvgFidelity::VectorOnly,
        ..Default::default()
    }
}

fn export(
    document: &SvgDocument,
    options: &SvgExportOptions,
    expected_hits: (usize, usize),
) -> SvgOutput {
    let output = document.export(options).unwrap();
    assert_eq!(
        expected_hits,
        (
            output.report.vector_cache_hits,
            output.report.prepared_vector_cache_hits
        ),
        "inspection {:?}",
        options.view.inspection
    );
    assert_eq!(0, output.report.raster_layers);
    output
}

fn assert_uncached_matches(document: &SvgDocument, options: &SvgExportOptions, output: &SvgOutput) {
    let uncached = export(
        document,
        &SvgExportOptions {
            cache_native_svg: false,
            ..options.clone()
        },
        (0, 0),
    );
    assert_eq!(output.svg, uncached.svg);
    assert_eq!(output.report.draw_calls, uncached.report.draw_calls);
    assert_eq!(
        output.report.embedded_images,
        uncached.report.embedded_images
    );
    assert_eq!(output.report.diagnostics, uncached.report.diagnostics);
}

#[test]
fn prepared_fragments_include_every_effective_inspection_parameter() {
    let document = document();
    let blue = Color::from_argb(255, 40, 170, 220);
    let red = Color::from_argb(255, 220, 40, 70);
    let translucent = Color::from_argb(128, 220, 40, 70);
    let styles = [
        SvgInspection::Normal,
        SvgInspection::XRay {
            fill_opacity: 0.1,
            stroke: blue,
            stroke_width: 0.8,
        },
        SvgInspection::XRay {
            fill_opacity: 0.2,
            stroke: blue,
            stroke_width: 0.8,
        },
        SvgInspection::XRay {
            fill_opacity: 0.2,
            stroke: red,
            stroke_width: 0.8,
        },
        SvgInspection::XRay {
            fill_opacity: 0.2,
            stroke: translucent,
            stroke_width: 0.8,
        },
        SvgInspection::XRay {
            fill_opacity: 0.2,
            stroke: translucent,
            stroke_width: 1.4,
        },
        SvgInspection::Wireframe {
            stroke: translucent,
            stroke_width: 1.4,
        },
        SvgInspection::Wireframe {
            stroke: blue,
            stroke_width: 1.4,
        },
        SvgInspection::Wireframe {
            stroke: Color::from_argb(128, 40, 170, 220),
            stroke_width: 1.4,
        },
        SvgInspection::Wireframe {
            stroke: Color::from_argb(128, 40, 170, 220),
            stroke_width: 0.8,
        },
        SvgInspection::Normal,
    ];
    for (index, inspection) in styles.into_iter().enumerate() {
        let options = SvgExportOptions {
            view: SvgView {
                inspection,
                ..Default::default()
            },
            ..options()
        };
        let first = export(&document, &options, (usize::from(index != 0), 0));
        let warm = export(&document, &options, (1, 1));
        assert_eq!(first.svg, warm.svg);
        assert_uncached_matches(&document, &options, &warm);
        export(&document, &options, (1, 1));
    }
}

#[test]
fn prepared_fragment_float_keys_preserve_negative_zero_serialization() {
    let document = document();
    let mut previous = None;
    for value in [0.0, -0.0, 0.0] {
        let options = SvgExportOptions {
            view: SvgView {
                inspection: SvgInspection::XRay {
                    fill_opacity: value,
                    stroke: Colors::BLUE,
                    stroke_width: 1.0,
                },
                ..Default::default()
            },
            ..options()
        };
        let output = document.export(&options).unwrap();
        assert_eq!(0, output.report.prepared_vector_cache_hits);
        assert!(output.svg.contains(&format!("fill-opacity=\"{value}\"")));
        if let Some(previous) = previous {
            assert_ne!(previous, output.svg);
        }
        let warm = export(&document, &options, (1, 1));
        assert_eq!(output.svg, warm.svg);
        assert_uncached_matches(&document, &options, &warm);
        previous = Some(output.svg);
    }
    // Stroke widths have a different contract: both signed zeroes remain
    // invalid even when a valid fragment is already cached.
    for stroke_width in [0.0, -0.0] {
        assert!(document
            .export(&SvgExportOptions {
                view: SvgView {
                    inspection: SvgInspection::Wireframe {
                        stroke: Colors::BLUE,
                        stroke_width
                    },
                    ..Default::default()
                },
                ..options()
            })
            .is_err());
    }
}

#[test]
fn native_writer_flag_changes_replace_both_cache_tiers() {
    let document = document();
    for (text, relative_paths) in [
        (SvgTextMode::Outlines, false),
        (SvgTextMode::Outlines, true),
        (SvgTextMode::Editable, true),
        (SvgTextMode::Editable, false),
        (SvgTextMode::Outlines, false),
    ] {
        let options = SvgExportOptions {
            text,
            relative_paths,
            ..options()
        };
        let first = export(&document, &options, (0, 0));
        let warm = export(&document, &options, (1, 1));
        assert_eq!(first.svg, warm.svg);
        assert_uncached_matches(&document, &options, &warm);
        assert_eq!(
            text == SvgTextMode::Editable,
            warm.report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("Unicode")),
        );
    }
}

#[test]
fn prepared_fragments_use_the_inspection_applied_to_each_layer() {
    let mut document = document();
    add_layer(&mut document, "second");
    export(&document, &options(), (0, 0));
    let mut options = SvgExportOptions {
        view: SvgView {
            inspection: SvgInspection::x_ray(),
            inspection_layers: SvgLayerFilter::id("shape"),
            ..Default::default()
        },
        ..options()
    };
    export(&document, &options, (2, 1));
    export(&document, &options, (2, 2));
    options.view.inspection_layers = SvgLayerFilter::id("second");
    export(&document, &options, (2, 0));
    options.view.inspection_layers = SvgLayerFilter::all();
    let output = export(&document, &options, (2, 1));
    assert_uncached_matches(&document, &options, &output);
}

#[test]
fn removing_a_layer_invalidates_changed_resource_prefixes_without_collisions() {
    let mut document = SvgDocument::new(Size::new(128.0, 80.0)).unwrap();
    let first = add_layer(&mut document, "first");
    add_layer(&mut document, "kept");
    export(&document, &options(), (0, 0));
    export(&document, &options(), (2, 2));
    // Filtering preserves original indices and can retain the prepared prefix.
    export(
        &document,
        &SvgExportOptions {
            view: SvgView {
                include: SvgLayerFilter::id("kept"),
                ..Default::default()
            },
            ..options()
        },
        (1, 1),
    );
    assert!(document.remove_layer(&first));
    add_layer(&mut document, "new");
    // The retained layer moves from prefix r1- to r0-. Reusing its old
    // prepared fragment would collide with the new layer's r1- resources.
    let changed = export(&document, &options(), (1, 0));
    assert_uncached_matches(&document, &options(), &changed);
    let warm = export(&document, &options(), (2, 2));
    assert_eq!(changed.svg, warm.svg);
}

#[test]
fn disabling_and_clearing_vector_caches_controls_both_tiers() {
    let document = document();
    let disabled = SvgExportOptions {
        cache_native_svg: false,
        ..options()
    };
    export(&document, &disabled, (0, 0));
    export(&document, &disabled, (0, 0));
    export(&document, &options(), (0, 0));
    let original = export(&document, &options(), (1, 1));
    let other = SvgExportOptions {
        view: SvgView {
            inspection: SvgInspection::x_ray(),
            ..Default::default()
        },
        ..disabled
    };
    export(&document, &other, (0, 0));
    export(&document, &other, (0, 0));
    // Disabled exports neither read nor overwrite the retained native/style slots.
    assert_eq!(original.svg, export(&document, &options(), (1, 1)).svg);
    document.clear_export_cache();
    assert_eq!(original.svg, export(&document, &options(), (0, 0)).svg);
    export(&document, &options(), (1, 1));
}

#[test]
fn warm_fragments_still_apply_all_outer_document_and_view_options() {
    let document = document();
    let original = export(&document, &options(), (0, 0));
    let variants = [
        SvgExportOptions {
            title: Some("Updated & inspected".into()),
            description: Some("A changed description <with> XML characters".into()),
            ..options()
        },
        SvgExportOptions {
            view_box: Some(Rect::new(3.125, 4.25, 60.5, 40.25)),
            ..options()
        },
        SvgExportOptions {
            units: SvgUnits::Points,
            ..options()
        },
        SvgExportOptions {
            responsive: true,
            ..options()
        },
        SvgExportOptions {
            pretty: true,
            precision: Some(2),
            deduplicate: false,
            ..options()
        },
        SvgExportOptions {
            designer_layers: false,
            background: Some(Color::from_argb(128, 230, 240, 250)),
            view: SvgView {
                show_bounds: true,
                ..Default::default()
            },
            ..options()
        },
        SvgExportOptions {
            view: SvgView {
                cutaway: Some(SvgCutaway {
                    rect: Rect::new(12.0, 8.0, 16.0, 12.0),
                    layers: SvgLayerFilter::all(),
                    mode: SvgCutawayMode::RemoveInside,
                    show_boundary: true,
                }),
                exploded: Some(SvgExplodedView {
                    step: Vector::new(6.0, -3.0),
                    show_connectors: true,
                    show_labels: false,
                }),
                ..Default::default()
            },
            ..options()
        },
    ];
    for options in variants {
        let output = export(&document, &options, (1, 1));
        assert_ne!(original.svg, output.svg);
        assert_uncached_matches(&document, &options, &output);
    }
}

#[test]
fn a_single_prepared_slot_is_bounded_across_repeated_view_switches() {
    let document = document();
    let views = [
        SvgView::default(),
        SvgView {
            inspection: SvgInspection::x_ray(),
            ..Default::default()
        },
        SvgView {
            inspection: SvgInspection::wireframe(),
            ..Default::default()
        },
        SvgView {
            cutaway: Some(SvgCutaway {
                rect: Rect::new(12.0, 8.0, 16.0, 12.0),
                layers: SvgLayerFilter::all(),
                mode: SvgCutawayMode::KeepInside,
                show_boundary: false,
            }),
            ..Default::default()
        },
        SvgView {
            exploded: Some(SvgExplodedView {
                show_labels: false,
                ..Default::default()
            }),
            ..Default::default()
        },
    ];
    for view in &views {
        document
            .export(&SvgExportOptions {
                view: view.clone(),
                ..options()
            })
            .unwrap();
    }
    let mut hits = 0;
    for (index, view) in views.iter().enumerate() {
        let output = export(
            &document,
            &SvgExportOptions {
                view: view.clone(),
                ..options()
            },
            (1, usize::from(index == 0 || index == 4)),
        );
        hits += output.report.prepared_vector_cache_hits;
    }
    assert_eq!(2, hits);
}

#[test]
fn warm_vector_fragments_do_not_bypass_validation_or_raster_policy() {
    let document = document();
    export(&document, &options(), (0, 0));
    export(&document, &options(), (1, 1));
    for invalid in [
        SvgExportOptions {
            precision: Some(13),
            ..options()
        },
        SvgExportOptions {
            view: SvgView {
                inspection: SvgInspection::XRay {
                    fill_opacity: f64::NAN,
                    stroke: Colors::BLUE,
                    stroke_width: 1.0,
                },
                ..Default::default()
            },
            ..options()
        },
        SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            max_raster_pixels: 1,
            ..options()
        },
    ] {
        assert!(document.export(&invalid).is_err());
    }
    let raster = document
        .export(&SvgExportOptions {
            fidelity: SvgFidelity::Raster,
            raster_scale: 1.0,
            ..options()
        })
        .unwrap();
    assert_eq!(1, raster.report.raster_layers);
    assert_eq!(0, raster.report.vector_cache_hits);
    assert_eq!(0, raster.report.prepared_vector_cache_hits);
    export(&document, &options(), (1, 1));
}
