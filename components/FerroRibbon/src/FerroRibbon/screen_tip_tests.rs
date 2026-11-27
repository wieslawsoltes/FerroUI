use super::*;
use crate::theming::RibbonThemeResources;
use ferroui_base::{
    FerroObject, Rect, Size,
    controls::ResourceDictionary,
    interactivity::RoutedEventArgs,
    layout::HorizontalAlignment,
    media::TextWrapping,
    threading::{Dispatcher, DispatcherFrame, DispatcherPriority},
};
use ferroui_controls::{
    Border, Control, StackPanel, TextBlock, Window, templates::FuncControlTemplate,
};
use serde_json::{Value, json};
use std::time::Duration;

// The converted popup ControlTheme retains the host's default ToolTip template.
// Supply the same base theme as the native sample for every ScreenTip case; no
// case may depend on a different test having realized that lazy resource first.
struct HostTheme {
    app: Ref<Application>,
    style: Rc<dyn ferroui_base::styling::IStyle>,
}
impl Drop for HostTheme {
    fn drop(&mut self) {
        self.app.styles().remove(self.style.clone());
    }
}
fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        let app = Application::current().unwrap();
        let style = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(style.clone());
        let _theme = HostTheme { app, style };
        test();
    });
}

const PARTS: [&str; 7] = [
    "PART_Description",
    "PART_HelpText",
    "PART_DisabledReason",
    "PART_Image",
    "PART_Extended",
    "PART_ExtendedDescription",
    "PART_ExtendedImage",
];
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/screen-tips.json")).unwrap()
}
fn text(v: &Value) -> Option<String> {
    if v.is_null() {
        None
    } else {
        Some(String::from_utf16(&units(v)).unwrap())
    }
}
fn units(v: &Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u16)
        .collect()
}
fn utf16(v: Option<String>) -> Value {
    v.map_or(Value::Null, |s| json!(s.encode_utf16().collect::<Vec<_>>()))
}
fn snapshot(t: &RibbonScreenTip) -> Value {
    json!({"kind":"control","title":utf16(t.title()),"shortcut":utf16(t.shortcut()),"description":utf16(t.description()),"help":utf16(t.help_text()),"disabled":utf16(t.disabled_reason()),"extended":utf16(t.extended_description()),"text":utf16(Some(t.title_text())),"isExtended":t.is_extended(),"hasExtended":t.has_extended_content(),"width":if t.max_width().is_infinite(){"Infinity".into()}else{t.max_width().to_string()}})
}
fn object_snapshot(v: &Option<BoxedValue>) -> Value {
    if let Some(t) = v
        .as_ref()
        .and_then(|v| ValueTypes::as_object(&**v))
        .and_then(|o| o.cast::<RibbonScreenTip>())
    {
        return snapshot(&t);
    }
    let s = v.as_ref().and_then(|v| v.downcast_ref::<String>()).cloned();
    json!({"kind":if s.is_some(){"string"}else if v.is_none(){"null"}else{"other"},"text":utf16(s)})
}
fn with_parts() -> Ref<RibbonScreenTip> {
    let t = RibbonScreenTip::new();
    t.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let root = StackPanel::new();
        for name in PARTS {
            let p = Border::new();
            scope.register(name, p.clone().upcast());
            root.children().add(p);
        }
        root.upcast()
    })));
    t.apply_template();
    t
}
fn state(t: &RibbonScreenTip) -> Value {
    json!({"tip":snapshot(t),"visible":PARTS.map(|p|t.part(p).map(|p|p.is_visible()))})
}
fn load(t: &RibbonScreenTip) {
    t.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()));
}
fn unload(t: &RibbonScreenTip) {
    t.raise_event(&RoutedEventArgs::with_event(Control::unloaded_event()));
}
fn pump(duration: Duration) {
    let frame = DispatcherFrame::new();
    let f = frame.clone();
    let stop = DispatcherTimer::run_once(
        move || f.set_continue(false),
        duration,
        DispatcherPriority::NORMAL,
    );
    Dispatcher::current_dispatcher().push_frame(&frame);
    stop.dispose();
}
fn reset() {
    RibbonScreenTipService::set_extended_delay(SignedDuration::from_ticks(15_000_000));
    RibbonScreenTipService::set_is_extended_enabled(true);
}
#[test]
fn whole_source_defaults_and_all_valid_copy_and_string_cases() {
    run(|| {
        reset();
        let c = corpus();
        assert_eq!(snapshot(&RibbonScreenTip::new()), c["defaults"]);
        for row in c["copies"].as_array().unwrap() {
            let title = row["title"].as_array().map(|_| units(&row["title"]));
            if title
                .as_ref()
                .is_some_and(|s| String::from_utf16(s).is_err())
            {
                continue;
            }
            let value: BoxedValue = if row["kind"] == "control" {
                let t = RibbonScreenTip::new();
                t.set_title(text(&row["title"]));
                t.set_shortcut(text(&row["shortcut"]));
                t.set_description(Some("D".into()));
                t.set_help_text(Some("H".into()));
                t.set_disabled_reason(Some("X".into()));
                t.set_extended_description(Some("E".into()));
                t.set_is_extended(true);
                Control::boxed(t)
            } else {
                let t = ModelScreenTip::new();
                t.set_title(text(&row["title"]).map(Into::into));
                t.set_shortcut(text(&row["shortcut"]).map(Into::into));
                t.set_description(Some("D".into()));
                t.set_help_text(Some("H".into()));
                t.set_disabled_reason(Some("X".into()));
                t.set_extended_description(Some("E".into()));
                t
            };
            let copy = RibbonScreenTipService::create_with_enabled(
                Some("Line\nTwo\r\nThree".into()),
                Some(value),
                Some("Ctrl+S".into()),
                row["enabled"].as_bool().unwrap(),
            )
            .unwrap();
            assert_eq!(object_snapshot(&copy), row["output"], "{row}");
        }
        for row in c["strings"].as_array().unwrap() {
            if [&row["label"], &row["description"]]
                .iter()
                .any(|v| !v.is_null() && String::from_utf16(&units(v)).is_err())
            {
                continue;
            }
            let input = text(&row["description"]).map(|s| Rc::new(s) as BoxedValue);
            let copy = RibbonScreenTipService::create(
                text(&row["label"]),
                input.clone(),
                Some("K".into()),
            )
            .unwrap();
            assert_eq!(object_snapshot(&copy), row["output"], "{row}");
            let same = match (&input, &copy) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            };
            assert_eq!(same, row["identity"].as_bool().unwrap());
            assert_eq!(
                RibbonScreenTipService::create_tool_tip(copy).is_some(),
                row["wrapped"].as_bool().unwrap()
            );
        }
    });
}
#[test]
fn copy_identity_images_disabled_state_and_explicit_utf16_boundary() {
    run(|| {
        let c = corpus();
        let original = with_parts();
        let image: BoxedValue = Rc::new(Cell::new(1));
        let extended_image: BoxedValue = Rc::new(Cell::new(2));
        original.set_image(Some(image.clone()));
        original.set_extended_image(Some(extended_image.clone()));
        original.set_is_extended(true);
        let make = || {
            ValueTypes::as_object(
                &*RibbonScreenTipService::create_with_enabled(
                    Some("Label".into()),
                    Some(Control::boxed(original.clone())),
                    Some("K".into()),
                    false,
                )
                .unwrap()
                .unwrap(),
            )
            .unwrap()
            .cast::<RibbonScreenTip>()
            .unwrap()
        };
        let first = make();
        let second = make();
        let unknown: BoxedValue = Rc::new(Cell::new(0));
        let output = RibbonScreenTipService::create(None, Some(unknown.clone()), None)
            .unwrap()
            .unwrap();
        let wrapped = RibbonScreenTipService::create_tool_tip(Some(unknown.clone())).unwrap();
        assert_eq!(
            json!({"newControls":first!=second&&first!=original,"image":Rc::ptr_eq(&first.image().unwrap(),&image),"extendedImage":Rc::ptr_eq(&first.extended_image().unwrap(),&extended_image),"notCopied":!first.is_extended(),"sourceUnchanged":original.is_extended(),"unknown":Rc::ptr_eq(&output,&unknown),"wrapped":Rc::ptr_eq(&wrapped.content().unwrap(),&unknown)}),
            c["identity"]
        );
        image.downcast_ref::<Cell<i32>>().unwrap().set(7);
        assert_eq!(
            first
                .image()
                .unwrap()
                .downcast_ref::<Cell<i32>>()
                .unwrap()
                .get(),
            7
        );
        let model = ModelScreenTip::new();
        model.set_title(Some(RibbonString::from_utf16(vec![0xd800])));
        assert_eq!(
            RibbonScreenTipService::create(None, Some(model.clone()), None)
                .unwrap_err()
                .property,
            "Title"
        );
        model.set_title(Some("".into()));
        model.set_disabled_reason(Some(RibbonString::from_utf16(vec![0xdc00])));
        assert!(RibbonScreenTipService::create(None, Some(model.clone()), None).is_ok());
        assert_eq!(
            RibbonScreenTipService::create_with_enabled(None, Some(model), None, false)
                .unwrap_err()
                .property,
            "DisabledReason"
        );
    });
}
#[test]
fn whole_source_template_visibility_width_and_replacement() {
    run(|| {
        let c = corpus();
        let t = with_parts();
        let mut updates = vec![state(&t)];
        t.set_title(Some("T".into()));
        t.set_shortcut(Some("S".into()));
        t.set_description(Some(" ".into()));
        t.set_help_text(Some("H".into()));
        t.set_disabled_reason(Some("D".into()));
        updates.push(state(&t));
        t.set_image(Some(Rc::new(())));
        t.set_extended_description(Some("More".into()));
        updates.push(state(&t));
        t.set_is_extended(true);
        updates.push(state(&t));
        t.set_extended_image(Some(Rc::new(())));
        t.set_extended_description(Some("\u{202f}".into()));
        updates.push(state(&t));
        t.set_extended_image(None);
        updates.push(state(&t));
        t.set_description(Some("\u{feff}".into()));
        updates.push(state(&t));
        let old = t.part(PARTS[0]).unwrap().downgrade();
        t.set_template(Some(FuncControlTemplate::new(|_, scope| {
            scope.register(PARTS[0], FerroObject::new());
            Border::new().upcast()
        })));
        t.apply_template();
        updates.push(state(&t));
        assert_eq!(json!(updates), c["updates"]);
        assert!(old.upgrade().is_none());
    });
}
#[test]
fn source_timer_states_with_controlled_delivery_and_no_pending_cycles() {
    run(|| {
        let c = corpus();
        for row in c["timers"].as_array().unwrap() {
            RibbonScreenTipService::set_extended_delay(SignedDuration::from_ticks(
                row["ticks"].as_i64().unwrap(),
            ));
            RibbonScreenTipService::set_is_extended_enabled(row["enabled"].as_bool().unwrap());
            let a = with_parts();
            a.set_extended_description(Some("E".into()));
            a.set_is_extended(true);
            load(&a);
            assert_eq!(state(&a), row["loaded"]);
            let timer = a.extend_timer.borrow().clone();
            assert_eq!(timer.is_some(), row["allocated"].as_bool().unwrap());
            if let Some(timer) = &timer {
                assert_eq!(
                    timer.interval(),
                    RibbonScreenTipService::extended_delay()
                        .to_duration()
                        .unwrap()
                );
            }
            RibbonScreenTipService::set_is_extended_enabled(false);
            a.set_extended_description(None);
            if let Some(timer) = &timer {
                // The C# seam fires a pending timer. Deliver the real native callback
                // without waiting SignedDuration.MaxValue; scheduler precision is separate.
                let frame = DispatcherFrame::new();
                let delivered = Rc::new(std::cell::Cell::new(false));
                let observed = delivered.clone();
                let done = frame.clone();
                let delivery = timer.tick(move |_| {
                    observed.set(true);
                    done.set_continue(false);
                });
                let expired = frame.clone();
                let timeout = DispatcherTimer::run_once(
                    move || expired.set_continue(false),
                    Duration::from_secs(2),
                    DispatcherPriority::NORMAL,
                );
                timer.set_interval(Duration::ZERO);
                Dispatcher::current_dispatcher().push_frame(&frame);
                timeout.dispose();
                delivery.dispose();
                assert!(delivered.get(), "native pending callback must be delivered");
                assert!(!timer.is_enabled());
            }
            assert_eq!(state(&a), row["fired"]);
            assert!(a.extend_timer.borrow().is_none());
            unload(&a);
            assert_eq!(state(&a), row["unloaded"]);
            a.set_extended_description(Some("Late".into()));
            assert_eq!(state(&a), row["late"]);
            load(&a);
            assert_eq!(state(&a), row["reload"]);
            unload(&a);
        }
        reset();
        let late = with_parts();
        load(&late);
        late.set_extended_description(Some("Late".into()));
        assert_eq!(state(&late), c["lateContent"]["state"]);
        assert!(late.extend_timer.borrow().is_none());
        unload(&late);
        let a = with_parts();
        a.set_extended_description(Some("E".into()));
        load(&a);
        let weak = a.downgrade();
        drop(a);
        assert!(weak.upgrade().is_some());
        let retained = weak.upgrade().unwrap();
        unload(&retained);
        drop(retained);
        assert!(weak.upgrade().is_none());
        reset();
    });
}
#[test]
fn cad_runtime_scenario_900ms_with_native_dispatcher_and_unload_cancellation() {
    run(|| {
        reset();
        RibbonScreenTipService::set_extended_delay(SignedDuration::from_ticks(9_000_000));
        let t = with_parts();
        t.set_title(Some("Line".into()));
        t.set_description(Some("Creates straight line segments.".into()));
        t.set_extended_description(Some("Specify points to draw connected segments.".into()));
        t.set_extended_image(Some(Rc::new("\u{e8c6}".to_owned())));
        let w = Window::new();
        w.set_width(400.);
        w.set_height(300.);
        w.set_content(Some(Control::boxed(t.clone())));
        w.show();
        pump(Duration::from_millis(40));
        assert!(!t.is_extended());
        assert!(t.extend_timer.borrow().is_some());
        pump(Duration::from_millis(1000));
        assert!(t.is_extended());
        assert_eq!(t.max_width(), 440.);
        let copy = RibbonScreenTipService::create(
            Some("Line".into()),
            Some(Control::boxed(t.clone())),
            Some("L".into()),
        )
        .unwrap();
        assert_eq!(
            object_snapshot(&copy)["extended"],
            utf16(t.extended_description())
        );
        assert_eq!(object_snapshot(&copy)["isExtended"], false);
        w.close();
        assert!(t.is_extended());
        load(&t);
        assert!(!t.is_extended());
        unload(&t);
        pump(Duration::from_millis(1000));
        assert!(!t.is_extended());
        reset();
    });
}
#[test]
fn source_reentrant_update_order_and_native_static_metadata() {
    run(|| {
        use ferroui_base::metadata::{MarkupTyped, from_markup_value, into_markup_value};
        let c = corpus();
        let t = with_parts();
        let order = Rc::new(RefCell::new(Vec::<String>::new()));
        let trace = order.clone();
        let part = t.part("PART_Description").unwrap();
        let a = part.property_changed(move |e| {
            if e.property() == ferroui_base::Visual::is_visible_property().as_property() {
                trace.borrow_mut().push(format!(
                    "PART_Description:{}",
                    if *e.new_value().downcast_ref::<bool>().unwrap() {
                        "Visible"
                    } else {
                        "Collapsed"
                    }
                ));
            }
        });
        let trace = order.clone();
        let weak = t.downgrade();
        let b = t.property_changed(move |e| {
            if e.property() == RibbonScreenTip::description_property().as_property() {
                trace.borrow_mut().push(format!(
                    "Description:{}",
                    weak.upgrade().unwrap().description().unwrap()
                ));
            }
            if e.property() == RibbonScreenTip::title_text_property().as_property() {
                trace.borrow_mut().push(format!(
                    "TitleText:{}",
                    weak.upgrade().unwrap().title_text()
                ));
                weak.upgrade()
                    .unwrap()
                    .set_description(Some("Reentrant".into()));
            }
        });
        t.set_title(Some("Changed".into()));
        assert_eq!(json!(*order.borrow()), c["reentry"]["order"]);
        assert_eq!(state(&t), c["reentry"]["state"]);
        a.dispose();
        b.dispose();
        let meta = RibbonScreenTipService::MARKUP;
        assert!(meta.constructors.is_empty());
        let delay = meta.find_static_property("ExtendedDelay").unwrap();
        (delay.set.unwrap())(&[into_markup_value(SignedDuration::from_ticks(-1))]).unwrap();
        assert_eq!(
            from_markup_value::<SignedDuration>(&(delay.get.unwrap())(&[]).unwrap()),
            Some(SignedDuration::from_ticks(-1))
        );
        assert_eq!(meta.find_methods("Create").count(), 2);
        let create = meta
            .find_methods("Create")
            .find(|m| m.parameters.len() == 3)
            .unwrap();
        let result = (create.invoke)(&[
            into_markup_value(Some("Label".to_owned())),
            None,
            into_markup_value(Some("K".to_owned())),
        ])
        .unwrap();
        assert_eq!(
            from_markup_value::<Option<BoxedValue>>(&result)
                .unwrap()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "Label (K)"
        );
        reset();
    });
}
#[test]
fn compiled_source_template_bindings_wrapping_and_progressive_parts() {
    run(|| {
        reset();
        let value =
            crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/ScreenTips.xaml")
                .unwrap()
                .unwrap();
        let dictionary = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ResourceDictionary>()
            .unwrap();
        let theme = ValueTypes::as_object(
            &*dictionary
                .try_get_resource(&<RibbonScreenTip as StaticType>::TYPE.into(), None)
                .flatten()
                .unwrap(),
        )
        .unwrap()
        .cast::<ControlTheme>()
        .unwrap();
        let t = RibbonScreenTip::new();
        t.set_theme(theme);
        t.set_title(Some("Title".into()));
        t.set_shortcut(Some("Ctrl+T".into()));
        t.set_description(Some("Description".into()));
        t.set_extended_description(Some("Extended".into()));
        let w = Window::new();
        w.set_width(600.);
        w.set_height(400.);
        w.set_content(Some(Control::boxed(t.clone())));
        w.show();
        w.measure(Size::new(600., 400.));
        w.arrange(Rect::new(0., 0., 600., 400.));
        assert_eq!(
            t.part(PARTS[0])
                .unwrap()
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("Description")
        );
        assert!(!t.part("PART_Extended").unwrap().is_visible());
        for part in ["PART_Image", "PART_ExtendedImage"] {
            assert_eq!(
                t.part(part).unwrap().horizontal_alignment(),
                HorizontalAlignment::Left,
                "{part} must honor the source template alignment"
            );
        }
        t.set_is_extended(true);
        assert!(t.part("PART_Extended").unwrap().is_visible());
        assert_eq!(t.max_width(), 440.);
        assert!(
            t.part("PART_ExtendedDescription")
                .unwrap()
                .is_effectively_visible()
        );
        t.set_description(Some("Updated".into()));
        assert_eq!(
            t.part(PARTS[0])
                .unwrap()
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("Updated")
        );
        w.close();
        reset();
    });
}
#[test]
fn tooltip_wrapper_application_resource_scope_and_content_identity() {
    run(|| {
        let app = Application::current().unwrap();
        let r = app.resources();
        let previous = r
            .try_get_resource(&"RibbonToolTipStyle".into(), None)
            .flatten();
        r.set("RibbonToolTipStyle", Some(Rc::new("wrong".to_owned())));
        assert!(
            RibbonScreenTipService::create_tool_tip(Some(Rc::new("T".to_owned())))
                .unwrap()
                .theme()
                .is_none()
        );
        let simple = ferroui_themes_simple::SimpleTheme::new();
        let simple_style = simple.as_style();
        app.styles().add(simple_style.clone());
        let resources = RibbonThemeResources::new();
        let style = resources
            .try_get_resource(&"RibbonToolTipStyle".into(), None)
            .flatten()
            .unwrap();
        r.set("RibbonToolTipStyle", Some(style.clone()));
        let content: BoxedValue =
            Rc::new("A tooltip with a long description that must wrap. ".repeat(10));
        let wrapped = RibbonScreenTipService::create_tool_tip(Some(content.clone())).unwrap();
        assert!(Rc::ptr_eq(&wrapped.content().unwrap(), &content));
        assert_eq!(
            wrapped.theme(),
            ValueTypes::as_object(&*style).unwrap().cast()
        );
        let window = Window::new();
        window.set_width(800.);
        window.set_height(600.);
        window.set_content(Some(Control::boxed(wrapped.clone())));
        window.show();
        wrapped.measure(Size::new(800., 600.));
        let presenter = wrapped.presenter().unwrap();
        assert_eq!(wrapped.max_width(), 480.);
        assert_eq!(presenter.max_width(), 320.);
        assert!(wrapped.desired_size().width <= 320.);
        assert_eq!(presenter.text_wrapping(), TextWrapping::Wrap);
        window.close();
        r.set("RibbonToolTipStyle", previous);
        app.styles().remove(simple_style);
    });
}
