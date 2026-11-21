use super::{
    cad_drawing_canvas::CadDrawingCanvas,
    cad_plan::{Entity, Plan},
};
use ferroui_base::{Point, Rect, Size, threading::CancellationToken};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use serde_json::Value;
fn source() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/cad-canvas.json")).unwrap()
}
fn number(actual: f64, expected: &Value) {
    let bits = expected.as_u64().unwrap();
    let expected = f64::from_bits(bits);
    if expected.is_nan() {
        assert!(actual.is_nan());
    } else {
        assert_eq!(
            actual.to_bits(),
            bits,
            "actual={actual:?} expected={expected:?}"
        );
    }
}
fn point(actual: Point, expected: &Value) {
    number(actual.x, &expected[0]);
    number(actual.y, &expected[1]);
}
#[test]
fn original_floor_plan_entities_texts_and_order_match_unchanged_source() {
    let source = source();
    let plan = Plan::new();
    assert_eq!(
        plan.entities.len(),
        source["entities"].as_array().unwrap().len()
    );
    assert_eq!(plan.texts.len(), source["texts"].as_array().unwrap().len());
    for (e, row) in plan
        .entities
        .iter()
        .zip(source["entities"].as_array().unwrap())
    {
        let key = e.key();
        assert_eq!(key.0, row["layer"].as_str().unwrap());
        assert_eq!(key.1, row["fill"].as_bool().unwrap());
        assert_eq!(key.2, row["dashed"].as_bool().unwrap());
        match e {
            Entity::Poly { points, closed, .. } => {
                assert_eq!(row["kind"], "PolyEntity");
                assert_eq!(*closed, row["closed"].as_bool().unwrap());
                assert_eq!(points.len(), row["points"].as_array().unwrap().len());
                for (p, expected) in points.iter().zip(row["points"].as_array().unwrap()) {
                    point(Point::new(p.0, p.1), expected);
                }
            }
            Entity::Arc {
                center,
                radius,
                start,
                sweep,
                ..
            } => {
                assert_eq!(row["kind"], "ArcEntity");
                point(Point::new(center.0, center.1), &row["center"]);
                number(*radius, &row["radius"]);
                number(*start, &row["start"]);
                number(*sweep, &row["sweep"]);
            }
        }
    }
    for (t, row) in plan.texts.iter().zip(source["texts"].as_array().unwrap()) {
        point(Point::new(t.point.0, t.point.1), &row["point"]);
        assert_eq!(t.text, row["text"].as_str().unwrap());
        assert_eq!(t.layer, row["layer"].as_str().unwrap());
        number(t.height, &row["height"]);
        number(t.angle, &row["angle"]);
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
#[test]
fn native_canvas_fit_zoom_and_zero_height_lifecycle_match_source() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                let source = source();
                let view = CadDrawingCanvas::new();
                let check = |v: &CadDrawingCanvas, row: &Value| {
                    number(v.view_scale(), &row["scale"]);
                    point(v.center.get(), &row["center"]);
                    assert_eq!(v.fitted.get(), row["fitted"].as_bool().unwrap());
                };
                check(&view, &source["snapshots"][0]);
                view.measure(Size::new(1200., 700.));
                view.arrange(Rect::new(0., 0., 1200., 700.));
                check(&view, &source["snapshots"][1]);
                view.zoom(1.15, Some(Point::new(321.25, 210.5)));
                check(&view, &source["snapshots"][5]);
                let low = CadDrawingCanvas::new();
                low.measure(Size::new(100., 0.));
                low.arrange(Rect::new(0., 0., 100., 0.));
                check(&low, &source["lifecycle"][0]);
                low.measure(Size::new(100., 300.));
                low.arrange(Rect::new(0., 0., 100., 300.));
                check(&low, &source["lifecycle"][1]);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

use ferroui_base::{
    Vector,
    media::{ArcSegment, IBrush, LineSegment, PathGeometry},
    styling::ThemeVariant,
};
use ferroui_controls::{
    Canvas, Control, TextBlock, Window,
    shapes::{Ellipse, Line, Path, Rectangle, Shape},
};
use ferroui_headless::HeadlessWindowExtensions;
use std::rc::Rc;
fn brush(actual: Option<Rc<dyn IBrush>>, expected: &Value) {
    if expected.is_null() {
        assert!(actual.is_none());
    } else {
        let color = actual.unwrap().as_solid_color_brush().unwrap().color();
        assert_eq!(
            serde_json::json!([color.a, color.r, color.g, color.b]),
            *expected
        );
    }
}
fn common(actual: &Control, expected: &Value) {
    let common = &expected["common"];
    assert_eq!(actual.is_visible(), common["visible"].as_bool().unwrap());
    number(Canvas::get_left(actual), &common["left"]);
    number(Canvas::get_top(actual), &common["top"]);
    number(actual.width(), &common["width"]);
    number(actual.height(), &common["height"]);
}
fn shape(actual: &Shape, expected: &Value) {
    brush(actual.fill(), &expected["fill"]);
    brush(actual.stroke(), &expected["stroke"]);
    number(actual.stroke_thickness(), &expected["thickness"]);
    let actual = actual.stroke_dash_array();
    if expected["dashes"].is_null() {
        assert!(actual.is_none());
    } else {
        let actual = actual.unwrap();
        let values = expected["dashes"].as_array().unwrap();
        assert_eq!(actual.len(), values.len());
        for (actual, expected) in actual.iter().zip(values) {
            number(actual, expected);
        }
    }
}
fn element(actual: &ferroui_base::Ref<Control>, expected: &Value) {
    let kind = expected["common"]["kind"].as_str().unwrap();
    let details = &expected["details"];
    if let Some(text) = actual.cast::<TextBlock>() {
        assert_eq!(kind, "TextBlock");
        assert_eq!(text.text().as_deref(), details["text"].as_str());
        number(text.font_size(), &details["size"]);
        brush(text.foreground(), &details["foreground"]); // The original measures a newly created, detached block before adding it.
        let detached = TextBlock::new();
        detached.set_text(text.text().as_deref());
        detached.set_font_size(text.font_size());
        detached.set_font_family(text.font_family());
        detached.measure(Size::new(f64::INFINITY, f64::INFINITY));
        let desired = detached.desired_size();
        number(
            Canvas::get_left(&text),
            &Value::from(
                (f64::from_bits(details["anchor"][0].as_u64().unwrap()) - desired.width / 2.)
                    .to_bits(),
            ),
        );
        number(
            Canvas::get_top(&text),
            &Value::from(
                (f64::from_bits(details["anchor"][1].as_u64().unwrap()) - desired.height / 2.)
                    .to_bits(),
            ),
        );
        if f64::from_bits(details["angle"].as_u64().unwrap()) != 0. {
            assert_eq!(
                text.render_transform_origin(),
                ferroui_base::RelativePoint::TOP_LEFT
            );
        }
        return;
    }
    common(actual, expected);
    match kind {
        "Path" => {
            let path = actual.cast::<Path>().unwrap();
            shape(&path, details);
            let geometry = path.data().unwrap().cast::<PathGeometry>().unwrap();
            let figures = geometry.figures().unwrap();
            let expected = details["figures"].as_array().unwrap();
            assert_eq!(figures.len(), expected.len());
            for (f, row) in figures.iter().zip(expected) {
                point(f.start_point(), &row["start"]);
                assert_eq!(f.is_closed(), row["closed"].as_bool().unwrap());
                assert_eq!(f.is_filled(), row["filled"].as_bool().unwrap());
                let segments = f.segments().unwrap();
                let expected = row["segments"].as_array().unwrap();
                assert_eq!(segments.len(), expected.len());
                for (s, row) in segments.iter().zip(expected) {
                    if row["kind"] == "line" {
                        let l = s.cast::<LineSegment>().unwrap();
                        point(l.point(), &row["point"]);
                    } else {
                        let a = s.cast::<ArcSegment>().unwrap();
                        point(a.point(), &row["point"]);
                        number(a.size().width, &row["size"][0]);
                        number(a.size().height, &row["size"][1]);
                        assert_eq!(a.is_large_arc(), row["large"].as_bool().unwrap());
                        assert_eq!(
                            a.sweep_direction() as i32,
                            row["sweep"].as_i64().unwrap() as i32
                        );
                    }
                }
            }
        }
        "Line" => {
            let line = actual.cast::<Line>().unwrap();
            point(line.start_point(), &details["start"]);
            point(line.end_point(), &details["end"]);
            brush(line.stroke(), &details["stroke"]);
            number(line.stroke_thickness(), &details["thickness"]);
        }
        "Ellipse" => shape(&actual.cast::<Ellipse>().unwrap(), details),
        "Rectangle" => shape(&actual.cast::<Rectangle>().unwrap(), details),
        _ => panic!("unmapped source visual {kind}"),
    }
}
fn snapshot(view: &CadDrawingCanvas, row: &Value) {
    eprintln!("checking {}", row["name"]);
    number(view.bounds().width, &row["width"]);
    number(view.bounds().height, &row["height"]);
    number(view.view_scale(), &row["scale"]);
    point(view.center.get(), &row["center"]);
    assert_eq!(view.fitted.get(), row["fitted"].as_bool().unwrap());
    if row["pan"].is_null() {
        assert!(view.pan_start.get().is_none());
    } else {
        point(view.pan_start.get().unwrap(), &row["pan"]);
    }
    point(view.cursor_position.get(), &row["cursor"]);
    brush(view.background(), &row["background"]);
    assert_eq!(view.is_grid_visible(), row["grid"].as_bool().unwrap());
    let layers = row["layers"].as_array().unwrap();
    for (layer, expected) in [
        view.grid_layer.clone(),
        view.drawing.clone(),
        view.overlay.clone(),
    ]
    .iter()
    .zip(layers)
    {
        let expected = expected.as_array().unwrap();
        assert_eq!(layer.children().count(), expected.len(), "{}", row["name"]);
        for (i, row) in expected.iter().enumerate() {
            element(&layer.children().get(i), row);
        }
    }
}
#[test]
fn source_geometry_theme_selection_crosshair_and_native_pointer_pipeline() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result=session.dispatch(||{
 ferroribbon::register_types();let source=source();let rows=source["snapshots"].as_array().unwrap();let view=CadDrawingCanvas::new();view.set_use_layout_rounding(false);let events=Rc::new(std::cell::RefCell::new(Vec::new()));let observed=events.clone();let weak=view.downgrade();view.add_cursor_moved(Some(ferroui_base::metadata::MarkupDelegate::new(move|args|{let v=weak.upgrade().unwrap();let sender=ferroui_base::data::core::ValueTypes::as_object(&**args[0].as_ref().unwrap()).unwrap();assert!(sender.ptr_eq(&v));let point=*args[1].as_ref().unwrap().downcast_ref::<Point>().unwrap();let cursor=v.cursor_position.get();observed.borrow_mut().push(serde_json::json!({"point":[point.x.to_bits(),point.y.to_bits()],"cursor":[cursor.x.to_bits(),cursor.y.to_bits()],"scale":v.view_scale().to_bits()}));None})));snapshot(&view,&rows[0]);view.measure(Size::new(1200.,700.));view.arrange(Rect::new(0.,0.,1200.,700.));snapshot(&view,&rows[1]);
 let window=Window::new();window.set_width(1200.);window.set_height(700.);window.set_content(Some(Control::boxed(view.clone())));window.set_use_layout_rounding(false);window.show();window.update_layout();snapshot(&view,&rows[2]);window.set_requested_theme_variant(Some(ThemeVariant::dark()));snapshot(&view,&rows[3]);view.set_crosshair_cursor(Point::new(20.5,31.5));snapshot(&view,&rows[4]);view.zoom(1.15,Some(Point::new(321.25,210.5)));snapshot(&view,&rows[5]);
 window.mouse_wheel(Point::new(321.25,210.5),Vector::new(0.,0.),ferroui_base::input::RawInputModifiers::NONE);snapshot(&view,&rows[6]);window.mouse_down(Point::new(100.,200.),ferroui_base::input::MouseButton::Middle,ferroui_base::input::RawInputModifiers::NONE);window.mouse_move(Point::new(130.,245.),ferroui_base::input::RawInputModifiers::MIDDLE_MOUSE_BUTTON);snapshot(&view,&rows[7]);window.mouse_up(Point::new(130.,245.),ferroui_base::input::MouseButton::Middle,ferroui_base::input::RawInputModifiers::NONE);window.mouse_move(Point::new(200.,210.),ferroui_base::input::RawInputModifiers::NONE);snapshot(&view,&rows[8]);view.set_is_pan_mode(true);window.mouse_down(Point::new(30.,45.),ferroui_base::input::MouseButton::Left,ferroui_base::input::RawInputModifiers::NONE);window.mouse_move(Point::new(80.,90.),ferroui_base::input::RawInputModifiers::LEFT_MOUSE_BUTTON);window.mouse_up(Point::new(80.,90.),ferroui_base::input::MouseButton::Left,ferroui_base::input::RawInputModifiers::NONE);snapshot(&view,&rows[9]);view.hidden_layers().borrow_mut().insert("A-FURN".to_owned());view.set_show_lineweights(true);view.set_is_grid_visible(false);view.redraw();snapshot(&view,&rows[10]);window.set_width(650.5);ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);window.set_height(340.25);ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);window.update_layout();snapshot(&view,&rows[11]);view.set_view_insets(ferroui_base::Thickness::new(-10.5,100.,650.,340.));view.zoom_extents();snapshot(&view,&rows[12]);window.mouse_move(Point::new(-10.,-10.),ferroui_base::input::RawInputModifiers::NONE);snapshot(&view,&rows[13]);assert_eq!(*events.borrow(),source["events"].as_array().unwrap().clone());window.close();
 },CancellationToken::none()).wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn filled_path_default_is_scoped_and_keeps_native_style_priority() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                let shape = super::cad_drawing_canvas::DrawingPath::new();
                assert_eq!(shape.stroke_thickness(), 1.);
                assert_eq!(Path::new().stroke_thickness(), 0.);
                assert!(
                    shape
                        .get_local_value(Shape::stroke_thickness_property())
                        .is_none()
                );
                let style = shape
                    .set_value_with_priority(
                        Shape::stroke_thickness_property(),
                        3.,
                        ferroui_base::data::BindingPriority::Style,
                    )
                    .unwrap();
                assert_eq!(shape.stroke_thickness(), 3.);
                style.dispose();
                assert_eq!(shape.stroke_thickness(), 1.);
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn source_clamps_extreme_factors_and_nan_without_drawing_area() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                let source = source();
                for row in source["math"].as_array().unwrap() {
                    let view = CadDrawingCanvas::new();
                    let a = &row["anchor"];
                    view.zoom(
                        f64::from_bits(row["factor"].as_u64().unwrap()),
                        Some(Point::new(
                            f64::from_bits(a[0].as_u64().unwrap()),
                            f64::from_bits(a[1].as_u64().unwrap()),
                        )),
                    );
                    number(view.view_scale(), &row["scale"]);
                    point(view.center.get(), &row["center"]);
                    point(view.model_to_screen(6000., 3600.), &row["probe"]);
                }
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
#[test]
fn native_theme_mapping_respects_inherited_light_variant() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                let view = CadDrawingCanvas::new();
                let window = Window::new();
                window.set_width(1200.);
                window.set_height(700.);
                window.set_requested_theme_variant(Some(ThemeVariant::new(
                    "CustomLight",
                    Some(ThemeVariant::light()),
                )));
                window.set_content(Some(Control::boxed(view.clone())));
                window.show();
                window.update_layout();
                assert_eq!(
                    view.canvas_color(),
                    ferroui_base::media::Color::from_rgb(0xFA, 0xFB, 0xFC)
                );
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
#[test]
fn cursor_reentrancy_unsubscribe_and_release_follow_source_trace() {
    let session = HeadlessUnitTestSession::start_new(build);
    let result=session.dispatch(||{ferroribbon::register_types();let source=source();let view=CadDrawingCanvas::new();let window=Window::new();window.set_width(1200.);window.set_height(700.);window.set_content(Some(Control::boxed(view.clone())));window.show();window.update_layout();let events=Rc::new(std::cell::RefCell::new(Vec::new()));let observed=events.clone();let weak=view.downgrade();let second=ferroui_base::metadata::MarkupDelegate::new(move|args|{let v=weak.upgrade().unwrap();let point=*args[1].as_ref().unwrap().downcast_ref::<Point>().unwrap();let cursor=v.cursor_position.get();let sender=ferroui_base::data::core::ValueTypes::as_object(&**args[0].as_ref().unwrap()).unwrap();observed.borrow_mut().push(serde_json::json!({"name":"second","point":[point.x.to_bits(),point.y.to_bits()],"cursor":[cursor.x.to_bits(),cursor.y.to_bits()],"scale":v.view_scale().to_bits(),"senderSame":sender.ptr_eq(&v)}));None});let observed=events.clone();let weak=view.downgrade();let remove=second.clone();let first=ferroui_base::metadata::MarkupDelegate::new(move|args|{let v=weak.upgrade().unwrap();let point=*args[1].as_ref().unwrap().downcast_ref::<Point>().unwrap();let cursor=v.cursor_position.get();let sender=ferroui_base::data::core::ValueTypes::as_object(&**args[0].as_ref().unwrap()).unwrap();observed.borrow_mut().push(serde_json::json!({"name":"first","point":[point.x.to_bits(),point.y.to_bits()],"cursor":[cursor.x.to_bits(),cursor.y.to_bits()],"scale":v.view_scale().to_bits(),"senderSame":sender.ptr_eq(&v)}));v.remove_cursor_moved(Some(remove.clone()));v.zoom(1.01,None);None});view.add_cursor_moved(Some(first.clone()));view.add_cursor_moved(Some(second));window.mouse_move(Point::new(300.,200.),ferroui_base::input::RawInputModifiers::NONE);window.mouse_move(Point::new(310.,205.),ferroui_base::input::RawInputModifiers::NONE);assert_eq!(*events.borrow(),source["reentrant"].as_array().unwrap().clone());view.remove_cursor_moved(Some(first));let weak=view.downgrade();window.set_content(None);window.close();drop(view);ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);assert!(weak.upgrade().is_none(),"Native events and theme callbacks must not retain the canvas");},CancellationToken::none()).wait();
    session.dispose();
    result.unwrap();
}

