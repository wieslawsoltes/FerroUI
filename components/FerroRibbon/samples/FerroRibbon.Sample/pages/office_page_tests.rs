use super::{PowerPointPage, ToolsPage};
use crate::{
    demo_settings::DemoSettings,
    office_view_models::{PowerPointAction, ToolsAction, ToolsViewModel},
};
use ferroribbon::controls::{RibbonContextualToolBar, RibbonGalleryItem, RibbonToggleButton};
use ferroui_base::{
    Ref,
    media::{Color, IBrush, SolidColorBrush},
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{AppBuilder, Application, Border, Control, StackPanel, TextBlock, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn run(f: impl FnOnce() + Send + 'static) {
    let s = HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>()
            .use_platform_detect()
            .use_headless(FerroHeadlessPlatformOptions {
                use_headless_drawing: false,
                ..Default::default()
            })
    });
    let r = s
        .dispatch(
            move || {
                crate::register_types();
                Application::current()
                    .unwrap()
                    .styles()
                    .add(ferroui_themes_simple::SimpleTheme::new().as_style());
                crate::install_application_resources();
                f();
            },
            CancellationToken::none(),
        )
        .wait();
    s.dispose();
    r.unwrap();
}
fn settings() -> Rc<DemoSettings> {
    DemoSettings::new(Rc::new(|| Ok("03:04:05".into())))
}
fn mount<T: ferroui_base::ObjectType + ferroui_base::Upcast<Control>>(page: Ref<T>) -> Ref<Window> {
    let w = Window::new();
    w.set_width(1440.);
    w.set_height(1000.);
    w.set_content(Some(Control::boxed(page.upcast::<Control>())));
    w.show();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/office-pages.json")).unwrap()
}
fn same(a: &Rc<dyn IBrush>, b: &Rc<dyn IBrush>) -> bool {
    match (a.as_object(), b.as_object()) {
        (Some(a), Some(b)) => a.to_ref() == b.to_ref(),
        _ => Rc::ptr_eq(a, b),
    }
}
fn color(c: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(Color::from_argb(
        (c >> 24) as u8,
        (c >> 16) as u8,
        (c >> 8) as u8,
        c as u8,
    ))
    .into()
}
#[test]
fn office_whole_markup_constructs_with_original_defaults_and_no_eager_powerpoint_settings() {
    run(|| {
        use ferroribbon_core::{RibbonDensity, RibbonDisplayMode};
        let s = settings();
        s.apply(RibbonDensity::Touch, Some(RibbonDisplayMode::Classic));
        let pp = PowerPointPage::new_with_settings(s.clone());
        let tools = ToolsPage::new_with_settings(s.clone());
        assert_eq!(pp.ribbon().density(), RibbonDensity::Comfortable);
        assert_eq!(pp.ribbon().display_mode(), RibbonDisplayMode::Simplified);
        assert_eq!(pp.ribbon().tabs().count(), 6);
        let map: Value =
            serde_json::from_str(include_str!("../../../docs/powerpoint-page-markup.json"))
                .unwrap();
        for n in map["source_names"].as_array().unwrap() {
            pp.named::<ferroui_base::FerroObject>(n.as_str().unwrap());
        }
        let map: Value =
            serde_json::from_str(include_str!("../../../docs/tools-page-markup.json")).unwrap();
        for n in map["source_names"].as_array().unwrap() {
            tools.named::<ferroui_base::FerroObject>(n.as_str().unwrap());
        }
        let thumbs = pp.named::<StackPanel>("Thumbnails");
        assert_eq!(thumbs.children().count(), 4);
        for i in 0..4 {
            let b = thumbs.children().get(i).cast::<Border>().unwrap();
            assert_eq!(b.height(), 90.);
            assert_eq!(b.border_thickness().left, if i == 0 { 2. } else { 1. });
            assert_eq!(
                b.child().unwrap().cast::<TextBlock>().unwrap().text(),
                Some(format!("Slide {}", i + 1))
            );
        }
        assert_eq!(tools.tools().items().count(), 14);
        assert_eq!(
            tools
                .named::<RibbonContextualToolBar>("Options")
                .active_context()
                .as_deref(),
            Some("brush")
        );
        assert_eq!(
            tools.named::<TextBlock>("ToolStatus").text().as_deref(),
            Some("Brush tool")
        );
        assert!(s.log().borrow().is_empty());
        let w = mount(pp);
        w.close();
        let w = mount(tools);
        w.close();
    });
}
#[test]
fn tools_presentation_matches_entire_source_class_event_cases() {
    use ferroui_system::component_model::INotifyPropertyChanged;
    for c in fixture()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["page"] == "tools")
    {
        let logs = Rc::new(RefCell::new(Vec::new()));
        let output = logs.clone();
        let vm = ToolsViewModel::new(Rc::new(move |s| {
            output.borrow_mut().push(s.to_owned());
            Ok(())
        }));
        let mut tokens = Vec::new();
        match c["kind"].as_str().unwrap() {
            "initial" | "append" => {
                if c["kind"] == "append" {
                    vm.apply(ToolsAction::Invoked {
                        label: Some("new".into()),
                    })
                    .unwrap();
                }
            }
            "check" => {
                let i = c["index"].as_u64().unwrap() as usize;
                if c["value"] == true && i != 2 {
                    let label = [
                        "Move (V)",
                        "Select (M)",
                        "Brush (B)",
                        "Pen (P)",
                        "Eraser (E)",
                        "Type (T)",
                    ][i]
                        .to_owned();
                    vm.apply(ToolsAction::Checked {
                        context: Some(
                            if i < 2 {
                                "move"
                            } else if i == 5 {
                                "text"
                            } else {
                                "brush"
                            }
                            .into(),
                        ),
                        label: Rc::new(move || Some(label.clone())),
                    })
                    .unwrap();
                }
            }
            "changed" => {
                let label = c["label"].as_str().map(str::to_owned);
                let logging = label.clone();
                vm.apply(ToolsAction::Checked {
                    context: c["tag"].as_str().map(str::to_owned),
                    label: Rc::new(move || label.clone()),
                })
                .unwrap();
                vm.apply(ToolsAction::Invoked { label: logging }).unwrap();
            }
            "invoke" | "other" => vm
                .apply(ToolsAction::Invoked {
                    label: c["label"].as_str().map(str::to_owned),
                })
                .unwrap(),
            "lateLabels" => {
                let label = Rc::new(RefCell::new(Some("Move (V)".to_owned())));
                let changing = label.clone();
                tokens.push(vm.property_changed().add(Rc::new(move |name| {
                    if name == "ActiveContext" {
                        changing.replace(Some("after context".into()));
                    } else if name == "ToolStatus" {
                        changing.replace(Some("after status".into()));
                    }
                })));
                vm.apply(ToolsAction::Checked {
                    context: Some("move".into()),
                    label: Rc::new(move || label.borrow().clone()),
                })
                .unwrap();
            }
            "contextReentry" => {
                let weak = Rc::downgrade(&vm);
                let entered = Cell::new(false);
                tokens.push(vm.property_changed().add(Rc::new(move |name| {
                    if name == "ActiveContext"
                        && !entered.replace(true)
                        && let Some(vm) = weak.upgrade()
                    {
                        vm.apply(ToolsAction::Checked {
                            context: Some("text".into()),
                            label: Rc::new(|| Some("Type (T)".into())),
                        })
                        .unwrap();
                    }
                })));
                vm.apply(ToolsAction::Checked {
                    context: Some("move".into()),
                    label: Rc::new(|| Some("Move (V)".into())),
                })
                .unwrap();
            }
            _ => panic!("{c}"),
        }
        for token in tokens {
            vm.property_changed().remove(token);
        }
        let actual = json!({"context":vm.active_context(),"status":vm.tool_status(),"canvas":vm.canvas_text(),"log":logs.borrow().clone()});
        let mut expected = c["state"].clone();
        expected.as_object_mut().unwrap().remove("checks");
        assert_eq!(actual, expected, "{c}");
    }
}
#[test]
fn tools_actual_checked_and_invoked_keep_source_labels_and_constructor_subscription_snapshot() {
    run(|| {
        let s = settings();
        let page = ToolsPage::new_with_settings(s.clone());
        let w = mount(page.clone());
        let toggles = page
            .tools()
            .items()
            .iter()
            .flatten()
            .filter_map(|c| c.cast::<RibbonToggleButton>())
            .collect::<Vec<_>>();
        super::cad_session::ribbon_item(&toggles[0].clone().upcast::<Control>())
            .unwrap()
            .invoke();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            page.named::<RibbonContextualToolBar>("Options")
                .active_context()
                .as_deref(),
            Some("move")
        );
        assert_eq!(
            page.named::<TextBlock>("CanvasText").text().as_deref(),
            Some("Move (V) is active")
        );
        assert_eq!(s.log().borrow()[0], "03:04:05  Tools · Move (V)");
        let extra = RibbonToggleButton::new();
        extra.set_label(Some("new".into()));
        extra.set_tag(Some(Rc::new("text".to_owned())));
        page.tools().items().add(Some(extra.clone().upcast()));
        extra.set_is_checked(Some(true));
        assert_eq!(page.view_model().active_context().as_deref(), Some("move"));
        page.tools().on_item_invoked(extra.upcast(), None, None);
        assert_eq!(s.log().borrow()[0], "03:04:05  Tools · new");
        w.close();
    });
}
#[test]
fn powerpoint_actual_gallery_preserves_mutable_brush_identity_and_preview_fallback() {
    run(|| {
        use ferroribbon::controls::RibbonGallery;
        let page = PowerPointPage::new_with_settings(settings());
        let w = mount(page.clone());
        let gallery = page.named::<RibbonGallery>("Themes");
        let theme = RibbonGalleryItem::new();
        let bg = color(0xff006080);
        let fg = color(0xfff0e0d0);
        theme.set_preview_background(Some(bg.clone()));
        theme.set_preview_foreground(Some(fg.clone()));
        gallery.pick(Some(Rc::new(theme.clone())));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(same(
            &page.named::<Border>("Slide").background().unwrap(),
            &bg
        ));
        assert!(same(
            &page.named::<TextBlock>("SlideTitle").foreground().unwrap(),
            &fg
        ));
        assert!(same(
            &page
                .named::<TextBlock>("SlideSubtitle")
                .foreground()
                .unwrap(),
            &fg
        ));
        let next = color(0xff203040);
        theme.set_preview_foreground(Some(next.clone()));
        page.view_model()
            .apply(PowerPointAction::Preview(None))
            .unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(same(
            &page.named::<TextBlock>("SlideTitle").foreground().unwrap(),
            &next
        ));
        w.close();
    });
}

