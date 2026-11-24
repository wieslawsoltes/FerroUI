use crate::xml::{self, InspectionStyle, XmlOptions};
use crate::{
    SvgCutawayMode, SvgDiagnostic, SvgDocument, SvgError, SvgExportOptions, SvgExportReport,
    SvgFidelity, SvgInspection, SvgLayer, SvgOutput, SvgTextMode,
};
use base64::Engine;
use ferroui_base::media::Color;
use ferroui_base::{Matrix, Rect, Vector};
use skia_safe as sk;
use std::fmt::Write;
use std::rc::Rc;

pub(crate) fn export(
    document: &SvgDocument,
    options: &SvgExportOptions,
) -> Result<SvgOutput, SvgError> {
    options.validate()?;
    let viewport = options
        .view_box
        .unwrap_or_else(|| Rect::from_size(document.size));
    let mut report = SvgExportReport::default();
    let mut annotation_font = None;
    if options.text == SvgTextMode::Editable {
        report.diagnostics.push(SvgDiagnostic { layer_id: None, message:
            "Editable text reconstructs Unicode from shaped glyph IDs and references installed fonts. Ligatures, contextual shaping, bidi and missing fonts may differ; use Outlines for portable appearance.".into() });
    }
    if options.precision.is_some() {
        report.diagnostics.push(SvgDiagnostic { layer_id: None, message:
            "Coordinate precision reduction is enabled; this is an explicitly lossy optimization.".into() });
    }

    let mut svg = String::with_capacity(4096);
    write!(svg, "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" version=\"1.1\" viewBox=\"{} {} {} {}\" overflow=\"hidden\"", viewport.x, viewport.y, viewport.width, viewport.height).unwrap();
    if !options.responsive {
        write!(
            svg,
            " width=\"{}\" height=\"{}\"",
            options.units.length(viewport.width),
            options.units.length(viewport.height)
        )
        .unwrap();
    }
    svg.push('>');
    if let Some(title) = &options.title {
        write!(svg, "<title>{}</title>", escape(title)).unwrap();
    }
    if let Some(description) = &options.description {
        write!(svg, "<desc>{}</desc>", escape(description)).unwrap();
    }
    svg.push_str("<metadata>FerroUI SVG export; layered Skia recording; 96 DPI logical coordinates</metadata>");
    if let Some(background) = options.background {
        write!(svg, "<rect id=\"ferro-internal-background\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" fill-opacity=\"{}\"/>", viewport.x, viewport.y, viewport.width, viewport.height, rgb(background), alpha(background)).unwrap();
    }
    if let Some(section) = &options.view.cutaway {
        let r = section.rect;
        svg.push_str(
            "<defs><clipPath id=\"ferro-internal-section\" clipPathUnits=\"userSpaceOnUse\">",
        );
        match section.mode {
            SvgCutawayMode::KeepInside => write_rect(&mut svg, r, ""),
            SvgCutawayMode::RemoveInside => {
                // Two closed contours with even-odd winding describe a window removed from the
                // viewport. This is ordinary SVG clipping, with no masks/filters required.
                write!(
                    svg,
                    "<path clip-rule=\"evenodd\" d=\"M{} {}H{}V{}H{}Z M{} {}H{}V{}H{}Z\"/>",
                    viewport.x,
                    viewport.y,
                    viewport.right(),
                    viewport.bottom(),
                    viewport.x,
                    r.x,
                    r.y,
                    r.right(),
                    r.bottom(),
                    r.x
                )
                .unwrap();
            }
        }
        svg.push_str("</clipPath></defs>");
    }

    let inspection = match &options.view.inspection {
        SvgInspection::Normal => InspectionStyle::Normal,
        SvgInspection::XRay {
            fill_opacity,
            stroke,
            stroke_width,
        } => InspectionStyle::XRay {
            fill_opacity: *fill_opacity,
            stroke: rgb(*stroke),
            stroke_width: *stroke_width,
            stroke_opacity: alpha(*stroke),
        },
        SvgInspection::Wireframe {
            stroke,
            stroke_width,
        } => InspectionStyle::Wireframe {
            stroke: rgb(*stroke),
            stroke_width: *stroke_width,
            stroke_opacity: alpha(*stroke),
        },
    };
    // Keep inspection annotations above all scene geometry. Emitting each annotation
    // beside its source layer lets later layers cover it in an exploded view.
    let mut guides = String::new();
    let mut labels = String::new();

    for (index, layer) in document.layers.iter().enumerate() {
        let properties = &layer.options;
        if (!properties.visible && !options.view.include_hidden)
            || !options
                .view
                .include
                .matches(&properties.id, &properties.tags)
            || options
                .view
                .exclude
                .as_ref()
                .is_some_and(|filter| filter.matches(&properties.id, &properties.tags))
        {
            continue;
        }
        let layer_inspection = if options
            .view
            .inspection_layers
            .matches(&properties.id, &properties.tags)
        {
            &inspection
        } else {
            &InspectionStyle::Normal
        };
        if !layer.audit.errors.is_empty() {
            return Err(SvgError::Recording {
                layer: properties.name.clone(),
                reasons: layer.audit.errors.clone(),
            });
        }
        let needs_raster = !layer.audit.raster_reasons.is_empty();
        if needs_raster && options.fidelity == SvgFidelity::VectorOnly {
            return Err(SvgError::VectorFidelity {
                layer: properties.name.clone(),
                reasons: layer.audit.raster_reasons.clone(),
            });
        }
        let raster = options.fidelity == SvgFidelity::Raster || needs_raster;
        report.layers += 1;
        report.draw_calls += layer.audit.draw_calls;
        let prefix = format!("r{index}-");
        let body = if raster {
            report.raster_layers += 1;
            if needs_raster {
                for reason in &layer.audit.raster_reasons {
                    report.diagnostics.push(SvgDiagnostic {
                        layer_id: Some(properties.id.clone()),
                        message: format!("Rasterized isolated layer: {reason}"),
                    });
                }
            }
            let mut body = String::new();
            if !matches!(layer_inspection, InspectionStyle::Wireframe { .. }) {
                let (png, hit) = cached_raster_layer(layer, options)?;
                report.raster_cache_hits += usize::from(hit);
                let data = base64::engine::general_purpose::STANDARD.encode(png);
                // Definitions at document scope are independent of the layer's transform,
                // opacity and clipping. Equal PNG payloads can therefore be shared safely.
                let image_id = format!("{prefix}raster");
                write!(svg,"<defs><image id=\"{image_id}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" xlink:href=\"data:image/png;base64,{data}\"/></defs>",layer.bounds.width,layer.bounds.height).unwrap();
                write!(
                    body,
                    "<use x=\"{}\" y=\"{}\" xlink:href=\"#{image_id}\"",
                    layer.bounds.x, layer.bounds.y
                )
                .unwrap();
                if let InspectionStyle::XRay { fill_opacity, .. } = layer_inspection {
                    write!(body, " opacity=\"{fill_opacity}\"").unwrap();
                }
                body.push_str("/>");
            }
            if !matches!(layer_inspection, InspectionStyle::Normal) {
                report.diagnostics.push(SvgDiagnostic { layer_id:Some(properties.id.clone()),message:
                    "Inspection draws the raster layer's recording bounds; internal pixel edges cannot become editable vector outlines. Wireframe suppresses its pixel image.".into() });
                let (color, width, opacity) = match layer_inspection {
                    InspectionStyle::XRay {
                        stroke,
                        stroke_width,
                        stroke_opacity,
                        ..
                    }
                    | InspectionStyle::Wireframe {
                        stroke,
                        stroke_width,
                        stroke_opacity,
                    } => (stroke, *stroke_width, *stroke_opacity),
                    _ => unreachable!(),
                };
                write_rect(
                    &mut body,
                    layer.bounds,
                    &format!(
                        " fill=\"none\" stroke=\"{color}\" stroke-opacity=\"{opacity}\" stroke-width=\"{width}\" vector-effect=\"non-scaling-stroke\""
                    ),
                );
            }
            body
        } else {
            report.vector_layers += 1;
            let (raw, hit) = vector_layer(layer, options)?;
            report.vector_cache_hits += usize::from(hit);
            let fragment = xml::skia_fragment(&raw, &prefix, layer_inspection)
                .map_err(|error| SvgError::Xml(error.to_string()))?;
            if layer.bounds.x != 0.0 || layer.bounds.y != 0.0 {
                format!(
                    "<g transform=\"translate({} {})\">{fragment}</g>",
                    layer.bounds.x, layer.bounds.y
                )
            } else {
                fragment
            }
        };

        let offset = options
            .view
            .exploded
            .as_ref()
            .map(|exploded| {
                Vector::new(
                    exploded.step.x * properties.depth,
                    exploded.step.y * properties.depth,
                )
            })
            .unwrap_or_default();
        if !offset.x.is_finite() || !offset.y.is_finite() {
            return Err(SvgError::InvalidOptions("exploded offset overflow".into()));
        }
        let transform = properties.transform * Matrix::create_translation_vector(offset);
        if ![
            transform.m11,
            transform.m12,
            transform.m21,
            transform.m22,
            transform.m31,
            transform.m32,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err(SvgError::InvalidOptions(
                "composed layer transform overflow".into(),
            ));
        }
        let transformed_bounds = layer.bounds.transform_to_aabb(transform);
        if !finite_rect(transformed_bounds) {
            return Err(SvgError::InvalidOptions(
                "transformed layer bounds overflow".into(),
            ));
        }
        let selected_section = options
            .view
            .cutaway
            .as_ref()
            .is_some_and(|section| section.layers.matches(&properties.id, &properties.tags));
        if selected_section {
            svg.push_str("<g clip-path=\"url(#ferro-internal-section)\">");
        }
        write!(
            svg,
            "<g id=\"ferro-{}\" data-name=\"{}\" data-depth=\"{}\" data-rendering=\"{}\"",
            properties.id,
            escape(&properties.name),
            properties.depth,
            if raster { "raster" } else { "vector" }
        )
        .unwrap();
        if options.designer_layers {
            write!(
                svg,
                " inkscape:groupmode=\"layer\" inkscape:label=\"{}\"",
                escape(&properties.name)
            )
            .unwrap();
        }
        if !transform.is_identity() {
            write!(svg, " transform=\"{}\"", matrix(transform)).unwrap();
        }
        if properties.opacity != 1.0 {
            write!(svg, " opacity=\"{}\"", properties.opacity).unwrap();
        }
        if !properties.tags.is_empty() {
            write!(svg, " data-tags=\"{}\"", escape(&properties.tags.join(" "))).unwrap();
        }
        for (key, value) in &properties.metadata {
            write!(svg, " data-meta-{key}=\"{}\"", escape(value)).unwrap();
        }
        svg.push('>');
        write!(svg, "<title>{}</title>", escape(&properties.name)).unwrap();
        if let Some(description) = &properties.description {
            write!(svg, "<desc>{}</desc>", escape(description)).unwrap();
        }
        svg.push_str(&body);
        if options.view.show_bounds {
            write_rect(
                &mut svg,
                layer.bounds,
                " fill=\"none\" stroke=\"#c43191\" stroke-width=\"0.75\" stroke-dasharray=\"4 3\" vector-effect=\"non-scaling-stroke\"",
            );
        }
        svg.push_str("</g>");
        if selected_section {
            svg.push_str("</g>");
        }
        if let Some(exploded) = &options.view.exploded {
            let bounds = layer.bounds.transform_to_aabb(properties.transform);
            if !finite_rect(bounds) {
                return Err(SvgError::InvalidOptions(
                    "exploded source bounds overflow".into(),
                ));
            }
            if exploded.show_connectors && (offset.x != 0.0 || offset.y != 0.0) {
                let (x, y) = (
                    bounds.x + bounds.width / 2.0,
                    bounds.y + bounds.height / 2.0,
                );
                write!(guides,"<path data-role=\"explosion-guide\" d=\"M{x} {y}L{} {}\" fill=\"none\" stroke=\"#76919b\" stroke-width=\"0.75\" stroke-dasharray=\"3 4\" vector-effect=\"non-scaling-stroke\"/>",x+offset.x,y+offset.y).unwrap();
            }
            if exploded.show_labels {
                let (x, y) = (transformed_bounds.x, transformed_bounds.y - 8.0);
                if !x.is_finite() || !y.is_finite() {
                    return Err(SvgError::InvalidOptions(
                        "annotation position overflow".into(),
                    ));
                }
                if options.text == SvgTextMode::Outlines {
                    if annotation_font.is_none() {
                        annotation_font = Some(LabelOutliner::new()?);
                    }
                    let outliner = annotation_font.as_mut().unwrap();
                    let path = outliner.outline(&properties.name)?;
                    if !properties.name.is_ascii() {
                        report.diagnostics.push(SvgDiagnostic { layer_id:Some(properties.id.clone()),message:
                            "Technical annotation labels use scalar glyph positioning. Record application-shaped text inside a layer for complex scripts and bidirectional labels.".into() });
                    }
                    write!(labels,"<g data-role=\"layer-label\" transform=\"translate({x} {y})\"><title>{}</title><path fill=\"#617886\" d=\"{path}\"/></g>",escape(&properties.name)).unwrap();
                } else {
                    write!(labels,"<text data-role=\"layer-label\" x=\"{x}\" y=\"{y}\" font-family=\"sans-serif\" font-size=\"12\" fill=\"#617886\">{}</text>",escape(&properties.name)).unwrap();
                }
            }
        }
    }
    svg.push_str(&guides);
    if let Some(section) = &options.view.cutaway {
        if section.show_boundary {
            write_rect(
                &mut svg,
                section.rect,
                " data-role=\"section-boundary\" fill=\"none\" stroke=\"#ef8b3b\" stroke-width=\"1.25\" stroke-dasharray=\"9 4 2 4\" vector-effect=\"non-scaling-stroke\"",
            );
        }
    }
    svg.push_str(&labels);
    svg.push_str("</svg>");
    report.unoptimized_bytes = svg.len();
    let optimized = xml::optimize(
        &svg,
        &XmlOptions {
            precision: options.precision,
            pretty: options.pretty,
            id_prefix: "r".into(),
            deduplicate: options.deduplicate,
        },
    )
    .map_err(|error| SvgError::Xml(error.to_string()))?;
    report.output_bytes = optimized.svg.len();
    report.definitions_deduplicated = optimized.stats.definitions_deduplicated;
    report.embedded_images = optimized.stats.image_elements as u64;
    Ok(SvgOutput {
        svg: optimized.svg,
        report,
    })
}

