//! Measure holds (desktop-performance.md section 9.15): a tree measured at one width and then at
//! another inside the holds of its elements has the desired sizes and, once arranged, the bounds
//! of a tree measured and arranged at the second width from nothing.

use crate::test_support::test_scope;
use crate::{Border, ColumnDefinitions, Decorator, Grid, StackPanel, TextBlock, WrapPanel};
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
fn tree(centered: bool, with_halves: bool) -> Ref<Border> {
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
    // A wrap panel of chips: bordered labels that do not wrap.
    let chips = WrapPanel::new();
    chips.set_item_spacing(3.0);
    for label in ["#rust", "#layout", "#text", "#performance", "#ui", "#gpu-rendering"] {
        let chip = Border::new();
        chip.set_padding(Thickness::new(4.0, 1.0, 4.0, 1.0));
        chip.set_child(&text(label, TextWrapping::NoWrap));
        chips.children().add(&chip);
    }
    stack.children().add(&chips);
    // A header grid "Auto,*": a fixed avatar and a stack of texts.
    let header = Grid::new();
    header.set_column_definitions(ColumnDefinitions::parse("Auto,*").expect("columns"));
    header.set_column_spacing(6.0);
    let avatar = Border::new();
    avatar.set_width(20.0);
    avatar.set_height(20.0);
    header.children().add(&avatar);
    let names = StackPanel::new();
    names.children().add(&text("Ada Lovelace wrote the first program", TextWrapping::Wrap));
    names.children().add(&text("@ada", TextWrapping::NoWrap));
    Grid::set_column(&names, 1);
    header.children().add(&names);
    stack.children().add(&header);
    // A progress row "*,Auto": a wrapped text and a label.
    let progress = Grid::new();
    progress.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("columns"));
    progress.children().add(&text("Progress of the reprehenderit task", TextWrapping::Wrap));
    let percent = text("42%", TextWrapping::NoWrap);
    Grid::set_column(&percent, 1);
    progress.children().add(&percent);
    stack.children().add(&progress);
    // Two star columns: no hold.
    let halves = Grid::new();
    halves.set_column_definitions(ColumnDefinitions::parse("*,*").expect("columns"));
    halves.children().add(&text("Left half text", TextWrapping::Wrap));
    let right = text("Right half text", TextWrapping::Wrap);
    Grid::set_column(&right, 1);
    halves.children().add(&right);
    let halves_box = Border::new();
    halves_box.set_child(&halves);
    if with_halves {
        stack.children().add(&halves_box);
    }
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
    for (centered, halves) in [(false, false), (true, false), (false, true)] {
        for start in (60..=420).step_by(30) {
            let start = f64::from(start);
            let root = tree(centered, halves);
            layout(&root, start);
            let mut width = start - 40.0;
            while width < start + 40.0 {
                if width > 0.0 {
                    if root.measure_hold().is_some_and(|hold| hold.contains(Size::new(width, f64::INFINITY))) {
                        held += 1;
                    }
                    layout(&root, width);
                    let fresh = tree(centered, halves);
                    layout(&fresh, width);
                    assert_eq!(sizes(&fresh), sizes(&root), "created at {start}, laid out again at {width}, centered {centered}, halves {halves}");
                }
                width += 0.25;
            }
        }
    }
    assert!(held > 100, "only {held} widths were held");
}

#[test]
fn a_centered_text_holds_only_its_width_and_so_do_its_parents() {
    let _scope = test_scope();
    let root = tree(true, false);
    layout(&root, 200.0);
    let holds_other_width = |root: &Ref<Border>| root.measure_hold().is_some_and(|hold| hold.contains(Size::new(201.0, f64::INFINITY)));
    assert!(!holds_other_width(&root));
    // A card of holding parts holds (at most widths: with the whole-pixel advances of the test
    // font a line can be exactly as wide as the width, where the hold keeps a margin).
    let held = (60..400).step_by(7).filter(|width| {
        let root = tree(false, false);
        layout(&root, f64::from(*width));
        root.measure_hold().is_some_and(|hold| hold.width.1 - hold.width.0 > 1.0)
    });
    assert!(held.count() > 20, "a card of holding parts holds");
    let root = tree(false, true);
    layout(&root, 200.0);
    // The grid of two star columns holds only its width: so does the card.
    assert!(!holds_other_width(&root));
    let stack = root.child().expect("a child");
    let header = stack.visual_children_snapshot().expect("children")[4].clone();
    let header_hold = header.downcast_ref::<ferroui_base::layout::Layoutable>().expect("a layoutable").measure_hold();
    assert!(header_hold.is_some_and(|hold| hold.width.1 - hold.width.0 > 1.0), "the header grid holds a range");
}