// Application behavior is compared with the whole unchanged C# classes. The
// descriptors do not implement the gallery's own routing: theme-only cases call
// the presentation action; invocation cases use the real native ribbon owner.
#[test]
fn powerpoint_matches_all_source_application_cases_with_native_properties_and_reentry() {
    run(|| {
        use ferroribbon::controls::{RibbonButton, RibbonGallery, RibbonTitleBar};
        use ferroribbon_core::{RibbonDensity, RibbonDisplayMode};
        use ferroui_base::{
            BoxedValue, FerroObject, FerroObjectExtensions, reactive::AnonymousObserver,
        };
        let fixture = fixture();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(
            cases.iter().filter(|c| c["page"] == "powerpoint").count(),
            63
        );
        for c in cases.iter().filter(|c| c["page"] == "powerpoint") {
            let s = settings();
            let kind = c["kind"].as_str().unwrap();
            if kind == "noEagerSettings" {
                s.apply(RibbonDensity::Touch, Some(RibbonDisplayMode::Classic));
            }
            let accent = color(0xff010203);
            Application::current()
                .unwrap()
                .resources()
                .set("RibbonAccentBrush", Some(Rc::new(accent.clone())));
            let page = PowerPointPage::new_with_settings(s.clone());
            let mut selected: Option<Ref<RibbonGalleryItem>> = None;
            let mut subscriptions = Vec::new();
            let theme = |label: &str, bg, fg| {
                let t = RibbonGalleryItem::new();
                t.set_label(Some(label.into()));
                t.set_preview_background(bg);
                t.set_preview_foreground(fg);
                t
            };
            let pick = |t: &Ref<RibbonGalleryItem>| {
                page.view_model()
                    .apply(PowerPointAction::Pick(
                        page.theme_application(Some(Rc::new(t.clone()))),
                    ))
                    .unwrap();
            };
            match kind {
                "initial" | "noEagerSettings" => {}
                "theme" => {
                    let t = theme(
                        "selected",
                        c["bg"].as_bool().unwrap().then(|| color(0xff102030)),
                        c["fg"].as_bool().unwrap().then(|| color(0xff405060)),
                    );
                    match c["mode"].as_str().unwrap() {
                        "pick" => {
                            pick(&t);
                            selected = Some(t);
                        }
                        "preview" => page
                            .view_model()
                            .apply(PowerPointAction::Preview(
                                page.theme_application(Some(Rc::new(t))),
                            ))
                            .unwrap(),
                        "end" => {
                            pick(&t);
                            selected = Some(t);
                            page.view_model()
                                .apply(PowerPointAction::Preview(None))
                                .unwrap();
                        }
                        "wrong" => {
                            pick(&t);
                            page.view_model()
                                .apply(PowerPointAction::Pick(
                                    page.theme_application(Some(Rc::new("wrong".to_owned()))),
                                ))
                                .unwrap();
                        }
                        _ => panic!("{c}"),
                    }
                }
                "invoke" | "galleryParameter" | "statusReentry" => {
                    if kind == "statusReentry" {
                        let status = page.named::<TextBlock>("Status");
                        let weak = status.downgrade();
                        let entered = Cell::new(false);
                        subscriptions.push(
                            FerroObjectExtensions::get_observable(
                                &*status.upcast::<FerroObject>(),
                                TextBlock::text_property(),
                            )
                            .subscribe(Rc::new(
                                AnonymousObserver::new(move |text: Option<String>| {
                                    if text.as_deref() == Some("pp.save") && !entered.replace(true)
                                    {
                                        weak.upgrade().unwrap().set_text(Some("reentrant"));
                                    }
                                }),
                            )),
                        );
                    }
                    let item = RibbonButton::new();
                    item.set_id(if kind == "galleryParameter" {
                        Some("pp.themes".into())
                    } else if kind == "statusReentry" {
                        Some("pp.save".into())
                    } else {
                        c["id"].as_str().map(Into::into)
                    });
                    let parameter: Option<BoxedValue> = if kind == "galleryParameter" {
                        Some(Rc::new(RibbonGalleryItem::new()))
                    } else {
                        c["parameter"]
                            .as_str()
                            .map(|s| Rc::new(s.to_owned()) as BoxedValue)
                    };
                    page.ribbon().on_item_invoked(
                        item.upcast(),
                        Some("must-not-fallback".into()),
                        parameter,
                    );
                }
                "settings" => s.apply(
                    RibbonDensity::from_raw(c["density"].as_i64().unwrap() as i32),
                    c["mode"]
                        .as_i64()
                        .map(|v| RibbonDisplayMode::from_raw(v as i32)),
                ),
                "mutableTheme" | "backgroundReentry" | "titleReentry" => {
                    let a = theme("a", Some(color(0xff112233)), Some(color(0xff445566)));
                    if kind == "backgroundReentry" {
                        let changing = a.clone();
                        subscriptions.push(
                            FerroObjectExtensions::get_observable(
                                &*page.named::<Border>("Slide").upcast::<FerroObject>(),
                                Border::background_property(),
                            )
                            .subscribe(Rc::new(
                                AnonymousObserver::new(move |bg: Option<Rc<dyn IBrush>>| {
                                    if paint_value(bg.as_ref()) == "a.bg" {
                                        changing.set_preview_foreground(Some(color(0xff778899)));
                                    }
                                }),
                            )),
                        );
                    }
                    if kind == "titleReentry" {
                        let b = theme("b", Some(color(0xffa1b2c3)), Some(color(0xffd4e5f6)));
                        let action = page.theme_application(Some(Rc::new(b.clone())));
                        let weak = Rc::downgrade(&page.view_model());
                        let entered = Cell::new(false);
                        subscriptions.push(
                            FerroObjectExtensions::get_observable(
                                &*page
                                    .named::<TextBlock>("SlideTitle")
                                    .upcast::<FerroObject>(),
                                TextBlock::foreground_property(),
                            )
                            .subscribe(Rc::new(
                                AnonymousObserver::new(move |fg: Option<Rc<dyn IBrush>>| {
                                    if paint_value(fg.as_ref()) == "a.fg" && !entered.replace(true)
                                    {
                                        weak.upgrade()
                                            .unwrap()
                                            .apply(PowerPointAction::Pick(action.clone()))
                                            .unwrap();
                                    }
                                }),
                            )),
                        );
                        selected = Some(b);
                    } else {
                        selected = Some(a.clone());
                    }
                    pick(&a);
                    if kind == "mutableTheme" {
                        a.set_preview_background(Some(color(0xffabcdef)));
                        a.set_preview_foreground(Some(color(0xfffedcba)));
                        page.view_model()
                            .apply(PowerPointAction::Preview(None))
                            .unwrap();
                    }
                }
                _ => panic!("unhandled source case {c}"),
            }
            Dispatcher::ui_thread().run_jobs(None);
            let bg = page.named::<Border>("Slide").background().unwrap();
            let fg = page.named::<TextBlock>("SlideTitle").foreground().unwrap();
            let sub = page
                .named::<TextBlock>("SlideSubtitle")
                .foreground()
                .unwrap();
            let children = page.named::<StackPanel>("Thumbnails").children();
            let thumbnails = (0..children.count()).map(|i| {
                let b = children.get(i);
                let b = b.cast::<Border>().unwrap();
                let text = b.child().unwrap().cast::<TextBlock>().unwrap();
                let stroke = b.border_brush().unwrap();
                json!({"Height":b.height(),"corner":b.corner_radius().top_left,"border":b.border_thickness().left,
                    "background":paint_value(b.background().as_ref()),"stroke":if same(&stroke,&accent){"accent".into()}else{paint_value(Some(&stroke))},
                    "accentIdentity":same(&stroke,&accent),"text":text.text(),"margin":text.margin().left,
                    "font":text.font_size(),"foreground":paint_value(text.foreground().as_ref())})
            }).collect::<Vec<_>>();
            let actual = json!({"background":paint_value(Some(&bg)),"title":paint_value(Some(&fg)),"subtitle":paint_value(Some(&sub)),
                "status":page.named::<TextBlock>("Status").text(),"sharedForeground":same(&fg,&sub),
                "sameBackground":selected.as_ref().and_then(|t|t.preview_background()).is_some_and(|b|same(&b,&bg)),
                "sameForeground":selected.as_ref().and_then(|t|t.preview_foreground()).is_some_and(|b|same(&b,&fg)),
                "density":page.ribbon().density().raw(),"display":page.ribbon().display_mode().raw(),
                "titleIdentity":page.named::<RibbonTitleBar>("TitleBar").ribbon()==Some(page.ribbon()),
                "thumbnails":thumbnails,"log":s.log().borrow().iter().rev().map(|s|s.strip_prefix("03:04:05  ").unwrap().to_owned()).collect::<Vec<_>>()});
            let mut expected = c["state"].clone();
            // Private selected identity is observed via the retained-reference and
            // end-preview cases above, not exposed as a new application API.
            expected.as_object_mut().unwrap().remove("theme");
            for thumb in expected["thumbnails"].as_array_mut().unwrap() {
                for name in ["Height", "corner", "border", "margin", "font"] {
                    thumb[name] = json!(thumb[name].as_f64().unwrap());
                }
            }
            assert_eq!(actual, expected, "{c}");
            for subscription in subscriptions {
                subscription.dispose();
            }
            assert!(page.named::<RibbonGallery>("Themes").items().count() > 0);
        }
    });
}

