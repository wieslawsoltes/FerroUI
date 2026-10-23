//! Source-window cropping without changing embedded image bytes.
use crate::{Editor, ImageScale, Node, NodeId, NodeKind, Paint, Point, Rect, Transform};

const FULL_IMAGE: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

pub fn valid_image_crop(crop: Rect) -> bool {
    crop.is_finite()
        && crop.x >= 0.0 && crop.x < 1.0
        && crop.y >= 0.0 && crop.y < 1.0
        && crop.width > 0.0 && crop.height > 0.0
        && crop.right() <= 1.0 + 1e-12 && crop.bottom() <= 1.0 + 1e-12
}

fn canonical_crop(crop: Rect) -> Option<Rect> {
    valid_image_crop(crop).then(|| Rect::new(
        crop.x, crop.y, crop.width.min(1.0 - crop.x), crop.height.min(1.0 - crop.y),
    ))
}

impl Paint {
    /// Normalized source-image coordinates; old documents use the full source.
    pub fn image_crop(&self) -> Option<Rect> {
        match self {
            Self::Image { crop, .. } => Some(crop.unwrap_or(FULL_IMAGE)),
            _ => None,
        }
    }

    /// Set the source window and preserve the authored scaling mode. In
    /// particular, numeric edits to Fill or Fit must never become Stretch.
    pub fn set_image_crop(&mut self, value: Rect) -> bool {
        let Some(value) = canonical_crop(value) else { return false; };
        let Self::Image { crop, .. } = self else { return false; };
        let value = (value != FULL_IMAGE).then_some(value);
        if *crop == value { return false; }
        *crop = value;
        true
    }
}

/// A pointer gesture captures the image-to-layer mapping once. Each update is
/// derived from that start, so long drags cannot accumulate geometry error.
/// Direct cropping supports Fill and Stretch. Fit has a letterboxed destination
/// and Tile has repeated destinations, so those modes use numeric source-window
/// authoring until a corresponding direct-edit mapping is implemented.
#[derive(Clone, Debug)]
pub struct ImageCropSession {
    pub node: NodeId,
    pub fill: usize,
    /// The source region actually visible in the layer at gesture start.
    pub source: Rect,
    pub width: f64,
    pub height: f64,
    transform: Transform,
    scale: ImageScale,
    original_crop: Option<Rect>,
}

impl ImageCropSession {
    pub fn new(node: &Node, fill: usize, image_size: Point) -> Option<Self> {
        if !matches!(node.kind, NodeKind::Rectangle | NodeKind::Ellipse | NodeKind::Polygon
            | NodeKind::Star | NodeKind::Frame | NodeKind::Section)
            || !node.children.is_empty() || node.layout.is_some()
            || !node.width.is_finite() || !node.height.is_finite()
            || node.width <= 0.0 || node.height <= 0.0
            || !image_size.x.is_finite() || !image_size.y.is_finite()
            || image_size.x <= 0.0 || image_size.y <= 0.0
        { return None; }
        let fill_value = node.fills.get(fill)?;
        if !fill_value.visible || fill_value.opacity <= 0.0 { return None; }
        let Paint::Image { scale, crop, .. } = &fill_value.paint else { return None; };
        if !matches!(scale, ImageScale::Fill | ImageScale::Stretch) { return None; }
        let transform = node.local_transform();
        if ![transform.a, transform.b, transform.c, transform.d, transform.e, transform.f]
            .iter().all(|value| value.is_finite()) || transform.inverse().is_none()
        { return None; }
        let mut source = canonical_crop(crop.unwrap_or(FULL_IMAGE))?;
        if *scale == ImageScale::Fill {
            let aspect = node.width / node.height;
            let source_aspect = image_size.x * source.width / (image_size.y * source.height);
            if !aspect.is_finite() || !source_aspect.is_finite() || source_aspect <= 0.0 {
                return None;
            }
            if source_aspect > aspect {
                let width = source.width * aspect / source_aspect;
                source.x += (source.width - width) * 0.5;
                source.width = width;
            } else {
                let height = source.height * source_aspect / aspect;
                source.y += (source.height - height) * 0.5;
                source.height = height;
            }
        }
        let source = canonical_crop(source)?;
        Some(Self { node: node.id, fill, source, width: node.width, height: node.height, transform, scale: *scale, original_crop: *crop })
    }

