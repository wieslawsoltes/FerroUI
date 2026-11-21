use super::*;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    task::{Context, Poll, Waker},
};
struct Surface {
    current: RefCell<String>,
    next: Cell<i32>,
    ribbon: Cell<Option<i32>>,
    trace: RefCell<Vec<String>>,
    waits: RefCell<Vec<u64>>,
    input: Value,
    saved: RefCell<Option<Vec<u8>>>,
    action_failed: Cell<bool>,
    popup_failures: Cell<usize>,
}
fn number_format(spec: &Value) -> ferroui_base::utilities::NumberFormatInfo {
    let mut format = ferroui_base::utilities::NumberFormatInfo::new();
    if let Some(value) = spec["positiveSign"].as_str() {
        format.set_positive_sign(value);
    }
    if let Some(value) = spec["negativeSign"].as_str() {
        format.set_negative_sign(value);
    }
    if let Some(value) = spec["negativePattern"].as_i64() {
        format.set_number_negative_pattern(value as i32);
    }
    format
}
#[test]
fn capture_integer_formats_match_actual_source_int32_parsing() {
    let data: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/capture-numbers.json")).unwrap();
    let mut mismatches = vec![];
    for case in data["formats"].as_array().unwrap() {
        let format = number_format(&case["result"]);
        for (value, expected) in case["input"]["values"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["result"]["values"].as_array().unwrap())
        {
            let actual =
                crate::native_capture::capture_integer_with_format(value.as_str(), &format);
            if json!(actual) != *expected {
                mismatches.push(json!({"positive":format.positive_sign(),"negative":format.negative_sign(),
                    "pattern":format.number_negative_pattern(),"text":value,"expected":expected,"actual":actual}));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} differences: {}",
        mismatches.len(),
        json!(mismatches)
    );
}
impl Surface {
    fn new(input: &Value) -> Self {
        let page = input["page"].as_str().unwrap_or("word");
        let has = page != "tools" && page != "settings";
        Self {
            current: RefCell::new(page.into()),
            next: Cell::new(i32::from(has)),
            ribbon: Cell::new(has.then_some(1)),
            trace: RefCell::new(vec![]),
            waits: RefCell::new(vec![]),
            input: input.clone(),
            saved: RefCell::new(None),
            action_failed: Cell::new(false),
            popup_failures: Cell::new(0),
        }
    }
    fn log(&self, s: impl Into<String>) {
        self.trace.borrow_mut().push(s.into());
    }
    fn rlog(&self, r: i32, s: impl AsRef<str>) {
        self.log(format!("r{r}:{}", s.as_ref()));
    }
    fn missing(&self) -> bool {
        self.input["missing"].as_bool().unwrap_or(false)
    }
}
impl CaptureSurface for Surface {
    type Ribbon = i32;
    type Cad = ();
    fn environment(&self, k: &str) -> Result<Option<String>, CaptureError> {
        Ok(self.input["env"][k].as_str().map(str::to_owned))
    }
    async fn wait(&self, d: Duration) {
        self.waits.borrow_mut().push(d.as_millis() as u64);
    }
    fn parse_integer(&self, v: Option<&str>) -> Option<i32> {
        crate::native_capture::capture_integer_with_format(
            v,
            &number_format(&self.input["numberFormat"]),
        )
    }
    fn resize(&self, w: i32, h: i32) -> Result<(), CaptureError> {
        let scale = self.input["scale"].as_f64().unwrap_or(1.);
        self.log(format!(
            "resize:{}:{}",
            (f64::from(w) * scale) as i32,
            (f64::from(h) * scale) as i32
        ));
        Ok(())
    }
    fn dark(&self) -> Result<(), CaptureError> {
        self.log("theme:Dark");
        Ok(())
    }
    fn show(&self, key: &str) -> Result<(), CaptureError> {
        self.log(format!("page:{key}"));
        if !["word", "excel", "powerpoint", "tools", "cad", "settings"].contains(&key) {
            return Err(CaptureError("KeyNotFoundException".into()));
        }
        self.current.replace(key.into());
        self.ribbon.set(if key == "tools" || key == "settings" {
            None
        } else {
            let id = self.next.get() + 1;
            self.next.set(id);
            Some(id)
        });
        Ok(())
    }
    fn find_ribbon(&self) -> Result<Option<i32>, CaptureError> {
        Ok(self.ribbon.get())
    }
    fn current_cad(&self) -> Result<Option<()>, CaptureError> {
        Ok((self.current.borrow().as_str() == "cad").then_some(()))
    }
    fn action(&self, r: &i32, a: CaptureAction) -> Result<(), CaptureError> {
        let value = match a {
            CaptureAction::ShowKeyTips => "keytips:show".into(),
            CaptureAction::HideKeyTips => "keytips:hide".into(),
            CaptureAction::SelectTab(key) => format!("tab:{key}"),
            CaptureAction::ActivateHomeHeader => {
                let home = self
                    .input
                    .get("homeId")
                    .and_then(Value::as_str)
                    .unwrap_or("home");
                if !self.missing() && home == "home" {
                    self.rlog(*r, "keytips:activate:home");
                }
                return Ok(());
            }
            CaptureAction::Backstage => "backstage:True".into(),
            CaptureAction::Simplified => "layout:Simplified".into(),
            CaptureAction::Classic => "layout:Classic".into(),
            CaptureAction::TabsOnly => "visibility:TabsOnly".into(),
            CaptureAction::FullScreen => "visibility:FullScreen".into(),
            CaptureAction::OpenMinimized => "minimized:open".into(),
            CaptureAction::Contextual => "context:table|chart".into(),
            CaptureAction::QatBelow => "qat:BelowRibbon".into(),
            CaptureAction::Palette => "palette".into(),
            CaptureAction::CustomizeRibbon => "customize:Ribbon".into(),
            CaptureAction::CustomizeQat => "customize:QuickAccessToolbar".into(),
            CaptureAction::AddQat(s) => format!("qat:add:{s}"),
            CaptureAction::Touch => "density:Touch".into(),
            CaptureAction::Compact => "density:Compact".into(),
            CaptureAction::Gallery | CaptureAction::ColorPicker | CaptureAction::Paste => {
                let id = match a {
                    CaptureAction::Gallery => "stylesGallery",
                    CaptureAction::ColorPicker => "fontColor",
                    _ => "paste",
                };
                self.rlog(*r, format!("find:{id}"));
                if self.missing() {
                    if id == "stylesGallery" {
                        self.rlog(*r, "all");
                    }
                } else {
                    self.log(format!("dropdown:{id}"));
                }
                return Ok(());
            }
            CaptureAction::Overflow => {
                if !self.missing() {
                    self.log("overflow");
                }
                return Ok(());
            }
            CaptureAction::CollapsedGroup => {
                if !self.missing() {
                    self.log("group:collapsed");
                }
                return Ok(());
            }
            CaptureAction::SearchFocus | CaptureAction::SearchText => {
                if !self.missing()
                    && !["tools", "settings"].contains(&self.current.borrow().as_str())
                {
                    self.log(if matches!(a, CaptureAction::SearchFocus) {
                        "search:focus"
                    } else {
                        "search:text:font col"
                    });
                }
                return Ok(());
            }
            CaptureAction::Colorful => {
                self.log("chrome:Colorful");
                return Ok(());
            }
            CaptureAction::SurfaceCad => {
                self.log("surface:Cad");
                return Ok(());
            }
        };
        self.rlog(*r, value);
        Ok(())
    }
    fn cad_action(&self, _: &(), r: &i32, a: CadCaptureAction) -> Result<(), CaptureError> {
        match a {
            CadCaptureAction::SlideOut => {
                self.log("pin:True");
                self.log("slideout");
            }
            CadCaptureAction::ApplicationMenu => self.log("cad:appmenu"),
            CadCaptureAction::InvokeSaveAs => self.log("invoke-saveas"),
            CadCaptureAction::PanelButtons => self.rlog(*r, "visibility:PanelButtons"),
            CadCaptureAction::DrawPopup => self.log("group:draw"),
            CadCaptureAction::PanelTitles => self.rlog(*r, "visibility:PanelTitles"),
            CadCaptureAction::LayersPopup => self.log("group:layers"),
            CadCaptureAction::Float => self.log("cad:float"),
            CadCaptureAction::Tooltip => {
                self.log("tip-delay:0");
                self.log("cad:tip:line");
            }
            CadCaptureAction::Layers => self.log("dropdown:layersDropdown"),
            CadCaptureAction::Contextual => self.log("cad:text"),
            CadCaptureAction::Hatch => self.log("cad:hatch"),
            CadCaptureAction::Light => self.log("cad:light:True"),
            CadCaptureAction::Command(s) => self.log(format!("cad:command:{s}")),
            CadCaptureAction::Circle | CadCaptureAction::Blocks => {
                let id = if matches!(a, CadCaptureAction::Circle) {
                    "circle"
                } else {
                    "insertBlock"
                };
                self.rlog(*r, format!("find:{id}"));
                if !self.missing() {
                    self.log(format!("dropdown:{id}"));
                }
            }
            CadCaptureAction::View => self.rlog(*r, "tab:view"),
            CadCaptureAction::Workspace3d => self.log("cad:workspace:3D Modeling"),
        };
        Ok(())
    }
    fn action_failed(&self, _: &CaptureError) {
        self.action_failed.set(true);
    }
    async fn save(&self, _: &str) -> Result<(), CaptureError> {
        if self.input["kind"] == "top" {
            let frame = compose(self)?;
            self.log(format!("encode:{}:{}:96:96", frame.width, frame.height));
            self.saved.replace(Some(frame.pixels));
        } else {
            self.log("save");
        }
        Ok(())
    }
}
impl CaptureRenderer for Surface {
    type Node = Value;
    type Popup = Value;
    fn root(&self) -> Result<Value, CaptureError> {
        Ok(
            json!({"name":"root", "aw": self.input["logicalWidth"].as_f64().unwrap_or(1.),
            "pw":self.input["pixelWidth"].as_i64().unwrap_or(1), "data":self.input["target"]}),
        )
    }
    fn render(&self, node: &Value) -> Result<BgraFrame, CaptureError> {
        self.log(format!(
            "render:{}",
            node["name"].as_str().unwrap_or("popup")
        ));
        if node["fail"].as_bool().unwrap_or(false) {
            return Err(CaptureError("Injected render failure".into()));
        }
        let pixels = node["data"]
            .as_array()
            .map(|bytes| bytes.iter().map(|b| b.as_u64().unwrap() as u8).collect())
            .unwrap_or_else(|| vec![40, 80, 120, 255]);
        Ok(BgraFrame {
            pixels,
            width: node["pw"].as_i64().unwrap_or(1) as i32,
            height: node["ph"].as_i64().unwrap_or(1) as i32,
        })
    }
    fn actual_width(&self, node: &Value) -> f64 {
        node["aw"].as_f64().unwrap_or(1.)
    }
    fn popups(&self) -> Vec<Value> {
        self.input["popups"].as_array().cloned().unwrap_or_default()
    }
    fn child(&self, popup: &Value) -> Option<Value> {
        (!popup.is_null()).then(|| popup.clone())
    }
    fn candidates(&self, child: &Value) -> [Option<Value>; 3] {
        [
            Some(child.clone()),
            child.get("first").cloned(),
            (child["type"] == "content")
                .then(|| child.get("content").cloned())
                .flatten(),
        ]
    }
    fn origin(&self, target: &Value, _: &Value) -> Result<(f64, f64), CaptureError> {
        Ok((
            target["x"].as_f64().unwrap_or(0.),
            target["y"].as_f64().unwrap_or(0.),
        ))
    }
    fn rendered(&self, _: &Value, _: i32, _: i32) {}
    fn popup_failed(&self, _: &CaptureError) {
        self.popup_failures.set(self.popup_failures.get() + 1);
    }
}
fn ready<T>(f: impl Future<Output = T>) -> T {
    let mut f = std::pin::pin!(f);
    match f.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(v) => v,
        Poll::Pending => panic!("Immediate injected clock expected"),
    }
}
#[test]
fn capture_current_culture_is_read_at_each_parse_without_mutating_its_format() {
    use ferroui_base::utilities::{CultureInfo, NumberFormatInfo};
    struct Restore(CultureInfo);
    impl Drop for Restore {
        fn drop(&mut self) {
            CultureInfo::set_current_formatting(self.0.clone());
        }
    }
    let _restore = Restore(CultureInfo::current_formatting());
    let mut custom = NumberFormatInfo::new();
    custom.set_positive_sign("up");
    custom.set_negative_sign("down");
    custom.set_number_negative_pattern(2);
    CultureInfo::set_current_formatting(
        CultureInfo::invariant_culture().with_number_format(custom),
    );
    let provider = CultureInfo::current_formatting().number_format();
    assert_eq!(
        crate::native_capture::capture_integer(Some("up123")),
        Some(123)
    );
    assert_eq!(
        crate::native_capture::capture_integer(Some("down123")),
        Some(-123)
    );
    assert_eq!(
        crate::native_capture::capture_integer(Some("down 123")),
        None
    );
    assert_eq!(
        provider.number_negative_pattern(),
        2,
        "Parsing must not modify the caller's format"
    );
    assert!(std::rc::Rc::ptr_eq(
        &provider,
        &CultureInfo::current_formatting().number_format()
    ));
    CultureInfo::set_current_formatting(CultureInfo::invariant_culture());
    assert_eq!(crate::native_capture::capture_integer(Some("up123")), None);
    assert_eq!(
        crate::native_capture::capture_integer(Some("+123")),
        Some(123)
    );
    // Source TryParse(null) returns before consulting CurrentCulture.NumberFormat.
    use ferroui_base::{FerroLocator, utilities::ICultureDataProvider};
    struct Provider {
        reads: std::rc::Rc<Cell<i32>>,
        format: std::rc::Rc<NumberFormatInfo>,
    }
    impl ICultureDataProvider for Provider {
        fn get_number_format(&self, _: &str) -> Option<std::rc::Rc<NumberFormatInfo>> {
            self.reads.set(self.reads.get() + 1);
            Some(self.format.clone())
        }
    }
    let _scope = FerroLocator::enter_scope();
    let reads = std::rc::Rc::new(Cell::new(0));
    FerroLocator::current_mutable()
        .bind::<dyn ICultureDataProvider>()
        .to_constant(std::rc::Rc::new(Provider {
            reads: reads.clone(),
            format: std::rc::Rc::new(NumberFormatInfo::new()),
        }) as std::rc::Rc<dyn ICultureDataProvider>);
    CultureInfo::set_current_formatting(CultureInfo::get_culture_info("en-US"));
    let data: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/capture-numbers.json")).unwrap();
    for (input, expected) in data["provider"]["input"]["values"]
        .as_array()
        .unwrap()
        .iter()
        .zip(data["provider"]["result"].as_array().unwrap())
    {
        let before = reads.get();
        let value = crate::native_capture::capture_integer(input.as_str());
        assert_eq!(
            json!({"success":value.is_some(),"value":value.unwrap_or(0),"reads":reads.get()-before}),
            *expected,
            "{input}"
        );
    }
}
#[test]
fn capture_cultured_width_and_height_match_whole_unchanged_source_requests() {
    let data: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/capture-numbers.json")).unwrap();
    for case in data["requests"].as_array().unwrap() {
        let mut input = case["input"].clone();
        input["numberFormat"] = case["format"].clone();
        let surface = Surface::new(&input);
        let request = CaptureRequest::read(&surface).unwrap().unwrap();
        let result = ready(request.run(&surface));
        assert_eq!(
            json!({"trace":surface.trace.borrow().clone(),"current":surface.current.borrow().clone(),
            "error":result.err().map(|e|e.0),"saved":surface.saved.borrow().clone(),
            "action_failed":surface.action_failed.get(),"popup_failures":surface.popup_failures.get()}),
            case["result"],
            "{}",
            case["input"]
        );
    }
}
#[test]
fn whole_source_capture_actions_and_blend_reference_corpus() {
    let data: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/capture.json")).unwrap();
    for case in data["cases"].as_array().unwrap() {
        let i = &case["input"];
        let expected = &case["result"];
        match i["kind"].as_str().unwrap() {
            "actions" => {
                let s = Surface::new(i);
                let ribbon = if i["noRibbon"].as_bool().unwrap_or(false) {
                    None
                } else {
                    s.ribbon.get()
                };
                let result = ready(run_actions(&s, i["action"].as_str(), ribbon));
                assert_eq!(
                    json!({"trace":s.trace.borrow().clone(),"current":s.current.borrow().clone(),"error":result.err().map(|e|e.0)}),
                    *expected,
                    "{i}"
                );
            }
            "blend" => {
                let bytes = |key: &str| {
                    i[key]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|b| b.as_u64().unwrap() as u8)
                        .collect::<Vec<_>>()
                };
                let mut target = bytes("target");
                let source = bytes("source");
                let n = |k: &str| i[k].as_i64().unwrap() as i32;
                let result = blend(
                    &mut target,
                    (n("tw"), n("th")),
                    &source,
                    (n("sw"), n("sh")),
                    (n("ox"), n("oy")),
                );
                assert_eq!(
                    json!({"target":target,"error":result.err().map(|_|"IndexOutOfRangeException")}),
                    *expected,
                    "{i}"
                );
            }
            "top" => {
                let s = Surface::new(i);
                let request = CaptureRequest::read(&s).unwrap();
                let result = request.map(|r| ready(r.run(&s))).transpose();
                assert_eq!(
                    json!({"trace":s.trace.borrow().clone(),"current":s.current.borrow().clone(),
                    "error":result.err().map(|e|e.0),"saved":s.saved.borrow().clone(),
                    "action_failed":s.action_failed.get(),"popup_failures":s.popup_failures.get()}),
                    *expected,
                    "{i}"
                );
            }
            other => panic!("Unknown source case {other}"),
        }
    }
}
#[test]
fn capture_delay_sequence_and_path_read_are_source_ordered() {
    let s = Surface::new(
        &json!({"env":{"RIBBONSPACE_CAPTURE":"out","RIBBONSPACE_WIDTH":" +1200 ","RIBBONSPACE_THEME":"Dark","RIBBONSPACE_PAGE":"cad","RIBBONSPACE_ACTION":"cad-appmenu,page:word,keytips-home"}}),
    );
    let request = CaptureRequest::read(&s).unwrap().unwrap();
    ready(request.run(&s)).unwrap();
    assert_eq!(
        *s.waits.borrow(),
        [600, 400, 1400, 500, 500, 800, 500, 300, 200, 500, 900]
    );
    assert_eq!(s.trace.borrow().last().unwrap(), "save");
    let none = Surface::new(&json!({}));
    assert!(CaptureRequest::read(&none).unwrap().is_none());
    assert!(none.waits.borrow().is_empty());
}