fn paint_value(brush: Option<&Rc<dyn IBrush>>) -> String {
    let Some(brush) = brush else {
        return "null".into();
    };
    let c = brush
        .as_solid_color_brush()
        .expect("source solid brush fixture")
        .color()
        .to_uint32();
    match c {
        0xff102030 => "background".into(),
        0xff405060 => "foreground".into(),
        0xff112233 => "a.bg".into(),
        0xff445566 => "a.fg".into(),
        0xff778899 => "changed.fg".into(),
        0xffa1b2c3 => "b.bg".into(),
        0xffd4e5f6 => "b.fg".into(),
        0xffabcdef => "new.bg".into(),
        0xfffedcba => "new.fg".into(),
        _ => format!("#{c:08X}"),
    }
}

#[test]
fn powerpoint_keyboard_preview_cancel_commit_preserves_source_parameter_and_invokes_once() {
    run(|| {
        use ferroui_base::input::{KeyModifiers, NavigationMethod, PhysicalKey, RawInputModifiers};
        use ferroui_controls::{Button, automation::AutomationProperties};
        use ferroui_headless::HeadlessWindowExtensions;
        let s = settings();
        let page = PowerPointPage::new_with_settings(s.clone());
        page.ribbon()
            .set_display_mode(ferroribbon_core::RibbonDisplayMode::Classic);
        page.ribbon().select_tab_id("design");
        let w = mount(page.clone());
        w.set_width(2000.);
        Dispatcher::ui_thread().run_jobs(None);
        let gallery = page.named::<ferroribbon::controls::RibbonGallery>("Themes");
        let facet = gallery
            .get_visual_descendants()
            .filter_map(|c| c.cast::<Button>())
            .find(|b| AutomationProperties::get_name(b).as_deref() == Some("Facet"))
            .expect("original Facet gallery button");
        let other = page
            .ribbon()
            .find_item(Some("pp.formatBackground".into()))
            .unwrap();
        assert!(facet.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            paint_value(page.named::<Border>("Slide").background().as_ref()),
            "#FF90C226"
        );
        assert!(s.log().borrow().is_empty());
        assert!(other.focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            paint_value(page.named::<Border>("Slide").background().as_ref()),
            "#FFFFFFFF"
        );
        assert!(facet.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
        w.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        w.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            s.log().borrow().as_slice(),
            &["03:04:05  PowerPoint · pp.themes (RibbonSpace.Controls.RibbonGalleryItem)"]
        );
        assert!(other.focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            paint_value(page.named::<Border>("Slide").background().as_ref()),
            "#FF90C226"
        );
        w.close();
    });
}