fn vector_layer(layer: &SvgLayer, options: &SvgExportOptions) -> Result<(Rc<str>, bool), SvgError> {
    use sk::svg::canvas::Flags;
    let outlined = options.text == SvgTextMode::Outlines;
    if options.cache_native_svg {
        if let Some((cached_outlined, cached_relative, data)) = &*layer.vector_cache.borrow() {
            if *cached_outlined == outlined && *cached_relative == options.relative_paths {
                return Ok((data.clone(), true));
            }
        }
    }
    let mut flags = Flags::NO_PRETTY_XML;
    if options.text == SvgTextMode::Outlines {
        flags |= Flags::CONVERT_TEXT_TO_PATHS;
    }
    if options.relative_paths {
        flags |= Flags::RELATIVE_PATH_ENCODING;
    }
    let canvas = sk::svg::Canvas::new(
        sk::Rect::from_wh(layer.bounds.width as f32, layer.bounds.height as f32),
        flags,
    );
    canvas.translate((-layer.bounds.x as f32, -layer.bounds.y as f32));
    canvas.draw_picture(&layer.picture, None, None);
    let data = canvas.end();
    let text =
        std::str::from_utf8(data.as_bytes()).map_err(|error| SvgError::Xml(error.to_string()))?;
    let data: Rc<str> = Rc::from(text);
    if options.cache_native_svg {
        *layer.vector_cache.borrow_mut() = Some((outlined, options.relative_paths, data.clone()));
    }
    Ok((data, false))
}

