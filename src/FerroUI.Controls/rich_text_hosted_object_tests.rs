//! The public BlockLayout host constraints, with real framework controls and
//! formatter continuations. These are source-contract tests, not pixel oracles.

use crate::documents::{EmbeddedControlRun, InlineUIContainer, LineBreak, Paragraph, Run};
use crate::test_support::test_scope;
use crate::{Control, ControlImpl, RichEditBox, RichTextBlock, RichTextBlockOverflow};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::TextWrapping;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref, Size,
    StyledElementImpl, Thickness, Visual, VisualImpl, VisualImplExt, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type MeasureCallback = Rc<dyn Fn(&HostedProbe)>;

#[repr(C)]
struct HostedProbe {
    base: Control,
    desired: Cell<Size>,
    constraints: RefCell<Vec<Size>>,
    parents: RefCell<Vec<Option<WeakRef<Visual>>>>,
    on_measure: RefCell<Option<MeasureCallback>>,
    arrangements: Cell<usize>,
    on_arrange: RefCell<Option<MeasureCallback>>,
    on_visual_detach: RefCell<Option<MeasureCallback>>,
}

ferro_class!(HostedProbe: Control);
ferro_impl_classes!(HostedProbe:
    FerroObjectImpl, StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl VisualImpl for HostedProbe {
    fn on_visual_parent_changed(
        this: &Self,
        old_parent: Option<&Ref<Visual>>,
        new_parent: Option<&Ref<Visual>>,
    ) {
        Self::parent_on_visual_parent_changed(this, old_parent, new_parent);
        if new_parent.is_none() {
            let callback = this.on_visual_detach.borrow_mut().take();
            if let Some(callback) = callback {
                callback(this);
            }
        }
    }
}

impl LayoutableImpl for HostedProbe {
    fn measure_override(this: &Self, available: Size) -> Size {
        this.constraints.borrow_mut().push(available);
        this.parents
            .borrow_mut()
            .push(this.visual_parent().map(|parent| parent.downgrade()));
        let callback = this.on_measure.borrow_mut().take();
        if let Some(callback) = callback {
            callback(this);
        }
        let desired = this.desired.get();
        Size::new(
            desired.width.min(available.width),
            desired.height.min(available.height),
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arrangements.set(this.arrangements.get() + 1);
        let callback = this.on_arrange.borrow_mut().take();
        if let Some(callback) = callback {
            callback(this);
        }
        final_size
    }
}

impl HostedProbe {
    fn new(desired: Size) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            desired: Cell::new(desired),
            constraints: RefCell::new(Vec::new()),
            parents: RefCell::new(Vec::new()),
            on_measure: RefCell::new(None),
            arrangements: Cell::new(0),
            on_arrange: RefCell::new(None),
            on_visual_detach: RefCell::new(None),
        })
    }

    fn last_constraint(&self) -> Size {
        *self
            .constraints
            .borrow()
            .last()
            .expect("object was measured")
    }
}

fn rich() -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.set_font_size(10.0);
    view.set_text_wrapping(TextWrapping::Wrap);
    view
}

fn measure(view: &RichTextBlock, size: Size) {
    view.measure(size);
    view.arrange(Rect::from_size(size));
}

fn object(paragraph: &Paragraph, desired: Size) -> Ref<HostedProbe> {
    let probe = HostedProbe::new(desired);
    paragraph.add_child(InlineUIContainer::with_child(probe.clone()));
    probe
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn rich_hosted_constraints_use_page_padding_remaining_height_and_collapsed_margins() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    view.set_padding(Thickness::new(4.0, 5.0, 6.0, 7.0));
    let first = Paragraph::with_text("x");
    first.set_margin(Thickness::new(0.0, 20.0, 0.0, 15.0));
    view.blocks().add(first);
    let second = Paragraph::with_text("abc");
    second.set_margin(Thickness::new(8.0, 7.0, 9.0, 99.0));
    second.set_text_indent(12.0);
    let child = object(&second, Size::new(30.0, 20.0));
    view.blocks().add(second);
    measure(&view, Size::new(100.0, 100.0));
    assert_eq!(child.last_constraint(), Size::new(73.0, 62.0));
    close(view.line_metrics()[0].bounds.y, 5.0);
    close(view.line_metrics()[1].bounds.y, 31.0);
    close(child.bounds().x, 39.0);
    // The mixed line retains the text descent below the object's baseline:
    // padding 12 + first line 11 + collapsed margin 15 + (baseline 20 + descent 3).
    close(view.desired_size().height, 61.0);
}