#[test]
fn tools_native_context_and_status_setter_reentry_read_current_labels_in_source_order() {
    run(|| {
        use ferroui_base::{FerroObject, FerroObjectExtensions, reactive::AnonymousObserver};
        let page = ToolsPage::new_with_settings(settings());
        let w = mount(page.clone());
        let button = page
            .tools()
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        let weak = button.downgrade();
        let context = page.named::<RibbonContextualToolBar>("Options");
        let a = FerroObjectExtensions::get_observable(
            &*context.upcast::<FerroObject>(),
            RibbonContextualToolBar::active_context_property(),
        )
        .subscribe(Rc::new(AnonymousObserver::new(
            move |context: Option<String>| {
                if context.as_deref() == Some("move") {
                    weak.upgrade()
                        .unwrap()
                        .set_label(Some("after context".into()));
                }
            },
        )));
        let weak = button.downgrade();
        let b = FerroObjectExtensions::get_observable(
            &*page
                .named::<TextBlock>("ToolStatus")
                .upcast::<FerroObject>(),
            TextBlock::text_property(),
        )
        .subscribe(Rc::new(AnonymousObserver::new(
            move |status: Option<String>| {
                if status.as_deref() == Some("after context") {
                    weak.upgrade()
                        .unwrap()
                        .set_label(Some("after status".into()));
                }
            },
        )));
        button.set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            page.named::<TextBlock>("ToolStatus").text().as_deref(),
            Some("after context")
        );
        assert_eq!(
            page.named::<TextBlock>("CanvasText").text().as_deref(),
            Some("after status is active")
        );
        a.dispose();
        b.dispose();
        w.close();
    });
}