fn cached_raster_layer(
    layer: &SvgLayer,
    options: &SvgExportOptions,
) -> Result<(Rc<[u8]>, bool), SvgError> {
    // Check allocation limits even if an earlier, more permissive export populated the cache.
    let (width, height) = raster_size(layer.bounds, options)?;
    if options.cache_raster_images {
        if let Some((cached_width, cached_height, data)) = &*layer.raster_cache.borrow() {
            if *cached_width == width && *cached_height == height {
                return Ok((data.clone(), true));
            }
        }
    }
    let data: Rc<[u8]> = raster_layer(layer, options)?.into();
    if options.cache_raster_images {
        *layer.raster_cache.borrow_mut() = Some((width, height, data.clone()));
    }
    Ok((data, false))
}

pub(crate) fn raster_layer(
    layer: &SvgLayer,
    options: &SvgExportOptions,
) -> Result<Vec<u8>, SvgError> {
    let (width, height) = raster_size(layer.bounds, options)?;
    let scale = (
        (f64::from(width) / layer.bounds.width) as f32,
        (f64::from(height) / layer.bounds.height) as f32,
    );
    let origin = (-layer.bounds.x as f32, -layer.bounds.y as f32);
    // An allocation may be small even when its logical-to-device mapping
    // overflows Skia's scalar range (for example a subnormal logical width).
    // Check the composed translation too, before allocating or replaying.
    if scale.0 <= 0.0
        || scale.1 <= 0.0
        || ![scale.0, scale.1, origin.0 * scale.0, origin.1 * scale.1]
            .iter()
            .all(|value| value.is_finite())
    {
        return Err(SvgError::InvalidOptions(
            "raster bounds and scale must produce a finite Skia device transform".into(),
        ));
    }
    let mut surface =
        sk::surfaces::raster_n32_premul((width, height)).ok_or(SvgError::RasterAllocation)?;
    let canvas = surface.canvas();
    canvas.clear(sk::Color::TRANSPARENT);
    // Map fractional logical dimensions onto the complete integer pixel grid. The inverse
    // mapping is the image element's exact width/height, so ceil does not stretch the scene.
    canvas.scale(scale);
    canvas.translate(origin);
    canvas.draw_picture(&layer.picture, None, None);
    encode_surface(&mut surface)
}

