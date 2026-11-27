use super::ExcelPage;
use crate::{demo_settings::DemoSettings, excel_model_native_tests};
use ferroribbon_core::{RibbonDensity, RibbonDisplayMode, model::*, theming::RibbonColor};
use ferroui_base::{
    FerroObject, FerroObjectExtensions, Ref,
    media::{FontStyle, FontWeight, IBrush},
    reactive::AnonymousObserver,
    threading::Dispatcher,
};
use ferroui_controls::{Border, Control, Grid, Window};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};
fn settings() -> Rc<DemoSettings> {
    DemoSettings::new(Rc::new(|| Ok("03:04:05".into())))
}
fn mount(p: &Ref<ExcelPage>) -> Ref<Window> {
    let w = Window::new();
    w.set_width(2000.);
    w.set_height(1000.);
    w.set_content(Some(Control::boxed(p.clone())));
    w.show();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn paint(v: Option<Rc<dyn IBrush>>) -> Option<u32> {
    v.map(|b| b.as_solid_color_brush().unwrap().color().to_uint32())
}
fn snapshot(p: &ExcelPage, s: &DemoSettings) -> Value {
    let vm = p.view_model();
    let cells=[(1,1),(2,2),(24,12)].map(|key|{let t=p.cell_text(key).unwrap();let b=t.parent().and_then(|b|b.cast::<Border>());json!({"row":key.0,"column":key.1,"text":t.text(),"weight":t.font_weight().0,"italic":t.font_style()==FontStyle::Italic,"underline":t.text_decorations().is_some(),"foreground":paint(t.foreground()),"background":b.as_ref().and_then(|b|paint(b.background())),"stroke":b.as_ref().and_then(|b|paint(b.border_brush())),"width":b.map(|b|b.border_thickness().left),"height":t.font_size(),"family":t.font_family().name(),"alignment":match t.horizontal_alignment(){ferroui_base::layout::HorizontalAlignment::Left=>0,ferroui_base::layout::HorizontalAlignment::Center=>1,ferroui_base::layout::HorizontalAlignment::Right=>2,ferroui_base::layout::HorizontalAlignment::Stretch=>3}})});
    let grid = p.named::<Grid>("Sheet");
    json!({"selected":[p.sheet().selected().0,p.sheet().selected().1],"name":vm.selected_cell(),"status":vm.status(),"bold":vm.bold().is_checked(),"italic":vm.italic().is_checked(),"underline":vm.underline().is_checked(),"align":vm.align().borrow().iter().map(|a|a.is_checked()).collect::<Vec<_>>(),"density":vm.ribbon().density().raw(),"display":vm.ribbon().display_mode().raw(),"cells":cells,"log":s.log().borrow().iter().rev().map(|s|s.strip_prefix("03:04:05  ").unwrap().to_owned()).collect::<Vec<_>>(),"columns":grid.column_definitions().snapshot().iter().map(|c|c.width().value()).collect::<Vec<_>>(),"rows":grid.row_definitions().snapshot().iter().map(|r|r.height().value()).collect::<Vec<_>>(),"children":grid.children().count(),"stored":(1..=24).flat_map(|r|(1..=12).map(move|c|(r,c))).filter(|key|p.cell_text(*key).is_some()).count(),"titleIdentity":p.named::<ferroribbon::controls::RibbonTitleBar>("TitleBar").ribbon()==Some(p.ribbon())})
}
fn normalize(v: &mut Value) {
    match v {
        Value::Number(n) => *v = json!(n.as_f64().unwrap()),
        Value::Array(a) => a.iter_mut().for_each(normalize),
        Value::Object(m) => m.values_mut().for_each(normalize),
        _ => {}
    }
}
#[test]
fn excel_whole_source_page_cases_match_actual_native_sheet_and_model_actions() {
    excel_model_native_tests::run(|| {
        let fixture: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/excel-page.json")).unwrap();
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 83);
        for c in fixture["cases"].as_array().unwrap() {
            let s = settings();
            let kind = c["kind"].as_str().unwrap();
            if kind == "settings" {
                s.apply(
                    RibbonDensity::from_raw(c["density"].as_i64().unwrap() as i32),
                    c["mode"]
                        .as_i64()
                        .map(|m| RibbonDisplayMode::from_raw(m as i32)),
                );
            }
            let p = ExcelPage::new_with_settings(s.clone());
            let vm = p.view_model();
            let mut subscription = None;
            match kind {
                "initial" | "settings" => {}
                "selection" => {
                    let key = (
                        c["row"].as_i64().unwrap() as i32,
                        c["column"].as_i64().unwrap() as i32,
                    );
                    p.sheet().select(key);
                    vm.bold().set_is_checked(true);
                    p.sheet().select(key);
                }
                "family" => vm
                    .font_family()
                    .set_text(c["text"].as_str().map(Into::into)),
                "size" => vm.font_size().set_text(c["text"].as_str().map(Into::into)),
                "style" => match c["style"].as_str().unwrap() {
                    "bold" => vm.bold().set_is_checked(true),
                    "italic" => vm.italic().set_is_checked(true),
                    "underline" => vm.underline().set_is_checked(true),
                    "fillNone" => vm.fill_color().set_selected_color(None),
                    "fontNone" => vm.font_color().set_selected_color(None),
                    "fill" => vm
                        .fill_color()
                        .set_selected_color(Some(RibbonColor::from_rgb(0x12, 0x34, 0x56))),
                    "font" => vm
                        .font_color()
                        .set_selected_color(Some(RibbonColor::from_rgb(0x12, 0x34, 0x56))),
                    "label" => vm.bold().set_label(Some("renamed".into())),
                    "alignCenter" => vm.align().borrow()[1].set_is_checked(true),
                    "alignRight" => vm.align().borrow()[2].set_is_checked(true),
                    "alignBoth" => {
                        let a = vm.align().borrow().clone();
                        a[2].set_is_checked(true);
                        a[1].set_is_checked(true);
                    }
                    "replaceAlign" => {
                        let replacement = RibbonToggleButtonModel::new(
                            Some("replacement".into()),
                            Some("replacement".into()),
                        );
                        replacement.set_is_checked(true);
                        vm.align().borrow_mut()[1] = replacement;
                        vm.bold().set_label(Some("format".into()));
                    }
                    _ => panic!("{c}"),
                },
                "cellSync" => {
                    let cell = p.cell_text((2, 2)).unwrap();
                    cell.set_font_weight(FontWeight(c["weight"].as_i64().unwrap() as i32));
                    cell.set_font_style(if c["italic"] == true {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    });
                    p.sheet().select((2, 2));
                }
                "invoke" => {
                    vm.item_invoked(
                        c["id"].as_str().map(str::to_owned),
                        c["command"].as_str().map(str::to_owned),
                    );
                }
                "weightReentry" => {
                    let weak = p.downgrade();
                    let entered = Cell::new(false);
                    subscription = Some(
                        FerroObjectExtensions::get_observable(
                            &*p.cell_text((1, 1)).unwrap().upcast::<FerroObject>(),
                            ferroui_controls::TextBlock::font_weight_property(),
                        )
                        .subscribe(Rc::new(AnonymousObserver::new(
                            move |w: FontWeight| {
                                if w.0 == 700 && !entered.replace(true) {
                                    weak.upgrade().unwrap().sheet().select((2, 2));
                                }
                            },
                        ))),
                    );
                    vm.bold().set_is_checked(true);
                }
                "missingParent" => {
                    let text = p.cell_text((2, 2)).unwrap();
                    text.parent()
                        .unwrap()
                        .cast::<Border>()
                        .unwrap()
                        .set_child(None::<Ref<Control>>);
                    p.sheet().select((2, 2));
                    vm.bold().set_is_checked(true);
                }
                _ => panic!("unhandled {c}"),
            }
            Dispatcher::ui_thread().run_jobs(None);
            let mut actual = snapshot(&p, &s);
            let mut expected = c["state"].clone();
            for (a, e) in actual["cells"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .zip(expected["cells"].as_array_mut().unwrap())
            {
                if e["family"].is_null() {
                    a.as_object_mut().unwrap().remove("family");
                    e.as_object_mut().unwrap().remove("family");
                }
            }
            normalize(&mut actual);
            normalize(&mut expected);
            assert_eq!(actual, expected, "{c}");
            if let Some(s) = subscription {
                s.dispose();
            }
        }
    });
}
#[test]
fn excel_actual_native_page_model_controls_apply_formats_and_release() {
    excel_model_native_tests::run(|| {
        let s = settings();
        let p = ExcelPage::new_with_settings(s.clone());
        let w = mount(&p);
        assert_eq!(p.ribbon().tabs().count(), 6);
        assert_eq!(p.cell_text((1, 1)).unwrap().font_size(), 13.);
        let bold = p.ribbon().find_item(Some("bold".into())).unwrap();
        excel_model_native_tests::invoke(bold);
        assert!(p.view_model().bold().is_checked());
        assert_eq!(p.cell_text((1, 1)).unwrap().font_size(), 11.);
        assert_eq!(
            paint(p.cell_text((1, 1)).unwrap().foreground()),
            Some(0xffc00000)
        );
        assert_eq!(p.view_model().status().as_deref(), Some("bold"));
        assert_eq!(s.log().borrow()[0], "03:04:05  Excel · bold");
        let weak = p.downgrade();
        let model = Rc::downgrade(&p.view_model());
        let sheet = Rc::downgrade(&p.sheet());
        let ribbon = p.ribbon().downgrade();
        w.set_content(None);
        w.close();
        drop(w);
        drop(p);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        assert!(model.upgrade().is_none());
        assert!(sheet.upgrade().is_none());
        assert!(ribbon.upgrade().is_none());
    });
}
#[test]
fn excel_native_pointer_selection_keyboard_and_resolved_command_counts_follow_source() {
    excel_model_native_tests::run(|| {
        use ferroui_base::{
            Point,
            input::{MouseButton, PhysicalKey, RawInputModifiers},
        };
        use ferroui_headless::HeadlessWindowExtensions;
        let s = settings();
        let p = ExcelPage::new_with_settings(s.clone());
        let w = mount(&p);
        let cell = p.cell_text((2, 2)).unwrap();
        let border = cell.parent().unwrap().cast::<Border>().unwrap();
        let point = border.translate_point(Point::new(8., 8.), &w).unwrap();
        w.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        w.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(p.sheet().selected(), (2, 2));
        assert_eq!(p.view_model().selected_cell().as_deref(), Some("B2"));
        assert_eq!(cell.font_size(), 13.);
        assert!(s.log().borrow().is_empty());
        let bold = p.ribbon().find_item(Some("bold".into())).unwrap();
        assert!(bold.focus());
        w.key_press_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        w.key_release_qwerty(PhysicalKey::Space, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(p.view_model().bold().is_checked());
        assert_eq!(cell.font_size(), 11.);
        assert_eq!(cell.font_weight(), FontWeight::Bold);
        assert_eq!(s.log().borrow().len(), 1);
        let paste = p
            .ribbon()
            .find_item(Some("paste".into()))
            .unwrap()
            .cast::<ferroribbon::controls::RibbonSplitButton>()
            .unwrap();
        paste.set_command_parameter(Some(Rc::new("cells".to_owned())));
        let before = s.log().borrow().len();
        excel_model_native_tests::invoke(paste.upcast());
        assert_eq!(s.log().borrow().len(), before + 2);
        assert_eq!(s.log().borrow()[0], "03:04:05  Excel · paste");
        assert_eq!(s.log().borrow()[1], "03:04:05  Excel · Paste (cells)");
        let map: Value =
            serde_json::from_str(include_str!("../../../docs/excel-page-markup.json")).unwrap();
        for value in map["source_values"].as_array().unwrap() {
            assert_eq!(
                p.cell_text((
                    value["row"].as_i64().unwrap() as i32,
                    value["column"].as_i64().unwrap() as i32
                ))
                .unwrap()
                .text()
                .as_deref(),
                value["text"].as_str()
            );
        }
        w.set_content(None);
        w.close();
    });
}
#[test]
fn excel_source_page_compiled_and_runtime_markup_share_actual_models_and_cells() {
    excel_model_native_tests::run(|| {
        use ferroui_base::{data::core::ValueTypes, utilities::UriRef};
        use ferroui_markup_xaml::FerroXamlLoader;
        use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
        FerroRuntimeXamlLoader::register_class_document(
            ExcelPage::TYPE,
            "ferres://ferroribbon_sample/ExcelPage.xaml",
        );
        for runtime in [false, true] {
            let value = if runtime {
                FerroRuntimeXamlLoader::parse(
                    include_str!("../ExcelPage.xaml"),
                    Some(&crate::ASSEMBLY),
                )
                .unwrap()
            } else {
                FerroXamlLoader::load(
                    &UriRef::parse_absolute("ferres://ferroribbon_sample/ExcelPage.xaml")
                        .unwrap()
                        .into(),
                    None,
                )
                .unwrap()
            };
            let page = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ExcelPage>()
                .unwrap();
            let w = mount(&page);
            assert_eq!(page.named::<Grid>("Sheet").children().count(), 325);
            assert_eq!(page.view_model().selected_cell().as_deref(), Some("A1"));
            assert_eq!(page.cell_text((1, 1)).unwrap().font_size(), 13.);
            page.view_model().font_size().set_text(Some("24".into()));
            assert_eq!(page.cell_text((1, 1)).unwrap().font_size(), 24.);
            w.set_content(None);
            w.close();
        }
    });
}