#[test]
fn rich_hosted_constraints_are_constant_within_a_paragraph_fragment() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::with_text("prefix");
    paragraph.set_text_indent(9.0);
    let first = object(&paragraph, Size::new(20.0, 12.0));
    paragraph.add_child(LineBreak::new());
    paragraph.add_child(Run::with_text(Some("later")));
    let second = object(&paragraph, Size::new(20.0, 12.0));
    view.blocks().add(paragraph);
    measure(&view, Size::new(100.0, 60.0));
    assert_eq!(first.last_constraint(), Size::new(100.0, 60.0));
    assert_eq!(second.last_constraint(), first.last_constraint());
    assert!(second.bounds().y > first.bounds().y);
    assert_eq!(first.constraints.borrow().len(), 1);
    assert_eq!(second.constraints.borrow().len(), 1);
}

#[test]
fn rich_hosted_overflow_rebinds_object_continuations_without_reshaping_or_mutating_head() {
    let _scope = test_scope();
    let text = TextTestScope::new();
    let view = rich();
    view.set_max_lines(1);
    let paragraph = Paragraph::with_text("aa ");
    let child = object(&paragraph, Size::new(200.0, 10.0));
    paragraph.add_child(Run::with_text(Some(" tail")));
    view.blocks().add(paragraph);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, Size::new(50.0, 11.0));
    let head = view.page_layout();
    assert!(head.embedded_controls(Point::default()).is_empty());
    assert_eq!(child.last_constraint(), Size::new(50.0, 11.0));
    let calls = text.shaper().shape_call_count();
    measure(&overflow, Size::new(90.0, 40.0));
    let wide = overflow.page_layout();
    assert_eq!(child.last_constraint(), Size::new(90.0, 40.0));
    assert_eq!(wide.embedded_controls(Point::default())[0].1.width, 90.0);
    assert_eq!(text.shaper().shape_call_count(), calls);
    let parent = child
        .parents
        .borrow()
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .upgrade()
        .unwrap();
    assert_eq!(parent, overflow.clone().upcast::<Visual>());
    measure(&overflow, Size::new(70.0, 50.0));
    assert_eq!(child.last_constraint(), Size::new(70.0, 50.0));
    assert_eq!(
        overflow.page_layout().embedded_controls(Point::default())[0]
            .1
            .width,
        70.0
    );
    assert_eq!(wide.embedded_controls(Point::default())[0].1.width, 90.0);
    assert_eq!(text.shaper().shape_call_count(), calls);
    let mut original_width = None;
    let _fork = head
        .next
        .previous_line_break
        .as_ref()
        .unwrap()
        .fork_with_object_runs(|run| {
            if run.is::<EmbeddedControlRun>() {
                original_width = Some(run.as_drawable().unwrap().size().width);
            }
            None
        });
    assert_eq!(original_width, Some(50.0));
}

#[test]
fn rich_hosted_overflow_does_not_remeasure_objects_before_its_start() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    view.set_max_lines(1);
    let paragraph = Paragraph::new();
    let first = object(&paragraph, Size::new(200.0, 10.0));
    paragraph.add_child(LineBreak::new());
    let second = object(&paragraph, Size::new(200.0, 10.0));
    view.blocks().add(paragraph);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, Size::new(50.0, 11.0));
    let before = view.page_layout();
    measure(&overflow, Size::new(90.0, 30.0));
    assert_eq!(
        first.constraints.borrow().as_slice(),
        &[Size::new(50.0, 11.0)]
    );
    assert_eq!(second.last_constraint(), Size::new(90.0, 30.0));
    assert_eq!(before.embedded_controls(Point::default())[0].1.width, 50.0);
    assert_eq!(first.visual_parent(), Some(view.clone().upcast()));
    assert_eq!(second.visual_parent(), Some(overflow.clone().upcast()));
}

