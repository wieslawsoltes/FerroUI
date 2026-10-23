use super::*;
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn frame(document: &mut core::Document, parent: NodeId, x: f64, y: f64, width: f64, height: f64) -> NodeId {
    let mut node = Node::new(NodeKind::Frame, "Grid frame", x, y, width, height);
    node.layout_grid = Some(LayoutGrid { size: 8.0, color: Color::rgba(255, 0, 0, 255), visible: true, ..LayoutGrid::default() });
    document.insert_node(parent, node).unwrap()
}

fn pixels(overlay: &mut LayoutGridOverlay, editor: &Editor, viewport: Viewport) -> (Vec<u8>, GridOverlayStats) {
    let width = viewport.width as i32;
    let height = viewport.height as i32;
    let bitmap = RenderTargetBitmap::new(PixelSize::new(width, height));
    let stats;
    {
        let mut context = bitmap.create_drawing_context();
        stats = overlay.render(&mut context, editor, viewport);
    }
    let mut pixels = vec![0; width as usize * height as usize * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, width, height), &mut pixels, width * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) { pixel.swap(0, 2); }
    }
    (pixels, stats)
}

fn viewport() -> Viewport {
    Viewport { pan: Point::default(), zoom: 1.0, width: 180.0, height: 160.0 }
}

fn inside_rounded(point: Point, width: f64, height: f64, radius: f64) -> bool {
    if !Rect::new(-1.5, -1.5, width + 3.0, height + 3.0).contains(point) { return false; }
    let center = Point::new(point.x.clamp(radius, width - radius), point.y.clamp(radius, height - radius));
    point.distance(center) <= radius + 1.5
}

#[test]
fn frame_grids_use_native_transforms_exact_spacing_and_rounded_clips() {
    let scope = backend();
    let mut document = core::Document::new("Grid pixels");
    let id = frame(&mut document, 1, 45.0, 40.0, 80.0, 70.0);
    let node = document.nodes.get_mut(&id).unwrap();
    node.rotation = 31.0;
    node.corner_radius = 18.0;
    node.affine_transform = Some(core::Transform { a: 1.0, b: 0.0, c: 0.25, d: 1.0, e: 0.0, f: 0.0 });
    let inverse = document.world_transform(id).unwrap().inverse().unwrap();
    let mut editor = Editor::new(document);
    let mut overlay = LayoutGridOverlay::default();
    let (image, stats) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(stats.layers, 1);
    assert!(stats.lines > 10 && stats.lines < 30);
    let mut painted = 0;
    for (index, rgba) in image.chunks_exact(4).enumerate().filter(|(_, rgba)| rgba[3] > 16) {
        let point = inverse.apply(Point::new((index % 180) as f64 + 0.5, (index / 180) as f64 + 0.5));
        assert!(inside_rounded(point, 80.0, 70.0, 18.0), "grid escaped rounded frame at {point:?}");
        let dx = point.x.rem_euclid(8.0).min((-point.x).rem_euclid(8.0));
        let dy = point.y.rem_euclid(8.0).min((-point.y).rem_euclid(8.0));
        assert!(dx < 1.5 || dy < 1.5, "grid changed authored spacing at {point:?}");
        assert!(rgba[0] > 0);
        painted += 1;
    }
    assert!(painted > 300);
    editor.edit_layout_grids(&[id], core::LayoutGridEdit::Visible(false)).unwrap();
    let (hidden, hidden_stats) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(hidden_stats.layers, 0);
    assert!(hidden.iter().all(|byte| *byte == 0));
    scope.dispose();
}

#[test]
fn frame_grids_honor_clipped_transformed_ancestors_and_hide_other_pages() {
    let scope = backend();
    let mut document = core::Document::new("Clipped grid");
    let outer = document.insert_node(1, Node::new(NodeKind::Frame, "Outer", 35.0, 25.0, 70.0, 80.0)).unwrap();
    let parent = document.nodes.get_mut(&outer).unwrap();
    parent.rotation = -24.0;
    parent.corner_radius = 14.0;
    parent.clip_content = true;
    let inner = frame(&mut document, outer, -20.0, -20.0, 160.0, 160.0);
    document.nodes.get_mut(&inner).unwrap().rotation = 12.0;
    let second = document.add_page("Invisible page");
    frame(&mut document, second, 0.0, 0.0, 180.0, 160.0);
    let inverse = document.world_transform(outer).unwrap().inverse().unwrap();
    let mut editor = Editor::new(document);
    let mut overlay = LayoutGridOverlay::default();
    let (image, stats) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(stats.layers, 1);
    let mut painted = 0;
    for (index, _) in image.chunks_exact(4).enumerate().filter(|(_, rgba)| rgba[3] > 16) {
        let point = inverse.apply(Point::new((index % 180) as f64 + 0.5, (index / 180) as f64 + 0.5));
        assert!(inside_rounded(point, 70.0, 80.0, 14.0), "grid escaped ancestor clip at {point:?}");
        painted += 1;
    }
    assert!(painted > 250);
    editor.update_node(outer, "Hide parent", |node| node.visible = false);
    assert_eq!(pixels(&mut overlay, &editor, viewport()).1.layers, 0);
    scope.dispose();
}

