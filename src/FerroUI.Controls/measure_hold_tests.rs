//! Measure holds (desktop-performance.md section 9.15): a tree measured at one width and then at
//! another inside the holds of its elements has the desired sizes and, once arranged, the bounds
//! of a tree measured and arranged at the second width from nothing.

use crate::test_support::test_scope;
use crate::{Border, Decorator, StackPanel, TextBlock};
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{TextAlignment, TextWrapping};
use ferroui_base::{Rect, Ref, Size, Thickness, Visual};

const TEXTS: [&str; 4] = [
    "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor.",
    "Short label",
    "Hyphenated-words and a longer reprehenderit voluptate velit esse cillum.",
    "Numbers 1,234.56 and +2.31% together.",
];

fn text(text: &str, wrapping: TextWrapping) -> Ref<TextBlock> {
    let block = TextBlock::new();
    block.set_text(Some(text));
    block.set_text_wrapping(wrapping);
    block
}

/// A card: a border with padding around a stack of wrapped text, a decorated label, an
/// unwrapped label and a centered text (which holds nothing).
fn tree(centered: bool) -> Ref<Border> {
    let stack = StackPanel::new();
    stack.set_spacing(2.0);
    stack.children().add(&text(TEXTS[0], TextWrapping::Wrap));
    let decorator = Decorator::new();
    decorator.set_padding(Thickness::uniform(1.5));
    decorator.set_child(&text(TEXTS[1], TextWrapping::NoWrap));
    stack.children().add(&decorator);
    let inner = Border::new();
    inner.set_border_thickness(Thickness::uniform(1.0));
    inner.set_child(&text(TEXTS[2], TextWrapping::Wrap));
    stack.children().add(&inner);
    let last = text(TEXTS[3], TextWrapping::Wrap);
    if centered {
        last.set_text_alignment(TextAlignment::Center);
    }
    stack.children().add(&last);
    let card = Border::new();
    card.set_padding(Thickness::new(4.0, 3.0, 4.0, 3.0));
    card.set_child(&stack);
    card
}

/// The desired size and bounds of every element of the tree, in order.
fn sizes(root: &Visual) -> Vec<(Size, Rect)> {
    let mut found = Vec::new();
    fn walk(visual: &Visual, found: &mut Vec<(Size, Rect)>) {
        if let Some(layoutable) = visual.downcast_ref::<Layoutable>() {
            found.push((layoutable.desired_size(), visual.bounds()));
        }
        if let Some(children) = visual.visual_children_snapshot() {
            for child in children.iter() {
                walk(child, found);
            }
        }
    }
    walk(root, &mut found);
    found
}

fn layout(root: &Border, width: f64) {
    root.measure(Size::new(width, f64::INFINITY));
    root.arrange(Rect::from_size(Size::new(width, root.desired_size().height)));
}

#[test]
fn a_tree_measured_inside_its_holds_lays_out_as_a_new_one() {
    let _scope = test_scope();
    let mut held = 0;
    for centered in [false, true] {
        for start in (60..=420).step_by(30) {
            let start = f64::from(start);
            let root = tree(centered);
            layout(&root, start);
            let mut width = start - 40.0;
            while width < start + 40.0 {
                if width > 0.0 {
                    if root.measure_hold().is_some_and(|hold| hold.contains(Size::new(width, f64::INFINITY))) {
                        held += 1;
                    }
                    layout(&root, width);
                    let fresh = tree(centered);
                    layout(&fresh, width);
                    assert_eq!(sizes(&fresh), sizes(&root), "created at {start}, laid out again at {width}, centered {centered}");
                }
                width += 0.25;
            }
        }
    }
    assert!(held > 100, "only {held} widths were held");
}

#[test]
fn a_centered_text_holds_nothing_and_breaks_its_parents_hold() {
    let _scope = test_scope();
    let root = tree(true);
    layout(&root, 200.0);
    assert!(root.measure_hold().is_none());
    let root = tree(false);
    layout(&root, 200.0);
    assert!(root.measure_hold().is_some());
}
