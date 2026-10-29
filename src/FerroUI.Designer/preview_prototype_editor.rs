//! Detached native forms for named prototype starts and source-backed interactions.
use crate::{
    PreviewPrototype, PreviewPrototypeAction, PreviewPrototypeFlow, PreviewPrototypeLink,
    PreviewPrototypeOverlayOptions, PreviewPrototypeOverlayPlacement, PreviewSourceAnchor,
    PreviewStageId, PreviewStageInfo, MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET,
};
use ferroui_base::input::{
    IKeyboardNavigationHandler, InputElement, Key, KeyboardNavigation, KeyboardNavigationHandler,
    KeyboardNavigationMode,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Button, CheckBox, ComboBox, Control, Grid, RowDefinitions, ScrollViewer, StackPanel, TextBlock,
    TextBox,
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
    _keyboard_navigation: Rc<KeyboardNavigationHandler>,
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
        let keyboard_navigation = KeyboardNavigationHandler::new();
        keyboard_navigation.set_owner(&root.clone().upcast());
        KeyboardNavigation::set_tab_navigation(&root, KeyboardNavigationMode::Cycle);
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
            _keyboard_navigation: keyboard_navigation,
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
        start.set_selected_index(selected.map_or(
            if initial.is_some() || stages.is_empty() {
                -1
            } else {
                0
            },
            |index| index as i32,
        ));
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
    overlay_fields: Ref<StackPanel>,
    placement: Ref<ComboBox>,
    manual_fields: Ref<StackPanel>,
    offset_x: Ref<TextBox>,
    offset_y: Ref<TextBox>,
    dismiss_on_outside: Ref<CheckBox>,
    dim_background: Ref<CheckBox>,
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
        let state = EditorState::new("Prototype interaction", "PrototypeLink", "Activate an authored element to navigate, open an overlay, or return through this journey's history.");
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
                .map_or(
                    if initial_source.is_some() || stages.is_empty() {
                        -1
                    } else {
                        0
                    },
                    |index| index as i32,
                ),
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
        for label in [
            "Navigate to screen",
            "Back",
            "Open overlay",
            "Replace top overlay",
            "Close overlay",
        ] {
            action.items().add(Some(Rc::new(label.to_owned())));
        }
        action.set_selected_index(initial.as_ref().map_or(0, |link| match link.action {
            PreviewPrototypeAction::Navigate { .. } => 0,
            PreviewPrototypeAction::Back => 1,
            PreviewPrototypeAction::OpenOverlay { .. } => 2,
            PreviewPrototypeAction::SwapOverlay { .. } => 3,
            PreviewPrototypeAction::CloseOverlay => 4,
        }));
        field(&state.body, "Action", &action);
        let destination = stage_picker("PrototypeLinkDestination", "Destination screen", &stages);
        let target = initial.as_ref().and_then(|link| link.action.target_stage());
        destination.set_selected_index(
            target
                .and_then(|id| stages.iter().position(|stage| stage.id == id))
                .map_or(
                    if target.is_some() || stages.is_empty() {
                        -1
                    } else {
                        0
                    },
                    |index| index as i32,
                ),
        );
        let destination_field = field(&state.body, "Destination screen", &destination);
        let overlay_fields = StackPanel::new();
        overlay_fields.set_name(Some("PrototypeLinkOverlayOptions".into()));
        overlay_fields.set_spacing(7.0);
        let options = initial
            .as_ref()
            .and_then(|link| match link.action {
                PreviewPrototypeAction::OpenOverlay { options, .. }
                | PreviewPrototypeAction::SwapOverlay { options, .. } => Some(options),
                _ => None,
            })
            .unwrap_or_default();
        let placement = picker("PrototypeLinkOverlayPlacement", "Overlay placement");
        for label in [
            "Center",
            "Top left",
            "Top center",
            "Top right",
            "Center left",
            "Center right",
            "Bottom left",
            "Bottom center",
            "Bottom right",
            "Manual position",
        ] {
            placement.items().add(Some(Rc::new(label.to_owned())));
        }
        placement.set_selected_index(match options.placement {
            PreviewPrototypeOverlayPlacement::Center => 0,
            PreviewPrototypeOverlayPlacement::TopLeft => 1,
            PreviewPrototypeOverlayPlacement::TopCenter => 2,
            PreviewPrototypeOverlayPlacement::TopRight => 3,
            PreviewPrototypeOverlayPlacement::CenterLeft => 4,
            PreviewPrototypeOverlayPlacement::CenterRight => 5,
            PreviewPrototypeOverlayPlacement::BottomLeft => 6,
            PreviewPrototypeOverlayPlacement::BottomCenter => 7,
            PreviewPrototypeOverlayPlacement::BottomRight => 8,
            PreviewPrototypeOverlayPlacement::Manual { .. } => 9,
        });
        field(&overlay_fields, "Placement", &placement);
        let manual_fields = StackPanel::new();
        manual_fields.set_name(Some("PrototypeLinkOverlayManual".into()));
        manual_fields.set_spacing(7.0);
        let (x, y) = match options.placement {
            PreviewPrototypeOverlayPlacement::Manual { x, y } => (x, y),
            _ => (0, 0),
        };
        let offset_x = input(
            "PrototypeLinkOverlayX",
            "Overlay horizontal position",
            &x.to_string(),
        );
        let offset_y = input(
            "PrototypeLinkOverlayY",
            "Overlay vertical position",
            &y.to_string(),
        );
        field(&manual_fields, "Horizontal position", &offset_x);
        field(&manual_fields, "Vertical position", &offset_y);
        let note = text("Positions use the base screen's logical coordinates.");
        note.classes().add("designer-muted");
        manual_fields.children().add(note.upcast::<Control>());
        overlay_fields
            .children()
            .add(manual_fields.clone().upcast::<Control>());
        let dismiss_on_outside = CheckBox::new();
        dismiss_on_outside.set_name(Some("PrototypeLinkOverlayDismiss".into()));
        dismiss_on_outside.set_content(Some(Rc::new("Close when clicking outside".to_owned())));
        dismiss_on_outside.set_is_checked(Some(options.dismiss_on_outside));
        let dim_background = CheckBox::new();
        dim_background.set_name(Some("PrototypeLinkOverlayDim".into()));
        dim_background.set_content(Some(Rc::new("Dim underlying screens".to_owned())));
        dim_background.set_is_checked(Some(options.dim_background));
        overlay_fields
            .children()
            .add(dismiss_on_outside.clone().upcast::<Control>());
        overlay_fields
            .children()
            .add(dim_background.clone().upcast::<Control>());
        state
            .body
            .children()
            .add(overlay_fields.clone().upcast::<Control>());
        let editor = Rc::new(Self {
            state,
            name,
            source: source_picker,
            element,
            action,
            destination,
            destination_field,
            overlay_fields,
            placement,
            manual_fields,
            offset_x,
            offset_y,
            dismiss_on_outside,
            dim_background,
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
        editor.placement.selection_changed(move |_, _| {
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
        let action = self.action.selected_index();
        self.destination_field
            .set_is_visible(matches!(action, 0 | 2 | 3));
        self.overlay_fields.set_is_visible(matches!(action, 2 | 3));
        self.manual_fields
            .set_is_visible(self.placement.selected_index() == 9);
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
            2 => PreviewPrototypeAction::OpenOverlay {
                stage: selected_stage(&self.destination, &self.stages)
                    .ok_or("Choose an overlay screen")?,
                options: self.overlay_options()?,
            },
            3 => PreviewPrototypeAction::SwapOverlay {
                stage: selected_stage(&self.destination, &self.stages)
                    .ok_or("Choose a replacement overlay screen")?,
                options: self.overlay_options()?,
            },
            4 => PreviewPrototypeAction::CloseOverlay,
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

    fn overlay_options(&self) -> Result<PreviewPrototypeOverlayOptions, String> {
        let position = |input: &TextBox, axis: &str| {
            input.text().unwrap_or_default().trim().parse::<i32>().map_err(|_| {
                format!("Enter a whole {axis} position between -{MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET} and {MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET}")
            })
        };
        let placement = match self.placement.selected_index() {
            0 => PreviewPrototypeOverlayPlacement::Center,
            1 => PreviewPrototypeOverlayPlacement::TopLeft,
            2 => PreviewPrototypeOverlayPlacement::TopCenter,
            3 => PreviewPrototypeOverlayPlacement::TopRight,
            4 => PreviewPrototypeOverlayPlacement::CenterLeft,
            5 => PreviewPrototypeOverlayPlacement::CenterRight,
            6 => PreviewPrototypeOverlayPlacement::BottomLeft,
            7 => PreviewPrototypeOverlayPlacement::BottomCenter,
            8 => PreviewPrototypeOverlayPlacement::BottomRight,
            9 => PreviewPrototypeOverlayPlacement::Manual {
                x: position(&self.offset_x, "horizontal")?,
                y: position(&self.offset_y, "vertical")?,
            },
            _ => return Err("Choose an overlay placement".into()),
        };
        let options = PreviewPrototypeOverlayOptions {
            placement,
            dismiss_on_outside: self.dismiss_on_outside.is_checked() == Some(true),
            dim_background: self.dim_background.is_checked() == Some(true),
        };
        options.validate()?;
        Ok(options)
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

#[cfg(test)]
#[path = "preview_prototype_editor_stage_tests.rs"]
mod stage_tests;