#[test]
fn grid_density_is_measured_perpendicular_to_affine_axes_and_work_is_bounded() {
    let scope = backend();
    let mut document = core::Document::new("Grid density");
    let id = frame(&mut document, 1, 0.0, 0.0, 10_000_000.0, 10_000_000.0);
    let mut editor = Editor::new(document);
    let mut overlay = LayoutGridOverlay::default();
    let (_, normal) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(normal.layers, 1);
    assert!(normal.lines < 50, "visible region must bound work for huge frames");
    let mut zoomed = viewport(); zoomed.zoom = 0.1;
    let (dense, stats) = pixels(&mut overlay, &editor, zoomed);
    assert_eq!(stats.lines, 0);
    assert_eq!(stats.suppressed_axes, 2);
    assert!(dense.iter().all(|byte| *byte == 0));
    editor.update_node(id, "Shear grid", |node| node.affine_transform = Some(core::Transform { a: 1.0, b: 0.0, c: 100.0, d: 1.0, e: 0.0, f: 0.0 }));
    let (_, sheared) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(sheared.suppressed_axes, 1);
    assert!(sheared.lines < 30);
    assert_eq!(axis_lines(0.0, 1e100, 8.0, 1.0), None);
    scope.dispose();
}

#[test]
fn grid_candidate_cache_reuses_camera_frames_and_scoped_edits_without_scanning_plain_layers() {
    let mut document = core::Document::new("Grid cache");
    let grid = frame(&mut document, 1, 0.0, 0.0, 80.0, 80.0);
    let mut leaf = 0;
    for index in 0..500 {
        leaf = document.insert_node(1, Node::new(NodeKind::Rectangle, "Plain layer", index as f64, 0.0, 10.0, 10.0)).unwrap();
    }
    let mut editor = Editor::new(document);
    let mut overlay = LayoutGridOverlay::default();
    overlay.refresh(&editor);
    assert_eq!(overlay.examined, 501);
    for _ in 0..100 { overlay.refresh(&editor); }
    assert_eq!(overlay.examined, 501);
    editor.update_node(leaf, "Nudge plain leaf", |node| node.x += 1.0);
    overlay.refresh(&editor);
    assert_eq!(overlay.examined, 502);
    assert_eq!(overlay.grids, BTreeSet::from([grid]));
    editor.edit_layout_grids(&[grid], core::LayoutGridEdit::Remove).unwrap();
    overlay.refresh(&editor);
    assert_eq!(overlay.examined, 503);
    assert!(overlay.grids.is_empty());
    editor.undo(); overlay.refresh(&editor);
    assert_eq!(overlay.examined, 504);
    assert_eq!(overlay.grids, BTreeSet::from([grid]));
    let mut replacement = Editor::new(editor.document.clone());
    replacement.document.nodes.get_mut(&grid).unwrap().layout_grid = None;
    replacement.refresh();
    overlay.refresh(&replacement);
    assert_eq!(overlay.examined, 1005);
    assert!(overlay.grids.is_empty());
}

fn column_grid(color: Color, alignment: core::LayoutGridAlignment, count: u32) -> LayoutGrid {
    LayoutGrid { size: 10.0, color, pattern: core::LayoutGridPattern::Columns {
        axis: core::LayoutGridAxis { alignment, count: Some(count), gutter: 10.0, offset: 10.0 }
    }, ..LayoutGrid::default() }
}

fn rgba(image: &[u8], width: usize, x: usize, y: usize) -> &[u8] {
    &image[(y * width + x) * 4..(y * width + x) * 4 + 4]
}

