//! Opt-in real Android selection operations; compiled consumer controls, no sample dependency.
use ferroboard_controls::{
    KeyboardVisibilityMode, OnScreenKeyboard, ScreenKeyboardHost, input::TextBoxInputTarget,
    primitives::TouchButton,
};
use ferroui_android::log::{self, LogPriority};
use ferroui_base::{
    Ref, Thickness, WeakRef,
    input::{InputElement, text_input::TextInputMethodClientRequestedEventArgs},
};
use ferroui_controls::{
    Control, Grid, GridLength, RowDefinition, StackPanel, TextBlock, TextBox,
    automation::AutomationProperties,
};
use std::{cell::Cell, rc::Rc};
type ProbeResult<T> = Result<T, Box<dyn std::error::Error>>;

struct State {
    keyboard: WeakRef<OnScreenKeyboard>,
    field: WeakRef<TextBox>,
    other: WeakRef<TextBox>,
    observer: Rc<TextBoxInputTarget>,
    armed: Cell<bool>,
    replace: Cell<bool>,
    first: Cell<Option<(i32, i32)>>,
    completed: Cell<u8>,
}
impl Drop for State {
    fn drop(&mut self) {
        self.observer.detach();
    }
}
fn require(ok: bool, message: &'static str) -> ProbeResult<()> {
    if ok { Ok(()) } else { Err(message.into()) }
}
fn client_view(field: &TextBox) -> Option<bool> {
    let request = TextInputMethodClientRequestedEventArgs::new();
    request.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    field.raise_event(&request);
    request
        .client()
        .map(|client| client.text_view_visual().is_some())
}
impl State {
    fn run(&self, case: &str) -> ProbeResult<String> {
        let keyboard = self.keyboard.upgrade().ok_or("keyboard released")?;
        let field = self.field.upgrade().ok_or("message released")?;
        let other = self.other.upgrade().ok_or("other field released")?;
        self.armed.set(false);
        self.first.set(None);
        self.replace.set(false);
        field.set_current_value(TextBox::text_property(), Some("a😀bcdef".to_owned()));
        field.set_caret_index(3);
        require(field.focus(), "message focus rejected")?;
        require(field.is_focused(), "message did not gain focus")?;
        require(
            keyboard.target_element() == Some(field.clone().upcast()),
            "message target mismatch",
        )?;
        require(
            client_view(&field) == Some(true),
            "active native IME client has no presenter",
        )?;
        let primary = keyboard.engine().target().ok_or("missing native target")?;
        let mask = match case {
            "Range" => {
                self.armed.set(true);
                primary.select(1, 5);
                require(
                    self.first.get() == Some((1, 5)),
                    "partial range notification",
                )?;
                require(
                    primary.selection_start() == 1
                        && primary.selection_length() == 5
                        && field.caret_index() == 6,
                    "range/caret mismatch",
                )?;
                1
            }
            "Reentry" => {
                self.replace.set(true);
                self.armed.set(true);
                primary.select(1, 5);
                require(self.first.get() == Some((1, 5)), "partial reentrant range")?;
                require(
                    field.text().as_deref() == Some("a!ef"),
                    "wrong reentrant replacement",
                )?;
                require(
                    field.caret_index() == 2,
                    "wrong reentrant replacement caret",
                )?;
                2
            }
            "Collapse" => {
                primary.select(1, 5);
                primary.select(6, 0);
                require(
                    primary.selection_start() == 6
                        && primary.selection_length() == 0
                        && field.caret_index() == 6,
                    "same-caret collapse retained selection",
                )?;
                4
            }
            "Inactive" => {
                let writer = TextBoxInputTarget::new(field.clone(), None)?;
                require(other.focus(), "other focus rejected")?;
                require(
                    other.is_focused() && !field.is_focused(),
                    "inactive field still focused",
                )?;
                require(
                    client_view(&field) == Some(false),
                    "old IME client still active",
                )?;
                self.first.set(None);
                self.replace.set(true);
                self.armed.set(true);
                writer.select(1, 5);
                writer.detach();
                require(
                    self.first.get() == Some((1, 5)),
                    "inactive notification missing or partial",
                )?;
                require(
                    field.text().as_deref() == Some("a!ef"),
                    "wrong inactive replacement",
                )?;
                require(field.caret_index() == 2, "wrong inactive replacement caret")?;
                require(
                    keyboard.target_element() == Some(other.clone().upcast()) && other.is_focused(),
                    "inactive replacement stole target/focus",
                )?;
                require(
                    other.text().as_deref() == Some("other"),
                    "other field was edited",
                )?;
                8
            }
            _ => return Err("unknown selection case".into()),
        };
        if case != "Inactive" {
            require(
                field.is_focused() && keyboard.target_element() == Some(field.clone().upcast()),
                "selection operation stole target/focus",
            )?;
        }
        self.completed.set(self.completed.get() | mask);
        Ok(format!(
            "PASS {case} mask={} first={:?}",
            self.completed.get(),
            self.first.get()
        ))
    }
}
pub fn view() -> Ref<Control> {
    let value = ferroboard_native_consumer::compiled_xaml::try_load(
        None,
        &format!(
            "{}Hosted.xaml",
            ferroboard_native_consumer::compiled_xaml::ROOT_URI
        ),
    )
    .unwrap()
    .expect("dependency compiled host");
    let host = value
        .downcast_ref::<Ref<ScreenKeyboardHost>>()
        .unwrap()
        .clone();
    host.set_visibility_mode(KeyboardVisibilityMode::Auto);
    let field = host.content().unwrap().cast::<TextBox>().unwrap();
    host.set_content(None);
    field.set_height(70.0);
    AutomationProperties::set_name(&field, Some("Selection message"));
    AutomationProperties::set_automation_id(&field, Some("SelectionMessage"));
    let other = TextBox::new();
    other.set_text(Some("other"));
    other.set_height(60.0);
    AutomationProperties::set_name(&other, Some("Selection other"));
    AutomationProperties::set_automation_id(&other, Some("SelectionOther"));
    let fields = StackPanel::new();
    fields.set_spacing(6.0);
    fields.children().add(field.clone());
    fields.children().add(other.clone());
    host.set_content(Some(fields.upcast()));
    let keyboard = host.keyboard();
    let settings = keyboard.settings();
    settings.set_auto_capitalization(false);
    settings.set_incognito_mode(true);
    settings.set_learn_words(false);
    settings.set_show_suggestions(false);
    settings.set_glide_typing(false);
    settings.set_clipboard_history_enabled(false);
    settings.set_clipboard_suggestion(false);
    settings.set_emoji_history_enabled(false);
    let observer = TextBoxInputTarget::new(field.clone(), None).expect("native observer");
    let state = Rc::new(State {
        keyboard: keyboard.downgrade(),
        field: field.downgrade(),
        other: other.downgrade(),
        observer,
        armed: Cell::new(false),
        replace: Cell::new(false),
        first: Cell::new(None),
        completed: Cell::new(0),
    });
    let weak = Rc::downgrade(&state);
    state.observer.changed().add(Rc::new(move |target| {
        if let Some(state) = weak.upgrade()
            && state.armed.replace(false)
        {
            state
                .first
                .set(Some((target.selection_start(), target.selection_length())));
            if state.replace.get()
                && let Some(field) = state.field.upgrade()
            {
                field.set_selected_text(Some("!"));
            }
        }
    }));
    let root = Grid::new();
    root.set_margin(Thickness::new(12.0, 32.0, 12.0, 0.0));
    for height in [GridLength::AUTO, GridLength::AUTO, GridLength::STAR] {
        root.row_definitions()
            .add(RowDefinition::with_height(height));
    }
    let toolbar = StackPanel::new();
    toolbar.set_orientation(ferroui_base::layout::Orientation::Horizontal);
    let status = TextBlock::new();
    status.set_text(Some("READY selection"));
    AutomationProperties::set_automation_id(&status, Some("SelectionStatus"));
    for case in ["Range", "Reentry", "Collapse", "Inactive"] {
        let button = TouchButton::new();
        button.set_width(82.0);
        button.set_height(44.0);
        AutomationProperties::set_name(&button, Some(case));
        let state = state.clone();
        let status = status.downgrade();
        button.click(move |_| {
            let result = state
                .run(case)
                .unwrap_or_else(|error| format!("FAIL {case}: {error}"));
            log::write(LogPriority::Info, "FerroBoardSelection", &result);
            if let Some(status) = status.upgrade() {
                status.set_text(Some(&result));
            }
        });
        toolbar.children().add(button);
    }
    Grid::set_row(&status, 1);
    Grid::set_row(&host, 2);
    root.children().add(toolbar);
    root.children().add(status);
    root.children().add(host.clone());
    let weak = field.downgrade();
    host.loaded(move |_, _| {
        if let Some(field) = weak.upgrade() {
            field.focus();
        }
    });
    root.upcast()
}