#[test]
fn office_compiled_and_runtime_class_documents_construct_the_same_native_pages() {
    run(|| {
        use ferroui_base::{data::core::ValueTypes, utilities::UriRef};
        use ferroui_markup_xaml::FerroXamlLoader;
        use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
        FerroRuntimeXamlLoader::register_class_document(
            PowerPointPage::TYPE,
            "ferres://ferroribbon_sample/PowerPointPage.xaml",
        );
        FerroRuntimeXamlLoader::register_class_document(
            ToolsPage::TYPE,
            "ferres://ferroribbon_sample/ToolsPage.xaml",
        );
        for (name, markup) in [
            ("PowerPointPage", include_str!("../PowerPointPage.xaml")),
            ("ToolsPage", include_str!("../ToolsPage.xaml")),
        ] {
            for runtime in [false, true] {
                let value = if runtime {
                    FerroRuntimeXamlLoader::parse(markup, Some(&crate::ASSEMBLY)).unwrap()
                } else {
                    FerroXamlLoader::load(
                        &UriRef::parse_absolute(&format!(
                            "ferres://ferroribbon_sample/{name}.xaml"
                        ))
                        .unwrap()
                        .into(),
                        None,
                    )
                    .unwrap()
                };
                let object = ValueTypes::as_object(&*value).unwrap();
                let w = if name == "PowerPointPage" {
                    let page = object.cast::<PowerPointPage>().unwrap();
                    assert_eq!(page.ribbon().tabs().count(), 6);
                    assert_eq!(
                        page.named::<TextBlock>("Status").text().as_deref(),
                        Some("Hover the Themes gallery for live preview")
                    );
                    mount(page)
                } else {
                    let page = object.cast::<ToolsPage>().unwrap();
                    assert_eq!(page.tools().items().count(), 14);
                    assert_eq!(
                        page.named::<TextBlock>("ToolStatus").text().as_deref(),
                        Some("Brush tool")
                    );
                    mount(page)
                };
                w.close();
            }
        }
    });
}

