//! A detached native form for preview combinations, with a count and bounded plan.
use crate::{
    PreviewFlowDirection, PreviewSourceAnchor, PreviewStageConfiguration, PreviewStageMatrix,
    PreviewStageMatrixSize, PreviewStageStateAxis, PreviewStageTarget, PreviewTheme,
    MAX_PREVIEW_STAGES, PREVIEW_SIZE_PRESETS,
};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Button, CheckBox, Control, Grid, RowDefinitions, ScrollViewer, StackPanel, TextBlock,
};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_core::Document;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Apply = Rc<dyn Fn(PreviewStageMatrix) -> Result<(), String>>;
type Choices<T> = Vec<(T, Ref<CheckBox>)>;
struct StateChoices {
    owner: PreviewSourceAnchor,
    group: String,
    values: Choices<String>,
}

/// Empty check-box groups retain the original stage's values. Applying requests a
/// complete matrix only after the host verifies the source and board still match.
pub struct PreviewStageMatrixEditor {
    root: Ref<Grid>,
    base: PreviewStageConfiguration,
    document: Document,
    available: usize,
    sizes: Choices<PreviewStageMatrixSize>,
    themes: Choices<PreviewTheme>,
    directions: Choices<PreviewFlowDirection>,
    data: Choices<bool>,
    states: Vec<StateChoices>,
    summary: Ref<TextBlock>,
    plan: Ref<TextBlock>,
    error: Ref<TextBlock>,
    apply_button: Ref<Button>,
    retired: Cell<bool>,
    apply: RefCell<Option<Apply>>,
    close: RefCell<Option<Rc<dyn Fn()>>>,
}

