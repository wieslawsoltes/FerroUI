//! What a layout pass split into three steps would cost, beside the pass of today: the inputs of
//! the measure and the arrange of a tree of panels are read into plain values on the thread of the
//! tree (`extract`), the sizes and the rectangles are worked out from those values alone, on one
//! thread or on several (`compute`), and the results are written to the elements on the thread of
//! the tree (`apply`). An addition: a decision experiment for laying out on worker threads, which
//! is worth building only when `extract`, `apply` and `compute` on several threads together take
//! clearly less than `existing`.
//!
//! The tree is a dashboard of stack panels and borders: a vertical stack panel of 100 groups, each
//! a border (margin, border thickness, padding) around a vertical stack panel of 11 rows, each row
//! a horizontal stack panel (spacing, margin, one of the four horizontal alignments) of 8 leaves,
//! each leaf a border with a fixed size and a margin. Some leaves have a least height in place of
//! a height, a greatest width under their width, sizes and margins with fractions, a vertical
//! alignment, or are not visible, and a few groups do not round their layout. `Nodes` counts the
//! root panel and everything under it, not the root of the test. The width of the root panel
//! alternates between two values, one an iteration, in every benchmark.
//!
//! - `existing`: the layout pass of the framework (measure and arrange) over the whole tree. The
//!   leaves have fixed sizes, so a change of the width alone leaves most measures held and most
//!   arranges valid; every element is therefore marked as not measured and not arranged before
//!   the pass, through the setters of the flags, as `measure.rs` does. The marking is in the
//!   iteration setup, outside the measurement. The pass includes the root of the test, one element
//!   more than `Nodes`;
//! - `existing_incremental`: the same pass after the change of the width alone, with nothing
//!   marked: what the framework does today for that change;
//! - `extract`: the walk over the tree that writes one plain value a node (kind and orientation,
//!   spacing, margin, padding and border thickness, the sizes and their limits, the alignments,
//!   visibility, layout rounding, first child and child count), read through the public getters
//!   of the elements. It applies the styling and the template of every element first, as the
//!   measure of the framework does, because both belong to the thread of the tree;
//! - `apply`: the results written to the elements: both validity flags, the bounds through the
//!   public setter, and the snap points event a stack panel raises at the end of its arrange. The
//!   setter of the desired size is private to the framework, and the previous measure, the
//!   previous arrange and the measure hold have no setters, so `apply` reads the desired size and
//!   compares it in place of setting it, and does not write the other three. It therefore counts
//!   less than a real apply by three writes of plain cells a node and, for a node whose desired
//!   size changes, by the notification of that change and of its parent (in this tree only the
//!   desired size of the root panel changes with the width);
//! - `compute`: the measure and the arrange over the plain values, with `Threads=1` on the
//!   calling thread, with 2 and 4 on scoped threads of the standard library, the groups divided
//!   evenly among them (the calling thread takes one part). The root is measured and arranged on
//!   the calling thread between a round of threads for the measure and one for the arrange;
//! - `scoped_spawn`: the two rounds of scoped threads of `compute` with nothing to do: what a
//!   pool of threads kept between passes would save of `compute` at the same `Threads`.
//!
//! Every time is one node's (`Nodes` operations an invocation). The ratio to `existing` is in
//! the table for the rows with the same parameters; the rows with `Threads` have none.
//!
//! The computation follows the rules of the framework for an element (visibility, margin, the
//! limits of the size, layout rounding at scale 1, the alignments), for a stack panel and for a
//! border, in the order the framework applies them, so its results are equal to the framework's
//! to the last bit. The setup of every benchmark asserts that: at both widths, for 1, 2 and 4
//! threads, the desired size and the bounds of every node are those the framework has after its
//! own pass, and after `apply` the bounds of every element are the computed ones. Not modelled,
//! since this tree has none: scales other than 1, container queries, measure holds, and panels
//! other than the two.

use crate::harness::Registry;
use crate::test_root::TestRoot;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::testing::{set_is_arrange_valid, set_is_measure_valid};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{Rect, Ref, Size, Thickness};
use ferroui_controls::testing::NullRenderer;
use ferroui_controls::{Border, Control, StackPanel};
use std::hint::black_box;
use std::ops::Range;

/// The groups of the dashboard.
const GROUPS: usize = 100;
/// The rows of a group.
const ROWS: usize = 11;
/// The leaves of a row.
const LEAVES: usize = 8;
/// The root panel and everything under it: a group is a border, a stack panel and its rows, a row
/// is a stack panel and its leaves.
const NODES: usize = 1 + GROUPS * (2 + ROWS * (1 + LEAVES));
/// The widths the root panel alternates between.
const WIDTHS: [f64; 2] = [1280.0, 1440.0];
/// The size of the root of the test: what the root panel is measured with.
const CLIENT_SIZE: Size = Size::new(1600.0, 900.0);

