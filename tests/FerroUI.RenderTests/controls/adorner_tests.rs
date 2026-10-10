//! Port of upstream's `Controls/AdornerTests.cs`.

use crate::test_base::{CompareOptions, TestBase};
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::Brushes;
use ferroui_base::{Rect, Ref, Size, Thickness};
use ferroui_controls::primitives::{AdornerLayer, VisualLayerManager};
use ferroui_controls::{Border, Control, Decorator};

fn base() -> TestBase {
    TestBase::new(r"Controls\Adorner")
}

fn check_adorned_content(
    t: &TestBase,
    content: Ref<Control>,
    adorned: Ref<Control>,
    adorner: Ref<Control>,
    width: i32,
    height: i32,
    test_name: &str,
) {
    let tree = Decorator::new();
    let manager = VisualLayerManager::new();
    manager.set_child(content);
    tree.set_child(manager);
    tree.set_width(width as f64);
    tree.set_height(height as f64);

    let size = Size::new(tree.width(), tree.height());
    tree.measure(size);
    tree.arrange(Rect::from_size(size));

    // The handler belongs to the adorned control, so it refers to it weakly: a strong reference would keep
    // the control alive for ever, where upstream has a garbage collector.
    let adorned_weak = adorned.downgrade();
    let _attached = adorned.attached_to_visual_tree(move |_| {
        let adorned = adorned_weak.upgrade().expect("the adorned control is alive while it raises its event");
        AdornerLayer::set_adorned_element(&adorner, &adorned);
        // The tree gets attached once per composited render, but the adorner stays
        // in the layer after detach
        let layer = AdornerLayer::get_adorner_layer(&adorned).expect("the adorned control has an adorner layer");
        if !layer.children().contains(&adorner) {
            layer.children().add(adorner.clone());
        }
    });

    tree.measure(size);
    tree.arrange(Rect::from_size(size));

    t.render_to_file(&tree, test_name);
    t.compare_images_with(test_name, CompareOptions { skip_immediate: true, ..Default::default() });
}

fn focus_adorner_is_properly_clipped(clip: bool) {
    let t = base();
    let content = Border::new();
    content.set_background(Some(Brushes::red()));
    content.set_padding(Thickness::new(10.0, 50.0, 10.0, 10.0));
    let inner = Border::new();
    inner.set_background(Some(Brushes::white()));
    inner.set_clip_to_bounds(true);
    inner.set_padding(Thickness::new(0.0, -30.0, 0.0, 0.0));
    let adorned = Border::new();
    adorned.set_background(Some(Brushes::green()));
    adorned.set_vertical_alignment(VerticalAlignment::Top);
    adorned.set_height(100.0);
    adorned.set_width(50.0);
    inner.set_child(&adorned);
    content.set_child(inner);
    let adorner = Border::new();
    adorner.set_border_thickness(Thickness::uniform(2.0));
    adorner.set_border_brush(Some(Brushes::black()));
    if !clip {
        AdornerLayer::set_is_clip_enabled(&adorner, false);
    }
    check_adorned_content(
        &t,
        content.upcast(),
        adorned.upcast(),
        adorner.upcast(),
        200,
        200,
        &format!("Focus_Adorner_Is_Properly_Clipped_Clip_{}", if clip { "True" } else { "False" }),
    );
}

#[test]
fn focus_adorner_is_properly_clipped_true() {
    focus_adorner_is_properly_clipped(true);
}

#[test]
fn focus_adorner_is_properly_clipped_false() {
    focus_adorner_is_properly_clipped(false);
}