impl PreviewStageMatrixEditor {
    pub fn new(base: PreviewStageConfiguration, document: Document, available: usize) -> Rc<Self> {
        let root = Grid::new();
        root.set_name(Some("PreviewStageMatrixEditor".into()));
        root.classes().add("designer-chrome");
        root.set_max_width(460.0);
        root.set_max_height(680.0);
        root.set_row_definitions(RowDefinitions::parse("Auto,*,Auto").unwrap());
        let heading = StackPanel::new();
        heading.set_spacing(6.0);
        heading.set_margin(Thickness::new(0.0, 0.0, 0.0, 10.0));
        let title = text("Compare preview matrix");
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        heading.children().add(title.upcast::<Control>());
        heading
            .children()
            .add(text(&format!("Base: {}", base.name)).upcast::<Control>());
        heading.children().add(text("Select values to combine. Leave a group empty to keep the base stage's value. Each combination becomes an independent stage.").upcast::<Control>());
        root.children().add(heading.upcast::<Control>());
        let body = StackPanel::new();
        body.set_spacing(6.0);
        section(&body, "Layout sizes");
        let mut size_values = vec![PreviewStageMatrixSize {
            name: "Current size".into(),
            width: base.environment.width,
            height: base.environment.height,
        }];
        for size in PREVIEW_SIZE_PRESETS {
            if !size_values
                .iter()
                .any(|value| value.width == size.width && value.height == size.height)
            {
                size_values.push(PreviewStageMatrixSize {
                    name: size
                        .name
                        .split('·')
                        .next()
                        .unwrap_or(size.name)
                        .trim()
                        .into(),
                    width: size.width,
                    height: size.height,
                });
            }
        }
        let sizes = size_values
            .into_iter()
            .enumerate()
            .map(|(index, size)| {
                let control = choice(
                    &format!("{} · {} × {}", size.name, size.width, size.height),
                    &format!("MatrixSize{index}"),
                );
                body.children().add(control.clone().upcast::<Control>());
                (size, control)
            })
            .collect();
        section(&body, "Themes");
        let themes = choices(
            &body,
            "MatrixTheme",
            &[
                (PreviewTheme::Light, "Light"),
                (PreviewTheme::Dark, "Dark"),
                (PreviewTheme::Inherit, "Application theme"),
            ],
        );
        section(&body, "Reading directions");
        let directions = choices(
            &body,
            "MatrixDirection",
            &[
                (PreviewFlowDirection::LeftToRight, "Left to right"),
                (PreviewFlowDirection::RightToLeft, "Right to left"),
                (PreviewFlowDirection::Inherit, "Inherited direction"),
            ],
        );
        section(&body, "Data projection");
        let data = choices(
            &body,
            "MatrixData",
            &[
                (true, "Design-time values and sample items"),
                (false, "Runtime values and bindings"),
            ],
        );
        let scope = match &base.target {
            PreviewStageTarget::Document => Some(document.ast().root()),
            PreviewStageTarget::Definition { anchor, .. } => anchor.resolve(&document).ok(),
        };
        let mut states = Vec::new();
        for group in VisualStateGroup::read(&document) {
            if !scope.is_some_and(|scope| {
                document
                    .ast()
                    .subtree(scope)
                    .any(|node| node.id == group.owner)
            }) {
                continue;
            }
            let Ok(owner) = PreviewSourceAnchor::capture(&document, group.owner) else {
                continue;
            };
            let owner_name = document
                .ast()
                .node(group.owner)
                .and_then(|node| node.xaml_name())
                .unwrap_or("Unnamed owner");
            section(&body, &format!("{owner_name} · {}", group.name));
            let values = group
                .states
                .iter()
                .enumerate()
                .map(|(index, state)| {
                    let control =
                        choice(&state.name, &format!("MatrixState{}_{index}", states.len()));
                    body.children().add(control.clone().upcast::<Control>());
                    (state.name.clone(), control)
                })
                .collect();
            states.push(StateChoices {
                owner,
                group: group.name,
                values,
            });
        }
        if states.is_empty() {
            body.children().add(
                text("This view has no unambiguous authored visual-state groups to vary.")
                    .upcast::<Control>(),
            );
        }
        section(&body, "Resulting stages");
        let plan = text("");
        plan.set_name(Some("MatrixPlan".into()));
        plan.classes().add("designer-muted");
        body.children().add(plan.clone().upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_name(Some("MatrixScroll".into()));
        scroll.set_max_height(430.0);
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_content(Some(Control::boxed(body.upcast::<Control>())));
        Grid::set_row(&scroll, 1);
        root.children().add(scroll.upcast::<Control>());
        let footer = StackPanel::new();
        footer.set_name(Some("MatrixFooter".into()));
        footer.set_spacing(7.0);
        footer.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
        Grid::set_row(&footer, 2);
        let summary = text("");
        summary.set_name(Some("MatrixSummary".into()));
        footer.children().add(summary.clone().upcast::<Control>());
        let error = text("");
        error.set_name(Some("MatrixError".into()));
        error.classes().add("designer-error");
        error.set_is_visible(false);
        footer.children().add(error.clone().upcast::<Control>());
        let buttons = StackPanel::new();
        buttons.set_orientation(Orientation::Horizontal);
        buttons.set_horizontal_alignment(HorizontalAlignment::Right);
        buttons.set_spacing(8.0);
        let cancel = button("Cancel", "MatrixCancel");
        let apply_button = button("Add comparison stages", "MatrixApply");
        apply_button.classes().add("primary");
        buttons.children().add(cancel.clone().upcast::<Control>());
        buttons
            .children()
            .add(apply_button.clone().upcast::<Control>());
        footer.children().add(buttons.upcast::<Control>());
        root.children().add(footer.upcast::<Control>());
        let editor = Rc::new(Self {
            root,
            base,
            document,
            available,
            sizes,
            themes,
            directions,
            data,
            states,
            summary,
            plan,
            error,
            apply_button,
            retired: Cell::new(false),
            apply: RefCell::new(None),
            close: RefCell::new(None),
        });
        let all = editor
            .sizes
            .iter()
            .map(|(_, control)| control)
            .chain(editor.themes.iter().map(|(_, control)| control))
            .chain(editor.directions.iter().map(|(_, control)| control))
            .chain(editor.data.iter().map(|(_, control)| control))
            .chain(
                editor
                    .states
                    .iter()
                    .flat_map(|group| group.values.iter().map(|(_, control)| control)),
            );
        for control in all {
            let weak = Rc::downgrade(&editor);
            control.is_checked_changed(move |_, _| {
                if let Some(editor) = weak.upgrade() {
                    editor.refresh();
                }
            });
        }
        let weak = Rc::downgrade(&editor);
        editor.apply_button.click(move |_, event| {
            if let Some(editor) = weak.upgrade() {
                let _ = editor.apply();
            }
            event.set_handled(true);
        });
        let weak = Rc::downgrade(&editor);
        cancel.click(move |_, event| {
            if let Some(editor) = weak.upgrade() {
                editor.cancel();
            }
            event.set_handled(true);
        });
        let weak = Rc::downgrade(&editor);
        editor
            .root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Escape {
                    if let Some(editor) = weak.upgrade() {
                        editor.cancel();
                    }
                    event.set_handled(true);
                }
            });
        editor.refresh();
        editor
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }
    pub fn on_apply(&self, apply: impl Fn(PreviewStageMatrix) -> Result<(), String> + 'static) {
        *self.apply.borrow_mut() = Some(Rc::new(apply));
    }
    pub fn on_close(&self, close: impl Fn() + 'static) {
        *self.close.borrow_mut() = Some(Rc::new(close));
    }
    pub fn retire(&self) {
        self.retired.set(true);
        self.apply_button.set_is_enabled(false);
    }
    pub fn show_error(&self, error: &str) {
        self.error.set_text(Some(error));
        self.error.set_is_visible(!error.is_empty());
    }

    pub fn draft(&self) -> PreviewStageMatrix {
        PreviewStageMatrix {
            sizes: selected(&self.sizes),
            themes: selected(&self.themes),
            directions: selected(&self.directions),
            design_time_data: selected(&self.data),
            state_groups: self
                .states
                .iter()
                .filter_map(|group| {
                    let states = selected(&group.values);
                    (!states.is_empty()).then(|| PreviewStageStateAxis {
                        owner: group.owner.clone(),
                        group: group.group.clone(),
                        states,
                    })
                })
                .collect(),
        }
    }

    pub fn apply(&self) -> Result<(), String> {
        let result = (|| {
            if self.retired.get() {
                return Err("This comparison editor is no longer current; reopen it".into());
            }
            let draft = self.draft();
            draft.expand(&self.base, &self.document, self.available)?;
            let apply = self
                .apply
                .borrow()
                .clone()
                .ok_or("No comparison host is attached")?;
            apply(draft)?;
            self.cancel();
            Ok(())
        })();
        self.show_error(result.as_ref().err().map(String::as_str).unwrap_or(""));
        result
    }

    pub fn cancel(&self) {
        self.retire();
        if let Some(close) = self.close.borrow().clone() {
            close();
        }
    }

    fn refresh(&self) {
        if self.retired.get() {
            return;
        }
        let draft = self.draft();
        match draft.expand(&self.base, &self.document, self.available) {
            Ok(configurations) => {
                self.summary.set_text(Some(&format!(
                    "{} new stages · {} available of {MAX_PREVIEW_STAGES}",
                    configurations.len(),
                    self.available
                )));
                self.plan.set_text(Some(
                    &configurations
                        .iter()
                        .map(|config| config.name.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                ));
                self.apply_button.set_is_enabled(true);
                self.show_error("");
            }
            Err(error) => {
                let count = draft
                    .count()
                    .map(|count| count.to_string())
                    .unwrap_or_else(|_| "Invalid".into());
                self.summary.set_text(Some(&format!(
                    "{count} combinations · {} available",
                    self.available
                )));
                self.plan
                    .set_text(Some("Reduce the selected values to fit the board."));
                self.apply_button.set_is_enabled(false);
                self.show_error(&error);
            }
        }
    }
}

