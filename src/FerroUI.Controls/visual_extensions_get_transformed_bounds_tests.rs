//! Port of `VisualTree/VisualExtensions_GetTransformedBounds.cs` (base unit
//! tests). The trees are built from borders, so the tests live with the
//! controls.

use crate::test_support::test_scope;
use crate::{Border, Control};
use ferroui_base::media::MatrixTransform;
use ferroui_base::visual_tree::TransformedBounds;
use ferroui_base::{Matrix, Rect, Ref, Size};

fn border(width: f64, height: f64, child: Option<Ref<Border>>) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    if let Some(child) = child {
        border.set_child(child);
    }
    border
}

fn layout(c: &Control) {
    c.measure(Size::INFINITY);
    c.arrange(Rect::from_size(c.desired_size()));
}

#[test]
fn root() {
    let _scope = test_scope();
    let root = border(100.0, 123.0, None);

    layout(&root);

    assert_eq!(
        Some(TransformedBounds::new(
            Rect::new(0.0, 0.0, 100.0, 123.0),
            Rect::new(0.0, 0.0, 100.0, 123.0),
            Matrix::IDENTITY
        )),
        root.get_transformed_bounds()
    );
}

#[test]
fn depth_1_no_transform_or_clip() {
    let _scope = test_scope();
    let target = border(500.0, 500.0, None);
    let root = border(1000.0, 1000.0, Some(target.clone()));

    layout(&root);

    assert_eq!(
        Some(TransformedBounds::new(
            Rect::new(0.0, 0.0, 500.0, 500.0),
            Rect::new(0.0, 0.0, 1000.0, 1000.0),
            Matrix::create_translation(250.0, 250.0)
        )),
        target.get_transformed_bounds()
    );
}

#[test]
fn depth_2_no_transform_or_clip() {
    let _scope = test_scope();
    let target = border(500.0, 500.0, None);
    let root = border(1000.0, 1000.0, Some(border(800.0, 800.0, Some(target.clone()))));

    layout(&root);

    assert_eq!(
        Some(TransformedBounds::new(
            Rect::new(0.0, 0.0, 500.0, 500.0),
            Rect::new(0.0, 0.0, 1000.0, 1000.0),
            Matrix::create_translation(250.0, 250.0)
        )),
        target.get_transformed_bounds()
    );
}

#[test]
fn depth_2_no_transform_with_clip() {
    let _scope = test_scope();
    let target = border(500.0, 500.0, None);
    let middle = border(800.0, 800.0, None);
    middle.set_clip_to_bounds(true);
    middle.set_child(target.clone());
    let root = border(1000.0, 1000.0, Some(middle));

    layout(&root);

    assert_eq!(
        Some(TransformedBounds::new(
            Rect::new(0.0, 0.0, 500.0, 500.0),
            Rect::new(100.0, 100.0, 800.0, 800.0),
            Matrix::create_translation(250.0, 250.0)
        )),
        target.get_transformed_bounds()
    );
}

#[test]
fn depth_2_transformed_clip() {
    let _scope = test_scope();
    let target = border(500.0, 500.0, None);
    let middle = border(800.0, 800.0, None);
    middle.set_clip_to_bounds(true);
    middle.set_render_transform(Some(MatrixTransform::with_matrix(Matrix::create_translation(10.0, 20.0)).into()));
    middle.set_child(target.clone());
    let root = border(1000.0, 1000.0, Some(middle));

    layout(&root);

    assert_eq!(
        Some(TransformedBounds::new(
            Rect::new(0.0, 0.0, 500.0, 500.0),
            Rect::new(110.0, 120.0, 800.0, 800.0),
            Matrix::create_translation(260.0, 270.0)
        )),
        target.get_transformed_bounds()
    );
}