/// The layout rule of a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NodeKind {
    StackPanel,
    Border,
}

/// What the measure and the arrange of one node read: plain values, so a slice of them can be
/// read from any thread.
#[derive(Clone, Copy)]
struct LayoutInput {
    kind: NodeKind,
    /// Of a stack panel.
    orientation: Orientation,
    /// Of a stack panel.
    spacing: f64,
    margin: Thickness,
    /// Of a border.
    padding: Thickness,
    /// Of a border.
    border_thickness: Thickness,
    width: f64,
    height: f64,
    min_width: f64,
    max_width: f64,
    min_height: f64,
    max_height: f64,
    horizontal_alignment: HorizontalAlignment,
    vertical_alignment: VerticalAlignment,
    is_visible: bool,
    use_layout_rounding: bool,
    /// The index of the first child; the children of a node follow each other.
    first_child: u32,
    child_count: u32,
}

/// What the measure and the arrange of one node produce.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct LayoutResult {
    desired_size: Size,
    bounds: Rect,
}

/// The tree as plain values. The root is node 0, its children (the groups) follow it, and after
/// them the descendants of each group are together, group after group, so the results of the
/// groups can be divided among threads as slices that do not overlap.
struct Snapshot {
    inputs: Vec<LayoutInput>,
    /// The element of every node, for `apply` and for the comparison with the framework.
    elements: Vec<Ref<Control>>,
    /// The nodes of the descendants of every group.
    groups: Vec<Range<usize>>,
}

impl Snapshot {
    fn new() -> Self {
        Self { inputs: Vec::new(), elements: Vec::new(), groups: Vec::new() }
    }

    /// Reads the tree under `root` again; the storage of the previous reading is used again.
    fn extract(&mut self, root: &Ref<Control>) {
        self.inputs.clear();
        self.elements.clear();
        self.groups.clear();

        self.push(root);
        self.push_children(0);
        let group_count = self.inputs[0].child_count as usize;
        for group in 0..group_count {
            let start = self.inputs.len();
            self.push_children(1 + group);
            // Breadth first: the children of a node follow each other.
            let mut cursor = start;
            while cursor < self.inputs.len() {
                self.push_children(cursor);
                cursor += 1;
            }
            self.groups.push(start..self.inputs.len());
        }
    }

    /// Adds the node of `element`, without its children.
    fn push(&mut self, element: &Ref<Control>) {
        // What the measure of the framework does first, on the thread of the tree.
        element.apply_styling();
        element.apply_template();

        let none = Thickness::default();
        let (kind, orientation, spacing, padding, border_thickness) =
            if let Some(panel) = element.downcast_ref::<StackPanel>() {
                (NodeKind::StackPanel, panel.orientation(), panel.spacing(), none, none)
            } else if let Some(border) = element.downcast_ref::<Border>() {
                (NodeKind::Border, Orientation::Vertical, 0.0, border.padding(), border.border_thickness())
            } else {
                panic!("the computation has the rules of a stack panel and of a border only");
            };

        self.inputs.push(LayoutInput {
            kind,
            orientation,
            spacing,
            margin: element.margin(),
            padding,
            border_thickness,
            width: element.width(),
            height: element.height(),
            min_width: element.min_width(),
            max_width: element.max_width(),
            min_height: element.min_height(),
            max_height: element.max_height(),
            horizontal_alignment: element.horizontal_alignment(),
            vertical_alignment: element.vertical_alignment(),
            is_visible: element.is_visible(),
            use_layout_rounding: element.use_layout_rounding(),
            first_child: 0,
            child_count: 0,
        });
        self.elements.push(element.clone());
    }

    /// Adds the nodes of the children of the node `index` at the end. An element that is not
    /// visible is not measured, so its children are left out.
    fn push_children(&mut self, index: usize) {
        let first_child = self.inputs.len();
        let mut child_count = 0u32;

        if self.inputs[index].is_visible {
            match self.inputs[index].kind {
                NodeKind::StackPanel => {
                    let children = {
                        let panel = self.elements[index].downcast_ref::<StackPanel>();
                        panel.expect("the node is a stack panel").children().snapshot()
                    };
                    for child in children.iter() {
                        self.push(child);
                        child_count += 1;
                    }
                }
                NodeKind::Border => {
                    let child = {
                        let border = self.elements[index].downcast_ref::<Border>();
                        border.expect("the node is a border").child()
                    };
                    if let Some(child) = child {
                        self.push(&child);
                        child_count = 1;
                    }
                }
            }
        }

        let input = &mut self.inputs[index];
        input.first_child = first_child as u32;
        input.child_count = child_count;
    }
}