    pub fn pan(&self, local_delta: Point) -> Option<Rect> {
        if !local_delta.x.is_finite() || !local_delta.y.is_finite() { return None; }
        Some(Rect::new(
            (self.source.x - local_delta.x * self.source.width / self.width)
                .clamp(0.0, (1.0 - self.source.width).max(0.0)),
            (self.source.y - local_delta.y * self.source.height / self.height)
                .clamp(0.0, (1.0 - self.source.height).max(0.0)),
            self.source.width, self.source.height,
        ))
    }

    /// Corner cropping changes the layer window while keeping every retained
    /// source pixel at its original parent-space position, including shear.
    /// Crossing the opposite corner changes the window without flipping pixels.
    pub fn resize(&self, corner: usize, local_point: Point) -> Option<(Rect, Rect)> {
        if corner > 3 || !local_point.x.is_finite() || !local_point.y.is_finite() { return None; }
        let opposite = match corner {
            0 => Point::new(self.source.right(), self.source.bottom()),
            1 => Point::new(self.source.x, self.source.bottom()),
            2 => Point::new(self.source.x, self.source.y),
            _ => Point::new(self.source.right(), self.source.y),
        };
        let point = Point::new(
            (self.source.x + local_point.x * self.source.width / self.width).clamp(0.0, 1.0),
            (self.source.y + local_point.y * self.source.height / self.height).clamp(0.0, 1.0),
        );
        let crop = canonical_crop(Rect::from_points(point, opposite))?;
        if crop.width < 1e-6 || crop.height < 1e-6 { return None; }
        let bounds = Rect::new(
            (crop.x - self.source.x) * self.width / self.source.width,
            (crop.y - self.source.y) * self.height / self.source.height,
            crop.width * self.width / self.source.width,
            crop.height * self.height / self.source.height,
        );
        bounds.is_finite().then_some((crop, bounds))
    }

    /// Allows the native gesture to cancel its snapshot on a click or a drag
    /// back to the start, restoring styles/bindings as well as visible pixels.
    pub fn is_original_window(&self, crop: Rect, bounds: Rect) -> bool {
        let near = |a: f64, b: f64| (a - b).abs() <= 1e-10 * a.abs().max(b.abs()).max(1.0);
        near(crop.x, self.source.x) && near(crop.y, self.source.y)
            && near(crop.width, self.source.width) && near(crop.height, self.source.height)
            && near(bounds.x, 0.0) && near(bounds.y, 0.0)
            && near(bounds.width, self.width) && near(bounds.height, self.height)
    }
}

impl Editor {
    pub fn set_image_crop(&mut self, id: NodeId, fill: usize, crop: Rect) -> bool {
        if !valid_image_crop(crop) || self.document.effective_locked(id) { return false; }
        self.update_node(id, "Crop image", |node| {
            let Some(fill) = node.fills.get_mut(fill) else { return false; };
            if !fill.paint.set_image_crop(crop) { return false; }
            node.overridden_properties.insert("fills".into());
            node.fill_style = None;
            node.variable_bindings.remove("fill");
            true
        }).unwrap_or(false)
    }

    /// A flow layout owns child positions. It cannot preserve fixed image
    /// pixels while its crop window moves; those children permit image panning
    /// and numeric UV edits, but direct corner resizing is disabled.
    pub fn can_resize_image_crop(&self, id: NodeId) -> bool {
        let Some(node) = self.document.nodes.get(&id) else { return false; };
        !self.document.effective_locked(id)
            && node.layout.is_none() && node.children.is_empty()
            && (node.layout_child.absolute || node.parent
                .and_then(|id| self.document.nodes.get(&id))
                .is_none_or(|parent| parent.layout.is_none()))
    }