fn selected<T: Clone>(choices: &Choices<T>) -> Vec<T> {
    choices
        .iter()
        .filter(|(_, control)| control.is_checked() == Some(true))
        .map(|(value, _)| value.clone())
        .collect()
}
fn choices<T: Clone>(body: &StackPanel, prefix: &str, values: &[(T, &str)]) -> Choices<T> {
    values
        .iter()
        .enumerate()
        .map(|(index, (value, label))| {
            let check = choice(label, &format!("{prefix}{index}"));
            body.children().add(check.clone().upcast::<Control>());
            (value.clone(), check)
        })
        .collect()
}
fn choice(label: &str, name: &str) -> Ref<CheckBox> {
    let check = CheckBox::new();
    check.set_name(Some(name.into()));
    check.set_content(Some(Rc::new(label.to_owned())));
    check.set_is_checked(Some(false));
    AutomationProperties::set_name(&check, Some(label));
    check
}
fn text(value: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
fn section(body: &StackPanel, value: &str) {
    let text = text(value);
    text.set_font_weight(FontWeight::SemiBold);
    text.set_margin(Thickness::new(0.0, 9.0, 0.0, 3.0));
    body.children().add(text.upcast::<Control>());
}
fn button(value: &str, name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_content(Some(Rc::new(value.to_owned())));
    AutomationProperties::set_name(&button, Some(value));
    button
}