/// The least and the greatest size of a node.
#[derive(Clone, Copy)]
struct Limits {
    min_width: f64,
    max_width: f64,
    min_height: f64,
    max_height: f64,
}

fn limits(node: &LayoutInput) -> Limits {
    let (min_width, max_width) = axis_limits(node.width, node.min_width, node.max_width);
    let (min_height, max_height) = axis_limits(node.height, node.min_height, node.max_height);
    Limits { min_width, max_width, min_height, max_height }
}

/// The limits of one axis: a size that is set is both limits, within the least and the greatest.
fn axis_limits(value: f64, min: f64, max: f64) -> (f64, f64) {
    let (low, high) = if value.is_nan() { (0.0, f64::INFINITY) } else { (value, value) };
    let upper = limit(high, min, max);
    let lower = limit(low, min, upper);
    (lower, upper)
}

/// The greatest first, then the least: the least may be over the greatest here.
fn limit(mut value: f64, min: f64, max: f64) -> f64 {
    if value > max {
        value = max;
    }
    if value < min {
        value = min;
    }
    value
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn clamp_size(size: Size, limits: &Limits) -> Size {
    Size::new(
        clamp(size.width, limits.min_width, limits.max_width),
        clamp(size.height, limits.min_height, limits.max_height),
    )
}

/// Layout rounding of a thickness at scale 1.
fn round_thickness(thickness: Thickness) -> Thickness {
    Thickness::new(
        thickness.left.round_ties_even(),
        thickness.top.round_ties_even(),
        thickness.right.round_ties_even(),
        thickness.bottom.round_ties_even(),
    )
}

/// Layout rounding of a size at scale 1, upwards.
fn round_size_up(size: Size) -> Size {
    Size::new(size.width.ceil(), size.height.ceil())
}

/// The padding and the border thickness of a border together. They are rounded when the border
/// has a child and rounds its layout.
fn around(node: &LayoutInput) -> Thickness {
    if node.child_count > 0 && node.use_layout_rounding {
        round_thickness(node.padding) + round_thickness(node.border_thickness)
    } else {
        node.padding + node.border_thickness
    }
}

/// The rectangle the root of the test arranges its child in: the desired size of the child,
/// rounded as the root rounds its own.
fn root_rect(desired: Size) -> Rect {
    Rect::from_size(round_size_up(desired))
}

/// The part of the measure of an element before its content is measured.
struct MeasureFrame {
    margin: Thickness,
    limits: Limits,
    /// What the content is measured with.
    constraint: Size,
}

fn measure_begin(node: &LayoutInput, available: Size) -> MeasureFrame {
    let margin = if node.use_layout_rounding { round_thickness(node.margin) } else { node.margin };
    let limits = limits(node);
    let constraint = clamp_size(available.deflate(margin), &limits);
    MeasureFrame { margin, limits, constraint }
}

/// The part of the measure of an element after its content is measured: the desired size.
fn measure_end(node: &LayoutInput, frame: &MeasureFrame, measured: Size, available: Size) -> Size {
    let mut width = clamp(measured.width, frame.limits.min_width, frame.limits.max_width);
    let mut height = clamp(measured.height, frame.limits.min_height, frame.limits.max_height);

    if node.use_layout_rounding {
        width = width.ceil();
        height = height.ceil();
    }

    width += frame.margin.left + frame.margin.right;
    height += frame.margin.top + frame.margin.bottom;

    if width > available.width {
        width = available.width;
    }
    if height > available.height {
        height = available.height;
    }
    if width < 0.0 {
        width = 0.0;
    }
    if height < 0.0 {
        height = 0.0;
    }

    Size::new(width, height)
}

/// The part of the arrange of an element before its content is arranged.
struct ArrangeFrame {
    origin_x: f64,
    origin_y: f64,
    /// The rectangle without the margin.
    available: Size,
    /// What the content is arranged in, and the size of the bounds: a stack panel and a border
    /// both use all of it.
    size: Size,
}

fn arrange_begin(node: &LayoutInput, desired: Size, final_rect: Rect) -> ArrangeFrame {
    let mut margin = node.margin;
    let origin_x = final_rect.x + margin.left;
    let origin_y = final_rect.y + margin.top;

    // The origin has the margin as it is; the sizes have the rounded one.
    if node.use_layout_rounding {
        margin = round_thickness(margin);
    }

    let available_width = (final_rect.width - margin.left - margin.right).max(0.0);
    let available_height = (final_rect.height - margin.top - margin.bottom).max(0.0);
    let mut available = Size::new(available_width, available_height);
    let mut size = available;

    if node.horizontal_alignment != HorizontalAlignment::Stretch {
        size.width = size.width.min(desired.width - margin.left - margin.right);
    }
    if node.vertical_alignment != VerticalAlignment::Stretch {
        size.height = size.height.min(desired.height - margin.top - margin.bottom);
    }

    size = clamp_size(size, &limits(node));

    if node.use_layout_rounding {
        size = round_size_up(size);
        available = round_size_up(available);
    }

    ArrangeFrame { origin_x, origin_y, available, size }
}

/// The part of the arrange of an element after its content is arranged: the bounds.
fn arrange_end(node: &LayoutInput, frame: &ArrangeFrame) -> Rect {
    let size = frame.size;
    let mut x = frame.origin_x;
    let mut y = frame.origin_y;

    match node.horizontal_alignment {
        HorizontalAlignment::Center | HorizontalAlignment::Stretch => x += (frame.available.width - size.width) / 2.0,
        HorizontalAlignment::Right => x += frame.available.width - size.width,
        HorizontalAlignment::Left => {}
    }
    match node.vertical_alignment {
        VerticalAlignment::Center | VerticalAlignment::Stretch => y += (frame.available.height - size.height) / 2.0,
        VerticalAlignment::Bottom => y += frame.available.height - size.height,
        VerticalAlignment::Top => {}
    }

    if node.use_layout_rounding {
        x = x.round_ties_even();
        y = y.round_ties_even();
    }

    Rect::new(x, y, size.width, size.height)
}

/// The measure of the content of a stack panel: the desired sizes of its children added up.
struct StackMeasure {
    horizontal: bool,
    spacing: f64,
    /// What every child is measured with: without a limit along the stack.
    slot: Size,
    desired: Size,
    has_visible_child: bool,
}

impl StackMeasure {
    fn new(node: &LayoutInput, constraint: Size) -> Self {
        let horizontal = node.orientation == Orientation::Horizontal;
        let slot = if horizontal {
            Size::new(f64::INFINITY, constraint.height)
        } else {
            Size::new(constraint.width, f64::INFINITY)
        };
        Self { horizontal, spacing: node.spacing, slot, desired: Size::default(), has_visible_child: false }
    }

    fn add(&mut self, is_visible: bool, child: Size) {
        if is_visible {
            self.has_visible_child = true;
        }
        let gap = if is_visible { self.spacing } else { 0.0 };
        if self.horizontal {
            self.desired.width = self.desired.width + gap + child.width;
            self.desired.height = max(self.desired.height, child.height);
        } else {
            self.desired.width = max(self.desired.width, child.width);
            self.desired.height = self.desired.height + gap + child.height;
        }
    }

    fn finish(&self) -> Size {
        let gap = if self.has_visible_child { self.spacing } else { 0.0 };
        if self.horizontal {
            Size::new(self.desired.width - gap, self.desired.height)
        } else {
            Size::new(self.desired.width, self.desired.height - gap)
        }
    }
}

/// The arrange of the content of a stack panel: the rectangle of every visible child, in order.
struct StackArrange {
    horizontal: bool,
    spacing: f64,
    final_size: Size,
    slot: Rect,
    previous: f64,
}

impl StackArrange {
    fn new(node: &LayoutInput, final_size: Size) -> Self {
        Self {
            horizontal: node.orientation == Orientation::Horizontal,
            spacing: node.spacing,
            final_size,
            slot: Rect::from_size(final_size),
            previous: 0.0,
        }
    }

    /// The rectangle of the next visible child, from its desired size.
    fn next(&mut self, child: Size) -> Rect {
        if self.horizontal {
            self.slot.x += self.previous;
            self.previous = child.width;
            self.slot.width = self.previous;
            self.slot.height = max(self.final_size.height, child.height);
            self.previous += self.spacing;
        } else {
            self.slot.y += self.previous;
            self.previous = child.height;
            self.slot.height = self.previous;
            self.slot.width = max(self.final_size.width, child.width);
            self.previous += self.spacing;
        }
        self.slot
    }
}

/// Measures the node `index` and everything under it, and returns its desired size. The desired
/// sizes of the nodes under it are stored: the result of the node `child` is `results[child -
/// base]`.
fn measure_node(
    inputs: &[LayoutInput],
    results: &mut [LayoutResult],
    base: usize,
    index: usize,
    available: Size,
) -> Size {
    let node = &inputs[index];
    if !node.is_visible {
        return Size::default();
    }

    let frame = measure_begin(node, available);
    let first = node.first_child as usize;
    let count = node.child_count as usize;

    let measured = match node.kind {
        NodeKind::StackPanel => {
            let mut stack = StackMeasure::new(node, frame.constraint);
            for child in first..first + count {
                let desired = measure_node(inputs, results, base, child, stack.slot);
                results[child - base].desired_size = desired;
                stack.add(inputs[child].is_visible, desired);
            }
            stack.finish()
        }
        NodeKind::Border => {
            let around = around(node);
            if count == 0 {
                Size::default().inflate(around)
            } else {
                let desired = measure_node(inputs, results, base, first, frame.constraint.deflate(around));
                results[first - base].desired_size = desired;
                desired.inflate(around)
            }
        }
    };

    measure_end(node, &frame, measured, available)
}

/// Arranges the visible node `index` and everything under it in `final_rect`, and returns its
/// bounds. The bounds of the nodes under it are stored, as [`measure_node`] stores the desired
/// sizes, which this reads.
fn arrange_node(
    inputs: &[LayoutInput],
    results: &mut [LayoutResult],
    base: usize,
    index: usize,
    desired: Size,
    final_rect: Rect,
) -> Rect {
    let node = &inputs[index];
    let frame = arrange_begin(node, desired, final_rect);
    let first = node.first_child as usize;
    let count = node.child_count as usize;

    match node.kind {
        NodeKind::StackPanel => {
            let mut stack = StackArrange::new(node, frame.size);
            for child in first..first + count {
                if !inputs[child].is_visible {
                    continue;
                }
                let child_desired = results[child - base].desired_size;
                let slot = stack.next(child_desired);
                let bounds = arrange_node(inputs, results, base, child, child_desired, slot);
                results[child - base].bounds = bounds;
            }
        }
        NodeKind::Border => {
            if count > 0 && inputs[first].is_visible {
                let slot = Rect::from_size(frame.size).deflate_thickness(around(node));
                let child_desired = results[first - base].desired_size;
                let bounds = arrange_node(inputs, results, base, first, child_desired, slot);
                results[first - base].bounds = bounds;
            }
        }
    }

    arrange_end(node, &frame)
}

/// The measure and the arrange of the whole tree on the calling thread. `available` is what the
/// root is measured with; `results` has one value a node.
fn compute(inputs: &[LayoutInput], results: &mut [LayoutResult], available: Size) {
    let Some((root, rest)) = results.split_first_mut() else {
        return;
    };
    let desired = measure_node(inputs, rest, 1, 0, available);
    root.desired_size = desired;
    if inputs[0].is_visible {
        root.bounds = arrange_node(inputs, rest, 1, 0, desired, root_rect(desired));
    }
}

/// The groups one thread lays out, with the results it writes.
struct Part<'a> {
    /// The first group of the part, among the groups.
    first_group: usize,
    /// The results of the groups of the part.
    groups: &'a mut [LayoutResult],
    /// The node of the first result of `descendants`.
    base: usize,
    /// The results of the descendants of the groups of the part.
    descendants: &'a mut [LayoutResult],
}