#[test]
fn office_views_models_and_selected_theme_release_after_native_detach() {
    run(|| {
        use ferroribbon::controls::RibbonGallery;
        let s = settings();
        let weak_settings = Rc::downgrade(&s);
        let pp = PowerPointPage::new_with_settings(s.clone());
        let tools = ToolsPage::new_with_settings(s.clone());
        let weak_pp = pp.downgrade();
        let weak_tools = tools.downgrade();
        let pp_vm = Rc::downgrade(&pp.view_model());
        let tools_vm = Rc::downgrade(&tools.view_model());
        let theme = RibbonGalleryItem::new();
        let weak_theme = theme.downgrade();
        pp.named::<RibbonGallery>("Themes")
            .pick(Some(Rc::new(theme.clone())));
        drop(theme);
        assert!(weak_theme.upgrade().is_some());
        let a = mount(pp.clone());
        let b = mount(tools.clone());
        a.set_content(None);
        b.set_content(None);
        a.close();
        b.close();
        drop(pp);
        drop(tools);
        drop(a);
        drop(b);
        drop(s);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_pp.upgrade().is_none());
        assert!(weak_tools.upgrade().is_none());
        assert!(pp_vm.upgrade().is_none());
        assert!(tools_vm.upgrade().is_none());
        assert!(weak_theme.upgrade().is_none());
        assert!(weak_settings.upgrade().is_none());
    });
}

