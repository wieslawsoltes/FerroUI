use crate::presenters::ScrollContentPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_scrolling::TestScrollable;
use crate::{Control, Panel, ScrollViewer};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{FerroObjectExtensions, Ref, Size, Vector};
use std::cell::Cell;
use std::rc::Rc;

fn create_target(content: &Ref<TestScrollable>) -> (Ref<ScrollViewer>, Ref<TestRoot>) {
    let result = ScrollViewer::new();
    result.set_content(Some(Control::boxed(content)));
    result.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })));

    let root = TestRoot::with_child(&result);
    root.layout_manager().execute_initial_layout_pass();

    (result, root)
}

#[test]
fn extent_offset_and_viewport_should_be_read_from_i_logical_scrollable() {
    let _scope = test_scope();
    let scrollable =
        TestScrollable::with_state(Size::new(100.0, 100.0), Vector::new(50.0, 50.0), Size::new(25.0, 25.0));

    let (target, _root) = create_target(&scrollable);

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());

    scrollable.set_extent(Size::new(200.0, 200.0));
    scrollable.set_offset(Vector::new(100.0, 100.0));
    scrollable.set_viewport(Size::new(50.0, 50.0));

    assert_eq!(scrollable.extent(), target.extent());
    assert_eq!(scrollable.offset(), target.offset());
    assert_eq!(scrollable.viewport(), target.viewport());
}

#[test]
fn logical_metrics_transactions_only_write_genuine_final_offset_corrections() {
    let _scope = test_scope();
    let selected = Vector::new(75.0, 75.0);
    let scrollable = TestScrollable::with_state(Size::new(100.0, 100.0), selected, Size::new(25.0, 25.0));
    let (target, _root) = create_target(&scrollable);
    assert_eq!(target.offset(), selected);
    scrollable.take_offset_writes();

    // Either publication order has an invalid intermediate range in one of
    // these transitions. The completed child state keeps the same legal end
    // offset; a transient presenter clamp must never become a child selection.
    for (extent, viewport) in [(275.0, 200.0), (125.0, 50.0)] {
        scrollable.publish_scroll_state(Size::new(extent, extent), selected, Size::new(viewport, viewport));
        assert_eq!(target.extent(), scrollable.extent());
        assert_eq!(target.viewport(), scrollable.viewport());
        assert_eq!(target.offset(), selected);
        assert_eq!(scrollable.offset(), selected);
        assert!(scrollable.take_offset_writes().is_empty());
    }

    // This final range genuinely excludes 75. The presenter can already have
    // been coerced to 30 while publishing the extent, so reconciliation must
    // not depend on the final offset property emitting another change.
    let clamped = Vector::new(30.0, 30.0);
    scrollable.publish_scroll_state(Size::new(80.0, 80.0), selected, Size::new(50.0, 50.0));
    assert_eq!(target.offset(), clamped);
    assert_eq!(scrollable.offset(), clamped);
    assert_eq!(scrollable.take_offset_writes(), vec![clamped]);

    // A real property observer may publish a newer child state while metrics
    // are being forwarded through the owning ScrollViewer. The older outer
    // snapshot must not overwrite it or issue any transient scroll request.
    let nested = Rc::new(Cell::new(false));
    let nested_seen = nested.clone();
    let nested_scrollable = scrollable.clone();
    let observed_target: &ferroui_base::FerroObject = &target;
    let _viewport = FerroObjectExtensions::get_observable(observed_target, ScrollViewer::viewport_property())
        .subscribe_fn(move |viewport| {
            if viewport == Size::new(200.0, 200.0) && !nested_seen.replace(true) {
                nested_scrollable.publish_scroll_state(
                    Size::new(350.0, 350.0),
                    Vector::new(50.0, 50.0),
                    Size::new(300.0, 300.0),
                );
            }
        });
    scrollable.publish_scroll_state(Size::new(275.0, 275.0), selected, Size::new(200.0, 200.0));
    assert!(nested.get(), "the viewport notification reentered the logical child");
    assert_eq!(target.extent(), Size::new(350.0, 350.0));
    assert_eq!(target.viewport(), Size::new(300.0, 300.0));
    assert_eq!(target.offset(), Vector::new(50.0, 50.0));
    assert_eq!(scrollable.offset(), target.offset());
    assert!(scrollable.take_offset_writes().is_empty());

    // Final offset publication is also observable. An older in-flight offset
    // notification must not overwrite a newer child transaction from one of
    // those observers after the metric setters themselves have completed.
    let offset_nested = Rc::new(Cell::new(false));
    let offset_nested_seen = offset_nested.clone();
    let offset_scrollable = scrollable.clone();
    let _offset = FerroObjectExtensions::get_observable(observed_target, ScrollViewer::offset_property())
        .subscribe_fn(move |offset| {
            if offset == Vector::new(80.0, 80.0) && !offset_nested_seen.replace(true) {
                offset_scrollable.publish_scroll_state(
                    Size::new(360.0, 360.0),
                    Vector::new(120.0, 120.0),
                    Size::new(240.0, 240.0),
                );
            }
        });
    scrollable.publish_scroll_state(
        Size::new(200.0, 200.0),
        Vector::new(80.0, 80.0),
        Size::new(100.0, 100.0),
    );
    assert!(offset_nested.get(), "the offset notification reentered the logical child");
    assert_eq!(target.extent(), Size::new(360.0, 360.0));
    assert_eq!(target.viewport(), Size::new(240.0, 240.0));
    assert_eq!(target.offset(), Vector::new(120.0, 120.0));
    assert_eq!(scrollable.offset(), target.offset());
    assert!(scrollable.take_offset_writes().is_empty());
}