impl Part<'_> {
    /// Measures the groups of the part, each with `slot`.
    fn measure(&mut self, inputs: &[LayoutInput], slot: Size) {
        let first_group = self.first_group;
        let base = self.base;
        for (offset, group) in self.groups.iter_mut().enumerate() {
            let index = 1 + first_group + offset;
            group.desired_size = measure_node(inputs, self.descendants, base, index, slot);
        }
    }

    /// Arranges the visible groups of the part, each in the rectangle that is in the place of its
    /// bounds.
    fn arrange(&mut self, inputs: &[LayoutInput]) {
        let first_group = self.first_group;
        let base = self.base;
        for (offset, group) in self.groups.iter_mut().enumerate() {
            let index = 1 + first_group + offset;
            if !inputs[index].is_visible {
                continue;
            }
            group.bounds = arrange_node(inputs, self.descendants, base, index, group.desired_size, group.bounds);
        }
    }
}

/// Divides the groups evenly into at most `threads` parts. `groups` has the results of the groups
/// and `descendants` those of every node after them; `blocks` has the nodes of the descendants of
/// every group.
fn split_parts<'a>(
    blocks: &[Range<usize>],
    mut groups: &'a mut [LayoutResult],
    mut descendants: &'a mut [LayoutResult],
    threads: usize,
) -> Vec<Part<'a>> {
    let group_count = blocks.len();
    let threads = threads.clamp(1, group_count.max(1));
    let mut parts = Vec::with_capacity(threads);
    let mut first_group = 0;

    for part in 0..threads {
        let end_group = group_count * (part + 1) / threads;
        if end_group == first_group {
            continue;
        }
        let base = blocks[first_group].start;
        let end = blocks[end_group - 1].end;
        let (part_groups, rest_groups) = std::mem::take(&mut groups).split_at_mut(end_group - first_group);
        let (part_descendants, rest_descendants) = std::mem::take(&mut descendants).split_at_mut(end - base);
        groups = rest_groups;
        descendants = rest_descendants;
        parts.push(Part { first_group, groups: part_groups, base, descendants: part_descendants });
        first_group = end_group;
    }

    parts
}

