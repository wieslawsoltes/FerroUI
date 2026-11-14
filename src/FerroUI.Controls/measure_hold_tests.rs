//! Measure holds (desktop-performance.md sections 9.15 and 9.56): a tree measured at one width
//! and then at another inside the holds of its elements has the desired sizes and, once
//! arranged, the bounds of a tree measured and arranged at the second width from nothing.

use crate::test_support::test_scope;
use crate::{Border, ColumnDefinitions, Control, ControlImpl, Decorator, Grid, Panel, StackPanel, TextBlock, WrapPanel};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{TextAlignment, TextWrapping};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_overrides, instantiate, FerroObjectImpl, Rect, Ref, Size,
    StyledElementImpl, Thickness, Visual, VisualImpl,
};

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

/// A grid of several columns for the card: its definitions, the spacing between its columns,
/// and whether the cells of its star columns are wrapped texts or elements with the measure of
/// `Layoutable` (a panel around a wrapped text and a control of a set height).
#[derive(Clone, Copy, Debug)]
struct Columns {
    definitions: &'static str,
    spacing: f64,
    default_measure: bool,
}

const fn columns(definitions: &'static str, spacing: f64, default_measure: bool) -> Option<Columns> {
    Some(Columns { definitions, spacing, default_measure })
}

/// The grids of several star columns the tests lay out.
const GRIDS: [Option<Columns>; 7] = [
    columns("*,*", 0.0, false),
    columns("2*,*", 0.0, false),
    columns("*,*,*", 6.0, false),
    columns("*,Auto,3*", 4.0, false),
    columns("1.5*,24,*", 2.5, false),
    columns("*,*", 0.0, true),
    columns("2*,3*", 6.0, true),
];

fn grid(columns: Columns) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(columns.definitions).expect("columns"));
    grid.set_column_spacing(columns.spacing);
    for (index, definition) in columns.definitions.split(',').enumerate() {
        let cell: Ref<Control> = if !definition.ends_with('*') {
            text("42%", TextWrapping::NoWrap).upcast()
        } else if columns.default_measure {
            let panel = Panel::new();
            panel.children().add(&text(TEXTS[index % TEXTS.len()], TextWrapping::Wrap));
            let bar = Control::new();
            bar.set_height(4.0);
            panel.children().add(&bar);
            panel.upcast()
        } else {
            text(TEXTS[index % TEXTS.len()], TextWrapping::Wrap).upcast()
        };
        Grid::set_column(&cell, index as i32);
        grid.children().add(&cell);
    }
    grid
}