    pub fn resize_image_crop(&mut self, session: &ImageCropSession, corner: usize, local_point: Point) -> bool {
        let Some((crop, bounds)) = session.resize(corner, local_point) else { return false; };
        if !self.can_resize_image_crop(session.node) { return false; }
        self.update_node(session.node, "Crop image bounds", |node| {
            let Some(fill) = node.fills.get_mut(session.fill) else { return false; };
            if !matches!(&fill.paint, Paint::Image { scale, .. } if *scale == session.scale) {
                return false;
            }
            let transform = session.transform * Transform::translation(bounds.x, bounds.y);
            let current_transform = node.local_transform();
            let geometry_changed = node.width != bounds.width || node.height != bounds.height
                || [current_transform.a - transform.a, current_transform.b - transform.b,
                    current_transform.c - transform.c, current_transform.d - transform.d,
                    current_transform.e - transform.e, current_transform.f - transform.f]
                    .iter().any(|delta| delta.abs() > 1e-10);
            if !geometry_changed && session.is_original_window(crop, bounds)
                && matches!(&node.fills[session.fill].paint, Paint::Image { crop, .. }
                    if *crop == session.original_crop)
            { return false; }
            let paint_changed = node.fills[session.fill].paint.set_image_crop(crop);
            if !geometry_changed && !paint_changed { return false; }
            node.width = bounds.width;
            node.height = bounds.height;
            node.set_local_transform(transform);
            node.overridden_properties.extend([
                "fills".into(), "width".into(), "height".into(), "x".into(), "y".into(), "position".into(),
                "rotation".into(), "flip_x".into(), "flip_y".into(), "affine_transform".into(),
                "layout_child".into(),
            ]);
            node.fill_style = None;
            node.variable_bindings.remove("fill");
            node.variable_bindings.remove("width");
            node.variable_bindings.remove("height");
            node.layout_child.width = crate::SizingMode::Fixed;
            node.layout_child.height = crate::SizingMode::Fixed;
            true
        }).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AutoLayout, Color, Document, Fill};

    fn image_node() -> Node {
        let mut node = Node::new(NodeKind::Rectangle, "Photo", 20.0, 40.0, 200.0, 100.0);
        node.fills = vec![Fill {
            paint: Paint::Image {
                href: "data:image/png;base64,fixture".into(), scale: ImageScale::Fill, crop: None,
            },
            ..Fill::solid(Color::WHITE)
        }];
        node
    }

    fn serialized(node: &Node) -> serde_json::Value { serde_json::to_value(node).unwrap() }

    #[test]
    fn cover_crop_uses_real_source_aspect_and_pan_clamps() {
        let node = image_node();
        let session = ImageCropSession::new(&node, 0, Point::new(100.0, 100.0)).unwrap();
        assert_eq!(session.source, Rect::new(0.0, 0.25, 1.0, 0.5));
        assert_eq!(session.pan(Point::new(0.0, 1000.0)).unwrap(), Rect::new(0.0, 0.0, 1.0, 0.5));
        assert_eq!(session.pan(Point::new(0.0, -1000.0)).unwrap(), Rect::new(0.0, 0.5, 1.0, 0.5));
        assert!(session.is_original_window(session.pan(Point::new(300.0, 0.0)).unwrap(), node.local_bounds()));
        assert!(session.pan(Point::new(f64::NAN, 0.0)).is_none());
    }

    #[test]
    fn nested_cover_window_retains_source_coordinates_and_scale_modes() {
        let mut node = image_node();
        for scale in [ImageScale::Fill, ImageScale::Fit, ImageScale::Stretch, ImageScale::Tile] {
            if let Paint::Image { scale: current, .. } = &mut node.fills[0].paint { *current = scale; }
            let crop = Rect::new(0.2, 0.1, 0.6, 0.8);
            node.fills[0].paint.set_image_crop(crop);
            assert!(matches!(&node.fills[0].paint, Paint::Image { scale: current, href, .. }
                if *current == scale && href == "data:image/png;base64,fixture"));
            let session = ImageCropSession::new(&node, 0, Point::new(800.0, 400.0));
            match scale {
                ImageScale::Fill => {
                    let source = session.unwrap().source;
                    assert!((source.x - 0.2).abs() < 1e-12);
                    assert!((source.y - 0.2).abs() < 1e-12);
                    assert!((source.width - 0.6).abs() < 1e-12);
                    assert!((source.height - 0.6).abs() < 1e-12);
                }
                ImageScale::Stretch => assert_eq!(session.unwrap().source, crop),
                ImageScale::Fit | ImageScale::Tile => assert!(session.is_none()),
            }
        }
    }

