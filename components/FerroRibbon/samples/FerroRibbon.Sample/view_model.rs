//! Application presentation independent of native controls and layout.
use ferroui_reactiveui::{
    ICommand, IReactiveObject, IReactiveObjectExt, ReactiveCommand, ReactiveObject, reactive_object,
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
pub trait DocumentActions {
    fn execute(&self, action: &str) -> String;
    fn find(&self, document: &str, query: &str) -> usize;
}
pub trait RibbonSession {
    fn execute(&self, action: &str) -> Result<String, String>;
}
pub struct InMemoryDocument;
impl DocumentActions for InMemoryDocument {
    fn find(&self, document: &str, query: &str) -> usize {
        if query.is_empty() {
            0
        } else {
            document
                .to_lowercase()
                .matches(&query.to_lowercase())
                .count()
        }
    }
    fn execute(&self, action: &str) -> String {
        format!("{action} applied to the sample document.")
    }
}
pub struct SampleViewModel {
    reactive: ReactiveObject,
    service: Rc<dyn DocumentActions>,
    session: RefCell<Option<Rc<dyn RibbonSession>>>,
    active_tool: RefCell<String>,
    status: RefCell<String>,
    document: RefCell<String>,
    command: OnceCell<Rc<dyn ICommand>>,
    find_command: OnceCell<Rc<dyn ICommand>>,
    font_command: OnceCell<Rc<dyn ICommand>>,
    font_family: RefCell<String>,
    font_size: RefCell<f64>,
    font_size_command: OnceCell<Rc<dyn ICommand>>,
    font_color: RefCell<Option<ferroribbon_core::theming::RibbonColor>>,
    font_color_command: OnceCell<Rc<dyn ICommand>>,
    opacity_percent: RefCell<f64>,
    opacity_command: OnceCell<Rc<dyn ICommand>>,
    zoom_percent: RefCell<f64>,
    zoom_command: OnceCell<Rc<dyn ICommand>>,
    table_command: OnceCell<Rc<dyn ICommand>>,
    search_text: RefCell<String>,
    bold: RefCell<Option<bool>>,
    show_guides: RefCell<Option<bool>>,
    simplified: RefCell<Option<bool>>,
    visibility_mode: RefCell<ferroribbon_core::RibbonVisibilityMode>,
}
reactive_object!(SampleViewModel);
impl PartialEq for SampleViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl SampleViewModel {
    pub fn new(service: Rc<dyn DocumentActions>) -> Rc<Self> {
        let vm=Rc::new(Self{reactive:ReactiveObject::new(),service,session:RefCell::new(None),active_tool:RefCell::new("text".into()),status:RefCell::new("Ready".into()),document:RefCell::new("FerroRibbon\n\nA native ribbon built from the ported Ribbon, tabs, groups and command controls.\n\nSelect Home, Insert or View. Resize the window to exercise the original adaptive layout. Drag a group caption away from the ribbon to float its panel. Commands update this document through ReactiveUI.".into()),command:OnceCell::new(),find_command:OnceCell::new(),font_command:OnceCell::new(),font_family:RefCell::new("Arial".into()),font_size:RefCell::new(17.),font_size_command:OnceCell::new(),font_color:RefCell::new(Some(ferroribbon_core::theming::RibbonColor::from_rgb(32,37,48))),font_color_command:OnceCell::new(),opacity_percent:RefCell::new(100.),opacity_command:OnceCell::new(),zoom_percent:RefCell::new(100.),zoom_command:OnceCell::new(),table_command:OnceCell::new(),search_text:RefCell::new("ribbon".into()),bold:RefCell::new(Some(false)),show_guides:RefCell::new(Some(false)),simplified:RefCell::new(Some(false)),visibility_mode:RefCell::new(ferroribbon_core::RibbonVisibilityMode::AlwaysShow)});
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.command
                .set(Rc::new(ReactiveCommand::create_param(
                    move |action: String| {
                        if let Some(vm) = weak.upgrade() {
                            if matches!(
                                action.as_str(),
                                "Customize ribbon"
                                    | "Customize quick access"
                                    | "Save ribbon"
                                    | "Restore ribbon"
                                    | "Reset ribbon"
                                    | "Reset quick access"
                            ) {
                                let session = vm.session.borrow().clone();
                                let result = session
                                    .ok_or_else(|| {
                                        "Ribbon session has not been connected".to_string()
                                    })
                                    .and_then(|s| s.execute(&action))
                                    .unwrap_or_else(|e| format!("Ribbon state: {e}"));
                                vm.raise_and_set_if_changed(&vm.status, result, "Status");
                                return;
                            }
                            if matches!(action.as_str(), "Text tools" | "Draw tools") {
                                vm.set_active_tool(
                                    if action == "Text tools" {
                                        "text"
                                    } else {
                                        "draw"
                                    }
                                    .into(),
                                );
                                vm.raise_and_set_if_changed(&vm.status, action, "Status");
                                return;
                            }
                            let result = vm.service.execute(&action);
                            vm.raise_and_set_if_changed(&vm.status, result.clone(), "Status");
                            let document = format!("{}\n\n{}", vm.document(), result);
                            vm.raise_and_set_if_changed(&vm.document, document, "Document");
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.find_command
                .set(Rc::new(ReactiveCommand::create_param(
                    move |query: String| {
                        if let Some(vm) = weak.upgrade() {
                            let count = vm.service.find(&vm.document(), &query);
                            vm.raise_and_set_if_changed(
                                &vm.status,
                                format!("Find “{query}”: {count} matches."),
                                "Status",
                            );
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.font_command
                .set(Rc::new(ReactiveCommand::create_param(
                    move |family: String| {
                        if let Some(vm) = weak.upgrade() {
                            vm.set_font_family(family.clone());
                            vm.raise_and_set_if_changed(
                                &vm.status,
                                format!("Font: {family}"),
                                "Status",
                            );
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.font_size_command
                .set(Rc::new(ReactiveCommand::create_param(move |size: f64| {
                    if let Some(vm) = weak.upgrade() {
                        vm.set_font_size(size);
                        vm.raise_and_set_if_changed(
                            &vm.status,
                            format!("Font size: {size}"),
                            "Status",
                        );
                    }
                })))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.opacity_command
                .set(Rc::new(ReactiveCommand::create_param(move |value: f64| {
                    if let Some(vm) = weak.upgrade() {
                        vm.set_opacity_percent(value);
                        vm.raise_and_set_if_changed(
                            &vm.status,
                            format!("Opacity: {value}%"),
                            "Status",
                        );
                    }
                })))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.zoom_command
                .set(Rc::new(ReactiveCommand::create_param(move |value: f64| {
                    if let Some(vm) = weak.upgrade() {
                        vm.set_zoom_percent(value);
                        vm.raise_and_set_if_changed(
                            &vm.status,
                            format!("Zoom: {value}%"),
                            "Status",
                        );
                    }
                })))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.table_command
                .set(Rc::new(ReactiveCommand::create_param(
                    move |size: ferroribbon_core::model::RibbonGridSize| {
                        if let Some(vm) = weak.upgrade() {
                            let result = vm
                                .service
                                .execute(&format!("Insert {}x{} table", size.columns, size.rows));
                            vm.raise_and_set_if_changed(&vm.status, result.clone(), "Status");
                            let document = format!("{}\n\n{}", vm.document(), result);
                            vm.raise_and_set_if_changed(&vm.document, document, "Document");
                        }
                    }
                )))
                .is_ok()
        );
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.font_color_command
                .set(Rc::new(ReactiveCommand::create_param(
                    move |color: Option<ferroribbon_core::theming::RibbonColor>| {
                        if let Some(vm) = weak.upgrade() {
                            vm.set_font_color(color);
                            vm.raise_and_set_if_changed(
                                &vm.status,
                                format!(
                                    "Font color: {}",
                                    color.map_or_else(|| "No Color".to_owned(), |c| c.to_hex(true))
                                ),
                                "Status",
                            );
                        }
                    }
                )))
                .is_ok()
        );
        vm
    }
    pub fn font_color(&self) -> Option<ferroribbon_core::theming::RibbonColor> {
        *self.font_color.borrow()
    }
    pub fn set_font_color(&self, color: Option<ferroribbon_core::theming::RibbonColor>) {
        if self.font_color() == color {
            return;
        }
        self.raise_and_set_if_changed(&self.font_color, color, "FontColor");
        self.raise_property_changed("DocumentForeground");
    }
    pub fn font_color_command(&self) -> Rc<dyn ICommand> {
        self.font_color_command
            .get()
            .expect("composition initialized")
            .clone()
    }
    pub fn table_command(&self) -> Rc<dyn ICommand> {
        self.table_command
            .get()
            .expect("composition initialized")
            .clone()
    }
    pub fn zoom_percent(&self) -> f64 {
        *self.zoom_percent.borrow()
    }
    pub fn zoom_scale(&self) -> f64 {
        self.zoom_percent() / 100.
    }
    pub fn set_zoom_percent(&self, value: f64) {
        if !value.is_finite() {
            return;
        }
        let value = value.clamp(10., 500.);
        if self.zoom_percent() == value {
            return;
        }
        self.raise_property_changing("ZoomScale");
        self.raise_and_set_if_changed(&self.zoom_percent, value, "ZoomPercent");
        self.raise_property_changed("ZoomScale");
    }
    pub fn zoom_command(&self) -> Rc<dyn ICommand> {
        self.zoom_command
            .get()
            .expect("composition initialized")
            .clone()
    }
    pub fn opacity_percent(&self) -> f64 {
        *self.opacity_percent.borrow()
    }
    pub fn document_opacity(&self) -> f64 {
        self.opacity_percent() / 100.
    }
    pub fn set_opacity_percent(&self, value: f64) {
        if self.opacity_percent() == value {
            return;
        }
        self.raise_property_changing("DocumentOpacity");
        self.raise_and_set_if_changed(&self.opacity_percent, value, "OpacityPercent");
        self.raise_property_changed("DocumentOpacity");
    }
    pub fn opacity_command(&self) -> Rc<dyn ICommand> {
        self.opacity_command
            .get()
            .expect("composition initialized")
            .clone()
    }
    pub fn font_size(&self) -> f64 {
        *self.font_size.borrow()
    }
    pub fn set_font_size(&self, value: f64) {
        self.raise_and_set_if_changed(&self.font_size, value, "FontSize");
    }
    pub fn font_size_command(&self) -> Rc<dyn ICommand> {
        self.font_size_command
            .get()
            .expect("composition initialized")
            .clone()
    }
    pub fn font_family(&self) -> String {
        self.font_family.borrow().clone()
    }
    pub fn set_font_family(&self, value: String) {
        self.raise_and_set_if_changed(&self.font_family, value, "FontFamily");
    }
    pub fn font_command(&self) -> Rc<dyn ICommand> {
        self.font_command
            .get()
            .expect("composition initialized")
            .clone()
    }
    pub fn search_text(&self) -> String {
        self.search_text.borrow().clone()
    }
    pub fn set_search_text(&self, value: String) {
        self.raise_and_set_if_changed(&self.search_text, value, "SearchText");
    }
    pub fn find_command(&self) -> Rc<dyn ICommand> {
        self.find_command
            .get()
            .expect("composition initialized")
            .clone()
    }
    pub fn set_ribbon_session(&self, session: Rc<dyn RibbonSession>) {
        self.session.replace(Some(session));
    }
    pub fn set_display_mode(&self, value: ferroribbon_core::RibbonDisplayMode) {
        self.set_simplified(Some(
            value == ferroribbon_core::RibbonDisplayMode::Simplified,
        ));
    }
    pub fn visibility_mode(&self) -> ferroribbon_core::RibbonVisibilityMode {
        *self.visibility_mode.borrow()
    }
    pub fn set_visibility_mode(&self, value: ferroribbon_core::RibbonVisibilityMode) {
        self.raise_and_set_if_changed(&self.visibility_mode, value, "VisibilityMode");
    }
    pub fn simplified(&self) -> Option<bool> {
        *self.simplified.borrow()
    }
    pub fn set_simplified(&self, value: Option<bool>) {
        if self.simplified() == value {
            return;
        }
        self.raise_property_changing("DisplayMode");
        self.raise_and_set_if_changed(&self.simplified, value, "Simplified");
        self.raise_property_changed("DisplayMode");
    }
    pub fn display_mode(&self) -> ferroribbon_core::RibbonDisplayMode {
        if self.simplified() == Some(true) {
            ferroribbon_core::RibbonDisplayMode::Simplified
        } else {
            ferroribbon_core::RibbonDisplayMode::Classic
        }
    }
    pub fn bold(&self) -> Option<bool> {
        *self.bold.borrow()
    }
    pub fn set_bold(&self, value: Option<bool>) {
        self.raise_and_set_if_changed(&self.bold, value, "Bold");
    }
    pub fn show_guides(&self) -> Option<bool> {
        *self.show_guides.borrow()
    }
    pub fn set_show_guides(&self, value: Option<bool>) {
        self.raise_and_set_if_changed(&self.show_guides, value, "ShowGuides");
    }
    pub fn command(&self) -> Rc<dyn ICommand> {
        self.command.get().expect("composition initialized").clone()
    }
    pub fn active_tool(&self) -> String {
        self.active_tool.borrow().clone()
    }
    pub fn set_active_tool(&self, value: String) {
        self.raise_and_set_if_changed(&self.active_tool, value, "ActiveTool");
    }
    pub fn status(&self) -> String {
        self.status.borrow().clone()
    }
    pub fn document(&self) -> String {
        self.document.borrow().clone()
    }
    pub fn set_document(&self, value: String) {
        self.raise_and_set_if_changed(&self.document, value, "Document");
    }
}