/// A card: a border with padding around a stack of wrapped text, a decorated label, an
/// unwrapped label, a grid of several columns where there is one, and a centered text (which
/// holds nothing).
fn tree(centered: bool, columns: Option<Columns>) -> Ref<Border> {
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
    // Several star columns: each has its share of the width (section 9.56).
    if let Some(columns) = columns {
        let columns_box = Border::new();
        columns_box.set_child(&grid(columns));
        stack.children().add(&columns_box);
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

/// Measures and arranges the tree at a width.
///
/// Every element is arranged, also at the rectangle it had. An element that is measured again
/// because its parent offers it another size is not arranged again at the same rectangle (as
/// upstream), and with layout rounding two widths less than a pixel apart give a grid the same
/// rectangle and other desired sizes of its cells. That is so with and without holds, and not
/// what these tests are about: they compare what a measure left behind, arranged.
fn layout(root: &Border, width: f64) {
    fn invalidate_arranges(visual: &Visual) {
        if let Some(layoutable) = visual.downcast_ref::<Layoutable>() {
            layoutable.invalidate_arrange();
        }
        if let Some(children) = visual.visual_children_snapshot() {
            for child in children.iter() {
                invalidate_arranges(child);
            }
        }
    }
    root.measure(Size::new(width, f64::INFINITY));
    invalidate_arranges(root);
    root.arrange(Rect::from_size(Size::new(width, root.desired_size().height)));
}

/// Lays a tree out at a width (every `step` pixels from 60 to 420) and then at every quarter of
/// a pixel within `reach` of it, and compares each layout with the one of a tree made for that
/// width. Returns how many of the widths the root held.
fn lays_out_as_a_new_one(centered: bool, columns: Option<Columns>, rounding: bool, step: usize, reach: f64) -> usize {
    let make = || {
        let root = tree(centered, columns);
        root.set_use_layout_rounding(rounding);
        root
    };
    let mut held = 0;
    for start in (60..=420).step_by(step) {
        let start = f64::from(start);
        let root = make();
        layout(&root, start);
        let mut width = start - reach;
        while width < start + reach {
            if width > 0.0 {
                if root.measure_hold().is_some_and(|hold| hold.contains(Size::new(width, f64::INFINITY))) {
                    held += 1;
                }
                layout(&root, width);
                let fresh = make();
                layout(&fresh, width);
                assert_eq!(
                    sizes(&fresh),
                    sizes(&root),
                    "created at {start}, laid out again at {width}, centered {centered}, {columns:?}, \
                     rounding {rounding}"
                );
            }
            width += 0.25;
        }
    }
    held
}

#[test]
fn a_tree_measured_inside_its_holds_lays_out_as_a_new_one() {
    let _scope = test_scope();
    let mut held = 0;
    for (centered, columns) in [(false, None), (true, None), (false, GRIDS[0])] {
        held += lays_out_as_a_new_one(centered, columns, true, 30, 40.0);
    }
    assert!(held > 100, "only {held} widths were held");
}

/// Section 9.56: a grid of several star columns holds what the cells of its columns hold, each
/// at its share of the width, with and without layout rounding, and so does an element with the
/// measure of `Layoutable`.
#[test]
fn a_grid_of_several_star_columns_measured_inside_its_holds_lays_out_as_a_new_one() {
    let _scope = test_scope();
    for columns in GRIDS {
        for rounding in [true, false] {
            let held = lays_out_as_a_new_one(false, columns, rounding, 90, 30.0);
            assert!(held > 50, "only {held} widths were held: {columns:?}, rounding {rounding}");
        }
    }
}

/// The widths around the one a grid was measured at that it holds are those at which every
/// star column keeps its cell inside the hold of the cell: a column of two thirds of the width
/// moves two thirds as far as the grid.
#[test]
fn a_grid_of_several_star_columns_holds_what_its_cells_hold_at_their_share() {
    let _scope = test_scope();
    for (definitions, share) in [("*,*", 0.5), ("2*,*", 2.0 / 3.0), ("*,3*", 0.25)] {
        let mut ranges = 0;
        for width in (200..=600).step_by(7) {
            let grid = Grid::new();
            grid.set_column_definitions(ColumnDefinitions::parse(definitions).expect("columns"));
            // One wrapped text, in the first column; the other column is empty.
            let cell = text(TEXTS[0], TextWrapping::Wrap);
            grid.children().add(&cell);
            grid.measure(Size::new(f64::from(width), f64::INFINITY));
            let width = f64::from(width);
            let holds_a_range = |hold: ferroui_base::layout::MeasureHold| hold.width.1 - hold.width.0 > 1.0e-6;
            // A text whose widest line fills its width holds only that width.
            let Some(cell_hold) = cell.measure_hold().filter(|hold| holds_a_range(*hold)) else {
                assert!(!grid.measure_hold().is_some_and(holds_a_range));
                continue;
            };
            // The text was measured at its share of the width, and the grid holds the widths
            // that give it a share inside the hold of the text, less a margin at each end: the
            // rounding step and the slack of the division, as wide as they are for the grid.
            let margin = (1.0 + 1.0 / 65536.0) / share;
            let (low, high) = (cell_hold.width.0 / share + margin, cell_hold.width.1 / share - margin);
            let hold = grid.measure_hold().expect("a measured grid");
            if low <= width && width < high {
                ranges += 1;
                assert!((hold.width.0 - low).abs() < 1.0e-6, "{definitions} at {width}: {hold:?}, {low}");
                assert!((hold.width.1 - high).abs() < 1.0e-6, "{definitions} at {width}: {hold:?}, {high}");
            } else {
                assert!(!holds_a_range(hold), "{definitions} at {width}: {hold:?}");
            }
        }
        assert!(ranges > 3, "{definitions}: the text held a range at {ranges} widths only");
    }
}

/// A decorator whose measure is another one, written as a plain `impl`: what `Decorator` holds
/// says nothing about it.
#[repr(C)]
struct WidthTaker {
    base: Decorator,
}

ferro_class!(WidthTaker: Decorator);
ferro_impl_classes!(
    WidthTaker: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for WidthTaker {
    /// Takes half of the width it is offered, whatever its child wants.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let size = Self::parent_measure_override(this, available_size);
        Size::new(available_size.width / 2.0, size.height)
    }
}

/// A decorator that arranges in its own way and says that this is all it overrides.
#[repr(C)]
struct Arranger {
    base: Decorator,
}

ferro_class!(Arranger: Decorator);
ferro_impl_classes!(
    Arranger: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferro_overrides! {
    impl LayoutableImpl for Arranger {
        fn arrange_override(this: &Self, final_size: Size) -> Size {
            Self::parent_arrange_override(this, final_size)
        }
    }
}

/// Section 9.56: a hold is inherited only with the measure it describes.
#[test]
fn a_hold_is_inherited_only_with_the_measure_it_describes() {
    let _scope = test_scope();
    let holds_other_width = |element: &Layoutable| {
        element.measure_hold().is_some_and(|hold| hold.contains(Size::new(201.0, f64::INFINITY)))
    };
    let measure = |element: &Layoutable| element.measure(Size::new(200.0, f64::INFINITY));

    // A decorator around a label holds every width that fits the label.
    let decorator = Decorator::new();
    decorator.set_child(&text(TEXTS[1], TextWrapping::NoWrap));
    measure(&decorator);
    assert!(holds_other_width(&decorator));

    // A class with another measure and no hold of its own holds nothing: its size follows the
    // width, and it is measured again at another one.
    let taker = instantiate(WidthTaker { base: Decorator::construct() });
    taker.set_child(&text(TEXTS[1], TextWrapping::NoWrap));
    measure(&taker);
    assert_eq!(100.0, taker.desired_size().width);
    assert!(!holds_other_width(&taker));
    taker.measure(Size::new(202.0, f64::INFINITY));
    assert_eq!(101.0, taker.desired_size().width);

    // A class that says it keeps the measure of its base class keeps the hold with it.
    let arranger = instantiate(Arranger { base: Decorator::construct() });
    arranger.set_child(&text(TEXTS[1], TextWrapping::NoWrap));
    measure(&arranger);
    assert!(holds_other_width(&arranger));

    // The measure of `Layoutable` itself: a panel holds what its visible children hold, and a
    // control without children every size.
    let leaf = Control::new();
    leaf.set_height(4.0);
    measure(&leaf);
    assert!(leaf.measure_hold().is_some_and(|hold| hold.contains(Size::new(1.0, f64::INFINITY))));
    let panel = Panel::new();
    panel.children().add(&text(TEXTS[1], TextWrapping::NoWrap));
    panel.children().add(&leaf);
    measure(&panel);
    assert!(holds_other_width(&panel));
    let centered = text(TEXTS[3], TextWrapping::Wrap);
    centered.set_text_alignment(TextAlignment::Center);
    panel.children().add(&centered);
    measure(&panel);
    assert!(!holds_other_width(&panel), "a child that holds only its width");
    // A child that is not visible is not measured, and holds nothing back.
    centered.set_is_visible(false);
    measure(&panel);
    assert!(holds_other_width(&panel));
}

#[test]
fn a_centered_text_holds_only_its_width_and_so_do_its_parents() {
    let _scope = test_scope();
    let root = tree(true, None);
    layout(&root, 200.0);
    let holds_other_width =
        |root: &Ref<Border>| root.measure_hold().is_some_and(|hold| hold.contains(Size::new(201.0, f64::INFINITY)));
    assert!(!holds_other_width(&root));
    // A card of holding parts holds (at most widths: with the whole-pixel advances of the test
    // font a line can be exactly as wide as the width, where the hold keeps a margin).
    let held = (60..400).step_by(7).filter(|width| {
        let root = tree(false, None);
        layout(&root, f64::from(*width));
        root.measure_hold().is_some_and(|hold| hold.width.1 - hold.width.0 > 1.0)
    });
    assert!(held.count() > 20, "a card of holding parts holds");
    let root = tree(false, None);
    layout(&root, 200.0);
    let stack = root.child().expect("a child");
    let header = stack.visual_children_snapshot().expect("children")[4].clone();
    let header_hold = header.downcast_ref::<Layoutable>().expect("a layoutable").measure_hold();
    assert!(header_hold.is_some_and(|hold| hold.width.1 - hold.width.0 > 1.0), "the header grid holds a range");
}


/// A change of `UseLayoutRounding` invalidates the measure (section 9.15): a hold must not keep a
/// size measured with the other rounding.
#[test]
fn a_change_of_layout_rounding_invalidates_the_measure() {
    let _scope = test_scope();
    let root = tree(false, None);
    layout(&root, 200.0);
    assert!(root.is_measure_valid());
    root.set_use_layout_rounding(!root.use_layout_rounding());
    assert!(!root.is_measure_valid());
}