#[test]
fn rich_hosted_full_page_does_not_measure_the_following_paragraph() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    view.set_max_lines(1);
    view.blocks().add(Paragraph::with_text("first"));
    let paragraph = Paragraph::new();
    paragraph.set_margin(Thickness::new(0.0, 45.0, 0.0, 20.0));
    let child = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, Size::new(100.0, 100.0));
    assert!(child.constraints.borrow().is_empty());
    measure(&overflow, Size::new(80.0, 60.0));
    assert_eq!(child.last_constraint(), Size::new(80.0, 60.0));
    close(overflow.line_metrics()[0].bounds.y, 0.0);
    close(overflow.page_layout().size.height, 20.0);
}

#[test]
fn rich_hosted_geometry_only_queries_never_measure_or_reparent() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph);
    measure(&view, Size::new(100.0, 60.0));
    let other = Control::new();
    let visual = child.clone().upcast::<Visual>();
    view.visual_children().remove(&visual);
    other.visual_children().add(visual);
    view.invalidate_measure();
    let calls = child.constraints.borrow().len();
    assert_eq!(view.get_rects_for_range(0..1).len(), 1);
    let _ = view.get_rect_from_character_index(0);
    let _ = view.line_metrics();
    assert_eq!(child.constraints.borrow().len(), calls);
    assert_eq!(child.visual_parent(), Some(other.clone().upcast()));
    // The actual next measure owns the object again, even if its size is equal.
    measure(&view, Size::new(100.0, 60.0));
    assert_eq!(child.visual_parent(), Some(view.clone().upcast()));
}

#[test]
fn rich_hosted_reentrant_document_replacement_retires_the_outer_measure() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let first = object(&paragraph, Size::new(20.0, 20.0));
    let second = object(&paragraph, Size::new(20.0, 20.0));
    let weak = view.downgrade();
    *first.on_measure.borrow_mut() = Some(Rc::new(move |_| {
        let view = weak.upgrade().unwrap();
        view.blocks().clear();
        view.blocks().add(Paragraph::with_text("replacement"));
    }));
    view.blocks().add(paragraph);
    view.measure(Size::new(100.0, 100.0));
    assert_eq!(view.text(), "replacement");
    assert!(view.cached_page_layout().is_none());
    assert!(second.constraints.borrow().is_empty());
    measure(&view, Size::new(100.0, 100.0));
    assert_eq!(view.content_end(), 11);
    assert!(view
        .page_layout()
        .embedded_controls(Point::default())
        .is_empty());
}

#[test]
fn rich_hosted_measure_callback_can_read_layout_without_nested_measurement() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    let weak = view.downgrade();
    *child.on_measure.borrow_mut() = Some(Rc::new(move |_| {
        let view = weak.upgrade().unwrap();
        let _ = view.line_metrics();
    }));
    view.blocks().add(paragraph);
    measure(&view, Size::new(100.0, 100.0));
    assert_eq!(child.constraints.borrow().len(), 1);
    assert_eq!(
        view.page_layout().embedded_controls(Point::default())[0]
            .1
            .size(),
        Size::new(20.0, 20.0)
    );
}

#[test]
fn rich_hosted_measure_callback_ownership_transfer_is_not_reclaimed_by_outer_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let other = Control::new();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    let weak_other = other.downgrade();
    *child.on_measure.borrow_mut() = Some(Rc::new(move |child| {
        let visual = child.to_ref().upcast::<Visual>();
        visual
            .visual_parent()
            .unwrap()
            .visual_children()
            .remove(&visual);
        weak_other.upgrade().unwrap().visual_children().add(visual);
    }));
    view.blocks().add(paragraph);
    view.measure(Size::new(100.0, 100.0));
    assert_eq!(child.visual_parent(), Some(other.upcast()));
    assert!(view.cached_page_layout().is_none());
}

