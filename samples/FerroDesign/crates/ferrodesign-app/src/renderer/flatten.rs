//! Editable outlines from the same FerroUI geometries that paint the canvas.
//! No backend downcasts, glyph guessing, SVG serialization or curve sampling.
use super::*;
use ferrodesign_core::{DocumentError, FlattenRequest, VectorPath};
use ferroui_base::media::{GeometryPath, GeometryPathSegment};

fn invalid(message: &str) -> DocumentError {
    DocumentError::Invalid(message.into())
}

pub(crate) fn flatten_selection(editor: &mut Editor) -> Result<NodeId, DocumentError> {
    editor.flatten_selection_with_geometry(outline)
}

fn outline(document: &Document, request: &FlattenRequest) -> Result<VectorPath, DocumentError> {
    let inverse = request
        .destination_world
        .inverse()
        .ok_or_else(|| invalid("The flatten coordinate transform is singular."))?;
    // Prepare an isolated immutable scene with outline requests enabled from
    // the root. Ordinary text paints need not construct paths, so reusing an
    // unrelated cached paint preparation would return incomplete text data.
    let mut painter = ScenePainter::default();
    painter.empty = Some(Rc::new(CachedNode::empty()));
    let mut result: Option<Ref<Geometry>> = None;
    for id in &request.roots {
        let node = &document.nodes[id];
        let parent_world = node
            .parent
            .and_then(|parent| document.world_transform(parent))
            .unwrap_or(Transform::IDENTITY);
        let index = painter
            .prepare_node(document, *id, parent_world, None, 0, true)
            .ok_or_else(|| invalid("The selected layer has no available visible outline."))?;
        let prepared = &painter.prepared[index];
        // A nonempty combined path can still omit an emoji or another glyph
        // without a monochrome outline. Verify every shaped descendant before
        // accepting any part of a group/Boolean or changing the document.
        for descendant in &painter.prepared[index..prepared.end] {
            let source = &document.nodes[&descendant.id];
            if source.kind == NodeKind::Text {
                let fallback;
                let text = if let Some(text) = descendant.visual.texts.first() {
                    text
                } else {
                    fallback = make_text_layout(source, solid_brush(Color::BLACK, 1.0));
                    &fallback
                };
                if text.layout.try_build_geometry(text.origin).is_none() {
                    return Err(invalid("This text contains bitmap, colour-font or unavailable glyph outlines. Keep it as text to preserve every glyph."));
                }
            }
        }
        if request.roots.len() == 1 {
            let data = prepared
                .visual
                .geometry
                .try_get_path_data()
                .ok_or_else(|| {
                    invalid("The active FerroUI renderer does not provide editable outline data.")
                })?;
            let visible_strokes = node
                .strokes
                .iter()
                .any(|stroke| stroke.visible && stroke.opacity > 0.0 && stroke.width > 0.0);
            let has_fills = !prepared.visual.fills.is_empty();
            if has_fills && visible_strokes && data.fill != data.stroke {
                return Err(invalid("The layer fills and strokes independent paths; keep its vector-network representation."));
            }
            return data.fill.or(data.stroke).map(vector_path).ok_or_else(|| {
                invalid("The selected layer has no editable fill or stroke outline.")
            });
        }
        // A geometry can be immutable. A mutable wrapper carries the affine
        // mapping without relying on invalidating a platform geometry.
        let group = GeometryGroup::new();
        // GeometryGroup supplies the rule of all appended figures. Retain the
        // source rule so an even-odd hole cannot become solid before union.
        let fill_rule = prepared
            .visual
            .geometry
            .try_get_path_data()
            .and_then(|data| data.fill.map(|path| path.fill_rule))
            .ok_or_else(|| {
                invalid("Every combined layer must expose a complete filled outline.")
            })?;
        group.set_fill_rule(fill_rule);
        group.children().add(prepared.visual.geometry.clone());
        group.set_transform(MatrixTransform::with_matrix(ui_matrix(
            inverse * prepared.world,
        )));
        let geometry: Ref<Geometry> = group.upcast();
        result = Some(match result {
            None => geometry,
            Some(previous) => combined(GeometryCombineMode::Union, previous, geometry),
        });
    }
    let data = result
        .and_then(|geometry| geometry.try_get_path_data())
        .ok_or_else(|| {
            invalid("The active FerroUI renderer could not export the combined outline.")
        })?;
    data.fill
        .map(vector_path)
        .ok_or_else(|| invalid("The combined selection has no filled outline."))
}

fn vector_path(path: GeometryPath) -> VectorPath {
    let point = |point: UiPoint| Point::new(point.x, point.y);
    VectorPath {
        fill_rule: match path.fill_rule {
            ferroui_base::media::FillRule::NonZero => FillRule::NonZero,
            _ => FillRule::EvenOdd,
        },
        segments: path
            .segments
            .into_iter()
            .map(|segment| match segment {
                GeometryPathSegment::MoveTo(p) => PathSegment::MoveTo { point: point(p) },
                GeometryPathSegment::LineTo(p) => PathSegment::LineTo { point: point(p) },
                GeometryPathSegment::QuadraticTo { control, point: p } => {
                    PathSegment::QuadraticTo {
                        control: point(control),
                        point: point(p),
                    }
                }
                GeometryPathSegment::CubicTo {
                    control1,
                    control2,
                    point: p,
                } => PathSegment::CubicTo {
                    control1: point(control1),
                    control2: point(control2),
                    point: point(p),
                },
                GeometryPathSegment::Close => PathSegment::Close,
            })
            .collect(),
        ..VectorPath::default()
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
#[path = "flatten_tests.rs"]
mod tests;
