use super::*;
use ferroui_base::input::PointerType;
use ferroui_base::{Point, Rect, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn event(id: i32, x: f64, y: f64) -> InkPointerEventArgs {
    InkPointerEventArgs::new(
        id,
        PointerType::Pen,
        InkPoint::new(Point::new(x, y), 0.5).unwrap(),
    )
}

#[test]
fn eraser_release_callback_can_start_a_new_stroke_with_the_same_pointer() {
    let p = InkPresenter::new();
    draw(&p, 1, 100.0);
    p.input_processing_configuration()
        .set_mode(InkInputProcessingMode::Erasing);
    let weak = Rc::downgrade(&p);
    p.strokes_erased(move |_| {
        let p = weak.upgrade().unwrap();
        p.input_processing_configuration()
            .set_mode(InkInputProcessingMode::Inking);
        p.process_pointer(InkPointerPhase::Pressed, &event(2, 500.0, 500.0))
            .unwrap();
    });
    p.process_pointer(InkPointerPhase::Pressed, &event(2, 0.0, 20.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Released, &event(2, 130.0, 20.0))
        .unwrap();
    assert!(p.has_active_pointer(2));
    assert!(p.stroke_container().is_empty());
    p.process_pointer(InkPointerPhase::Released, &event(2, 510.0, 500.0))
        .unwrap();
    assert_eq!(p.stroke_container().len(), 1);
    assert_eq!(
        p.stroke_container().get_strokes()[0].ink_points()[0].position,
        Point::new(500.0, 500.0)
    );
}

#[test]
fn duplicate_press_respects_input_disabled_by_the_canceled_handler() {
    let presenter = InkPresenter::new();
    presenter
        .process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    let weak = Rc::downgrade(&presenter);
    presenter.stroke_input().stroke_canceled(move |_| {
        weak.upgrade().unwrap().set_is_input_enabled(false);
    });

    assert!(!presenter
        .process_pointer(InkPointerPhase::Pressed, &event(1, 50.0, 0.0))
        .unwrap());
    assert!(!presenter.is_input_enabled());
    assert!(!presenter.has_active_pointer(1));
    assert!(presenter.visible_strokes().is_empty());
    assert!(presenter.stroke_container().is_empty());
}

#[test]
fn duplicate_press_preserves_a_reentrant_replacement_with_the_same_id() {
    let presenter = InkPresenter::new();
    presenter
        .set_predefined_configuration(InkPresenterPredefinedConfiguration::SimpleMultiplePointer);
    presenter
        .process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    let weak = Rc::downgrade(&presenter);
    presenter.stroke_input().stroke_canceled(move |_| {
        weak.upgrade()
            .unwrap()
            .process_pointer(InkPointerPhase::Pressed, &event(1, 1000.0, 1000.0))
            .unwrap();
    });

    presenter
        .process_pointer(InkPointerPhase::Pressed, &event(1, 50.0, 0.0))
        .unwrap();
    assert!(presenter.has_active_pointer(1));
    presenter
        .process_pointer(InkPointerPhase::Released, &event(1, 1001.0, 1000.0))
        .unwrap();
    let strokes = presenter.stroke_container().get_strokes();
    assert_eq!(strokes.len(), 1);
    assert_eq!(
        strokes[0].ink_points()[0].position,
        Point::new(1000.0, 1000.0)
    );
}

#[test]
fn backwards_input_timestamp_is_rejected_before_preview_becomes_invalid() {
    let presenter = InkPresenter::new();
    let mut sample = event(1, 0.0, 0.0);
    sample.point.timestamp = 100;
    presenter
        .process_pointer(InkPointerPhase::Pressed, &sample)
        .unwrap();
    assert_eq!(presenter.visible_strokes().len(), 1);
    sample.point.timestamp = 99;
    sample.point.position.x = 1.0;
    assert_eq!(
        presenter.process_pointer(InkPointerPhase::Moved, &sample),
        Err(InkError::InvalidInput(
            "Ink timestamps must be nondecreasing"
        ))
    );
    assert!(!presenter.has_active_pointer(1));
    assert!(presenter.visible_strokes().is_empty());
    assert!(presenter.stroke_container().is_empty());
}

#[test]
fn exact_sample_limit_accepts_an_identical_release_and_rejects_an_extra_point() {
    let p = InkPresenter::new();
    let limit = InkPersistenceLimits::DEFAULT.max_points_per_stroke;
    // Time-varying stationary samples exercise the raw input budget separately
    // from the rendered geometry budget.
    let sample = |id, timestamp: usize| {
        let mut e = event(id, 0.0, 0.0);
        e.point.timestamp = timestamp as u64;
        e
    };
    p.process_pointer(InkPointerPhase::Pressed, &sample(1, 0))
        .unwrap();
    for i in 1..limit {
        p.process_pointer(InkPointerPhase::Moved, &sample(1, i))
            .unwrap();
    }
    p.process_pointer(InkPointerPhase::Released, &sample(1, limit - 1))
        .unwrap();
    assert_eq!(
        p.stroke_container().get_strokes()[0].ink_points().len(),
        limit
    );
    p.process_pointer(InkPointerPhase::Pressed, &sample(2, 0))
        .unwrap();
    for i in 1..limit {
        p.process_pointer(InkPointerPhase::Moved, &sample(2, i))
            .unwrap();
    }
    assert_eq!(
        p.process_pointer(InkPointerPhase::Released, &sample(2, limit)),
        Err(InkError::LimitExceeded)
    );
    assert!(!p.has_active_pointer(2));
    assert_eq!(p.stroke_container().len(), 1);
}

#[test]
fn active_geometry_limit_reports_cancellation_before_preview_can_disappear() {
    let p = InkPresenter::new();
    let cancellations = Rc::new(Cell::new(0));
    let count = cancellations.clone();
    p.stroke_input()
        .stroke_canceled(move |_| count.set(count.get() + 1));
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    for i in 1..2048 {
        p.process_pointer(InkPointerPhase::Moved, &event(1, i as f64 * 32.0, 0.0))
            .unwrap();
    }
    assert_eq!(p.visible_strokes().len(), 1);
    assert_eq!(
        p.process_pointer(InkPointerPhase::Moved, &event(1, 2048.0 * 32.0, 0.0)),
        Err(InkError::LimitExceeded)
    );
    assert_eq!(cancellations.get(), 1);
    assert!(!p.has_active_pointer(1));
    assert!(p.visible_strokes().is_empty());
    assert!(p.stroke_container().is_empty());
}

#[test]
fn multi_pointer_admission_and_total_active_geometry_are_bounded() {
    let p = InkPresenter::new();
    p.set_predefined_configuration(InkPresenterPredefinedConfiguration::SimpleMultiplePointer);
    for id in 0..InkPresenter::MAX_ACTIVE_POINTERS as i32 {
        p.process_pointer(InkPointerPhase::Pressed, &event(id, 0.0, 0.0))
            .unwrap();
    }
    assert_eq!(
        p.process_pointer(InkPointerPhase::Pressed, &event(256, 0.0, 0.0)),
        Err(InkError::LimitExceeded)
    );
    assert!(p.has_active_pointer(255));
    p.cancel_all();
    for id in 0..4 {
        p.process_pointer(InkPointerPhase::Pressed, &event(id, 0.0, id as f64))
            .unwrap();
        for i in 1..2048 {
            p.process_pointer(
                InkPointerPhase::Moved,
                &event(id, i as f64 * 32.0, id as f64),
            )
            .unwrap();
        }
    }
    p.process_pointer(InkPointerPhase::Pressed, &event(4, 0.0, 4.0))
        .unwrap();
    for i in 1..4 {
        p.process_pointer(InkPointerPhase::Moved, &event(4, i as f64 * 32.0, 4.0))
            .unwrap();
    }
    assert_eq!(
        p.process_pointer(InkPointerPhase::Moved, &event(4, 128.0, 4.0)),
        Err(InkError::LimitExceeded)
    );
    assert!(!p.has_active_pointer(4));
    assert!((0..4).all(|id| p.has_active_pointer(id)));
    p.cancel_all();
}

#[test]
fn total_active_telemetry_is_bounded_even_when_stationary_geometry_is_shared() {
    let presenter = InkPresenter::new();
    presenter
        .set_predefined_configuration(InkPresenterPredefinedConfiguration::SimpleMultiplePointer);
    let limits = InkPersistenceLimits::DEFAULT;
    let sample = |id, timestamp: usize| {
        let mut sample = event(id, id as f64, 0.0);
        sample.point.timestamp = timestamp as u64;
        sample
    };
    let mut remaining = limits.max_total_points;
    let mut id = 0;
    let mut last_count = 0;
    while remaining != 0 {
        let count = remaining.min(limits.max_points_per_stroke);
        presenter
            .process_pointer(InkPointerPhase::Pressed, &sample(id, 0))
            .unwrap();
        for timestamp in 1..count {
            presenter
                .process_pointer(InkPointerPhase::Moved, &sample(id, timestamp))
                .unwrap();
        }
        remaining -= count;
        last_count = count;
        id += 1;
    }
    assert_eq!(id, 16);
    assert!(last_count < limits.max_points_per_stroke);
    assert_eq!(
        presenter.process_pointer(InkPointerPhase::Pressed, &sample(id, 0)),
        Err(InkError::LimitExceeded)
    );
    assert!(!presenter.has_active_pointer(id));
    // A duplicate consumes no space even when the aggregate budget is full.
    presenter
        .process_pointer(InkPointerPhase::Moved, &sample(id - 1, last_count - 1))
        .unwrap();
    assert_eq!(
        presenter.process_pointer(InkPointerPhase::Moved, &sample(id - 1, last_count)),
        Err(InkError::LimitExceeded)
    );
    assert!(!presenter.has_active_pointer(id - 1));
    assert!((0..id - 1).all(|other| presenter.has_active_pointer(other)));
    assert!(presenter.stroke_container().is_empty());
    presenter.cancel_all();
}

#[test]
fn markup_unprocessed_event_preserves_shared_handled_state() {
    use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupTyped};
    crate::register_types();
    let p = InkPresenter::new();
    p.input_processing_configuration()
        .set_mode(InkInputProcessingMode::None);
    let input = p.unprocessed_input();
    let pressed = <InkUnprocessedInput as MarkupTyped>::MARKUP
        .find_event("PointerPressed")
        .unwrap();
    let handled = <InkPointerEventArgs as MarkupTyped>::MARKUP
        .find_property("Handled")
        .unwrap();
    let handler = MarkupDelegate::new(move |args| {
        (handled.set.unwrap())(&[args[1].clone(), into_markup_value(true)]).unwrap();
        None
    });
    (pressed.add)(&[into_markup_value(input), into_markup_value(handler)]).unwrap();
    let first = event(1, 0.0, 0.0);
    assert!(p.process_pointer(InkPointerPhase::Pressed, &first).unwrap());
    assert!(first.handled());
    assert!(p.stroke_container().is_empty());
}
fn draw(presenter: &InkPresenter, id: i32, x: f64) {
    presenter
        .process_pointer(InkPointerPhase::Pressed, &event(id, x, 20.0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Moved, &event(id, x + 10.0, 20.0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Released, &event(id, x + 20.0, 20.0))
        .unwrap();
}

#[test]
fn presenter_defaults_and_services_match_the_contract() {
    let p = InkPresenter::new();
    assert_eq!(p.input_device_types(), CoreInputDeviceTypes::PEN);
    assert!(p.is_input_enabled());
    assert_eq!(
        p.input_processing_configuration().mode(),
        InkInputProcessingMode::Inking
    );
    assert_eq!(
        p.input_processing_configuration().right_drag_action(),
        InkInputRightDragAction::LeaveUnprocessed
    );
    assert!(p.input_configuration().is_eraser_input_enabled());
    assert!(p
        .input_configuration()
        .is_primary_barrel_button_input_enabled());
    assert!(Rc::ptr_eq(&p.stroke_container(), &p.stroke_container()));
    assert!(Rc::ptr_eq(&p.stroke_input(), &p.stroke_input()));
    assert!(Rc::ptr_eq(&p.unprocessed_input(), &p.unprocessed_input()));
    assert!(Rc::ptr_eq(&p.stroke_input().ink_presenter().unwrap(), &p));
}

#[test]
fn weak_service_back_pointer_does_not_keep_presenter_alive() {
    let p = InkPresenter::new();
    let service = p.stroke_input();
    let unprocessed = p.unprocessed_input();
    drop(p);
    assert!(service.ink_presenter().is_none());
    assert!(unprocessed.ink_presenter().is_none());
}

#[test]
fn collected_follows_ended_and_uses_a_snapshot_of_attributes() {
    let p = InkPresenter::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    p.stroke_input()
        .stroke_started(move |_| log.borrow_mut().push("start"));
    let log = events.clone();
    p.stroke_input()
        .stroke_continued(move |_| log.borrow_mut().push("move"));
    let log = events.clone();
    p.stroke_input()
        .stroke_ended(move |_| log.borrow_mut().push("end"));
    let log = events.clone();
    p.strokes_collected(move |_| log.borrow_mut().push("collect"));
    let original = p.copy_default_drawing_attributes();
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    let mut changed = original;
    changed.size = Size::new(17.0, 17.0);
    p.update_default_drawing_attributes(&changed).unwrap();
    p.process_pointer(InkPointerPhase::Moved, &event(1, 10.0, 0.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Released, &event(1, 20.0, 0.0))
        .unwrap();
    assert_eq!(&*events.borrow(), &["start", "move", "end", "collect"]);
    assert_eq!(
        p.stroke_container().get_strokes()[0].drawing_attributes(),
        original
    );
    draw(&p, 2, 30.0);
    assert_eq!(
        p.stroke_container().get_strokes()[1].drawing_attributes(),
        changed
    );
}

#[test]
fn mouse_is_opt_in_and_unprocessed_secondary_input_can_be_handled() {
    let p = InkPresenter::new();
    let mut mouse = event(1, 1.0, 2.0);
    mouse.device_type = PointerType::Mouse;
    assert!(!p.process_pointer(InkPointerPhase::Pressed, &mouse).unwrap());
    p.set_input_device_types(CoreInputDeviceTypes::PEN | CoreInputDeviceTypes::MOUSE);
    mouse.is_right_button_pressed = true;
    p.unprocessed_input()
        .pointer_pressed(|e| e.set_handled(true));
    assert!(p.process_pointer(InkPointerPhase::Pressed, &mouse).unwrap());
    assert!(!p.has_active_pointer(1));
    p.input_processing_configuration()
        .set_right_drag_action(InkInputRightDragAction::AllowProcessing);
    assert!(p.process_pointer(InkPointerPhase::Pressed, &mouse).unwrap());
    assert!(p.has_active_pointer(1));
}

#[test]
fn disabled_barrel_and_eraser_inputs_are_ignored() {
    let p = InkPresenter::new();
    p.input_configuration().set_is_eraser_input_enabled(false);
    p.input_configuration()
        .set_is_primary_barrel_button_input_enabled(false);
    let mut e = event(1, 0.0, 0.0);
    e.is_eraser = true;
    assert!(!p.process_pointer(InkPointerPhase::Pressed, &e).unwrap());
    e.is_eraser = false;
    e.is_barrel_button_pressed = true;
    assert!(!p.process_pointer(InkPointerPhase::Pressed, &e).unwrap());
    assert!(!p.has_active_pointer(1));
}

#[test]
fn unprocessed_pointer_loss_is_delivered() {
    let p = InkPresenter::new();
    p.input_processing_configuration()
        .set_mode(InkInputProcessingMode::None);
    let lost = Rc::new(Cell::new(false));
    let target = lost.clone();
    p.unprocessed_input()
        .pointer_lost(move |_| target.set(true));
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Lost, &event(1, 0.0, 0.0))
        .unwrap();
    assert!(lost.get());
}

#[test]
fn cancel_disable_and_mode_switch_never_commit_partial_strokes() {
    let p = InkPresenter::new();
    let canceled = Rc::new(Cell::new(0));
    let count = canceled.clone();
    p.stroke_input()
        .stroke_canceled(move |_| count.set(count.get() + 1));
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    p.cancel_pointer(1);
    p.process_pointer(InkPointerPhase::Pressed, &event(2, 0.0, 0.0))
        .unwrap();
    p.set_is_input_enabled(false);
    p.set_is_input_enabled(true);
    p.process_pointer(InkPointerPhase::Pressed, &event(3, 0.0, 0.0))
        .unwrap();
    p.input_processing_configuration()
        .set_mode(InkInputProcessingMode::None);
    p.process_pointer(InkPointerPhase::Released, &event(3, 50.0, 0.0))
        .unwrap();
    assert_eq!(canceled.get(), 3);
    assert!(p.stroke_container().get_strokes().is_empty());
}

#[test]
fn reentrant_end_handler_can_cancel_collection() {
    let p = InkPresenter::new();
    let weak = Rc::downgrade(&p);
    p.stroke_input()
        .stroke_ended(move |_| weak.upgrade().unwrap().set_is_input_enabled(false));
    draw(&p, 1, 0.0);
    assert!(p.stroke_container().get_strokes().is_empty());
}

#[test]
fn reentrant_collected_handler_can_clear_container_and_draw_again() {
    let p = InkPresenter::new();
    let weak = Rc::downgrade(&p);
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    p.strokes_collected(move |_| {
        let p = weak.upgrade().unwrap();
        count.set(count.get() + 1);
        if count.get() == 1 {
            p.stroke_container().clear();
            draw(&p, 2, 100.0);
        }
    });
    draw(&p, 1, 0.0);
    assert_eq!(calls.get(), 2);
    assert_eq!(p.stroke_container().get_strokes().len(), 1);
}

#[test]
fn multi_pointer_mode_keeps_samples_separate() {
    let p = InkPresenter::new();
    p.set_predefined_configuration(InkPresenterPredefinedConfiguration::SimpleMultiplePointer);
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Pressed, &event(2, 100.0, 100.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Released, &event(1, 10.0, 0.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Released, &event(2, 110.0, 100.0))
        .unwrap();
    let strokes = p.stroke_container().get_strokes();
    assert_eq!(strokes.len(), 2);
    assert_eq!(strokes[0].ink_points()[0].position.x, 0.0);
    assert_eq!(strokes[1].ink_points()[0].position.x, 100.0);
}

#[test]
fn sparse_eraser_motion_crosses_strokes_and_reports_removed_snapshot() {
    let p = InkPresenter::new();
    draw(&p, 1, 0.0);
    let erased = Rc::new(Cell::new(0));
    let count = erased.clone();
    p.strokes_erased(move |e| count.set(count.get() + e.strokes().len()));
    p.input_processing_configuration()
        .set_mode(InkInputProcessingMode::Erasing);
    p.process_pointer(InkPointerPhase::Pressed, &event(2, 10.0, -100.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Released, &event(2, 10.0, 100.0))
        .unwrap();
    assert_eq!(erased.get(), 1);
    assert!(p.stroke_container().get_strokes().is_empty());
}

#[test]
fn invalid_sample_cancels_pending_input_without_committing() {
    let p = InkPresenter::new();
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    let mut invalid = event(1, 10.0, 0.0);
    invalid.point.position.x = f64::NAN;
    assert!(p.process_pointer(InkPointerPhase::Moved, &invalid).is_err());
    assert!(!p.has_active_pointer(1));
    assert!(p.stroke_container().get_strokes().is_empty());
}

#[test]
fn custom_drying_rejects_late_activation_and_is_idempotent() {
    let p = InkPresenter::new();
    let sync = p.activate_custom_drying().unwrap();
    assert!(Rc::ptr_eq(&sync, &p.activate_custom_drying().unwrap()));
    p.set_size(Size::new(100.0, 100.0));
    assert!(p.activate_custom_drying().is_ok());
    let p = InkPresenter::new();
    p.set_size(Size::new(100.0, 100.0));
    assert!(p.activate_custom_drying().is_err());
}

#[test]
fn drying_transaction_does_not_consume_strokes_arriving_after_begin() {
    let p = InkPresenter::new();
    let sync = p.activate_custom_drying().unwrap();
    draw(&p, 1, 0.0);
    let first = sync.begin_dry().unwrap();
    assert_eq!(first.len(), 1);
    assert!(sync.begin_dry().is_err());
    draw(&p, 2, 100.0);
    assert_eq!(p.visible_strokes().len(), 2);
    sync.end_dry().unwrap();
    let visible = p.visible_strokes();
    assert_eq!(visible.len(), 1);
    assert_ne!(visible[0].id(), first[0].id());
    assert_eq!(p.stroke_container().get_strokes().len(), 2);
    assert!(sync.end_dry().is_err());
    assert_eq!(sync.begin_dry().unwrap().len(), 1);
    sync.end_dry().unwrap();
    assert!(p.visible_strokes().is_empty());
}

#[test]
fn clearing_container_removes_pending_custom_dry_ink() {
    let p = InkPresenter::new();
    let sync = p.activate_custom_drying().unwrap();
    draw(&p, 1, 0.0);
    assert_eq!(sync.begin_dry().unwrap().len(), 1);
    p.stroke_container().clear();
    assert!(p.visible_strokes().is_empty());
    sync.end_dry().unwrap();
}

#[test]
fn presenter_preview_is_reused_between_input_events() {
    let p = InkPresenter::new();
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 0.0, 0.0))
        .unwrap();
    assert!(Rc::ptr_eq(&p.visible_strokes()[0], &p.visible_strokes()[0]));
}

#[test]
fn stencils_are_exclusive_and_constrain_ink_in_local_coordinates() {
    let p = InkPresenter::new();
    p.set_ruler_visible(true);
    p.process_pointer(InkPointerPhase::Pressed, &event(1, 10.0, 3.0))
        .unwrap();
    p.process_pointer(InkPointerPhase::Released, &event(1, 100.0, 4.0))
        .unwrap();
    assert_eq!(
        p.stroke_container().get_strokes()[0].ink_points()[0]
            .position
            .y,
        0.0
    );
    p.set_protractor_visible(true);
    assert!(!p.ruler_visible());
    assert!(p.protractor_visible());
    assert!(p.ruler().set_length(f64::NAN).is_err());
    assert!(p.protractor().set_radius(-1.0).is_err());
}

#[test]
fn ink_canvas_participates_in_layout_and_uses_a_stable_presenter() {
    let _scope = crate::test_support::test_scope();
    let canvas = InkCanvas::new();
    let p = canvas.ink_presenter();
    assert!(Rc::ptr_eq(&p, &canvas.ink_presenter()));
    canvas.measure(Size::new(300.0, 200.0));
    canvas.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
    assert_eq!(canvas.bounds().size(), Size::new(300.0, 200.0));
    assert!(p.activate_custom_drying().is_err());
    let second = InkCanvas::new();
    assert!(!Rc::ptr_eq(&p, &second.ink_presenter()));
}

#[test]
fn canvas_peer_is_a_pane_and_respects_ancestor_clipping() {
    let _scope = crate::test_support::test_scope();
    let canvas = InkCanvas::new();
    canvas.set_width(300.0);
    canvas.set_height(200.0);
    let root = crate::test_support::TestRoot::with_child(canvas.clone());
    root.set_clip_to_bounds(true);
    root.measure(Size::new(100.0, 80.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 80.0));
    let peer = InkCanvasAutomationPeer::new(&canvas);
    assert_eq!(
        peer.get_automation_control_type(),
        ferroui_controls::automation::peers::AutomationControlType::Pane
    );
    let bounds = peer.get_bounding_rectangle();
    assert!(bounds.width <= 100.0);
    assert!(bounds.height <= 80.0);
    root.set_is_visible(false);
    assert!(peer.is_offscreen());
}

#[test]
fn nominal_pressure_draws_the_configured_width() {
    let builder = InkStrokeBuilder::new();
    builder
        .set_default_drawing_attributes(InkDrawingAttributes {
            size: Size::new(4.0, 4.0),
            ..InkDrawingAttributes::default()
        })
        .unwrap();
    // The builder's points have the nominal pressure 0.5, as mouse and touch input.
    let stroke = builder.create_stroke([Point::new(10.0, 10.0)]).unwrap();
    let bounds = stroke.bounding_rect();
    assert!((bounds.width - 4.0).abs() < 1.0e-9, "{bounds:?}");
    assert!((bounds.height - 4.0).abs() < 1.0e-9, "{bounds:?}");
    let full = InkStroke::new(
        vec![InkPoint::new(Point::new(10.0, 10.0), 1.0).unwrap()],
        builder.default_drawing_attributes(),
    )
    .unwrap();
    assert!(full.bounding_rect().width > bounds.width);
}