#[test]
fn rich_hosted_geometry_reuses_last_host_measure_after_independent_child_measure() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(200.0, 20.0));
    view.blocks().add(paragraph);
    measure(&view, Size::new(100.0, 60.0));
    let original = view.page_layout();
    child.measure(Size::new(40.0, 60.0));
    assert_eq!(child.desired_size().width, 40.0);
    view.invalidate_measure();
    let read = view.page_layout();
    assert_eq!(read.embedded_controls(Point::default())[0].1.width, 100.0);
    // Selection/format changes rebuild source runs. The cache belongs to the
    // container, so rebuilding cannot adopt the unrelated 40-wide measure.
    view.invalidate_document_layout();
    let rebuilt = view.page_layout();
    assert!(!Rc::ptr_eq(&read.source, &rebuilt.source));
    assert_eq!(
        rebuilt.embedded_controls(Point::default())[0].1.width,
        100.0
    );
    assert_eq!(
        original.embedded_controls(Point::default())[0].1.width,
        100.0
    );
    assert_eq!(child.last_constraint(), Size::new(40.0, 60.0));
    measure(&view, Size::new(80.0, 60.0));
    assert_eq!(child.last_constraint(), Size::new(80.0, 60.0));
    assert_eq!(
        view.page_layout().embedded_controls(Point::default())[0]
            .1
            .width,
        80.0
    );
}

#[test]
fn rich_hosted_detach_callback_transfer_retires_measure_without_reclaiming_child() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph);
    let previous = Control::new();
    let successor = Control::new();
    let visual = child.clone().upcast::<Visual>();
    view.visual_children().remove(&visual);
    previous.visual_children().add(visual);
    let weak = successor.downgrade();
    *child.on_visual_detach.borrow_mut() = Some(Rc::new(move |child| {
        weak.upgrade()
            .unwrap()
            .visual_children()
            .add(child.to_ref().upcast());
    }));
    view.measure(Size::new(100.0, 60.0));
    assert_eq!(child.visual_parent(), Some(successor.upcast()));
    assert!(child.constraints.borrow().is_empty());
    assert!(view.cached_page_layout().is_none());
}

#[test]
fn rich_hosted_detach_callback_transfer_aborts_arrange_synchronization() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph);
    view.measure(Size::new(100.0, 60.0));
    let previous = Control::new();
    let successor = Control::new();
    let visual = child.clone().upcast::<Visual>();
    view.visual_children().remove(&visual);
    previous.visual_children().add(visual);
    let weak = successor.downgrade();
    *child.on_visual_detach.borrow_mut() = Some(Rc::new(move |child| {
        weak.upgrade()
            .unwrap()
            .visual_children()
            .add(child.to_ref().upcast());
    }));
    view.arrange(Rect::new(0.0, 0.0, 100.0, 60.0));
    assert_eq!(child.visual_parent(), Some(successor.upcast()));
    assert_eq!(child.arrangements.get(), 0);
    assert!(view.cached_page_layout().is_none());
}

#[test]
fn rich_hosted_arrange_callback_replacement_stops_retired_children() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let first = object(&paragraph, Size::new(20.0, 20.0));
    let second = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph);
    view.measure(Size::new(100.0, 60.0));
    let weak = view.downgrade();
    *first.on_arrange.borrow_mut() = Some(Rc::new(move |_| {
        let view = weak.upgrade().unwrap();
        view.blocks().clear();
        view.blocks().add(Paragraph::with_text("replacement"));
    }));
    view.arrange(Rect::new(0.0, 0.0, 100.0, 60.0));
    assert_eq!(view.text(), "replacement");
    assert_eq!(first.arrangements.get(), 1);
    assert_eq!(second.arrangements.get(), 0);
    assert!(first.visual_parent().is_none());
    assert!(second.visual_parent().is_none());
    assert!(view.cached_page_layout().is_none());
}