/// Runs `work` for every part at once: the last part on the calling thread, each of the others
/// on a scoped thread of its own. Returns when all of them are done.
fn for_each_part<'a, F>(parts: &mut [Part<'a>], work: F)
where
    F: Fn(&mut Part<'a>) + Sync,
{
    let Some((last, others)) = parts.split_last_mut() else {
        return;
    };
    let work = &work;
    std::thread::scope(|scope| {
        for part in others {
            scope.spawn(move || work(part));
        }
        work(last);
    });
}

/// The measure and the arrange of the whole tree, the groups divided among `threads` threads.
/// The root is measured and arranged on the calling thread: its measure adds up the desired
/// sizes of the groups, and its arrange gives every group its rectangle.
fn compute_on_threads(snapshot: &Snapshot, results: &mut [LayoutResult], available: Size, threads: usize) {
    let inputs = snapshot.inputs.as_slice();
    let group_count = snapshot.groups.len();
    let Some(root) = inputs.first().copied() else {
        return;
    };
    if threads <= 1 || group_count == 0 || !root.is_visible || root.kind != NodeKind::StackPanel {
        compute(inputs, results, available);
        return;
    }

    let (head, descendants) = results.split_at_mut(1 + group_count);
    let (root_result, group_results) = head.split_first_mut().expect("the tree has a root");
    let mut parts = split_parts(&snapshot.groups, group_results, descendants, threads);

    // The measure of the groups, then of the root.
    let frame = measure_begin(&root, available);
    let mut stack = StackMeasure::new(&root, frame.constraint);
    let slot = stack.slot;
    for_each_part(&mut parts, |part| part.measure(inputs, slot));
    for part in &parts {
        for (offset, group) in part.groups.iter().enumerate() {
            stack.add(inputs[1 + part.first_group + offset].is_visible, group.desired_size);
        }
    }
    let desired = measure_end(&root, &frame, stack.finish(), available);

    // The arrange of the root, then of the groups. The rectangle of a group travels to the
    // thread of its part in the place of its bounds, which the arrange then replaces.
    let arrange_frame = arrange_begin(&root, desired, root_rect(desired));
    let mut slots = StackArrange::new(&root, arrange_frame.size);
    for part in &mut parts {
        let first_group = part.first_group;
        for (offset, group) in part.groups.iter_mut().enumerate() {
            if !inputs[1 + first_group + offset].is_visible {
                continue;
            }
            group.bounds = slots.next(group.desired_size);
        }
    }
    for_each_part(&mut parts, |part| part.arrange(inputs));
    drop(parts);

    root_result.desired_size = desired;
    root_result.bounds = arrange_end(&root, &arrange_frame);
}

pub struct ComputeApplySpike {
    root: Ref<TestRoot>,
    /// The root panel of the dashboard.
    panel: Ref<Control>,
    snapshot: Snapshot,
    /// What `compute` writes.
    results: Vec<LayoutResult>,
    /// What `apply` writes: the results at each of the two widths.
    tables: [Vec<LayoutResult>; 2],
    /// What the root is measured with.
    available: Size,
    /// Whether the root panel has the second width.
    wide: bool,
    threads: usize,
    /// The dispatcher of the thread of the benchmark: the layout pass of the root reads it, and
    /// a dispatcher belongs to the thread that first asks for it. Declared last: released after
    /// the tree.
    _dispatcher: UnitTestDispatcherScope,
}

impl ComputeApplySpike {
    pub fn new(threads: usize) -> Self {
        let dispatcher = Dispatcher::unit_test_scope();
        let panel = Self::build();

        let root = TestRoot::new();
        root.set_client_size(CLIENT_SIZE);
        root.set_child(&panel);
        root.set_renderer(NullRenderer::new());
        root.layout_manager().execute_initial_layout_pass();

        let panel: Ref<Control> = panel.upcast();
        let mut snapshot = Snapshot::new();
        snapshot.extract(&panel);
        let results = vec![LayoutResult::default(); snapshot.inputs.len()];

        let mut spike = Self {
            root,
            panel,
            snapshot,
            results,
            tables: [Vec::new(), Vec::new()],
            available: CLIENT_SIZE,
            wide: false,
            threads,
            _dispatcher: dispatcher,
        };
        spike.verify();
        spike
    }

    /// The dashboard: see the module documentation.
    fn build() -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(6.0);
        panel.set_width(WIDTHS[0]);

        for group in 0..GROUPS {
            let rows = StackPanel::new();
            rows.set_spacing(2.0);

            for row in 0..ROWS {
                let line = StackPanel::new();
                line.set_orientation(Orientation::Horizontal);
                line.set_spacing(4.0);
                line.set_margin(Thickness::symmetric(0.0, 1.0));
                line.set_horizontal_alignment(match row % 4 {
                    0 => HorizontalAlignment::Stretch,
                    1 => HorizontalAlignment::Left,
                    2 => HorizontalAlignment::Center,
                    _ => HorizontalAlignment::Right,
                });
                for index in 0..LEAVES {
                    let leaf = Self::leaf(group, row, index);
                    line.children().add(&leaf);
                }
                rows.children().add(&line);
            }

            let frame = Border::new();
            frame.set_margin(Thickness::symmetric(4.0, 2.0));
            frame.set_border_thickness(Thickness::uniform(1.0));
            frame.set_padding(Thickness::new(8.0, 6.0, 8.0, 6.0));
            if group % 25 == 7 {
                // Its descendants inherit it.
                frame.set_use_layout_rounding(false);
            }
            frame.set_child(&rows);
            panel.children().add(&frame);
        }

        panel
    }

    /// A leaf: a border with a fixed size and a margin, or one of the variants.
    fn leaf(group: usize, row: usize, index: usize) -> Ref<Border> {
        let leaf = Border::new();
        let width = 72.0 + (index % 5) as f64 * 8.0;
        let height = 20.0 + (row % 3) as f64 * 4.0;
        leaf.set_width(width);
        leaf.set_margin(Thickness::symmetric(2.0, 1.0));

        match (group * 7 + row * 3 + index) % 16 {
            2 => {
                // No height: its padding, its border and its least height give it one.
                leaf.set_padding(Thickness::uniform(3.0));
                leaf.set_border_thickness(Thickness::uniform(1.0));
                leaf.set_min_height(12.0);
            }
            5 => {
                leaf.set_height(height);
                leaf.set_max_width(64.0);
            }
            7 => {
                leaf.set_width(90.5);
                leaf.set_height(height + 0.4);
                leaf.set_margin(Thickness::new(1.5, 0.5, 2.5, 1.25));
            }
            9 => {
                leaf.set_height(height - 6.0);
                leaf.set_vertical_alignment(VerticalAlignment::Top);
            }
            11 => {
                leaf.set_height(height - 6.0);
                leaf.set_vertical_alignment(VerticalAlignment::Center);
            }
            13 => {
                leaf.set_height(height - 6.0);
                leaf.set_vertical_alignment(VerticalAlignment::Bottom);
            }
            15 => {
                leaf.set_height(height);
                leaf.set_is_visible(false);
            }
            _ => leaf.set_height(height),
        }

        leaf
    }

    /// Changes to the other width, and returns its index.
    fn toggle(&mut self) -> usize {
        self.wide = !self.wide;
        usize::from(self.wide)
    }

    /// Marks every element as not measured and not arranged.
    fn mark_all(&self) {
        self.root.invalidate_measure();

        for element in &self.snapshot.elements {
            // Not `invalidate_measure`: see the documentation of `measure.rs`.
            set_is_measure_valid(element, false);
            set_is_arrange_valid(element, false);
        }
    }

    /// Asserts that the computation gives what the framework does, at both widths and for every
    /// count of threads, and that `apply` writes it; keeps the results of both widths for
    /// `apply`. Leaves the tree laid out at the first width.
    fn verify(&mut self) {
        assert_eq!(self.snapshot.inputs.len(), NODES, "the dashboard has the nodes the benchmarks are named with");

        for wide in [true, false] {
            self.wide = wide;
            self.panel.set_width(WIDTHS[usize::from(wide)]);
            self.mark_all();
            self.root.layout_manager().execute_layout_pass();
            self.snapshot.extract(&self.panel);
            assert_eq!(self.snapshot.inputs.len(), self.results.len());

            for threads in [1, 2, 4] {
                self.results.fill(LayoutResult::default());
                compute_on_threads(&self.snapshot, &mut self.results, self.available, threads);

                for (index, element) in self.snapshot.elements.iter().enumerate() {
                    let framework = LayoutResult { desired_size: element.desired_size(), bounds: element.bounds() };
                    let computed = self.results[index];
                    assert!(
                        framework == computed,
                        "node {index} ({:?}) at the width {}, on {threads} thread(s): the framework has {framework:?}, the computation {computed:?}",
                        self.snapshot.inputs[index].kind,
                        WIDTHS[usize::from(wide)],
                    );
                }
            }

            self.tables[usize::from(wide)] = self.results.clone();
        }

        // Twice: to the second width and back to the first, which the tree is laid out at.
        for _ in 0..2 {
            self.apply();
            let table = &self.tables[usize::from(self.wide)];
            for (index, element) in self.snapshot.elements.iter().enumerate() {
                assert!(
                    element.bounds() == table[index].bounds,
                    "node {index}: after the apply the element has {:?}, the computation {:?}",
                    element.bounds(),
                    table[index].bounds,
                );
            }
        }
    }

    /// The iteration setup of `existing`: the other width, and every element marked.
    pub fn prepare_existing(&mut self) {
        let width = WIDTHS[self.toggle()];
        self.panel.set_width(width);
        self.mark_all();
    }

    /// The iteration setup of `existing_incremental`: the other width alone.
    pub fn prepare_existing_incremental(&mut self) {
        let width = WIDTHS[self.toggle()];
        self.panel.set_width(width);
    }

    pub fn existing(&mut self) {
        self.root.layout_manager().execute_layout_pass();
    }

    pub fn extract(&mut self) -> usize {
        self.snapshot.extract(&self.panel);
        black_box(self.snapshot.inputs.len())
    }

    /// Writes the results of the other width to the elements. Returns how many desired sizes
    /// differ from the elements': see the module documentation.
    pub fn apply(&mut self) -> usize {
        let index = self.toggle();
        let table = &self.tables[index];
        let mut changed = 0;

        for ((element, input), result) in self.snapshot.elements.iter().zip(&self.snapshot.inputs).zip(table) {
            set_is_measure_valid(element, true);
            if element.desired_size() != result.desired_size {
                changed += 1;
            }
            if !input.is_visible {
                continue;
            }

            set_is_arrange_valid(element, true);
            element.set_bounds(result.bounds);

            if input.kind == NodeKind::StackPanel {
                // What a stack panel raises at the end of its arrange.
                let event = if input.orientation == Orientation::Horizontal {
                    StackPanel::horizontal_snap_points_changed_event()
                } else {
                    StackPanel::vertical_snap_points_changed_event()
                };
                element.raise_event(&RoutedEventArgs::with_event(event));
            }
        }

        black_box(changed)
    }

    pub fn compute(&mut self) -> Size {
        let width = WIDTHS[self.toggle()];
        self.snapshot.inputs[0].width = width;
        compute_on_threads(&self.snapshot, &mut self.results, self.available, self.threads);
        black_box(self.results[0].desired_size)
    }

    /// The two rounds of scoped threads of `compute`, with nothing to do.
    pub fn scoped_spawn(&mut self) -> usize {
        let group_count = self.snapshot.groups.len();
        let (head, descendants) = self.results.split_at_mut(1 + group_count);
        let (_, group_results) = head.split_first_mut().expect("the tree has a root");
        let mut parts = split_parts(&self.snapshot.groups, group_results, descendants, self.threads);

        for_each_part(&mut parts, |part| {
            black_box(part.groups.len());
        });
        for_each_part(&mut parts, |part| {
            black_box(part.descendants.len());
        });

        parts.len()
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("layout", "ComputeApplySpike").min_iteration_time(300);
    let nodes = NODES as u32;
    let parameters = format!("Nodes={NODES}");

    class
        .benchmark_with_iteration_setup(
            "existing",
            parameters.clone(),
            || ComputeApplySpike::new(1),
            |b| b.prepare_existing(),
            |b| b.existing(),
        )
        .baseline()
        .operations_per_invoke(nodes);
    class
        .benchmark_with_iteration_setup(
            "existing_incremental",
            parameters.clone(),
            || ComputeApplySpike::new(1),
            |b| b.prepare_existing_incremental(),
            |b| b.existing(),
        )
        .operations_per_invoke(nodes);
    class
        .benchmark("extract", parameters.clone(), || ComputeApplySpike::new(1), |b| b.extract())
        .operations_per_invoke(nodes);
    class
        .benchmark("apply", parameters.clone(), || ComputeApplySpike::new(1), |b| b.apply())
        .operations_per_invoke(nodes);

    for threads in [1usize, 2, 4] {
        let parameters = format!("Nodes={NODES}, Threads={threads}");
        class
            .benchmark("compute", parameters, move || ComputeApplySpike::new(threads), |b| b.compute())
            .operations_per_invoke(nodes);
    }
    for threads in [2usize, 4] {
        let parameters = format!("Nodes={NODES}, Threads={threads}");
        class
            .benchmark("scoped_spawn", parameters, move || ComputeApplySpike::new(threads), |b| b.scoped_spawn())
            .operations_per_invoke(nodes);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compute_apply_spike() {
        crate::harness::smoke_class(super::register, "ComputeApplySpike");
    }
}
