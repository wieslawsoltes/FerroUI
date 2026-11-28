//! Controls written in another language (docs/c-abi.md, section 15): classes registered at run
//! time, their callbacks, data, registered properties and drawing.

use super::support::*;
use crate::*;
use ferroui_base::animation::{Animation, FillMode, IClock, KeyFrame, PlayState, SignedDuration};
use ferroui_base::media::DrawingContext;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::rendering::composition::drawing::RenderDataDrawingContext;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{BoxedValue, FerroProperty, Ref};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use ferroui_controls::{Border, Control};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

fn register(name: &str, base: i32) -> i64 {
    // SAFETY: a pointer and the length of a Rust string.
    let id = unsafe { ferro_class_register(name.as_ptr().cast(), name.len() as i64, base) };
    assert!(id > 0, "{name}: {}", last_error());
    id
}

static CREATED: AtomicI64 = AtomicI64::new(0);
static RELEASED: AtomicI64 = AtomicI64::new(0);
static CHANGED: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn on_create(class_data: *mut c_void, object: FerroHandle) -> FerroStatus {
    CREATED.fetch_add(1, Ordering::Relaxed);
    // The data of the instance: a number the callbacks read back.
    let data = class_data as usize + 1;
    ferro_object_set_data(object, data as *mut c_void, Some(release))
}

unsafe extern "C" fn release(_data: *mut c_void) {
    RELEASED.fetch_add(1, Ordering::Relaxed);
}

unsafe extern "C" fn measure(
    data: *mut c_void,
    _object: FerroHandle,
    _w: f64,
    _h: f64,
    width: *mut f64,
    height: *mut f64,
) -> FerroStatus {
    // SAFETY: the out-parameters of the call.
    unsafe {
        *width = 40.0 + data as usize as f64;
        *height = 20.0;
    }
    0
}

unsafe extern "C" fn measure_by_call(
    _data: *mut c_void,
    _object: FerroHandle,
    _w: f64,
    _h: f64,
    _: *mut f64,
    _: *mut f64,
) -> FerroStatus {
    ferro_layout_result(33.0, 11.0)
}

unsafe extern "C" fn failing(
    _data: *mut c_void,
    _object: FerroHandle,
    _w: f64,
    _h: f64,
    _: *mut f64,
    _: *mut f64,
) -> FerroStatus {
    -42
}

unsafe extern "C" fn changed(_data: *mut c_void, _object: FerroHandle, property: i64) -> FerroStatus {
    CHANGED.store(property, Ordering::Relaxed);
    0
}

static BRUSH: AtomicI64 = AtomicI64::new(0);
static DRAWN: AtomicI64 = AtomicI64::new(0);
static PATH: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn render(_data: *mut c_void, _object: FerroHandle, context: i64) -> FerroStatus {
    let brush = BRUSH.load(Ordering::Relaxed);
    let mut code = ferro_draw_push_opacity(context, 0.5);
    code = code.min(ferro_draw_rect(context, 0.0, 0.0, 10.0, 10.0, 2.0, 2.0, brush, 0));
    code = code.min(ferro_draw_ellipse(context, 0.0, 0.0, 4.0, 4.0, brush, 0));
    // A geometry needs the render interface of a platform (an application has one; the tests
    // run without): the failure is a status, not an unwinding.
    // SAFETY: a NUL-terminated literal.
    let path = unsafe { ferro_draw_path(context, c"M 0,0 L 10,10 L 0,10 Z".as_ptr(), -1, brush, 0) };
    PATH.store(i64::from(path), Ordering::Relaxed);
    code = code.min(ferro_draw_pop(context));
    // Left pushed: popped after the callback.
    code = code.min(ferro_draw_push_clip(context, 0.0, 0.0, 5.0, 5.0));
    DRAWN.fetch_add(1, Ordering::Relaxed);
    code
}