#[test]
fn rich_hosted_arrange_callback_transfer_stops_retired_children() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let first = object(&paragraph, Size::new(20.0, 20.0));
    let second = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph);
    view.measure(Size::new(100.0, 60.0));
    let successor = Control::new();
    let weak = successor.downgrade();
    *first.on_arrange.borrow_mut() = Some(Rc::new(move |child| {
        let visual = child.to_ref().upcast::<Visual>();
        visual
            .visual_parent()
            .unwrap()
            .visual_children()
            .remove(&visual);
        weak.upgrade().unwrap().visual_children().add(visual);
    }));
    view.arrange(Rect::new(0.0, 0.0, 100.0, 60.0));
    assert_eq!(first.visual_parent(), Some(successor.upcast()));
    assert_eq!(first.arrangements.get(), 1);
    assert_eq!(second.arrangements.get(), 0);
    assert!(view.cached_page_layout().is_none());
}

#[test]
fn rich_hosted_arrange_property_callback_cannot_install_retired_objects() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    view.set_max_lines(1);
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    paragraph.add_child(Run::with_text(Some(" tail")));
    view.blocks().add(paragraph);
    view.measure(Size::new(100.0, 60.0));
    let weak = view.downgrade();
    let _subscription = view.property_changed(move |change| {
        if change.property().name() == "HasOverflowContent" {
            let view = weak.upgrade().unwrap();
            view.blocks().clear();
            view.blocks().add(Paragraph::with_text("replacement"));
        }
    });
    view.arrange(Rect::new(0.0, 0.0, 20.0, 60.0));
    assert_eq!(view.text(), "replacement");
    assert!(child.visual_parent().is_none());
    assert_eq!(child.arrangements.get(), 0);
    assert!(view.cached_page_layout().is_none());
}

#[test]
fn rich_hosted_child_replacement_preserves_logical_detach_visual_transfer() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = HostedProbe::new(Size::new(20.0, 20.0));
    let inline = InlineUIContainer::with_child(child.clone());
    paragraph.add_child(inline.clone());
    view.blocks().add(paragraph);
    let successor = Control::new();
    let weak_successor = successor.downgrade();
    let weak_child = child.downgrade();
    let _subscription = child.property_changed(move |change| {
        if change.property().name() == "Parent" {
            let child = weak_child.upgrade().unwrap();
            if child.parent().is_none() {
                let visual = child.upcast::<Visual>();
                visual
                    .visual_parent()
                    .unwrap()
                    .visual_children()
                    .remove(&visual);
                weak_successor
                    .upgrade()
                    .unwrap()
                    .visual_children()
                    .add(visual);
            }
        }
    });
    let replacement = HostedProbe::new(Size::new(25.0, 25.0));
    inline.set_child(replacement.clone());
    assert_eq!(child.visual_parent(), Some(successor.upcast()));
    assert_eq!(replacement.visual_parent(), Some(view.upcast()));
}

#[test]
fn rich_hosted_child_replacement_callback_supersedes_the_outer_setter() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = HostedProbe::new(Size::new(20.0, 20.0));
    let inline = InlineUIContainer::with_child(child.clone());
    paragraph.add_child(inline.clone());
    view.blocks().add(paragraph);
    let nested = HostedProbe::new(Size::new(30.0, 30.0));
    let weak_inline = inline.downgrade();
    let weak_child = child.downgrade();
    let weak_nested = nested.downgrade();
    let _subscription = child.property_changed(move |change| {
        if change.property().name() == "Parent" && weak_child.upgrade().unwrap().parent().is_none()
        {
            weak_inline
                .upgrade()
                .unwrap()
                .set_child(weak_nested.upgrade().unwrap());
        }
    });
    let outer = HostedProbe::new(Size::new(25.0, 25.0));
    inline.set_child(outer.clone());
    assert_eq!(inline.child(), Some(nested.clone().upcast()));
    assert_eq!(nested.parent(), Some(inline.upcast()));
    assert_eq!(nested.visual_parent(), Some(view.upcast()));
    assert!(outer.parent().is_none());
    assert!(outer.visual_parent().is_none());
    assert!(child.visual_parent().is_none());
}