#[test]
fn compiled_and_runtime_canvas_constructor_properties_and_grid_children() {
    use crate::ASSEMBLY;
    use ferroui_base::metadata::MarkupAssembly;
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            || {
                ferroribbon::register_types();
                MarkupAssembly::register(&ASSEMBLY);
                ferroui_base::TypeInfo::register_namespaces(&[(
                    "ferroribbon_sample::cad::cad_drawing_canvas",
                    "FerroRibbon.Sample.Cad",
                )]);
                ferroui_base::TypeInfo::register_all(&[CadDrawingCanvas::TYPE]);
                crate::generated::compiled_markup::register();
                let compiled = crate::generated::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_sample/CadCanvas.xaml",
                )
                .unwrap()
                .unwrap();
                let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("../CadCanvas.xaml"),
                    Some(&ASSEMBLY),
                )
                .unwrap();
                for value in [compiled, runtime] {
                    let view = ferroui_base::data::core::ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<CadDrawingCanvas>()
                        .unwrap();
                    assert_eq!(
                        view.view_insets(),
                        ferroui_base::Thickness::new(41., 31., 151., 131.)
                    );
                    assert!(view.clip_to_bounds());
                    assert!(view.is_pan_mode());
                    assert!(!view.is_grid_visible());
                    assert!(view.show_lineweights());
                    assert_eq!(view.children().count(), 4);
                    let extra = ferroui_base::controls::NameScopeExtensions::find::<
                        ferroui_controls::Border,
                    >(&view, "ExtraOverlay")
                    .unwrap();
                    view.measure(Size::new(1200., 700.));
                    view.arrange(Rect::new(0., 0., 1200., 700.));
                    view.redraw();
                    assert!(view.children().get(3).ptr_eq(&extra));
                }
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