    #[test]
    fn corner_crop_preserves_source_pixel_positions_under_affine_transform() {
        let mut document = Document::new("Crop");
        let page = document.pages[0].id;
        let mut node = image_node();
        node.set_local_transform(Transform { a: 1.2, b: 0.3, c: 0.2, d: 0.8, e: 35.0, f: -10.0 });
        let id = document.insert_node(page, node).unwrap();
        let mut editor = Editor::new(document);
        let original = editor.document.nodes[&id].clone();
        let session = ImageCropSession::new(&original, 0, Point::new(200.0, 100.0)).unwrap();
        let image_point = Point::new(150.0, 75.0);
        let before = original.local_transform().apply(image_point);
        assert!(!editor.resize_image_crop(&session, 0, Point::default()));
        assert!(editor.undo_label().is_none());
        editor.begin_node_transaction(&[id], "Crop gesture");
        assert!(editor.resize_image_crop(&session, 0, Point::new(20.0, 10.0)));
        assert!(editor.resize_image_crop(&session, 0, Point::new(40.0, 20.0)));
        assert!(editor.commit_transaction());
        let changed = &editor.document.nodes[&id];
        let after = changed.local_transform().apply(image_point - Point::new(40.0, 20.0));
        assert!(before.distance(after) < 1e-9);
        assert_eq!(changed.fills[0].paint.image_crop(), Some(Rect::new(0.2, 0.2, 0.8, 0.8)));
        assert!(matches!(changed.fills[0].paint, Paint::Image { scale: ImageScale::Fill, .. }));
        let cropped = serialized(changed);
        assert!(editor.undo());
        assert_eq!(serialized(&editor.document.nodes[&id]), serialized(&original));
        assert!(!editor.undo(), "A drag with multiple updates must be one undo entry");
        assert!(editor.redo());
        assert_eq!(serialized(&editor.document.nodes[&id]), cropped);
    }

    #[test]
    fn all_corners_and_expansion_keep_affine_pixels_fixed() {
        let mut node = image_node();
        node.fills[0].paint.set_image_crop(Rect::new(0.2, 0.25, 0.6, 0.5));
        node.set_local_transform(Transform { a: -0.8, b: 0.6, c: 0.35, d: 1.2, e: 270.0, f: 80.0 });
        // This source aspect equals the node's aspect; cover introduces no trim.
        let session = ImageCropSession::new(&node, 0, Point::new(500.0, 300.0)).unwrap();
        for (corner, point) in [
            (0, Point::new(20.0, 10.0)), (1, Point::new(170.0, 10.0)),
            (2, Point::new(180.0, 80.0)), (3, Point::new(20.0, 80.0)),
            (0, Point::new(-2000.0, -2000.0)),
        ] {
            let (crop, bounds) = session.resize(corner, point).unwrap();
            let uv = crop.center();
            let old_local = Point::new((uv.x - session.source.x) * session.width / session.source.width,
                (uv.y - session.source.y) * session.height / session.source.height);
            let new_local = Point::new((uv.x - crop.x) * bounds.width / crop.width,
                (uv.y - crop.y) * bounds.height / crop.height);
            let after = (session.transform * Transform::translation(bounds.x, bounds.y)).apply(new_local);
            assert!(session.transform.apply(old_local).distance(after) < 1e-9);
            assert!(valid_image_crop(crop));
        }
        let (expanded, _) = session.resize(0, Point::new(-2000.0, -2000.0)).unwrap();
        assert_eq!(expanded.x, 0.0);
        assert_eq!(expanded.y, 0.0);
        assert!(session.resize(8, Point::default()).is_none());
        assert!(session.resize(0, Point::new(node.width, node.height)).is_none());
    }

