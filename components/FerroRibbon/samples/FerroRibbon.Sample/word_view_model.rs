// Copyright (c) 2026 Wiesław Šoltés. MIT. Presentation behavior from WordPage.xaml.cs.
//! Formatting state and actions are independent of native controls, brushes and windows.
use ferroui_reactiveui::{
    ICommand, IReactiveObject, IReactiveObjectExt, ReactiveCommand, ReactiveObject, reactive_object,
};
use ferroui_system::invariant_numbers::{NumberStyles, try_parse_double, write_double};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordAlignment {
    Left,
    Center,
    Right,
    Justify,
}
pub type WordParameterText = Rc<dyn Fn() -> Result<String, String>>;
#[derive(Clone)]
pub enum WordAction {
    Invoked {
        item_id: Option<String>,
        command_id: Option<String>,
        parameter: Option<WordParameterText>,
    },
    TabChanged(Option<String>),
    FontFamilyCommitted(Option<String>),
    FontSizeCommitted(Option<String>),
    FontColor(Option<u32>),
    PageColor(Option<u32>),
    StylePreview(Option<String>),
    StyleClick(Option<String>),
    TablePicked {
        rows: i32,
        columns: i32,
    },
    ToggleTable,
    ToggleDark,
    Zoom(f64),
    Save,
    BackstageSave,
}
pub trait WordServices {
    fn write(&self, message: &str) -> Result<(), String>;
    fn hide_table_flyout(&self);
    fn set_table_context(&self, visible: bool);
    fn toggle_dark_theme(&self);
}
pub struct WordViewModel {
    reactive: ReactiveObject,
    services: Rc<dyn WordServices>,
    action: OnceCell<Rc<dyn ICommand>>,
    buttons: OnceCell<[Rc<dyn ICommand>; 3]>,
    selected_style: RefCell<String>,
    toggles: Cell<[Option<bool>; 4]>,
    body_font_size: RefCell<f64>,
    body_font_family: RefCell<String>,
    body_bold: RefCell<bool>,
    body_italic: RefCell<bool>,
    body_underline: RefCell<bool>,
    body_strike: RefCell<bool>,
    body_foreground: Cell<u32>,
    body_alignment: RefCell<WordAlignment>,
    title_font_size: RefCell<f64>,
    title_foreground: Cell<u32>,
    title_italic: RefCell<bool>,
    title_bold: RefCell<bool>,
    font_size_text: RefCell<Option<String>>,
    page_background: Cell<u32>,
    table_visible: RefCell<bool>,
    zoom_scale: Cell<Option<f64>>,
    last_command: RefCell<String>,
}
reactive_object!(WordViewModel);
impl PartialEq for WordViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl WordViewModel {
    pub fn new(services: Rc<dyn WordServices>) -> Rc<Self> {
        let vm = Rc::new(Self {
            reactive: ReactiveObject::new(),
            services,
            action: OnceCell::new(),
            buttons: OnceCell::new(),
            selected_style: RefCell::new("Normal".into()),
            toggles: Cell::new([Some(false); 4]),
            body_font_size: RefCell::new(16.),
            body_font_family: RefCell::new("Segoe UI".into()),
            body_bold: RefCell::new(false),
            body_italic: RefCell::new(false),
            body_underline: RefCell::new(false),
            body_strike: RefCell::new(false),
            body_foreground: Cell::new(0xff242424),
            body_alignment: RefCell::new(WordAlignment::Left),
            title_font_size: RefCell::new(30.),
            title_foreground: Cell::new(0xff1f3864),
            title_italic: RefCell::new(false),
            title_bold: RefCell::new(false),
            font_size_text: RefCell::new(Some("16".into())),
            page_background: Cell::new(0xffffffff),
            table_visible: RefCell::new(false),
            zoom_scale: Cell::new(None),
            last_command: RefCell::new("Last command: —".into()),
        });
        let weak = Rc::downgrade(&vm);
        assert!(
            vm.action
                .set(Rc::new(ReactiveCommand::create_param(
                    move |action: Option<WordAction>| {
                        let action = action.expect("Word action requires a parameter");
                        if let Some(vm) = weak.upgrade() {
                            vm.apply(action).expect("Word action failed");
                        }
                    }
                )))
                .is_ok()
        );
        let buttons = [
            WordAction::ToggleTable,
            WordAction::ToggleDark,
            WordAction::BackstageSave,
        ]
        .map(|action| {
            let weak = Rc::downgrade(&vm);
            Rc::new(ReactiveCommand::create(move || {
                if let Some(vm) = weak.upgrade() {
                    vm.apply(action.clone()).expect("Word action failed");
                }
            })) as Rc<dyn ICommand>
        });
        assert!(vm.buttons.set(buttons).is_ok());
        vm
    }
    pub fn button_action(&self, index: usize) -> Rc<dyn ICommand> {
        self.buttons.get().unwrap()[index].clone()
    }
    pub fn action(&self) -> Rc<dyn ICommand> {
        self.action.get().unwrap().clone()
    }
    pub fn selected_style(&self) -> String {
        self.selected_style.borrow().clone()
    }
    pub fn checked(&self, index: usize) -> Option<bool> {
        self.toggles.get()[index]
    }
    pub fn set_checked(&self, index: usize, value: Option<bool>) {
        let mut all = self.toggles.get();
        if all[index] != value {
            let name = ["Bold", "Italic", "Underline", "Strike"][index];
            self.raise_property_changing(name);
            all[index] = value;
            self.toggles.set(all);
            self.raise_property_changed(name);
        }
    }
    pub fn apply(&self, action: WordAction) -> Result<(), String> {
        match action {
            WordAction::Invoked {
                item_id,
                command_id,
                parameter,
            } => {
                let id = item_id.or(command_id).unwrap_or_else(|| "(no id)".into());
                match id.as_str() {
                    "bold" | "italic" | "underline" | "strikethrough" => self.update_font(),
                    "alignLeft" => self.set_body_alignment(WordAlignment::Left),
                    "alignCenter" => self.set_body_alignment(WordAlignment::Center),
                    "alignRight" => self.set_body_alignment(WordAlignment::Right),
                    "justify" => self.set_body_alignment(WordAlignment::Justify),
                    "growFont" | "shrinkFont" => {
                        let size = self.body_font_size();
                        let size = if size.is_nan() {
                            size
                        } else if id == "growFont" {
                            (size + 2.).min(96.)
                        } else {
                            (size - 2.).max(8.)
                        };
                        self.set_body_font_size(size);
                        let mut text = String::new();
                        write_double(&mut text, size).expect("writing to String");
                        self.set_font_size_text(Some(text));
                    }
                    "clearFormatting" => {
                        for index in (0..4).rev() {
                            self.set_checked(index, Some(false));
                        }
                        self.set_body_font_size(16.);
                        self.set_body_foreground(0xff000000);
                        self.update_font();
                    }
                    _ => {}
                }
                self.log(&match parameter {
                    Some(p) => format!("{id} ({})", p()?),
                    None => id,
                })?;
            }
            WordAction::TabChanged(header) => {
                self.log(&format!("Tab: {}", header.unwrap_or_default()))?
            }
            WordAction::FontFamilyCommitted(text) => {
                self.set_body_font_family(text.unwrap_or_else(|| "Segoe UI".into()))
            }
            WordAction::FontSizeCommitted(text) => {
                if let Some(size) = text
                    .as_deref()
                    .and_then(|s| try_parse_double(s, NumberStyles::FLOAT))
                    && size > 1.
                    && size < 400.
                {
                    self.set_body_font_size(size);
                }
            }
            WordAction::FontColor(color) => self.set_body_foreground(color.unwrap_or(0xff000000)),
            WordAction::PageColor(color) => self.set_page_background(color.unwrap_or(0xffffffff)),
            WordAction::StylePreview(style) => {
                self.apply_style(style.as_deref().unwrap_or(&self.selected_style()))
            }
            WordAction::StyleClick(style) => {
                let style = style.unwrap_or_else(|| "Normal".into());
                self.selected_style.replace(style.clone());
                self.apply_style(&style);
                self.log(&format!("Style: {}", self.selected_style()))?;
            }
            WordAction::TablePicked { rows, columns } => {
                self.services.hide_table_flyout();
                self.set_table_visible(true);
                self.services.set_table_context(true);
                self.log(&format!("Inserted {columns}×{rows} table"))?;
            }
            WordAction::ToggleTable => {
                let show = !self.table_visible();
                self.set_table_visible(show);
                self.services.set_table_context(show);
            }
            WordAction::ToggleDark => self.services.toggle_dark_theme(),
            WordAction::Zoom(value) => {
                self.zoom_scale.set(Some(value / 100.));
                self.raise_property_changed("ZoomTransform");
            }
            WordAction::Save => self.log("Document saved")?,
            WordAction::BackstageSave => self.log("Saved from backstage")?,
        }
        Ok(())
    }
    fn update_font(&self) {
        self.set_body_bold(self.checked(0) == Some(true));
        self.set_body_italic(self.checked(1) == Some(true));
        let underline = self.checked(2) == Some(true);
        let strike = self.checked(3) == Some(true);
        if self.body_underline() != underline || self.body_strike() != strike {
            self.raise_property_changing("BodyDecorations");
            self.body_underline.replace(underline);
            self.body_strike.replace(strike);
            self.raise_property_changed("BodyDecorations");
        }
    }
    fn apply_style(&self, style: &str) {
        let (size, color) = match style {
            "Title" => (40., 0xff000000),
            "Heading1" => (30., 0xff2f5496),
            "Heading2" => (24., 0xff2f5496),
            "Subtitle" => (22., 0xff595959),
            "Quote" | "IntenseQuote" => (26., 0xff404040),
            _ => (30., 0xff1f3864),
        };
        self.set_title_font_size(size);
        self.set_title_foreground(color);
        self.set_title_italic(matches!(style, "Emphasis" | "Quote" | "IntenseQuote"));
        self.set_title_bold(style == "Strong");
    }
    fn log(&self, message: &str) -> Result<(), String> {
        self.set_last_command(format!("Last command: {message}"));
        self.services.write(&format!("Word · {message}"))
    }
    pub fn body_foreground(&self) -> u32 {
        self.body_foreground.get()
    }
    pub fn set_body_foreground(&self, value: u32) {
        self.body_foreground.set(value);
        self.raise_property_changed("BodyForeground");
    }
    pub fn title_foreground(&self) -> u32 {
        self.title_foreground.get()
    }
    pub fn set_title_foreground(&self, value: u32) {
        self.title_foreground.set(value);
        self.raise_property_changed("TitleForeground");
    }
    pub fn page_background(&self) -> u32 {
        self.page_background.get()
    }
    pub fn set_page_background(&self, value: u32) {
        self.page_background.set(value);
        self.raise_property_changed("PageBackground");
    }
    pub fn zoom_scale(&self) -> Option<f64> {
        self.zoom_scale.get()
    }
    pub fn body_font_size(&self) -> f64 {
        *self.body_font_size.borrow()
    }
    pub fn set_body_font_size(&self, value: f64) {
        self.raise_and_set_if_changed(&self.body_font_size, value, "BodyFontSize");
    }
    pub fn body_font_family(&self) -> String {
        self.body_font_family.borrow().clone()
    }
    pub fn set_body_font_family(&self, value: String) {
        self.raise_and_set_if_changed(&self.body_font_family, value, "BodyFontFamily");
    }
    pub fn body_bold(&self) -> bool {
        *self.body_bold.borrow()
    }
    pub fn set_body_bold(&self, value: bool) {
        self.raise_and_set_if_changed(&self.body_bold, value, "BodyFontWeight");
    }
    pub fn body_italic(&self) -> bool {
        *self.body_italic.borrow()
    }
    pub fn set_body_italic(&self, value: bool) {
        self.raise_and_set_if_changed(&self.body_italic, value, "BodyFontStyle");
    }
    pub fn body_underline(&self) -> bool {
        *self.body_underline.borrow()
    }
    pub fn set_body_underline(&self, value: bool) {
        self.raise_and_set_if_changed(&self.body_underline, value, "BodyDecorations");
    }
    pub fn body_strike(&self) -> bool {
        *self.body_strike.borrow()
    }
    pub fn set_body_strike(&self, value: bool) {
        self.raise_and_set_if_changed(&self.body_strike, value, "BodyDecorations");
    }
    pub fn body_alignment(&self) -> WordAlignment {
        *self.body_alignment.borrow()
    }
    pub fn set_body_alignment(&self, value: WordAlignment) {
        self.raise_and_set_if_changed(&self.body_alignment, value, "BodyAlignment");
    }
    pub fn title_font_size(&self) -> f64 {
        *self.title_font_size.borrow()
    }
    pub fn set_title_font_size(&self, value: f64) {
        self.raise_and_set_if_changed(&self.title_font_size, value, "TitleFontSize");
    }
    pub fn title_bold(&self) -> bool {
        *self.title_bold.borrow()
    }
    pub fn set_title_bold(&self, value: bool) {
        self.raise_and_set_if_changed(&self.title_bold, value, "TitleFontWeight");
    }
    pub fn title_italic(&self) -> bool {
        *self.title_italic.borrow()
    }
    pub fn set_title_italic(&self, value: bool) {
        self.raise_and_set_if_changed(&self.title_italic, value, "TitleFontStyle");
    }
    pub fn font_size_text(&self) -> Option<String> {
        self.font_size_text.borrow().clone()
    }
    pub fn set_font_size_text(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.font_size_text, value, "FontSizeText");
    }
    pub fn table_visible(&self) -> bool {
        *self.table_visible.borrow()
    }
    pub fn set_table_visible(&self, value: bool) {
        self.raise_and_set_if_changed(&self.table_visible, value, "TableVisible");
    }
    pub fn last_command(&self) -> String {
        self.last_command.borrow().clone()
    }
    pub fn set_last_command(&self, value: String) {
        self.raise_and_set_if_changed(&self.last_command, value, "LastCommand");
    }
}
