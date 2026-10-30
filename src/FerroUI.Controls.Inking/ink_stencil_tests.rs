//! Presenter integration for masking, circular tracing, and custom drying.

use super::*;
use ferroui_base::input::PointerType;
use ferroui_base::{Point, Rect, Size};
use std::cell::Cell;
use std::rc::Rc;

fn point(id: i32, x: f64, y: f64, timestamp: u64) -> InkPointerEventArgs {
    let mut point = InkPoint::new(Point::new(x, y), 0.75).unwrap();
    point.timestamp = timestamp;
    InkPointerEventArgs::new(id, PointerType::Pen, point)
}

fn ruler_presenter() -> Rc<InkPresenter> {
    let presenter = InkPresenter::new();
    presenter.ruler().set_length(300.0).unwrap();
    presenter.ruler().set_is_visible(true);
    presenter
}

#[test]
fn sparse_stroke_across_ruler_collects_two_outside_fragments_in_one_event() {
    let presenter = ruler_presenter();
    let events = Rc::new(Cell::new(0));
    let count = events.clone();
    presenter.strokes_collected(move |args| {
        assert_eq!(args.strokes().len(), 2);
        count.set(count.get() + 1);
    });
    presenter
        .process_pointer(InkPointerPhase::Pressed, &point(1, -30.0, 60.0, 0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Released, &point(1, 330.0, 60.0, 360))
        .unwrap();
    let strokes = presenter.stroke_container().get_strokes();
    assert_eq!(events.get(), 1);
    assert_eq!(strokes.len(), 2);
    assert_eq!(strokes[0].ink_points()[0].position, Point::new(-30.0, 60.0));
    assert_eq!(strokes[0].ink_points()[1].position, Point::new(0.0, 60.0));
    assert_eq!(strokes[1].ink_points()[0].position, Point::new(300.0, 60.0));
    assert_eq!(strokes[1].ink_points()[1].position, Point::new(330.0, 60.0));
    assert_eq!(strokes[0].ink_points()[1].timestamp, 30);
    assert_eq!(strokes[1].ink_points()[0].timestamp, 330);
    assert!(strokes.iter().all(|stroke| !stroke
        .bounding_rect()
        .intersects(Rect::new(10.0, 10.0, 280.0, 100.0))));
}

#[test]
fn interior_only_ink_has_no_preview_or_collection_but_balanced_input_events() {
    let presenter = ruler_presenter();
    let starts = Rc::new(Cell::new(0));
    let ends = Rc::new(Cell::new(0));
    let collections = Rc::new(Cell::new(0));
    let target = starts.clone();
    presenter
        .stroke_input()
        .stroke_started(move |_| target.set(target.get() + 1));
    let target = ends.clone();
    presenter
        .stroke_input()
        .stroke_ended(move |_| target.set(target.get() + 1));
    let target = collections.clone();
    presenter.strokes_collected(move |_| target.set(target.get() + 1));
    presenter
        .process_pointer(InkPointerPhase::Pressed, &point(1, 100.0, 60.0, 0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Moved, &point(1, 150.0, 60.0, 10))
        .unwrap();
    assert!(presenter.visible_strokes().is_empty());
    presenter
        .process_pointer(InkPointerPhase::Released, &point(1, 200.0, 60.0, 20))
        .unwrap();
    assert!(presenter.stroke_container().get_strokes().is_empty());
    assert_eq!((starts.get(), ends.get(), collections.get()), (1, 1, 0));
}

#[test]
fn cancellation_drops_all_preview_fragments_and_drying_commits_them_together() {
    let presenter = ruler_presenter();
    let synchronizer = presenter.activate_custom_drying().unwrap();
    presenter
        .process_pointer(InkPointerPhase::Pressed, &point(1, -30.0, 60.0, 0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Moved, &point(1, 150.0, 60.0, 180))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Moved, &point(1, 330.0, 60.0, 360))
        .unwrap();
    assert_eq!(presenter.visible_strokes().len(), 2);
    presenter.cancel_all();
    assert!(presenter.visible_strokes().is_empty());
    assert!(presenter.stroke_container().get_strokes().is_empty());
    presenter
        .process_pointer(InkPointerPhase::Pressed, &point(2, -30.0, 60.0, 0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Moved, &point(2, 150.0, 60.0, 180))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Released, &point(2, 330.0, 60.0, 360))
        .unwrap();
    assert_eq!(presenter.visible_strokes().len(), 2);
    let dry = synchronizer.begin_dry().unwrap();
    assert_eq!(dry.len(), 2);
    synchronizer.end_dry().unwrap();
    assert!(presenter.visible_strokes().is_empty());
    assert_eq!(presenter.stroke_container().get_strokes().len(), 2);
}

#[test]
fn sparse_protractor_trace_follows_circle_instead_of_becoming_an_interior_chord() {
    let presenter = InkPresenter::new();
    let guide = presenter.protractor();
    guide.set_radius(100.0).unwrap();
    guide.set_is_visible(true);
    presenter
        .process_pointer(InkPointerPhase::Pressed, &point(1, 203.0, 100.0, 0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Released, &point(1, 100.0, 203.0, 100))
        .unwrap();
    let strokes = presenter.stroke_container().get_strokes();
    assert_eq!(strokes.len(), 1);
    assert!(strokes[0].ink_points().len() > 2);
    assert!(strokes[0].ink_points().len() <= 257);
    assert_eq!(strokes[0].ink_points().first().unwrap().timestamp, 0);
    assert_eq!(strokes[0].ink_points().last().unwrap().timestamp, 100);
    for point in strokes[0].ink_points() {
        assert!(((point.position.x - 100.0).hypot(point.position.y - 100.0) - 100.0).abs() < 1e-9);
    }
    assert!((guide.angle().unwrap() - 90.0).abs() < 1e-9);
}

#[test]
fn hover_and_eraser_do_not_change_arc_measurement_or_snap_eraser_motion() {
    let presenter = InkPresenter::new();
    let guide = presenter.protractor();
    guide.set_radius(100.0).unwrap();
    guide.set_is_visible(true);
    presenter
        .process_pointer(InkPointerPhase::Pressed, &point(1, 203.0, 100.0, 0))
        .unwrap();
    presenter
        .process_pointer(InkPointerPhase::Released, &point(1, 100.0, 203.0, 10))
        .unwrap();
    let angle = guide.angle();
    presenter
        .process_pointer(InkPointerPhase::Moved, &point(1, 0.0, 100.0, 20))
        .unwrap();
    assert_eq!(guide.angle(), angle);
    let target = InkStroke::new(
        vec![InkPoint::new(Point::new(100.0, 218.0), 1.0).unwrap()],
        InkDrawingAttributes {
            size: Size::new(1.0, 1.0),
            ..Default::default()
        },
    )
    .unwrap();
    presenter
        .stroke_container()
        .add_stroke(target.clone())
        .unwrap();
    presenter
        .input_processing_configuration()
        .set_mode(InkInputProcessingMode::Erasing);
    presenter
        .process_pointer(InkPointerPhase::Pressed, &point(2, 100.0, 210.0, 30))
        .unwrap();
    assert!(presenter
        .stroke_container()
        .get_stroke_by_id(target.id())
        .is_none());
    assert_eq!(guide.angle(), angle);
    presenter
        .process_pointer(InkPointerPhase::Released, &point(2, 100.0, 210.0, 40))
        .unwrap();
    assert_eq!(guide.angle(), angle);
}
