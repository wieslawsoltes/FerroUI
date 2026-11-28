use crate::word_view_model::{WordAction, WordAlignment, WordServices, WordViewModel};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Services {
    log: RefCell<Vec<String>>,
    trace: RefCell<Vec<String>>,
    contexts: RefCell<Vec<String>>,
    root: bool,
    theme: Cell<u8>,
}
impl WordServices for Services {
    fn write(&self, text: &str) -> Result<(), String> {
        self.log.borrow_mut().push(text.into());
        Ok(())
    }
    fn hide_table_flyout(&self) {
        self.trace.borrow_mut().push("hideTable".into());
    }
    fn set_table_context(&self, on: bool) {
        self.contexts
            .replace(if on { vec!["table".into()] } else { vec![] });
        self.trace
            .borrow_mut()
            .push(format!("contexts:{}", if on { "table" } else { "" }));
    }
    fn toggle_dark_theme(&self) {
        if self.root {
            self.theme.set(if self.theme.get() == 2 { 1 } else { 2 });
        }
    }
}
fn numeric(n: f64) -> String {
    let mut s = String::new();
    ferroui_system::invariant_numbers::write_double(&mut s, n).unwrap();
    s
}
fn number(v: &Value) -> f64 {
    match v.as_str().unwrap() {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        s => s.parse().unwrap(),
    }
}
fn string(v: &Value) -> Option<String> {
    v.as_str().map(str::to_owned)
}
fn invoked(
    id: Option<String>,
    command_id: Option<String>,
    parameter: Option<String>,
) -> WordAction {
    WordAction::Invoked {
        item_id: id,
        command_id,
        parameter: parameter
            .map(|p| Rc::new(move || Ok(p.clone())) as crate::word_view_model::WordParameterText),
    }
}
fn snapshot(vm: &WordViewModel, s: &Services) -> Value {
    json!({"bodySize":numeric(vm.body_font_size()),"family":vm.body_font_family(),"bodyWeight":if vm.body_bold(){700}else{400},"bodyStyle":if vm.body_italic(){"Italic"}else{"Normal"},"decorations":i32::from(vm.body_underline())|if vm.body_strike(){2}else{0},"bodyColor":vm.body_foreground(),"alignment":match vm.body_alignment(){WordAlignment::Left=>"Left",WordAlignment::Center=>"Center",WordAlignment::Right=>"Right",WordAlignment::Justify=>"Justify"},"titleSize":numeric(vm.title_font_size()),"titleColor":vm.title_foreground(),"titleStyle":if vm.title_italic(){"Italic"}else{"Normal"},"titleWeight":if vm.title_bold(){700}else{400},"sizeText":vm.font_size_text(),"tableVisible":vm.table_visible(),"pageColor":vm.page_background(),"last":vm.last_command(),"checks":(0..4).map(|i|vm.checked(i)).collect::<Vec<_>>(),"style":vm.selected_style(),"zoom":vm.zoom_scale().map(|scale|json!({"scale":numeric(scale),"center":"380"})),"theme":if s.root{Some(match s.theme.get(){1=>"Light",2=>"Dark",_=>"Default"})}else{None},"contexts":s.contexts.borrow().clone(),"trace":s.trace.borrow().clone(),"log":s.log.borrow().clone()})
}
#[test]
fn whole_unchanged_word_class_matches_rust_presentation_cases() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/word-page.json")).unwrap();
    let mut compared = 0;
    for c in fixture["cases"].as_array().unwrap() {
        if c["kind"] == "settings" {
            continue;
        }
        let services = Rc::new(Services {
            log: RefCell::new(vec![]),
            trace: RefCell::new(vec![]),
            contexts: RefCell::new(vec![]),
            root: c["root"].as_bool().unwrap_or(true),
            theme: Cell::new(0),
        });
        let vm = WordViewModel::new(services.clone());
        let mut subscription = None;
        let action = match c["kind"].as_str().unwrap() {
            "styleReentry" => {
                use ferroui_system::component_model::INotifyPropertyChanged;
                let weak = Rc::downgrade(&vm);
                let entered = Cell::new(false);
                subscription = Some(vm.property_changed().add(Rc::new(move |name| {
                    if name == "TitleFontSize"
                        && !entered.replace(true)
                        && let Some(vm) = weak.upgrade()
                    {
                        vm.apply(WordAction::StyleClick(Some("Strong".into())))
                            .unwrap();
                    }
                })));
                Some(WordAction::StyleClick(Some("Heading2".into())))
            }
            "parameterTiming" => {
                use ferroui_system::component_model::INotifyPropertyChanged;
                let value = Rc::new(RefCell::new("before".to_string()));
                let updating = value.clone();
                subscription = Some(vm.property_changed().add(Rc::new(move |name| {
                    if name == "BodyFontSize" {
                        updating.replace("after".into());
                    }
                })));
                Some(WordAction::Invoked {
                    item_id: Some("growFont".into()),
                    command_id: None,
                    parameter: Some(Rc::new(move || Ok(value.borrow().clone()))),
                })
            }
            "initial" => None,
            "checks" => {
                for (i, v) in c["values"].as_array().unwrap().iter().enumerate() {
                    vm.set_checked(i, v.as_bool());
                }
                Some(invoked(Some("bold".into()), None, None))
            }
            "invoke" => {
                for i in 0..4 {
                    vm.set_checked(i, Some(true));
                }
                Some(invoked(
                    string(&c["id"]),
                    string(&c["command"]),
                    string(&c["parameter"]),
                ))
            }
            "resize" => {
                vm.set_body_font_size(number(&c["size"]));
                Some(invoked(string(&c["id"]), None, None))
            }
            "size" => {
                vm.set_font_size_text(string(&c["value"]));
                Some(WordAction::FontSizeCommitted(string(&c["value"])))
            }
            "family" => Some(WordAction::FontFamilyCommitted(string(&c["value"]))),
            "tab" => Some(WordAction::TabChanged(string(&c["value"]))),
            "style" => {
                vm.apply(WordAction::StyleClick(Some("Heading2".into())))
                    .unwrap();
                if c["preview"] == true {
                    vm.apply(WordAction::StylePreview(string(&c["style"])))
                        .unwrap();
                    Some(WordAction::StylePreview(None))
                } else {
                    Some(WordAction::StyleClick(string(&c["style"])))
                }
            }
            "color" => {
                let color = c["color"].as_u64().map(|n| n as u32);
                Some(if c["action"] == "pageColor" {
                    WordAction::PageColor(color)
                } else {
                    WordAction::FontColor(color)
                })
            }
            "table" => {
                vm.apply(WordAction::TablePicked {
                    rows: c["rows"].as_i64().unwrap() as i32,
                    columns: c["columns"].as_i64().unwrap() as i32,
                })
                .unwrap();
                vm.apply(WordAction::ToggleTable).unwrap();
                Some(WordAction::ToggleTable)
            }
            "zoom" => Some(WordAction::Zoom(number(&c["value"]))),
            "theme" => {
                for _ in 0..2 {
                    vm.apply(WordAction::ToggleDark).unwrap();
                }
                Some(WordAction::ToggleDark)
            }
            "save" => Some(if c["action"] == "save" {
                WordAction::Save
            } else {
                WordAction::BackstageSave
            }),
            _ => panic!("Unknown case {c}"),
        };
        if let Some(action) = action {
            vm.apply(action).unwrap();
        }
        if let Some(token) = subscription {
            use ferroui_system::component_model::INotifyPropertyChanged;
            vm.property_changed().remove(token);
        }
        let mut expected = c["state"].clone();
        for key in [
            "density",
            "display",
            "wordIdentity",
            "titleIdentity",
            "candidates",
        ] {
            expected.as_object_mut().unwrap().remove(key);
        }
        assert_eq!(snapshot(&vm, &services), expected, "{c}");
        compared += 1;
    }
    assert_eq!(compared, 263);
}
