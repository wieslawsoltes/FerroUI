//! Native text artwork in ordinary public FerroUI XAML paths. The complete
//! result is prepared before appending anything; unsupported text never falls
//! back to a differently laid-out TextBlock or a partial set of glyphs.
use super::*;
use crate::renderer::text_export::{self as native, SolidPaint, TextArtwork};
use ferroui_base::media::{GeometryPath, GeometryPathSegment, PenLineCap, PenLineJoin};
use ferroui_base::Matrix;
use std::fmt::Write;

fn data(path: &GeometryPath) -> String {
    let mut output = match path.fill_rule {
        ferroui_base::media::FillRule::EvenOdd => "F0 ",
        ferroui_base::media::FillRule::NonZero => "F1 ",
    }.to_string();
    for segment in &path.segments {
        match segment {
            GeometryPathSegment::MoveTo(point) => { let _ = write!(output, "M{},{} ", point.x, point.y); }
            GeometryPathSegment::LineTo(point) => { let _ = write!(output, "L{},{} ", point.x, point.y); }
            GeometryPathSegment::QuadraticTo { control, point } => {
                let _ = write!(output, "Q{},{} {},{} ", control.x, control.y, point.x, point.y);
            }
            GeometryPathSegment::CubicTo { control1, control2, point } => {
                let _ = write!(output, "C{},{} {},{} {},{} ", control1.x, control1.y, control2.x, control2.y, point.x, point.y);
            }
            GeometryPathSegment::Close => output.push_str("Z "),
        }
    }
    output
}

fn matrix(matrix: Matrix) -> String {
    format!("{},{},{},{},{},{}", matrix.m11, matrix.m12, matrix.m21, matrix.m22, matrix.m31, matrix.m32)
}

fn brush(property: &str, paint: SolidPaint, depth: usize, output: &mut String) {
    let pad = "  ".repeat(depth);
    let color = paint.color;
    let _ = writeln!(output, "{pad}<Path.{property}><SolidColorBrush Color=\"#{:02X}{:02X}{:02X}{:02X}\" Opacity=\"{}\"/></Path.{property}>",
        color.a, color.r, color.g, color.b, paint.opacity);
}

fn write_artwork(node: &Node, artwork: &TextArtwork, depth: usize, output: &mut String) {
    let pad = "  ".repeat(depth);
    let transform = node.local_transform();
    let _ = writeln!(output, "{pad}<Canvas Width=\"{}\" Height=\"{}\" IsVisible=\"{}\" UseLayoutRounding=\"False\" ClipToBounds=\"False\" RenderTransformOrigin=\"0%,0%\">",
        node.width, node.height, if node.visible { "True" } else { "False" });
    let _ = writeln!(output, "{pad}  <Canvas.RenderTransform><MatrixTransform Matrix=\"{},{},{},{},{},{}\"/></Canvas.RenderTransform>",
        transform.a, transform.b, transform.c, transform.d, transform.e, transform.f);
    if node.opacity != 1.0 {
        // Composite the complete glyph/decoration/fill stack first. Applying
        // layer opacity to each Path would change overlap alpha and colors.
        let _ = writeln!(output, "{pad}  <Canvas.OpacityMask><SolidColorBrush Color=\"#FFFFFFFF\" Opacity=\"{}\"/></Canvas.OpacityMask>", node.opacity);
    }
    for operation in &artwork.operations {
        // These are solid paints. A future gradient writer must use
        // operation.brush_bounds, not the path's (different) ink bounds.
        debug_assert!([operation.brush_bounds.x, operation.brush_bounds.y,
            operation.brush_bounds.width, operation.brush_bounds.height].into_iter().all(f64::is_finite));
        let mut attributes = format!("Data=\"{}\" Stretch=\"None\" UseLayoutRounding=\"False\" RenderTransformOrigin=\"0%,0%\"", data(&operation.path));
        if let Some(stroke) = &operation.stroke {
            let cap = match stroke.cap { PenLineCap::Flat => "Flat", PenLineCap::Round => "Round", PenLineCap::Square => "Square" };
            let join = match stroke.join { PenLineJoin::Miter => "Miter", PenLineJoin::Round => "Round", PenLineJoin::Bevel => "Bevel" };
            let _ = write!(attributes, " StrokeThickness=\"{}\" StrokeLineCap=\"{cap}\" StrokeJoin=\"{join}\" StrokeMiterLimit=\"{}\" StrokeDashOffset=\"{}\"",
                stroke.thickness, stroke.miter_limit, stroke.dash_offset);
            if let Some(dashes) = &stroke.dashes {
                if !dashes.is_empty() {
                    let _ = write!(attributes, " StrokeDashArray=\"{}\"", dashes.iter().map(f64::to_string).collect::<Vec<_>>().join(","));
                }
            }
        }
        let _ = writeln!(output, "{pad}  <Path {attributes}>");
        let _ = writeln!(output, "{pad}    <Path.RenderTransform><MatrixTransform Matrix=\"{}\"/></Path.RenderTransform>", matrix(operation.transform));
        if let Some(fill) = operation.fill { brush("Fill", fill, depth + 2, output); }
        if let Some(stroke) = &operation.stroke { brush("Stroke", stroke.paint, depth + 2, output); }
        let _ = writeln!(output, "{pad}  </Path>");
    }
    let _ = writeln!(output, "{pad}</Canvas>");
}

/// Append one complete text layer or return a clear error without changing the
/// caller's buffer. The enclosing exporter propagates this error before any
/// download, making a mixed selection atomic too.
pub(super) fn write_node(node: &Node, depth: usize, output: &mut String) -> Result<(), String> {
    let artwork = native::prepare(node)?;
    let transform = node.local_transform();
    if ![node.width, node.height, node.opacity, transform.a, transform.b, transform.c,
        transform.d, transform.e, transform.f].into_iter().all(f64::is_finite)
        || node.width < 0.0 || node.height < 0.0 || !(0.0..=1.0).contains(&node.opacity) {
        return Err(format!("Text layer ‘{}’ has invalid dimensions, opacity or affine transform.", node.name));
    }
    let mut complete = String::new();
    write_artwork(node, &artwork, depth, &mut complete);
    output.push_str(&complete);
    Ok(())
}

#[cfg(all(test, not(target_os = "emscripten")))]
#[path = "text_export_tests.rs"]
mod tests;