#[test]
fn tools_all_source_application_cases_match_actual_native_controls() {
    run(|| {
        use ferroribbon::controls::RibbonButton;
        use ferroui_base::{
            BoxedValue, FerroObject, FerroObjectExtensions, reactive::AnonymousObserver,
        };
        let fixture = fixture();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.iter().filter(|c| c["page"] == "tools").count(), 48);
        for c in cases.iter().filter(|c| c["page"] == "tools") {
            let s = settings();
            let page = ToolsPage::new_with_settings(s.clone());
            let w = mount(page.clone());
            let buttons = page
                .tools()
                .items()
                .iter()
                .flatten()
                .filter_map(|c| c.cast::<RibbonToggleButton>())
                .collect::<Vec<_>>();
            let context = page.named::<RibbonContextualToolBar>("Options");
            let status = page.named::<TextBlock>("ToolStatus");
            let mut subscriptions = Vec::new();
            let invoke = |item: Ref<Control>| page.tools().on_item_invoked(item, None, None);
            match c["kind"].as_str().unwrap() {
                "initial" => {}
                "check" => buttons[c["index"].as_u64().unwrap() as usize]
                    .set_is_checked(c["value"].as_bool()),
                "changed" => {
                    buttons[0].set_label(c["label"].as_str().map(str::to_owned));
                    let tag: Option<BoxedValue> = if let Some(text) = c["tag"].as_str() {
                        Some(Rc::new(text.to_owned()))
                    } else {
                        c["tag"].as_i64().map(|n| Rc::new(n) as BoxedValue)
                    };
                    buttons[0].set_tag(tag);
                    buttons[0].set_is_checked(Some(true));
                    invoke(buttons[0].clone().upcast());
                }
                "invoke" => {
                    let item = RibbonButton::new();
                    item.set_label(c["label"].as_str().map(str::to_owned));
                    invoke(item.upcast());
                }
                "other" => invoke(Border::new().upcast()),
                "append" => {
                    let item = RibbonToggleButton::new();
                    item.set_label(Some("new".into()));
                    item.set_tag(Some(Rc::new("text".to_owned())));
                    page.tools().items().add(Some(item.clone().upcast()));
                    item.set_is_checked(Some(true));
                    invoke(item.upcast());
                }
                "lateLabels" => {
                    let weak = buttons[0].downgrade();
                    subscriptions.push(
                        FerroObjectExtensions::get_observable(
                            &*context.clone().upcast::<FerroObject>(),
                            RibbonContextualToolBar::active_context_property(),
                        )
                        .subscribe(Rc::new(AnonymousObserver::new(
                            move |context: Option<String>| {
                                if context.as_deref() == Some("move") {
                                    weak.upgrade()
                                        .unwrap()
                                        .set_label(Some("after context".into()));
                                }
                            },
                        ))),
                    );
                    let weak = buttons[0].downgrade();
                    subscriptions.push(
                        FerroObjectExtensions::get_observable(
                            &*status.clone().upcast::<FerroObject>(),
                            TextBlock::text_property(),
                        )
                        .subscribe(Rc::new(AnonymousObserver::new(
                            move |text: Option<String>| {
                                if text.as_deref() == Some("after context") {
                                    weak.upgrade()
                                        .unwrap()
                                        .set_label(Some("after status".into()));
                                }
                            },
                        ))),
                    );
                    buttons[0].set_is_checked(Some(true));
                }
                "contextReentry" => {
                    let weak = buttons[5].downgrade();
                    let entered = Cell::new(false);
                    subscriptions.push(
                        FerroObjectExtensions::get_observable(
                            &*context.clone().upcast::<FerroObject>(),
                            RibbonContextualToolBar::active_context_property(),
                        )
                        .subscribe(Rc::new(AnonymousObserver::new(
                            move |context: Option<String>| {
                                if context.as_deref() == Some("move") && !entered.replace(true) {
                                    weak.upgrade().unwrap().set_is_checked(Some(true));
                                }
                            },
                        ))),
                    );
                    buttons[0].set_is_checked(Some(true));
                }
                _ => panic!("unhandled source case {c}"),
            }
            Dispatcher::ui_thread().run_jobs(None);
            let actual = json!({"context":context.active_context(),"status":status.text(),
                "canvas":page.named::<TextBlock>("CanvasText").text(),
                "log":s.log().borrow().iter().rev().map(|s|s.strip_prefix("03:04:05  ").unwrap().to_owned()).collect::<Vec<_>>()});
            let mut expected = c["state"].clone();
            // The descriptor deliberately has no radio-group engine. Real group
            // state is checked below and in the library's source-backed tests.
            expected.as_object_mut().unwrap().remove("checks");
            assert_eq!(actual, expected, "{c}");
            if c["kind"] == "check" && c["value"] == true && c["index"] != 2 {
                assert_eq!(
                    buttons
                        .iter()
                        .filter(|b| b.is_checked() == Some(true))
                        .count(),
                    1
                );
            }
            for subscription in subscriptions {
                subscription.dispose();
            }
            w.set_content(None);
            w.close();
        }
    });
}
