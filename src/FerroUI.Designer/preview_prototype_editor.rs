//! Detached native forms for named prototype starts and source-backed interactions.
use crate::{
    PreviewPrototype, PreviewPrototypeAction, PreviewPrototypeFlow, PreviewPrototypeLink,
    PreviewSourceAnchor, PreviewStageId, PreviewStageInfo,
};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Button, ComboBox, Control, Grid, RowDefinitions, ScrollViewer, StackPanel, TextBlock, TextBox,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The workspace supplies only eligible authored elements from its accepted sources.
#[derive(Clone, Debug)]
pub struct PreviewPrototypeElementChoice {
    pub stage: PreviewStageId,
    pub label: String,
    pub anchor: PreviewSourceAnchor,
}

type Apply<T> = Rc<dyn Fn(T) -> Result<(), String>>;

struct EditorState<T> {
    root: Ref<Grid>,
    body: Ref<StackPanel>,
    apply_button: Ref<Button>,
    cancel_button: Ref<Button>,
    error: Ref<TextBlock>,
    retired: Cell<bool>,
    apply: RefCell<Option<Apply<T>>>,
    close: RefCell<Option<Rc<dyn Fn()>>>,
}

impl<T> EditorState<T> {
    fn new(title: &str, prefix: &str, explanation: &str) -> Self {
        let root = Grid::new();
        root.set_name(Some(format!("{prefix}Editor")));
        root.classes().add("designer-chrome");
        root.set_max_width(460.0);
        root.set_max_height(640.0);
        root.set_row_definitions(RowDefinitions::parse("Auto,*,Auto").unwrap());
        let heading = StackPanel::new();
        heading.set_spacing(7.0);
        heading.set_margin(Thickness::new(0.0, 0.0, 0.0, 12.0));
        let title = text(title);
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        heading.children().add(title.upcast::<Control>());
        heading
            .children()
            .add(text(explanation).upcast::<Control>());
        root.children().add(heading.upcast::<Control>());
        let body = StackPanel::new();
        body.set_spacing(7.0);
        let scroll = ScrollViewer::new();
        scroll.set_name(Some(format!("{prefix}Scroll")));
        scroll.set_max_height(430.0);
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_content(Some(Control::boxed(body.clone().upcast::<Control>())));
        Grid::set_row(&scroll, 1);
        root.children().add(scroll.upcast::<Control>());
        let footer = StackPanel::new();
        footer.set_name(Some(format!("{prefix}Footer")));
        footer.set_margin(Thickness::new(0.0, 12.0, 0.0, 0.0));
        footer.set_spacing(7.0);
        Grid::set_row(&footer, 2);
        let error = text("");
        error.set_name(Some(format!("{prefix}Error")));
        error.classes().add("designer-error");
        error.set_is_visible(false);
        footer.children().add(error.clone().upcast::<Control>());
        let commands = StackPanel::new();
        commands.set_orientation(Orientation::Horizontal);
        commands.set_horizontal_alignment(HorizontalAlignment::Right);
        commands.set_spacing(8.0);
        let cancel_button = button("Cancel", &format!("{prefix}Cancel"));
        let apply_button = button("Apply", &format!("{prefix}Apply"));
        apply_button.classes().add("primary");
        commands
            .children()
            .add(cancel_button.clone().upcast::<Control>());
        commands
            .children()
            .add(apply_button.clone().upcast::<Control>());
        footer.children().add(commands.upcast::<Control>());
        root.children().add(footer.upcast::<Control>());
        root.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                // Each concrete editor handles Escape after construction so its
                // cancellation callback can run. Other unhandled keys stay local.
                if event.key != Key::Escape {
                    event.set_handled(true);
                }
            },
            RoutingStrategies::BUBBLE,
            false,
        );
        Self {
            root,
            body,
            apply_button,
            cancel_button,
            error,
            retired: Cell::new(false),
            apply: RefCell::new(None),
            close: RefCell::new(None),
        }
    }

    fn submit(&self, value: Result<T, String>) -> Result<(), String> {
        if self.retired.get() {
            return Err("This prototype editor is closed; reopen it to apply changes".into());
        }
        let result = value.and_then(|value| {
            self.apply
                .borrow()
                .clone()
                .ok_or("No prototype editor host is attached")?(value)
        });
        self.show_error(result.as_ref().err().map(String::as_str).unwrap_or(""));
        if result.is_ok() {
            self.cancel();
        }
        result
    }

    fn cancel(&self) {
        self.retire();
        if let Some(callback) = self.close.borrow().clone() {
            callback();
        }
    }

    fn retire(&self) {
        self.retired.set(true);
        self.apply_button.set_is_enabled(false);
    }

    fn show_error(&self, message: &str) {
        self.error.set_text(Some(message));
        self.error.set_is_visible(!message.is_empty());
    }
}