#[test]
fn a_registered_class_is_created_by_name_and_is_of_its_type() {
    ui(|| {
        let class = register("CSwatch", FERRO_BASE_CONTROL);
        assert_eq!(type_id("CSwatch"), class);
        // SAFETY: a NUL-terminated literal.
        assert_eq!(
            unsafe { ferro_class_register(c"CSwatch".as_ptr(), -1, FERRO_BASE_CONTROL) },
            i64::from(FERRO_INVALID_STATE)
        );
        // SAFETY: as above.
        assert_eq!(unsafe { ferro_class_register(c"Bad".as_ptr(), -1, 99) }, i64::from(FERRO_INVALID_ARGUMENT));
        let object = ferro_create(class);
        assert_ne!(object, 0, "{}", last_error());
        assert_eq!(ferro_type_of(object), class);
        let rust = object_of(object).expect("the object");
        assert_eq!(rust.get_type().name(), "CSwatch");
        assert!(rust.is::<ferroui_controls::Control>());
        // Without callbacks the base measures: a control without a template is empty.
        assert_eq!(ferro_layout_measure(object, 100.0, 100.0), FERRO_OK);
        assert_eq!(ferro_layout_desired_width(object), 0.0);
        assert_eq!(ferro_release(object), FERRO_OK);
        // Off the UI thread.
        std::thread::spawn(|| {
            // SAFETY: a NUL-terminated literal.
            assert_eq!(unsafe { ferro_class_register(c"Other".as_ptr(), -1, 1) }, i64::from(FERRO_WRONG_THREAD));
        })
        .join()
        .expect("the thread runs");
    });
}

#[test]
fn callbacks_measure_with_the_data_of_the_instance_and_failures_fall_back_to_the_base() {
    ui(|| {
        let class = register("CMeasured", FERRO_BASE_CONTROL);
        assert_eq!(ferro_class_set_data(class, 4 as *mut c_void), FERRO_OK);
        assert_eq!(ferro_class_on_create(class, Some(on_create)), FERRO_OK);
        assert_eq!(ferro_class_on_measure(class, Some(measure)), FERRO_OK);
        let created = CREATED.load(Ordering::Relaxed);
        let object = ferro_create(class);
        assert_eq!(CREATED.load(Ordering::Relaxed), created + 1);
        assert_eq!(ferro_object_data(object) as usize, 5);
        assert_eq!(ferro_layout_measure(object, 100.0, 100.0), FERRO_OK);
        assert_eq!((ferro_layout_desired_width(object), ferro_layout_desired_height(object)), (45.0, 20.0));
        // A language that cannot write memory gives the size with a call.
        assert_eq!(ferro_class_on_measure(class, Some(measure_by_call)), FERRO_OK);
        let invalidate =
            object_of(object).and_then(|o| o.cast::<ferroui_base::layout::Layoutable>()).expect("layoutable");
        invalidate.invalidate_measure();
        assert_eq!(ferro_layout_measure(object, 100.0, 100.0), FERRO_OK);
        assert_eq!(ferro_layout_desired_width(object), 33.0);
        // A failing callback is recorded and the base measures.
        assert_eq!(ferro_class_on_measure(class, Some(failing)), FERRO_OK);
        invalidate.invalidate_measure();
        ferro_clear_error();
        assert_eq!(ferro_layout_measure(object, 100.0, 100.0), FERRO_OK);
        assert_eq!(ferro_layout_desired_width(object), 0.0);
        assert_eq!(ferro_last_error_code(), -42);
        assert!(last_error().contains("measure callback of the class CMeasured"), "{}", last_error());
        assert_eq!(ferro_layout_result(1.0, 1.0), FERRO_INVALID_STATE, "no callback runs");
        let released = RELEASED.load(Ordering::Relaxed);
        drop(invalidate);
        assert_eq!(ferro_release(object), FERRO_OK);
        assert_eq!(RELEASED.load(Ordering::Relaxed), released + 1, "the data is released with the instance");
    });
}

