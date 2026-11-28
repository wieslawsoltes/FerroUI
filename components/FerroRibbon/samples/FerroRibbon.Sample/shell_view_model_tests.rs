//! Differential application-state checks against the complete unchanged C# shell.
use super::*;
use serde_json::{Value, json};
use std::rc::Weak;
#[derive(Clone)]
struct Page {
    key: String,
    id: usize,
    visible: Rc<Cell<bool>>,
    theme: Rc<Cell<i32>>,
}
impl Page {
    fn name(&self) -> String {
        format!("{}#{}", self.key, self.id)
    }
    fn has_ribbon(&self) -> bool {
        self.key != "settings" && self.key != "tools"
    }
}
#[derive(Clone)]
struct Host(Rc<State>);
struct State {
    model: RefCell<Weak<ShellViewModel<Host>>>,
    host: RefCell<Vec<Page>>,
    next: Cell<usize>,
    rail: RefCell<[bool; 6]>,
    rail_theme: Cell<i32>,
    style: Cell<RibbonThemeStyle>,
    settings_style: Cell<RibbonThemeStyle>,
    chrome: Cell<RibbonChromeStyle>,
    palette: RefCell<Rc<RibbonThemePalette>>,
    override_palette: RefCell<Option<Rc<RibbonThemePalette>>>,
    trace: RefCell<Vec<String>>,
    hook: RefCell<Option<(String, bool)>>,
    version: Cell<usize>,
    mutation: Cell<Option<bool>>,
}
const KEYS: [&str; 6] = ["word", "excel", "powerpoint", "tools", "cad", "settings"];
impl Host {
    fn new() -> (Self, Rc<ShellViewModel<Self>>) {
        let h = Self(Rc::new(State {
            model: RefCell::new(Weak::new()),
            host: RefCell::new(Vec::new()),
            next: Cell::new(0),
            rail: RefCell::new([true, false, false, false, false, false]),
            rail_theme: Cell::new(0),
            style: Cell::new(RibbonThemeStyle::Office),
            settings_style: Cell::new(RibbonThemeStyle::Office),
            chrome: Cell::new(RibbonChromeStyle::Neutral),
            palette: RefCell::new(RibbonThemePalette::word()),
            override_palette: RefCell::new(None),
            trace: RefCell::new(Vec::new()),
            hook: RefCell::new(None),
            version: Cell::new(0),
            mutation: Cell::new(None),
        }));
        let vm = ShellViewModel::new(h.clone());
        h.0.model.replace(Rc::downgrade(&vm));
        (h, vm)
    }
    fn model(&self) -> Rc<ShellViewModel<Self>> {
        self.0.model.borrow().upgrade().unwrap()
    }
    fn log(&self, line: String) {
        self.0.trace.borrow_mut().push(line.clone());
        if line == "visible:word#1:True"
            && let Some(restore) = self.0.mutation.take()
        {
            let extra = self.create_page("excel").unwrap();
            self.add_page(&extra);
            if restore {
                self.remove_page(&extra);
            }
        }
        let reenter = {
            let mut hook = self.0.hook.borrow_mut();
            if let Some((point, entered)) = hook.as_mut() {
                if !*entered && *point == line {
                    *entered = true;
                    Some(if point.starts_with("visible") {
                        "excel"
                    } else {
                        "tools"
                    })
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(key) = reenter {
            self.model().show(key).unwrap();
        }
    }
    fn set_style(&self, style: i32) {
        let style = RibbonThemeStyle::from_raw(style);
        if self.0.settings_style.get() == style {
            return;
        }
        self.0.settings_style.set(style);
        self.0.style.set(style);
        self.log(format!("style:{}", style.raw()));
        self.model().invalidate_office_pages();
    }
    fn snapshot(&self) -> Value {
        let vm = self.model();
        let content = match vm.current_content() {
            Ok(p) => p.map(|p| p.name()),
            Err(ShellError::NullKey) => Some("error:NullKey".into()),
            Err(e) => panic!("{e}"),
        };
        json!({"current":vm.current_page(),"content":content,"cache":vm.pages.borrow().slots.iter().flatten().map(|(_,p)|p.name()).collect::<Vec<_>>(),"host":self.0.host.borrow().iter().map(|p|json!({"page":p.name(),"visible":p.visible.get()})).collect::<Vec<_>>(),"rail":*self.0.rail.borrow(),"railTheme":self.0.rail_theme.get(),"style":self.0.style.get().raw(),"chrome":self.0.chrome.get().raw(),"palette":self.0.palette.borrow().name.as_ref().unwrap().to_utf8().unwrap(),"savedChrome":vm.office_chrome.get().raw(),"trace":self.0.trace.borrow().clone()})
    }
}
impl ShellSurface for Host {
    type Page = Page;
    fn surface_style(&self) -> RibbonThemeStyle {
        self.0.settings_style.get()
    }
    fn theme_style(&self) -> RibbonThemeStyle {
        self.0.style.get()
    }
    fn chrome(&self) -> RibbonChromeStyle {
        self.0.chrome.get()
    }
    fn palette(&self) -> Rc<RibbonThemePalette> {
        self.0.palette.borrow().clone()
    }
    fn palette_override(&self) -> Option<Rc<RibbonThemePalette>> {
        self.0.override_palette.borrow().clone()
    }
    fn apply_theme(
        &self,
        p: Rc<RibbonThemePalette>,
        c: RibbonChromeStyle,
        s: RibbonThemeStyle,
    ) -> Result<(), ShellError> {
        self.0.palette.replace(p.clone());
        self.0.chrome.set(c);
        self.0.style.set(s);
        self.log(format!(
            "apply:{}:{}:{}",
            p.name.as_ref().unwrap().to_utf8().unwrap(),
            c.raw(),
            s.raw()
        ));
        Ok(())
    }
    fn apply_palette(&self, p: Rc<RibbonThemePalette>) -> Result<(), ShellError> {
        self.0.palette.replace(p.clone());
        self.log(format!(
            "palette:{}",
            p.name.as_ref().unwrap().to_utf8().unwrap()
        ));
        Ok(())
    }
    fn create_page(&self, key: &str) -> Result<Page, ShellError> {
        if !KEYS.contains(&key) {
            return Err(ShellError::UnknownPage(key.into()));
        }
        let id = self.0.next.get() + 1;
        self.0.next.set(id);
        let p = Page {
            key: key.into(),
            id,
            visible: Rc::new(Cell::new(true)),
            theme: Rc::new(Cell::new(if key == "cad" { 2 } else { 1 })),
        };
        self.log(format!("create:{}", p.name()));
        Ok(p)
    }
    fn add_page(&self, p: &Page) {
        self.0.host.borrow_mut().push(p.clone());
        self.0.version.set(self.0.version.get() + 1);
        self.log(format!("add:{}", p.name()));
    }
    fn remove_page(&self, p: &Page) {
        self.log(format!("remove:{}", p.name()));
        let count = self.0.host.borrow().len();
        self.0.host.borrow_mut().retain(|x| x.id != p.id);
        if self.0.host.borrow().len() != count {
            self.0.version.set(self.0.version.get() + 1);
        }
    }
    fn show_only(&self, p: &Page) -> Result<(), ShellError> {
        let version = self.0.version.get();
        let children = self.0.host.borrow().clone();
        for child in children {
            if version != self.0.version.get() {
                return Err(ShellError::ChildrenChanged);
            }
            child.visible.set(child.id == p.id);
            self.log(format!(
                "visible:{}:{}",
                child.name(),
                if child.visible.get() { "True" } else { "False" }
            ));
        }
        if version != self.0.version.get() {
            return Err(ShellError::ChildrenChanged);
        }
        Ok(())
    }
    fn check_rail(&self, key: Option<&str>) -> Result<(), ShellError> {
        for (i, k) in KEYS.iter().enumerate() {
            let value = Some(*k) == key;
            let old = self.0.rail.borrow()[i];
            if old != value {
                self.0.rail.borrow_mut()[i] = value;
                self.log(format!(
                    "checked:{k}:{}",
                    if value { "True" } else { "False" }
                ));
                if value {
                    self.model().show(k)?;
                }
            }
        }
        Ok(())
    }
    fn suspend(&self, p: &Page) {
        if p.has_ribbon() {
            self.log(format!("suspend:{}", p.name()));
        }
    }
    fn resume(&self, p: &Page) {
        if p.has_ribbon() {
            self.log(format!("resume:{}", p.name()));
        }
    }
    fn is_cad(&self, p: &Page) -> bool {
        p.key == "cad"
    }
    fn follow_cad_theme(&self, p: &Page) {
        self.0.rail_theme.set(p.theme.get());
        self.log(format!("theme-:{}", p.name()));
        self.log(format!("theme+:{}", p.name()));
    }
    fn reset_rail_theme(&self) {
        self.0.rail_theme.set(0);
    }
}
fn error(r: Result<(), ShellError>) -> Option<&'static str> {
    match r {
        Ok(()) => None,
        Err(ShellError::NullKey) => Some("NullKey"),
        Err(ShellError::UnknownPage(_)) => Some("UnknownPage"),
        Err(ShellError::ChildrenChanged) => Some("ChildrenChanged"),
        Err(e) => panic!("{e}"),
    }
}
#[test]
fn complete_unchanged_shell_navigation_cache_style_palette_and_reentry_corpus() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/shell.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        let (h, vm) = Host::new();
        let a = &case["args"];
        let mut actual_error = None;
        match case["kind"].as_str().unwrap() {
            "initial" => {}
            "sequence" => {
                for key in a["keys"].as_array().unwrap() {
                    actual_error = error(vm.show_optional(key.as_str()));
                }
            }
            "invalidate" => {
                let key = a["current"].as_str().unwrap();
                for k in [
                    "word",
                    "excel",
                    "powerpoint",
                    "tools",
                    "cad",
                    "settings",
                    key,
                ] {
                    vm.show(k).unwrap();
                }
                h.0.trace.borrow_mut().clear();
                h.set_style(a["style"].as_i64().unwrap() as i32);
            }
            "override" => {
                h.0.chrome.set(RibbonChromeStyle::from_raw(
                    a["chrome"].as_i64().unwrap() as i32
                ));
                h.0.override_palette
                    .replace(Some(RibbonThemePalette::one_note()));
                vm.show(a["key"].as_str().unwrap()).unwrap();
            }
            "reuseSlots" => {
                for key in a["keys"].as_array().unwrap() {
                    vm.show(key.as_str().unwrap()).unwrap();
                }
                h.set_style(1);
                vm.show("word").unwrap();
                vm.show("excel").unwrap();
                h.0.trace.borrow_mut().clear();
                h.set_style(0);
            }
            "cadTheme" => {
                vm.show("cad").unwrap();
                let cad = vm.current_content().unwrap().unwrap();
                vm.show(a["current"].as_str().unwrap()).unwrap();
                h.0.trace.borrow_mut().clear();
                cad.theme.set(a["theme"].as_i64().unwrap() as i32);
                if vm.current_page().as_deref() == Some("cad") {
                    h.0.rail_theme.set(cad.theme.get());
                }
            }
            "childrenMutation" => {
                vm.show("word").unwrap();
                vm.current_content().unwrap().unwrap().visible.set(false);
                h.0.trace.borrow_mut().clear();
                h.0.mutation.set(Some(a["restore"].as_bool().unwrap()));
                actual_error = error(vm.show("word"));
            }
            "reentry" => {
                vm.show("word").unwrap();
                vm.show("excel").unwrap();
                vm.show("word").unwrap();
                h.0.trace.borrow_mut().clear();
                let point = a["point"].as_str().unwrap();
                h.0.hook.replace(Some((point.into(), false)));
                vm.show(if point.starts_with("visible") {
                    "word"
                } else if point.starts_with("apply") {
                    "cad"
                } else {
                    "excel"
                })
                .unwrap();
                h.0.hook.replace(None);
            }
            other => panic!("Unknown source case {other}"),
        }
        assert_eq!(actual_error, case["error"].as_str(), "case {case}");
        assert_eq!(
            h.snapshot(),
            case["state"],
            "case {} args {}",
            case["kind"],
            a
        );
    }
}