pub(crate) fn encode_surface(surface: &mut sk::Surface) -> Result<Vec<u8>, SvgError> {
    let image = surface.image_snapshot();
    let pixels = image.peek_pixels().ok_or(SvgError::ImageEncoding)?;
    let mut png = Vec::new();
    let mut options = sk::png_encoder::Options::default();
    options.z_lib_level = 6;
    if !sk::png_encoder::encode(&pixels, &mut png, &options) {
        return Err(SvgError::ImageEncoding);
    }
    Ok(png)
}

pub(crate) fn raster_size(
    bounds: Rect,
    options: &SvgExportOptions,
) -> Result<(i32, i32), SvgError> {
    let width = (bounds.width * options.raster_scale).ceil() as u64;
    let height = (bounds.height * options.raster_scale).ceil() as u64;
    if width == 0
        || height == 0
        || width > u64::from(options.max_raster_dimension.min(i32::MAX as u32))
        || height > u64::from(options.max_raster_dimension.min(i32::MAX as u32))
        || width
            .checked_mul(height)
            .is_none_or(|pixels| pixels > options.max_raster_pixels)
    {
        return Err(SvgError::RasterLimit {
            width,
            height,
            max_pixels: options.max_raster_pixels,
        });
    }
    Ok((width as i32, height as i32))
}