#[test]
fn registered_properties_are_styled_properties_of_their_class() {
    ui(|| {
        let class = register("CGauge", FERRO_BASE_CONTROL);
        let name = "Level";
        // SAFETY: pointers and lengths of Rust strings, a NULL default text.
        let level = unsafe {
            ferro_property_register(
                class,
                name.as_ptr().cast(),
                name.len() as i64,
                FERRO_SIGNAL_F64,
                0.25,
                std::ptr::null(),
                FERRO_PROPERTY_AFFECTS_MEASURE,
            )
        };
        assert!(level > 0, "{}", last_error());
        assert_eq!(level, property_id("Level"));
        // SAFETY: as above, a text default.
        let caption = unsafe {
            ferro_property_register(
                class,
                c"Caption".as_ptr(),
                -1,
                FERRO_SIGNAL_TEXT,
                0.0,
                c"none".as_ptr(),
                FERRO_PROPERTY_ATTACHED,
            )
        };
        assert_eq!(caption, property_id("CGauge.Caption"));
        assert_eq!(ferro_class_on_property_changed(class, Some(changed)), FERRO_OK);
        let gauge = ferro_create(class);
        assert_eq!(get_f64(gauge, "Level"), Ok(0.25));
        assert_eq!(ferro_set_f64(gauge, level, 0.75), FERRO_OK);
        assert_eq!(get_f64(gauge, "Level"), Ok(0.75));
        assert_eq!(CHANGED.load(Ordering::Relaxed), level);
        // Not a property of another class.
        let other = create("TextBlock");
        assert_eq!(ferro_set_f64(other, level, 1.0), FERRO_NOT_FOUND);
        // The attached text property goes on any control, by its owner's name.
        assert_eq!(set_str(other, "CGauge.Caption", "on a text block"), FERRO_OK, "{}", last_error());
        assert_eq!(get_str(other, "CGauge.Caption").as_deref(), Ok("on a text block"));
        assert_eq!(get_str(gauge, "CGauge.Caption").as_deref(), Ok("none"));
        // A frame sets it as any property.
        let root = create("Border");
        assert_eq!(ferro_frame_begin(), FERRO_OK);
        assert_eq!(ferro_frame_open(class, 1), FERRO_OK);
        assert_eq!(ferro_frame_f64(level, 0.5), FERRO_OK);
        assert_eq!(ferro_frame_close(), FERRO_OK);
        assert_eq!(ferro_frame_commit(root), FERRO_OK, "{}", last_error());
        let framed = ferro_frame_find(root, 1);
        assert_eq!(get_f64(framed, "Level"), Ok(0.5));
        assert_eq!(ferro_release(framed), FERRO_OK);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
        for handle in [gauge, other, root] {
            assert_eq!(ferro_release(handle), FERRO_OK);
        }
    });
}

#[test]
fn a_render_callback_records_into_the_drawing_context() {
    ui(|| {
        let class = register("CPainted", FERRO_BASE_CONTROL);
        assert_eq!(ferro_class_on_render(class, Some(render)), FERRO_OK);
        let brush = ferro_draw_brush(0xFF33_6699);
        assert!(brush > 0);
        assert_eq!(ferro_draw_brush(0xFF33_6699), brush, "the same colour is the same brush");
        BRUSH.store(brush, Ordering::Relaxed);
        let object = ferro_create(class);
        let visual = object_of(object).and_then(|o| o.cast::<ferroui_base::Visual>()).expect("a visual");
        let mut recorder = RenderDataDrawingContext::new(None);
        {
            let mut context = DrawingContext::new(&mut recorder);
            let drawn = DRAWN.load(Ordering::Relaxed);
            ferro_clear_error();
            visual.render(&mut context);
            assert_eq!(DRAWN.load(Ordering::Relaxed), drawn + 1);
            assert!(
                PATH.load(Ordering::Relaxed) == i64::from(FERRO_OK)
                    || PATH.load(Ordering::Relaxed) == i64::from(FERRO_PANIC)
            );
        }
        assert!(!recorder.recorded_nothing(), "the drawing is recorded");
        // Outside a render callback a context is refused.
        assert_eq!(ferro_draw_rect(1, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, brush, 0), FERRO_INVALID_STATE);
        drop(visual);
        assert_eq!(ferro_release(object), FERRO_OK);
    });
}

