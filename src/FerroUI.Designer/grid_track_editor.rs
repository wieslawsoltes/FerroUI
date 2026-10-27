//! A compact native Grid axis editor. The host owns document mutations and history.
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::SolidColorBrush;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, ColumnDefinitions, ComboBox, Control, Grid, RowDefinitions, TextBlock, TextBox, ToolTip,
};
use ferroui_designer_tools::{Axis, GridLength, GridTrackChange};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ChangeHandler = Rc<dyn Fn(GridTrackChange) -> Result<(), String>>;
type SelectionHandler = Rc<dyn Fn(usize)>;

/// An axis picker, length/unit editor and insertion/removal commands in one native row.
/// Configure it from accepted state; a change callback must validate and apply the intent.
pub struct GridTrackEditor {
    axis: Axis,
    root: Ref<Grid>,
    track: Ref<ComboBox>,
    value: Ref<TextBox>,
    unit: Ref<ComboBox>,
    add: Ref<Button>,
    remove: Ref<Button>,
    error: Ref<TextBlock>,
    values: RefCell<Vec<String>>,
    writable: Cell<bool>,
    guard: Cell<bool>,
    change: RefCell<Option<ChangeHandler>>,
    selection: RefCell<Option<SelectionHandler>>,
}

impl GridTrackEditor {
    pub fn new(axis: Axis) -> Rc<Self> {
        let title = match axis {
            Axis::Vertical => "Rows",
            Axis::Horizontal => "Columns",
        };
        let root = Grid::new();
        root.set_column_definitions(ColumnDefinitions::parse("20,84,*,84,24,24").unwrap());
        root.set_row_definitions(RowDefinitions::parse("Auto,Auto").unwrap());
        root.classes().add("grid-track-editor");
        let axis_icon = crate::property_controls::property_icon(
            match axis {
                Axis::Vertical => {
                    "M1,1 L15,1 15,15 1,15 Z M2.5,2.5 L2.5,5 13.5,5 13.5,2.5 Z M2.5,6.5 L2.5,9.5 13.5,9.5 13.5,6.5 Z M2.5,11 L2.5,13.5 13.5,13.5 13.5,11 Z"
                }
                Axis::Horizontal => {
                    "M1,1 L15,1 15,15 1,15 Z M2.5,2.5 L2.5,13.5 5,13.5 5,2.5 Z M6.5,2.5 L6.5,13.5 9.5,13.5 9.5,2.5 Z M11,2.5 L11,13.5 13.5,13.5 13.5,2.5 Z"
                }
            },
            13.0,
        );
        ToolTip::set_tip(&axis_icon, Some(Rc::new(format!("Grid {title}"))));
        root.children().add(axis_icon);
        let track = ComboBox::new();
        track.classes().add("designer-property-field");
        track.set_font_size(11.0);
        track.set_min_width(0.0);
        track.set_horizontal_alignment(HorizontalAlignment::Stretch);
        track.set_padding(Thickness::symmetric(5.0, 2.0));
        track.set_margin(Thickness::new(0.0, 0.0, 3.0, 0.0));
        track.set_min_height(26.0);
        Grid::set_column(&track, 1);
        AutomationProperties::set_name(&track, Some(&format!("{title} selected track")));
        ToolTip::set_tip(
            &track,
            Some(Rc::new(format!(
                "Select a {title} track by its zero-based XAML index"
            ))),
        );
        root.children().add(track.clone().upcast::<Control>());
        let value = TextBox::new();
        value.classes().add("designer-property-field");
        value.set_font_size(11.0);
        value.set_min_width(0.0);
        value.set_min_height(26.0);
        value.set_padding(Thickness::symmetric(5.0, 3.0));
        value.set_margin(Thickness::new(0.0, 0.0, 3.0, 0.0));
        Grid::set_column(&value, 2);
        AutomationProperties::set_name(&value, Some(&format!("{title} track length")));
        ToolTip::set_tip(
            &value,
            Some(Rc::new(
                "Enter to apply; Escape to restore the accepted length".to_owned(),
            )),
        );
        root.children().add(value.clone().upcast::<Control>());
        let unit = ComboBox::new();
        unit.classes().add("designer-property-field");
        unit.set_font_size(11.0);
        unit.set_min_width(0.0);
        unit.set_min_height(26.0);
        unit.set_padding(Thickness::symmetric(4.0, 2.0));
        unit.set_margin(Thickness::new(0.0, 0.0, 3.0, 0.0));
        unit.set_horizontal_alignment(HorizontalAlignment::Stretch);
        for text in ["*", "px", "Auto"] {
            unit.items().add(Some(Rc::new(text.to_owned())));
        }
        Grid::set_column(&unit, 3);
        AutomationProperties::set_name(&unit, Some(&format!("{title} sizing unit")));
        ToolTip::set_tip(
            &unit,
            Some(Rc::new(
                "* · proportion of available space\npx · fixed size\nAuto · fit content".to_owned(),
            )),
        );
        root.children().add(unit.clone().upcast::<Control>());
        let make_button = |path: &str, help: &str, column: i32| {
            let button = Button::new();
            let icon = crate::property_controls::property_icon(path, 12.0);
            button.set_content(Some(Control::boxed(&icon)));
            button.set_width(24.0);
            button.set_min_width(0.0);
            button.set_min_height(26.0);
            button.set_padding(Thickness::uniform(2.0));
            button.classes().add("property-track-command");
            Grid::set_column(&button, column);
            AutomationProperties::set_name(&button, Some(help));
            ToolTip::set_tip(&button, Some(Rc::new(help.to_owned())));
            root.children().add(button.clone().upcast::<Control>());
            button
        };
        let add = make_button(
            "M7,1 L9,1 9,7 15,7 15,9 9,9 9,15 7,15 7,9 1,9 1,7 7,7 Z",
            &format!("Insert {title} track after selection"),
            4,
        );
        let remove = make_button(
            "M1,7 L15,7 15,9 1,9 Z",
            &format!("Remove selected {title} track and preserve child cells"),
            5,
        );
        let error = TextBlock::new();
        error.set_font_size(10.0);
        error.set_foreground(Some(SolidColorBrush::from_uint32(0xfff29c9c).into()));
        error.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        error.set_is_visible(false);
        Grid::set_row(&error, 1);
        Grid::set_column_span(&error, 6);
        root.children().add(error.clone().upcast::<Control>());
        let this = Rc::new(Self {
            axis,
            root,
            track,
            value,
            unit,
            add,
            remove,
            error,
            values: RefCell::new(vec!["*".into()]),
            writable: Cell::new(true),
            guard: Cell::new(false),
            change: RefCell::new(None),
            selection: RefCell::new(None),
        });
        this.configure(vec!["*".into()], 0, true);
        let weak = Rc::downgrade(&this);
        this.track.selection_changed(move |_, _| {
            let Some(this) = weak.upgrade().filter(|this| !this.guard.get()) else {
                return;
            };
            this.refresh_length();
            let handler = this.selection.borrow().clone();
            if let Some(handler) = handler {
                handler(this.selected_track());
            }
        });
        let weak = Rc::downgrade(&this);
        this.value
            .add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                match event.key {
                    Key::Enter => {
                        this.commit_length();
                        event.set_handled(true);
                    }
                    Key::Escape => {
                        this.refresh_length();
                        event.set_handled(true);
                    }
                    _ => {}
                }
            });
        let weak = Rc::downgrade(&this);
        this.unit.selection_changed(move |_, _| {
            let Some(this) = weak.upgrade().filter(|this| !this.guard.get()) else {
                return;
            };
            if this.unit.selected_index() != 2 && this.value.text().as_deref() == Some("Auto") {
                this.value.set_text(Some("1"));
            }
            this.commit_length();
        });
        let weak = Rc::downgrade(&this);
        this.add.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.request(GridTrackChange::Insert {
                    index: this.selected_track() + 1,
                    length: GridLength::Star(1.0),
                });
            }
        });
        let weak = Rc::downgrade(&this);
        this.remove.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.request(GridTrackChange::Remove {
                    index: this.selected_track(),
                });
            }
        });
        this
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn axis(&self) -> Axis {
        self.axis
    }
    pub fn track_selector(&self) -> Ref<ComboBox> {
        self.track.clone()
    }
    pub fn length_editor(&self) -> Ref<TextBox> {
        self.value.clone()
    }
    pub fn unit_selector(&self) -> Ref<ComboBox> {
        self.unit.clone()
    }
    pub fn selected_track(&self) -> usize {
        self.track.selected_index().max(0) as usize
    }
    pub fn on_change(&self, handler: impl Fn(GridTrackChange) -> Result<(), String> + 'static) {
        *self.change.borrow_mut() = Some(Rc::new(handler));
    }
    pub fn on_selected(&self, handler: impl Fn(usize) + 'static) {
        *self.selection.borrow_mut() = Some(Rc::new(handler));
    }

    /// Empty definitions represent the native implicit star track. Expressions are displayed
    /// read-only; selecting or configuring the control never emits an authoring command.
    pub fn configure(&self, mut values: Vec<String>, selected: usize, writable: bool) {
        if values.is_empty() {
            values.push("*".into());
        }
        self.guard.set(true);
        self.track.items().clear();
        for index in 0..values.len() {
            self.track.items().add(Some(Rc::new(format!(
                "{} {index}",
                match self.axis {
                    Axis::Vertical => "Row",
                    Axis::Horizontal => "Col",
                }
            ))));
        }
        self.track
            .set_selected_index(selected.min(values.len() - 1) as i32);
        *self.values.borrow_mut() = values;
        self.writable.set(writable);
        self.guard.set(false);
        self.refresh_length();
    }

    fn refresh_length(&self) {
        self.guard.set(true);
        let text = self
            .values
            .borrow()
            .get(self.selected_track())
            .cloned()
            .unwrap_or_else(|| "*".into());
        let parsed = GridLength::parse(&text).ok();
        let (unit, value) = match parsed {
            Some(GridLength::Star(value)) => (0, value.to_string()),
            Some(GridLength::Pixels(value)) => (1, value.to_string()),
            Some(GridLength::Auto) => (2, "Auto".into()),
            None => (-1, text),
        };
        self.unit.set_selected_index(unit);
        self.value.set_text(Some(&value));
        self.value
            .set_is_read_only(!self.writable.get() || parsed.is_none() || unit == 2);
        self.unit
            .set_is_enabled(self.writable.get() && parsed.is_some());
        self.add.set_is_enabled(self.writable.get());
        self.remove
            .set_is_enabled(self.writable.get() && self.values.borrow().len() > 1);
        self.error.set_is_visible(false);
        self.guard.set(false);
    }

    fn commit_length(&self) {
        if self.guard.get() || !self.writable.get() {
            return;
        }
        let number = self.value.text().unwrap_or_default();
        let text = match self.unit.selected_index() {
            0 => format!("{}*", number.trim()),
            1 => number.trim().to_owned(),
            2 => "Auto".into(),
            _ => return,
        };
        match GridLength::parse(&text) {
            Ok(length) => self.request(GridTrackChange::Set {
                index: self.selected_track(),
                length,
            }),
            Err(error) => self.show_error(&error.to_string()),
        }
    }

    fn request(&self, change: GridTrackChange) {
        if !self.writable.get() {
            return;
        }
        let handler = self.change.borrow().clone();
        if let Some(handler) = handler {
            match handler(change) {
                Ok(()) => self.error.set_is_visible(false),
                Err(error) => self.show_error(&error),
            }
        }
    }

    fn show_error(&self, error: &str) {
        self.error.set_text(Some(error));
        self.error.set_is_visible(true);
    }
}

#[cfg(test)]
#[path = "grid_track_editor_tests.rs"]
mod tests;