fn write_rect(output: &mut String, rect: Rect, attributes: &str) {
    write!(
        output,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{attributes}/>",
        rect.x, rect.y, rect.width, rect.height
    )
    .unwrap();
}

fn matrix(m: Matrix) -> String {
    format!(
        "matrix({} {} {} {} {} {})",
        m.m11, m.m12, m.m21, m.m22, m.m31, m.m32
    )
}
fn rgb(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}
fn alpha(color: Color) -> f64 {
    f64::from(color.a) / 255.0
}
fn finite_rect(rect: Rect) -> bool {
    [
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        rect.right(),
        rect.bottom(),
    ]
    .iter()
    .all(|value| value.is_finite())
}
fn escape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            '\r' => result.push_str("&#13;"),
            '\n' => result.push_str("&#10;"),
            '\t' => result.push_str("&#9;"),
            c => result.push(c),
        }
    }
    result
}

/// Annotation-only outlines, independent of installed fonts in the consuming
/// design application. Application text retains FerroUI's normal shaping path.
struct LabelOutliner {
    font: sk::Font,
    paths: std::collections::HashMap<u16, sk::Path>,
}

impl LabelOutliner {
    fn new() -> Result<Self, SvgError> {
        let manager = sk::FontMgr::new();
        let face = ["sans-serif","Inter","Arial","DejaVu Sans"].iter()
            .find_map(|family|manager.match_family_style(family,sk::FontStyle::normal()))
            .or_else(||manager.legacy_make_typeface(None,sk::FontStyle::normal()))
            .ok_or_else(||SvgError::InvalidOptions("outlined inspection labels require an installed sans-serif font; install one or disable exploded labels".into()))?;
        Ok(Self {
            font: sk::Font::new(face, 12.0),
            paths: std::collections::HashMap::new(),
        })
    }

    fn outline(&mut self, label: &str) -> Result<String, SvgError> {
        // SVG's default text whitespace policy collapses runs to one space.
        let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
        let glyphs = self.font.str_to_glyphs_vec(&label);
        if glyphs.contains(&0) {
            return Err(SvgError::InvalidOptions(format!(
                "the annotation font does not cover layer label '{label}'; supply the label as application text or disable exploded labels"
            )));
        }
        let mut positions = vec![0.0; glyphs.len()];
        self.font.get_x_pos(&glyphs, &mut positions, None);
        let mut path = sk::PathBuilder::new();
        for ((glyph, position), character) in glyphs.into_iter().zip(positions).zip(label.chars()) {
            if !position.is_finite() {
                return Err(SvgError::InvalidOptions(
                    "annotation glyph positions overflow".into(),
                ));
            }
            if character.is_whitespace() {
                continue;
            }
            let outline = match self.paths.entry(glyph) {
                std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(self.font.get_path(glyph).ok_or_else(|| {
                        SvgError::InvalidOptions(format!(
                            "the annotation font has no vector outline for a glyph in '{label}'"
                        ))
                    })?)
                }
            };
            path.add_path_with_offset(outline, (position, 0.0), None);
        }
        Ok(path.detach().to_svg())
    }
}

#[cfg(test)]
#[path = "render_regression_tests.rs"]
mod regression_tests;