#[test]
fn every_base_makes_instances_of_its_kind() {
    ui(|| {
        for (name, base) in [
            ("CPanel", FERRO_BASE_PANEL),
            ("CDecorator", FERRO_BASE_DECORATOR),
            ("CContent", FERRO_BASE_CONTENT_CONTROL),
        ] {
            let class = register(name, base);
            let object = ferro_create(class);
            assert_ne!(object, 0);
            let child = create("TextBlock");
            assert_eq!(ferro_add_child(object, child), FERRO_OK, "{name}: {}", last_error());
            if base == FERRO_BASE_PANEL {
                assert_eq!(ferro_panel_child_count(object), 1);
                let first = ferro_panel_child(object, 0);
                assert_eq!(first, child);
                assert_eq!(ferro_release(first), FERRO_OK);
            }
            if base == FERRO_BASE_CONTENT_CONTROL {
                // SAFETY: a NUL-terminated literal.
                assert_eq!(unsafe { ferro_template_part(object, c"PART_None".as_ptr(), -1) }, 0);
                assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
            }
            assert_eq!(ferro_release(child), FERRO_OK);
            assert_eq!(ferro_release(object), FERRO_OK);
        }
    });
}

static TEMPLATE_PART: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" fn applied(_data: *mut c_void, object: FerroHandle, _property: i64) -> FerroStatus {
    // SAFETY: a NUL-terminated literal.
    let part = unsafe { ferro_template_part(object, c"PART_Inner".as_ptr(), -1) };
    TEMPLATE_PART.store(part, Ordering::Relaxed);
    0
}

#[test]
fn the_parts_of_a_real_template_are_found_from_the_apply_template_callback() {
    ui(|| {
        let class = register("CTemplated", FERRO_BASE_CONTENT_CONTROL);
        assert_eq!(ferro_class_on_apply_template(class, Some(applied)), FERRO_OK);
        let object = ferro_create(class);
        assert_ne!(object, 0, "{}", last_error());
        let target = crate::handles::get(object).unwrap().cast::<TemplatedControl>().unwrap();
        target.set_template(Some(FuncControlTemplate::new(|_, scope| {
            let inner = Border::new();
            inner.set_name(Some("PART_Inner".to_string()));
            inner.register_in_name_scope(&**scope).upcast()
        })));
        target.apply_template();

        let part = TEMPLATE_PART.swap(0, Ordering::Relaxed);
        assert!(part > 0, "{}", last_error());
        assert!(crate::handles::get(part).unwrap().cast::<Border>().is_some());
        // SAFETY: a NUL-terminated literal.
        assert_eq!(unsafe { ferro_template_part(object, c"Missing".as_ptr(), -1) }, 0);
        assert!(last_error().contains("Missing"), "{}", last_error());
        for handle in [part, object] {
            assert_eq!(ferro_release(handle), FERRO_OK);
        }
    });
}

/// A number property registered on `class`, with the default `0.25`.
fn number_property(class: i64, name: &str) -> i64 {
    // SAFETY: a pointer and the length of a Rust string, a NULL default text.
    let id = unsafe {
        ferro_property_register(class, name.as_ptr().cast(), name.len() as i64, FERRO_SIGNAL_F64, 0.25, std::ptr::null(), 0)
    };
    assert!(id > 0, "{name}: {}", last_error());
    id
}

/// The property behind the ID `id` on `object`.
fn registered(object: FerroHandle, id: i64) -> &'static FerroProperty {
    let target = crate::handles::get(object).unwrap();
    match crate::ids::property(target.get_type(), id) {
        Ok(crate::ids::Property::Registered(property)) => property,
        _ => panic!("{id} is not a registered property of the object"),
    }
}