#[test]
fn child_owned_overscroll_survives_unchanged_scroll_metrics() {
    let _scope = test_scope();
    let extent = Size::new(100.0, 100.0);
    let viewport = Size::new(25.0, 25.0);
    let scrollable = TestScrollable::with_state(extent, Vector::default(), viewport);
    let (target, _root) = create_target(&scrollable);
    scrollable.take_offset_writes();

    // A wrapping carousel retains negative fractional progress while its
    // owning ScrollViewer stays at zero. Repeated notifications with unchanged
    // metrics must not turn that observation into an unsolicited scroll.
    for position in [-0.375, -0.75] {
        let progress = Vector::new(position, position);
        scrollable.publish_scroll_state(extent, progress, viewport);
        assert_eq!(Vector::default(), target.offset());
        assert_eq!(progress, scrollable.offset());
        assert!(scrollable.take_offset_writes().is_empty());
    }

    // A metric change alone is still not an owner scroll request when the
    // effective boundary offset remains zero.
    let progress = Vector::new(-0.75, -0.75);
    let enlarged_extent = Size::new(200.0, 200.0);
    let enlarged_viewport = Size::new(50.0, 50.0);
    scrollable.publish_scroll_state(enlarged_extent, progress, enlarged_viewport);
    assert_eq!(enlarged_extent, target.extent());
    assert_eq!(enlarged_viewport, target.viewport());
    assert_eq!(Vector::default(), target.offset());
    assert_eq!(progress, scrollable.offset());
    assert!(scrollable.take_offset_writes().is_empty());
    scrollable.publish_scroll_state(extent, progress, viewport);
    assert_eq!(progress, scrollable.offset());
    assert!(scrollable.take_offset_writes().is_empty());

    // An actual owner request still overrides the child-owned position.
    let requested = Vector::new(25.0, 25.0);
    target.set_offset(requested);
    assert_eq!(requested, scrollable.offset());
    assert_eq!(vec![requested], scrollable.take_offset_writes());

    // Unlike an unchanged boundary notification, this invalid child offset
    // changes the presenter's effective offset and must be written back once.
    scrollable.publish_scroll_state(extent, Vector::new(-0.375, -0.375), viewport);
    assert_eq!(Vector::default(), target.offset());
    assert_eq!(Vector::default(), scrollable.offset());
    assert_eq!(vec![Vector::default()], scrollable.take_offset_writes());

    // The same ownership rule applies beyond the other end of the range.
    let end = Vector::new(75.0, 75.0);
    target.set_offset(end);
    assert_eq!(vec![end], scrollable.take_offset_writes());
    let progress = Vector::new(75.375, 75.375);
    scrollable.publish_scroll_state(extent, progress, viewport);
    assert_eq!(end, target.offset());
    assert_eq!(progress, scrollable.offset());
    assert!(scrollable.take_offset_writes().is_empty());

    // A genuine range change still reconciles an invalid final position.
    let clamped = Vector::new(70.0, 70.0);
    scrollable.publish_scroll_state(Size::new(95.0, 95.0), progress, viewport);
    assert_eq!(clamped, target.offset());
    assert_eq!(clamped, scrollable.offset());
    assert_eq!(vec![clamped], scrollable.take_offset_writes());
}