#[test]
fn stacked_columns_and_rows_render_native_bands_in_authored_order_with_exact_gutters() {
    let scope = backend();
    let mut document = core::Document::new("Stacked guides");
    let id = frame(&mut document, 1, 20.0, 20.0, 100.0, 100.0);
    let columns = column_grid(Color::rgb(255, 0, 0), core::LayoutGridAlignment::Start, 3);
    let rows = LayoutGrid { color: Color::rgb(0, 0, 255), pattern: core::LayoutGridPattern::Rows {
        axis: columns.pattern.axis().unwrap()
    }, ..columns };
    document.nodes.get_mut(&id).unwrap().set_layout_guides(&[columns, rows]);
    let mut editor = Editor::new(document);
    let mut overlay = LayoutGridOverlay::default();
    let (image, stats) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(stats.layers, 1); assert_eq!(stats.bands, 6); assert_eq!(stats.lines, 0);
    assert_eq!(rgba(&image, 180, 35, 25), &[255, 0, 0, 255]);
    assert_eq!(rgba(&image, 180, 25, 35), &[0, 0, 255, 255]);
    assert_eq!(rgba(&image, 180, 35, 35), &[0, 0, 255, 255], "later guide paints over prior guide");
    assert_eq!(rgba(&image, 180, 45, 45), &[0, 0, 0, 0], "gutters stay clear");
    editor.edit_layout_guides(&[id], core::LayoutGuideEdit::Move { index: 1, to: 0 }).unwrap();
    let (image, _) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(rgba(&image, 180, 35, 35), &[255, 0, 0, 255]);
    editor.undo();
    editor.edit_layout_guides(&[id], core::LayoutGuideEdit::Set { index: 1, property: core::LayoutGuideProperty::Visible(false) }).unwrap();
    let (image, stats) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(stats.bands, 3); assert_eq!(rgba(&image, 180, 25, 35), &[0, 0, 0, 0]);
    scope.dispose();
}

#[test]
fn stretched_native_columns_resize_without_changing_authored_margin_or_gutter() {
    let scope = backend();
    let mut document = core::Document::new("Stretch guide pixels");
    let id = frame(&mut document, 1, 10.0, 10.0, 100.0, 100.0);
    document.nodes.get_mut(&id).unwrap().layout_grid = Some(column_grid(Color::rgb(0, 255, 0), core::LayoutGridAlignment::Stretch, 2));
    let mut editor = Editor::new(document); let mut overlay = LayoutGridOverlay::default();
    let (small, _) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(rgba(&small, 180, 25, 25), &[0, 255, 0, 255]);
    assert_eq!(rgba(&small, 180, 58, 25), &[0, 0, 0, 0]);
    assert_eq!(rgba(&small, 180, 104, 25), &[0, 0, 0, 0]);
    editor.update_node(id, "Resize frame", |node| node.width = 140.0);
    let (large, _) = pixels(&mut overlay, &editor, viewport());
    assert_eq!(rgba(&large, 180, 58, 25), &[0, 255, 0, 255]);
    assert_eq!(rgba(&large, 180, 78, 25), &[0, 0, 0, 0]);
    assert_eq!(rgba(&large, 180, 145, 25), &[0, 0, 0, 0]);
    scope.dispose();
}

#[test]
fn million_column_guides_do_only_viewport_work_and_preserve_transformed_clips() {
    let scope = backend();
    let mut document = core::Document::new("Large guide stack");
    let id = frame(&mut document, 1, 20.0, 20.0, 50_000_000.0, 100.0);
    document.nodes.get_mut(&id).unwrap().layout_grid = Some(column_grid(Color::rgb(255, 0, 0), core::LayoutGridAlignment::Start, 1_000_000));
    let mut editor = Editor::new(document); let mut overlay = LayoutGridOverlay::default();
    let (_, stats) = pixels(&mut overlay, &editor, viewport());
    assert!(stats.bands < 10); assert!(!stats.budget_exhausted);
    editor.update_node(id, "Transform guide", |node| {
        node.width = 90.0; node.height = 90.0; node.rotation = 35.0; node.corner_radius = 20.0;
        node.affine_transform = Some(core::Transform { a: 1.0, b: 0.0, c: 0.25, d: 1.0, e: 0.0, f: 0.0 });
    });
    let inverse = editor.document.world_transform(id).unwrap().inverse().unwrap();
    let (image, stats) = pixels(&mut overlay, &editor, viewport());
    assert!(stats.bands < 8);
    for (index, _) in image.chunks_exact(4).enumerate().filter(|(_, rgba)| rgba[3] > 16) {
        let local = inverse.apply(Point::new((index % 180) as f64 + 0.5, (index / 180) as f64 + 0.5));
        assert!(inside_rounded(local, 90.0, 90.0, 20.0));
    }
    let mut dense = viewport(); dense.zoom = 0.1;
    let (_, stats) = pixels(&mut overlay, &editor, dense);
    assert_eq!(stats.bands, 0); assert_eq!(stats.suppressed_axes, 1);
    scope.dispose();
}