fn control(object: FerroHandle) -> Ref<Control> {
    crate::handles::get(object).unwrap().cast::<Control>().unwrap()
}

#[test]
fn a_style_sets_a_registered_property() {
    ui(|| {
        let class = register("CStyled", FERRO_BASE_CONTROL);
        let level = number_property(class, "StyledLevel");
        let object = ferro_create(class);
        let target = control(object);
        let style = Style::with_selector(Selectors::is::<Control>());
        style.add_setter(Setter::new_untyped(registered(object, level), Rc::new(0.75f64)));
        target.styles().add(style);
        target.apply_styling();
        assert_eq!(get_f64(object, "StyledLevel"), Ok(0.75));
        // A local value is above the style, as for a property of the framework.
        assert_eq!(ferro_set_f64(object, level, 0.5), FERRO_OK);
        assert_eq!(get_f64(object, "StyledLevel"), Ok(0.5));
        assert_eq!(ferro_release(object), FERRO_OK);
    });
}

#[test]
fn a_binding_of_a_style_drives_a_registered_property() {
    ui(|| {
        let class = register("CBound", FERRO_BASE_CONTROL);
        let level = number_property(class, "BoundLevel");
        let object = ferro_create(class);
        let target = control(object);
        let source = Rc::new(LightweightSubject::<BoxedValue>::new());
        let style = Style::with_selector(Selectors::is::<Control>());
        style.add_setter(Setter::new_binding(registered(object, level), source.clone()));
        target.styles().add(style);
        target.apply_styling();
        source.on_next(Rc::new(0.6f64));
        assert_eq!(get_f64(object, "BoundLevel"), Ok(0.6));
        source.on_next(Rc::new(0.9f64));
        assert_eq!(get_f64(object, "BoundLevel"), Ok(0.9));
        assert_eq!(ferro_release(object), FERRO_OK);
    });
}

/// A clock the test advances.
#[derive(Default)]
struct StepClock {
    subject: LightweightSubject<SignedDuration>,
}

impl IObservable<SignedDuration> for StepClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<SignedDuration>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for StepClock {
    fn play_state(&self) -> PlayState {
        PlayState::Run
    }

    fn set_play_state(&self, _: PlayState) {}
}

#[test]
fn an_animation_runs_on_a_registered_property() {
    ui(|| {
        let class = register("CAnimated", FERRO_BASE_CONTROL);
        let level = number_property(class, "AnimatedLevel");
        let object = ferro_create(class);
        let target = control(object);
        let property = registered(object, level);
        let frame = |at: f64, value: f64| {
            let setter = Setter::new_untyped(property, Rc::new(value));
            KeyFrame::with_key_time(SignedDuration::from_seconds(at), [setter as _])
        };
        let animation = Animation::new();
        animation.set_duration(SignedDuration::from_seconds(1.0));
        animation.set_fill_mode(FillMode::Both);
        animation.children().add(frame(0.0, 0.0));
        animation.children().add(frame(1.0, 1.0));

        let clock = Rc::new(StepClock::default());
        let running = animation.apply_for_preview(&target, Some(clock.clone() as Rc<dyn IClock>), None);
        clock.subject.on_next(SignedDuration::ZERO);
        clock.subject.on_next(SignedDuration::from_seconds(0.5));
        let midway = get_f64(object, "AnimatedLevel").unwrap();
        assert!((midway - 0.5).abs() < 1e-6, "{midway}");
        clock.subject.on_next(SignedDuration::from_seconds(1.0));
        assert_eq!(get_f64(object, "AnimatedLevel"), Ok(1.0));
        running.dispose();
        // The default again, once the animation is gone.
        assert_eq!(get_f64(object, "AnimatedLevel"), Ok(0.25));
        assert_eq!(ferro_release(object), FERRO_OK);
    });
}