#[test]
fn rich_hosted_inline_host_transition_preserves_detach_callback_transfer() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph.clone());
    let successor = Control::new();
    let weak = successor.downgrade();
    *child.on_visual_detach.borrow_mut() = Some(Rc::new(move |child| {
        weak.upgrade()
            .unwrap()
            .visual_children()
            .add(child.to_ref().upcast());
    }));
    view.blocks().clear();
    let next = rich();
    next.blocks().add(paragraph);
    assert_eq!(child.visual_parent(), Some(successor.upcast()));
    assert!(next.visual_children().is_empty());
}

#[test]
fn rich_hosted_replacing_overflow_child_detaches_its_actual_visual_owner_immediately() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    view.set_max_lines(1);
    view.blocks().add(Paragraph::with_text("head"));
    let paragraph = Paragraph::new();
    let child = HostedProbe::new(Size::new(20.0, 20.0));
    let inline = InlineUIContainer::with_child(child.clone());
    paragraph.add_child(inline.clone());
    view.blocks().add(paragraph);
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, Size::new(100.0, 100.0));
    measure(&overflow, Size::new(100.0, 100.0));
    assert_eq!(child.visual_parent(), Some(overflow.clone().upcast()));
    let replacement = HostedProbe::new(Size::new(25.0, 25.0));
    inline.set_child(replacement.clone());
    assert!(child.visual_parent().is_none());
    assert!(!overflow
        .visual_children()
        .contains(&child.upcast::<Visual>()));
    measure(&view, Size::new(100.0, 100.0));
    measure(&overflow, Size::new(100.0, 100.0));
    assert_eq!(replacement.visual_parent(), Some(overflow.upcast()));
    assert_eq!(replacement.bounds().size(), Size::new(25.0, 25.0));
}

#[test]
fn rich_hosted_read_cache_is_remeasured_even_with_an_unchanged_constraint() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let paragraph = Paragraph::new();
    let child = object(&paragraph, Size::new(20.0, 20.0));
    view.blocks().add(paragraph);
    let initial = view.page_layout();
    let initial_size = initial.embedded_controls(Point::default())[0].1.size();
    assert!(child.constraints.borrow().is_empty());
    // The view's default constraint is already unbounded.
    view.measure(Size::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(
        child.last_constraint(),
        Size::new(f64::INFINITY, f64::INFINITY)
    );
    assert!(!Rc::ptr_eq(&initial, &view.page_layout()));
    assert_eq!(
        initial.embedded_controls(Point::default())[0].1.size(),
        initial_size
    );
    assert_eq!(
        view.page_layout().embedded_controls(Point::default())[0]
            .1
            .size(),
        Size::new(20.0, 20.0)
    );
}

#[test]
fn rich_hosted_editor_preserves_additive_paragraph_spacing() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let editor = RichEditBox::new();
    editor.set_font_size(10.0);
    let first = Paragraph::with_text("first");
    first.set_margin(Thickness::new(0.0, 7.0, 0.0, 11.0));
    let second = Paragraph::with_text("second");
    second.set_margin(Thickness::new(0.0, 13.0, 0.0, 17.0));
    editor.document().blocks().add(first);
    editor.document().blocks().add(second);
    let view = editor.text_presenter();
    measure(&view, Size::new(100.0, 200.0));
    assert!(!view.page_layout().source.collapse_paragraph_margins);
    close(view.line_metrics()[0].bounds.y, 7.0);
    close(view.line_metrics()[1].bounds.y, 42.0);
    close(view.page_layout().size.height, 70.0);
}