    #[test]
    fn canceled_crop_restores_styles_bindings_and_geometry_without_history() {
        let mut document = Document::new("Cancel crop");
        let page = document.pages[0].id;
        let mut node = image_node();
        node.fill_style = Some(90);
        node.variable_bindings.insert("fill".into(), 91);
        node.variable_bindings.insert("width".into(), 92);
        let id = document.insert_node(page, node).unwrap();
        let mut editor = Editor::new(document);
        let original = serialized(&editor.document.nodes[&id]);
        let session = ImageCropSession::new(&editor.document.nodes[&id], 0, Point::new(200.0, 100.0)).unwrap();
        editor.begin_node_transaction(&[id], "Crop gesture");
        editor.resize_image_crop(&session, 0, Point::new(20.0, 10.0));
        assert!(editor.document.nodes[&id].variable_bindings.is_empty());
        editor.cancel_transaction();
        assert_eq!(serialized(&editor.document.nodes[&id]), original);
        assert!(editor.undo_label().is_none());
    }

    #[test]
    fn invalid_crop_is_atomic_and_legacy_images_default_to_full_source() {
        let mut paint: Paint = serde_json::from_str(
            r#"{"type":"image","href":"data:image/png;base64,x","scale":"fill"}"#,
        ).unwrap();
        assert_eq!(paint.image_crop(), Some(FULL_IMAGE));
        let original = paint.clone();
        for invalid in [
            Rect::new(-0.1, 0.0, 1.0, 1.0), Rect::new(0.0, 0.0, 0.0, 1.0),
            Rect::new(0.8, 0.0, 0.5, 1.0), Rect::new(f64::NAN, 0.0, 1.0, 1.0),
            Rect::new(1.0, 0.0, 1e-13, 1.0),
        ] {
            assert!(!paint.set_image_crop(invalid));
            assert_eq!(paint, original);
        }
        assert!(!paint.set_image_crop(FULL_IMAGE));
        assert!(paint.set_image_crop(Rect::new(0.1, 0.0, 0.9 + 1e-14, 1.0)));
        assert!(paint.image_crop().unwrap().right() <= 1.0);
    }

    #[test]
    fn direct_crop_rejects_invalid_mapping_and_flow_resize_is_atomic() {
        let mut node = image_node();
        for size in [Point::new(0.0, 100.0), Point::new(f64::INFINITY, 100.0), Point::new(100.0, f64::NAN)] {
            assert!(ImageCropSession::new(&node, 0, size).is_none());
        }
        let size = Point::new(200.0, 100.0);
        node.kind = NodeKind::Vector;
        assert!(ImageCropSession::new(&node, 0, size).is_none());
        node.kind = NodeKind::Text;
        assert!(ImageCropSession::new(&node, 0, size).is_none());
        node.kind = NodeKind::Rectangle;
        let mut document = Document::new("Layout crop");
        let mut parent = Node::new(NodeKind::Frame, "Flow", 0.0, 0.0, 400.0, 200.0);
        parent.layout = Some(AutoLayout::default());
        let parent = document.insert_node(document.pages[0].id, parent).unwrap();
        let id = document.insert_node(parent, node).unwrap();
        let mut editor = Editor::new(document);
        let original = serialized(&editor.document.nodes[&id]);
        let session = ImageCropSession::new(&editor.document.nodes[&id], 0, size).unwrap();
        assert!(!editor.can_resize_image_crop(id));
        assert!(!editor.resize_image_crop(&session, 0, Point::new(20.0, 10.0)));
        assert_eq!(serialized(&editor.document.nodes[&id]), original);
        assert!(editor.undo_label().is_none());
        assert!(editor.set_image_crop(id, 0, Rect::new(0.0, 0.25, 1.0, 0.5)));
        editor.clear_history();
        editor.document.nodes.get_mut(&parent).unwrap().locked = true;
        assert!(!editor.set_image_crop(id, 0, FULL_IMAGE));
        assert!(editor.undo_label().is_none());
    }
}