pub struct PreviewPrototypeFlowEditor {
    state: EditorState<PreviewPrototypeFlow>,
    name: Ref<TextBox>,
    start: Ref<ComboBox>,
    stages: Vec<PreviewStageInfo>,
}

impl PreviewPrototypeFlowEditor {
    pub fn new(initial: Option<PreviewPrototypeFlow>, stages: Vec<PreviewStageInfo>) -> Rc<Self> {
        let state = EditorState::new("Prototype flow", "PrototypeFlow", "Name a journey and choose its first screen. Interactions connect the screens reached from this start.");
        let name = input(
            "PrototypeFlowName",
            "Flow name",
            initial.as_ref().map_or("", |flow| flow.name.as_str()),
        );
        field(&state.body, "Name", &name);
        let start = stage_picker("PrototypeFlowStart", "Starting screen", &stages);
        let selected = initial
            .as_ref()
            .and_then(|flow| stages.iter().position(|stage| stage.id == flow.start));
        start.set_selected_index(
            selected.map_or(if stages.is_empty() { -1 } else { 0 }, |index| index as i32),
        );
        field(&state.body, "Starting screen", &start);
        let editor = Rc::new(Self {
            state,
            name,
            start,
            stages,
        });
        let weak = Rc::downgrade(&editor);
        editor.state.apply_button.click(move |_, event| {
            if let Some(editor) = weak.upgrade() {
                let _ = editor.apply();
            }
            event.set_handled(true);
        });
        let weak = Rc::downgrade(&editor);
        editor.state.cancel_button.click(move |_, event| {
            if let Some(editor) = weak.upgrade() {
                editor.cancel();
            }
            event.set_handled(true);
        });
        let weak = Rc::downgrade(&editor);
        editor
            .state
            .root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Escape {
                    if let Some(editor) = weak.upgrade() {
                        editor.cancel();
                    }
                    event.set_handled(true);
                }
            });
        editor
    }

    pub fn control(&self) -> Ref<Control> {
        self.state.root.clone().upcast()
    }
    pub fn on_apply(
        &self,
        callback: impl Fn(PreviewPrototypeFlow) -> Result<(), String> + 'static,
    ) {
        *self.state.apply.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_close(&self, callback: impl Fn() + 'static) {
        *self.state.close.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn cancel(&self) {
        self.state.cancel();
    }
    pub fn retire(&self) {
        self.state.retire();
    }
    pub fn show_error(&self, message: &str) {
        self.state.show_error(message);
    }

    pub fn value(&self) -> Result<PreviewPrototypeFlow, String> {
        let start = selected_stage(&self.start, &self.stages).ok_or("Choose a starting screen")?;
        let value = PreviewPrototypeFlow {
            name: self.name.text().unwrap_or_default().trim().to_owned(),
            start,
        };
        PreviewPrototype {
            flows: vec![value.clone()],
            links: Vec::new(),
        }
        .validate(&self.stages.iter().map(|stage| stage.id).collect::<Vec<_>>())?;
        Ok(value)
    }

    pub fn apply(&self) -> Result<(), String> {
        self.state.submit(self.value())
    }
}

pub struct PreviewPrototypeLinkEditor {
    state: EditorState<PreviewPrototypeLink>,
    name: Ref<TextBox>,
    source: Ref<ComboBox>,
    element: Ref<ComboBox>,
    action: Ref<ComboBox>,
    destination: Ref<ComboBox>,
    destination_field: Ref<StackPanel>,
    element_note: Ref<TextBlock>,
    stages: Vec<PreviewStageInfo>,
    elements: Vec<PreviewPrototypeElementChoice>,
    visible_elements: RefCell<Vec<PreviewPrototypeElementChoice>>,
}

impl PreviewPrototypeLinkEditor {
    pub fn new(
        initial: Option<PreviewPrototypeLink>,
        source: Option<PreviewStageId>,
        stages: Vec<PreviewStageInfo>,
        elements: Vec<PreviewPrototypeElementChoice>,
    ) -> Rc<Self> {
        let state = EditorState::new("Prototype interaction", "PrototypeLink", "Activate an authored element to navigate to another screen or return through this journey's history. XAML remains unchanged.");
        let name = input(
            "PrototypeLinkName",
            "Interaction name",
            initial.as_ref().map_or("", |link| link.name.as_str()),
        );
        field(&state.body, "Name", &name);
        let source_picker = stage_picker("PrototypeLinkSource", "Source screen", &stages);
        let initial_source = initial.as_ref().map(|link| link.source).or(source);
        source_picker.set_selected_index(
            initial_source
                .and_then(|id| stages.iter().position(|stage| stage.id == id))
                .map_or(if stages.is_empty() { -1 } else { 0 }, |index| index as i32),
        );
        field(&state.body, "Source screen", &source_picker);
        let element = picker("PrototypeLinkElement", "Authored hotspot element");
        field(&state.body, "Activate element", &element);
        let element_note = text("");
        element_note.set_name(Some("PrototypeLinkElementNote".into()));
        element_note.classes().add("designer-muted");
        state
            .body
            .children()
            .add(element_note.clone().upcast::<Control>());
        let action = picker("PrototypeLinkAction", "Prototype action");
        for label in ["Navigate to screen", "Back"] {
            action.items().add(Some(Rc::new(label.to_owned())));
        }
        action.set_selected_index(i32::from(
            initial
                .as_ref()
                .is_some_and(|link| matches!(link.action, PreviewPrototypeAction::Back)),
        ));
        field(&state.body, "Action", &action);
        let destination = stage_picker("PrototypeLinkDestination", "Destination screen", &stages);
        let target = initial.as_ref().and_then(|link| match link.action {
            PreviewPrototypeAction::Navigate { stage } => Some(stage),
            PreviewPrototypeAction::Back => None,
        });
        destination.set_selected_index(
            target
                .and_then(|id| stages.iter().position(|stage| stage.id == id))
                .map_or(if stages.is_empty() { -1 } else { 0 }, |index| index as i32),
        );
        let destination_field = field(&state.body, "Destination screen", &destination);
        let editor = Rc::new(Self {
            state,
            name,
            source: source_picker,
            element,
            action,
            destination,
            destination_field,
            element_note,
            stages,
            elements,
            visible_elements: RefCell::new(Vec::new()),
        });
        editor.rebuild_elements(
            initial.as_ref().map(|link| &link.trigger),
            initial.is_none(),
        );
        editor.refresh_action();
        let weak = Rc::downgrade(&editor);
        editor.source.selection_changed(move |_, _| {
            if let Some(editor) = weak.upgrade().filter(|editor| !editor.state.retired.get()) {
                let selected = usize::try_from(editor.element.selected_index())
                    .ok()
                    .and_then(|index| {
                        editor
                            .visible_elements
                            .borrow()
                            .get(index)
                            .map(|choice| choice.anchor.clone())
                    });
                editor.rebuild_elements(selected.as_ref(), false);
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.action.selection_changed(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                editor.refresh_action();
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.state.apply_button.click(move |_, event| {
            if let Some(editor) = weak.upgrade() {
                let _ = editor.apply();
            }
            event.set_handled(true);
        });
        let weak = Rc::downgrade(&editor);
        editor.state.cancel_button.click(move |_, event| {
            if let Some(editor) = weak.upgrade() {
                editor.cancel();
            }
            event.set_handled(true);
        });
        let weak = Rc::downgrade(&editor);
        editor
            .state
            .root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Escape {
                    if let Some(editor) = weak.upgrade() {
                        editor.cancel();
                    }
                    event.set_handled(true);
                }
            });
        editor
    }

    fn rebuild_elements(&self, selected: Option<&PreviewSourceAnchor>, default_first: bool) {
        let source = selected_stage(&self.source, &self.stages);
        let choices = self
            .elements
            .iter()
            .filter(|element| Some(element.stage) == source)
            .cloned()
            .collect::<Vec<_>>();
        self.element.items().clear();
        for choice in &choices {
            self.element
                .items()
                .add(Some(Rc::new(choice.label.clone())));
        }
        self.element.set_selected_index(
            selected
                .and_then(|anchor| choices.iter().position(|choice| &choice.anchor == anchor))
                .map_or(
                    if selected.is_some() || choices.is_empty() || !default_first {
                        -1
                    } else {
                        0
                    },
                    |index| index as i32,
                ),
        );
        self.element_note.set_text(Some(if choices.is_empty() {
            "No authored hotspot is available in this screen. Open its accepted source and refresh the stage."
        } else if selected.is_some() && self.element.selected_index() < 0 {
            "The original hotspot is unavailable. Choose a current authored element before applying."
        } else {
            "Hotspots follow their authored element's current native bounds. Generated or repeated template parts require a definition stage."
        }));
        *self.visible_elements.borrow_mut() = choices;
    }

    fn refresh_action(&self) {
        self.destination_field
            .set_is_visible(self.action.selected_index() == 0);
    }
    pub fn control(&self) -> Ref<Control> {
        self.state.root.clone().upcast()
    }
    pub fn on_apply(
        &self,
        callback: impl Fn(PreviewPrototypeLink) -> Result<(), String> + 'static,
    ) {
        *self.state.apply.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_close(&self, callback: impl Fn() + 'static) {
        *self.state.close.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn cancel(&self) {
        self.state.cancel();
    }
    pub fn retire(&self) {
        self.state.retire();
    }
    pub fn show_error(&self, message: &str) {
        self.state.show_error(message);
    }

    pub fn value(&self) -> Result<PreviewPrototypeLink, String> {
        let source = selected_stage(&self.source, &self.stages).ok_or("Choose a source screen")?;
        let trigger = usize::try_from(self.element.selected_index())
            .ok()
            .and_then(|index| {
                self.visible_elements
                    .borrow()
                    .get(index)
                    .map(|choice| choice.anchor.clone())
            })
            .ok_or("Choose an available authored hotspot")?;
        let action = match self.action.selected_index() {
            0 => PreviewPrototypeAction::Navigate {
                stage: selected_stage(&self.destination, &self.stages)
                    .ok_or("Choose a destination screen")?,
            },
            1 => PreviewPrototypeAction::Back,
            _ => return Err("Choose a prototype action".into()),
        };
        let value = PreviewPrototypeLink {
            name: self.name.text().unwrap_or_default().trim().to_owned(),
            source,
            trigger,
            action,
        };
        PreviewPrototype {
            flows: Vec::new(),
            links: vec![value.clone()],
        }
        .validate(&self.stages.iter().map(|stage| stage.id).collect::<Vec<_>>())?;
        Ok(value)
    }

    pub fn apply(&self) -> Result<(), String> {
        self.state.submit(self.value())
    }
}

fn text(value: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
fn button(value: &str, name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_content(Some(Rc::new(value.to_owned())));
    AutomationProperties::set_name(&button, Some(value));
    button
}
fn input(name: &str, label: &str, value: &str) -> Ref<TextBox> {
    let input = TextBox::new();
    input.set_name(Some(name.into()));
    input.set_text(Some(value));
    AutomationProperties::set_name(&input, Some(label));
    input
}
fn picker(name: &str, label: &str) -> Ref<ComboBox> {
    let picker = ComboBox::new();
    picker.set_name(Some(name.into()));
    picker.set_horizontal_alignment(HorizontalAlignment::Stretch);
    AutomationProperties::set_name(&picker, Some(label));
    picker
}
fn stage_picker(name: &str, label: &str, stages: &[PreviewStageInfo]) -> Ref<ComboBox> {
    let picker = picker(name, label);
    for stage in stages {
        let file = stage
            .configuration
            .source_uri
            .rsplit('/')
            .next()
            .unwrap_or(&stage.configuration.source_uri);
        picker.items().add(Some(Rc::new(format!(
            "{} · {file}",
            stage.configuration.name
        ))));
    }
    picker
}
fn selected_stage(picker: &ComboBox, stages: &[PreviewStageInfo]) -> Option<PreviewStageId> {
    usize::try_from(picker.selected_index())
        .ok()
        .and_then(|index| stages.get(index))
        .map(|stage| stage.id)
}
fn field(body: &StackPanel, label: &str, control: &Control) -> Ref<StackPanel> {
    let field = StackPanel::new();
    field.set_spacing(5.0);
    field.children().add(text(label).upcast::<Control>());
    field.children().add(control.to_ref());
    body.children().add(field.clone().upcast::<Control>());
    field
}
