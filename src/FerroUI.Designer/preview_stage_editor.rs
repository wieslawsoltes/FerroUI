//! A complete, staged editor for one native preview scenario. Controls edit only a
//! detached configuration; the host validates the accepted board/source epoch on Apply.
use crate::{
    DefinitionPreviewOptions, PreviewEnvironment, PreviewFlowDirection, PreviewSourceAnchor,
    PreviewStageConfiguration, PreviewStageLayout, PreviewStageMode, PreviewStageSource,
    PreviewStageState, PreviewStageTarget, PreviewTheme, PREVIEW_SIZE_PRESETS,
};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{
    Button, CheckBox, ColumnDefinitions, ComboBox, Control, Grid, RowDefinitions, ScrollViewer,
    StackPanel, TextBlock, TextBox,
};
use ferroui_designer_animation::VisualStateGroup;
use ferroui_designer_tools::ComponentCatalog;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Apply = Rc<dyn Fn(PreviewStageConfiguration) -> Result<(), String>>;

struct StateChoice {
    owner: PreviewSourceAnchor,
    group: String,
    values: Vec<String>,
    picker: Ref<ComboBox>,
}

/// A reusable form with an independently scrolling body and fixed Apply/Cancel footer.
/// Source documents are immutable snapshots used to offer actual definitions and states.
pub struct PreviewStageEditor {
    root: Ref<Grid>,
    name: Ref<TextBox>,
    source: Ref<ComboBox>,
    source_uris: Vec<String>,
    sources: Vec<PreviewStageSource>,
    target: Ref<ComboBox>,
    targets: RefCell<Vec<PreviewStageTarget>>,
    definition_fields: Ref<StackPanel>,
    target_type: Ref<TextBox>,
    sample_content: Ref<TextBox>,
    sample_width: Ref<TextBox>,
    sample_height: Ref<TextBox>,
    layout: Ref<ComboBox>,
    width_caption: Ref<TextBlock>,
    height_caption: Ref<TextBlock>,
    layout_note: Ref<TextBlock>,
    width: Ref<TextBox>,
    height: Ref<TextBox>,
    theme: Ref<ComboBox>,
    flow: Ref<ComboBox>,
    mode: Ref<ComboBox>,
    design_data: Ref<CheckBox>,
    pinned: Ref<CheckBox>,
    auto_update: Ref<CheckBox>,
    state_host: Ref<StackPanel>,
    states: RefCell<Vec<StateChoice>>,
    retained_states: RefCell<Vec<PreviewStageState>>,
    error: Ref<TextBlock>,
    apply_button: Ref<Button>,
    guard: Cell<bool>,
    retired: Cell<bool>,
    apply: RefCell<Option<Apply>>,
    close: RefCell<Option<Rc<dyn Fn()>>>,
}

impl PreviewStageEditor {
    pub fn new(initial: PreviewStageConfiguration, sources: Vec<PreviewStageSource>) -> Rc<Self> {
        let root = Grid::new();
        root.set_name(Some("PreviewStageEditor".into()));
        root.classes().add("designer-chrome");
        root.set_max_width(440.0);
        root.set_max_height(650.0);
        root.set_row_definitions(RowDefinitions::parse("Auto,*,Auto").unwrap());
        let heading = StackPanel::new();
        heading.set_spacing(6.0);
        heading.set_margin(Thickness::new(0.0, 0.0, 0.0, 10.0));
        let title = label("Preview stage");
        title.set_font_size(16.0);
        title.set_font_weight(FontWeight::SemiBold);
        heading.children().add(title.upcast::<Control>());
        heading.children().add(label("Compare a view at a logical size, theme and scenario. Stage changes leave the XAML document intact.").upcast::<Control>());
        root.children().add(heading.upcast::<Control>());

        let body = StackPanel::new();
        body.set_spacing(9.0);
        let name = field(&initial.name, "StageName", "Stage name");
        row(&body, "Name", &name);
        let source = picker("StageSource", "Source document");
        let mut source_uris = sources
            .iter()
            .map(|source| source.uri.clone())
            .collect::<Vec<_>>();
        source_uris.sort();
        source_uris.dedup();
        if !source_uris.contains(&initial.source_uri) {
            source_uris.push(initial.source_uri.clone());
        }
        for uri in &source_uris {
            let state = sources.iter().find(|source| source.uri == *uri);
            let suffix = match state {
                Some(source) if source.pending => " · source draft",
                Some(_) => "",
                None => " · unavailable",
            };
            item(&source, &format!("{}{suffix}", short_uri(uri)));
        }
        source.set_selected_index(
            source_uris
                .iter()
                .position(|uri| uri == &initial.source_uri)
                .unwrap_or(0) as i32,
        );
        row(&body, "Document", &source);
        let target = picker("StageTarget", "Preview document or authored definition");
        row(&body, "View", &target);
        let target_note = label("Changing the document or view resets the visual-state choices.");
        target_note.classes().add("designer-muted");
        body.children().add(target_note.upcast::<Control>());

        let definition_fields = StackPanel::new();
        definition_fields.set_spacing(8.0);
        let options = match &initial.target {
            PreviewStageTarget::Definition { options, .. } => options.clone(),
            PreviewStageTarget::Document => DefinitionPreviewOptions::default(),
        };
        let target_type = field(
            options.target_type.as_deref().unwrap_or(""),
            "StageTargetType",
            "Definition sample target type",
        );
        target_type.set_placeholder_text(Some("Use the definition's target type"));
        let sample_content = field(
            &options.sample_content,
            "StageSampleContent",
            "Definition sample content",
        );
        let sample_width = field(
            &options.width.to_string(),
            "StageSampleWidth",
            "Definition sample width",
        );
        let sample_height = field(
            &options.height.to_string(),
            "StageSampleHeight",
            "Definition sample height",
        );
        row(&definition_fields, "Sample type", &target_type);
        row(&definition_fields, "Sample content", &sample_content);
        row(&definition_fields, "Sample width", &sample_width);
        row(&definition_fields, "Sample height", &sample_height);
        body.children()
            .add(definition_fields.clone().upcast::<Control>());

        let layout = picker("StageLayout", "Preview layout sizing");
        for value in [
            "Fixed size",
            "Fit content",
            "Fixed width · fit height",
            "Fit width · fixed height",
        ] {
            item(&layout, value);
        }
        layout.set_selected_index(match initial.layout {
            PreviewStageLayout::Fixed => 0,
            PreviewStageLayout::SizeThatFits => 1,
            PreviewStageLayout::FixedWidth => 2,
            PreviewStageLayout::FixedHeight => 3,
        });
        row(&body, "Layout", &layout);
        let layout_note = label("");
        layout_note.set_name(Some("StageLayoutNote".into()));
        layout_note.classes().add("designer-muted");
        body.children().add(layout_note.clone().upcast::<Control>());
        let preset = picker("StageSizePreset", "Logical preview size preset");
        item(&preset, "Custom size");
        for value in PREVIEW_SIZE_PRESETS {
            item(&preset, value.name);
        }
        preset.set_selected_index(0);
        row(&body, "Size preset", &preset);
        let width = field(
            &initial.environment.width.to_string(),
            "StageWidth",
            "Stage logical width",
        );
        let height = field(
            &initial.environment.height.to_string(),
            "StageHeight",
            "Stage logical height",
        );
        let width_caption = captioned_row(&body, "Width", &width);
        let height_caption = captioned_row(&body, "Height", &height);
        width_caption.set_name(Some("StageWidthCaption".into()));
        height_caption.set_name(Some("StageHeightCaption".into()));
        let rotate = action("Swap width and height", "StageRotate");
        row(&body, "Orientation", &rotate);
        let theme = picker("StageTheme", "Stage theme");
        for value in ["Light", "Dark", "Application theme"] {
            item(&theme, value);
        }
        theme.set_selected_index(match initial.environment.theme {
            PreviewTheme::Light => 0,
            PreviewTheme::Dark => 1,
            PreviewTheme::Inherit => 2,
        });
        row(&body, "Theme", &theme);
        let flow = picker("StageDirection", "Stage reading direction");
        for value in ["Inherited", "Left to right", "Right to left"] {
            item(&flow, value);
        }
        flow.set_selected_index(match initial.environment.flow_direction {
            PreviewFlowDirection::Inherit => 0,
            PreviewFlowDirection::LeftToRight => 1,
            PreviewFlowDirection::RightToLeft => 2,
        });
        row(&body, "Direction", &flow);
        let mode = picker("StageMode", "Stage interaction mode");
        item(&mode, "Select authored controls");
        item(&mode, "Live interaction");
        mode.set_selected_index(if initial.mode == PreviewStageMode::Live {
            1
        } else {
            0
        });
        row(&body, "Interaction", &mode);
        let design_data = check(
            "Use design-time values and sample data",
            "StageDesignData",
            initial.design_time_data,
        );
        let pinned = check(
            "Keep visible across document tabs",
            "StagePinned",
            initial.pinned,
        );
        let auto_update = check(
            "Update when the source changes",
            "StageAutoUpdate",
            initial.auto_update,
        );
        for value in [&design_data, &pinned, &auto_update] {
            body.children().add(value.clone().upcast::<Control>());
        }
        let state_title = label("Visual states");
        state_title.set_font_weight(FontWeight::SemiBold);
        state_title.set_margin(Thickness::new(0.0, 8.0, 0.0, 0.0));
        body.children().add(state_title.upcast::<Control>());
        let state_host = StackPanel::new();
        state_host.set_spacing(8.0);
        body.children().add(state_host.clone().upcast::<Control>());

        let scroll = ScrollViewer::new();
        scroll.set_name(Some("StageEditorScroll".into()));
        scroll.set_max_height(430.0);
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_content(Some(Control::boxed(body.upcast::<Control>())));
        Grid::set_row(&scroll, 1);
        root.children().add(scroll.upcast::<Control>());

        let footer = StackPanel::new();
        footer.set_name(Some("StageEditorFooter".into()));
        footer.set_spacing(8.0);
        footer.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
        Grid::set_row(&footer, 2);
        let error = label("");
        error.set_name(Some("StageEditorError".into()));
        error.classes().add("designer-error");
        footer.children().add(error.clone().upcast::<Control>());
        let buttons = StackPanel::new();
        buttons.set_orientation(Orientation::Horizontal);
        buttons.set_horizontal_alignment(HorizontalAlignment::Right);
        buttons.set_spacing(8.0);
        let cancel = action("Cancel", "StageEditorCancel");
        let apply_button = action("Apply stage", "StageEditorApply");
        apply_button.classes().add("primary");
        buttons.children().add(cancel.clone().upcast::<Control>());
        buttons
            .children()
            .add(apply_button.clone().upcast::<Control>());
        footer.children().add(buttons.upcast::<Control>());
        root.children().add(footer.upcast::<Control>());

        let editor = Rc::new(Self {
            root,
            name,
            source,
            source_uris,
            sources,
            target,
            targets: RefCell::new(Vec::new()),
            definition_fields,
            target_type,
            sample_content,
            sample_width,
            sample_height,
            layout,
            width_caption,
            height_caption,
            layout_note,
            width,
            height,
            theme,
            flow,
            mode,
            design_data,
            pinned,
            auto_update,
            state_host,
            states: RefCell::new(Vec::new()),
            retained_states: RefCell::new(initial.states.clone()),
            error,
            apply_button,
            guard: Cell::new(true),
            retired: Cell::new(false),
            apply: RefCell::new(None),
            close: RefCell::new(None),
        });
        editor.rebuild_targets(Some(&initial.target));
        editor.rebuild_states(&initial.states);
        editor.refresh_layout_captions();
        editor.guard.set(false);
        let weak = Rc::downgrade(&editor);
        editor.layout.selection_changed(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                editor.refresh_layout_captions();
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.source.selection_changed(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                if !editor.guard.get() && !editor.retired.get() {
                    editor.guard.set(true);
                    editor.retained_states.borrow_mut().clear();
                    editor.rebuild_targets(None);
                    editor.rebuild_states(&[]);
                    editor.guard.set(false);
                }
            }
        });
        let weak = Rc::downgrade(&editor);
        editor.target.selection_changed(move |_, _| {
            if let Some(editor) = weak.upgrade() {
                if !editor.guard.get() && !editor.retired.get() {
                    editor.retained_states.borrow_mut().clear();
                    editor.rebuild_states(&[]);
                    editor.refresh_definition_visibility();
                }
            }
        });
        let weak = Rc::downgrade(&editor);
        let weak_preset = preset.downgrade();
        preset.selection_changed(move |_, _| {
            let (Some(editor), Some(preset)) = (weak.upgrade(), weak_preset.upgrade()) else {
                return;
            };
            if let Some(value) = preset
                .selected_index()
                .checked_sub(1)
                .and_then(|index| usize::try_from(index).ok())
                .and_then(|index| PREVIEW_SIZE_PRESETS.get(index))
            {
                editor.width.set_text(Some(&value.width.to_string()));
                editor.height.set_text(Some(&value.height.to_string()));
            }
        });
        let weak = Rc::downgrade(&editor);
        let weak_preset = preset.downgrade();
        rotate.click(move |_, event| {
            if let Some(editor) = weak.upgrade() {
                let width = editor.width.text();
                let height = editor.height.text();
                editor.width.set_text(height.as_deref());
                editor.height.set_text(width.as_deref());
                match editor.layout.selected_index() {
                    2 => editor.layout.set_selected_index(3),
                    3 => editor.layout.set_selected_index(2),
                    _ => {}
                }
                if let Some(preset) = weak_preset.upgrade() {
                    preset.set_selected_index(0);
                }
            }
            event.set_handled(true);
        });
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
        editor
    }

    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast()
    }

    fn refresh_layout_captions(&self) {
        let mode = self.layout.selected_index();
        self.width_caption.set_text(Some(if matches!(mode, 1 | 3) {
            "Maximum width"
        } else {
            "Width"
        }));
        self.height_caption.set_text(Some(if matches!(mode, 1 | 2) {
            "Maximum height"
        } else {
            "Height"
        }));
        self.layout_note.set_text(Some(if mode == 0 {
            "The preview uses this logical size. The card zoom changes only its presentation."
        } else {
            "Native layout measures the content within these finite limits. Fitted axes use its desired size; authored sizes remain in effect."
        }));
    }

    fn selected_source(&self) -> Option<&PreviewStageSource> {
        let uri = self
            .source_uris
            .get(usize::try_from(self.source.selected_index()).ok()?)?;
        self.sources.iter().find(|source| &source.uri == uri)
    }

    fn rebuild_targets(&self, initial: Option<&PreviewStageTarget>) {
        self.target.items().clear();
        let mut targets = vec![PreviewStageTarget::Document];
        item(&self.target, "Document view");
        if let Some(source) = self.selected_source() {
            if let Ok(catalog) = ComponentCatalog::from_document(&source.document) {
                for entry in catalog.entries {
                    if let Ok(anchor) = PreviewSourceAnchor::capture(&source.document, entry.node) {
                        item(
                            &self.target,
                            &format!("{} · {:?}", entry.display_name, entry.kind),
                        );
                        targets.push(PreviewStageTarget::Definition {
                            anchor,
                            options: DefinitionPreviewOptions::default(),
                        });
                    }
                }
            }
        }
        let selected = initial
            .and_then(|initial| {
                targets
                    .iter()
                    .position(|candidate| match (candidate, initial) {
                        (PreviewStageTarget::Document, PreviewStageTarget::Document) => true,
                        (
                            PreviewStageTarget::Definition { anchor, .. },
                            PreviewStageTarget::Definition { anchor: other, .. },
                        ) => anchor == other,
                        _ => false,
                    })
            })
            .unwrap_or_else(|| {
                if let Some(initial @ PreviewStageTarget::Definition { .. }) = initial {
                    item(&self.target, "Unavailable definition · choose another view");
                    targets.push(initial.clone());
                    targets.len() - 1
                } else {
                    0
                }
            });
        *self.targets.borrow_mut() = targets;
        self.target.set_selected_index(selected as i32);
        self.refresh_definition_visibility();
    }

    fn refresh_definition_visibility(&self) {
        self.definition_fields
            .set_is_visible(self.target.selected_index() > 0);
    }

    fn rebuild_states(&self, selected: &[PreviewStageState]) {
        self.state_host.children().clear();
        self.states.borrow_mut().clear();
        let Some(source) = self.selected_source() else {
            self.state_host.children().add(
                label("Open this source document to choose its authored visual states.")
                    .upcast::<Control>(),
            );
            return;
        };
        let target_node = self
            .targets
            .borrow()
            .get(self.target.selected_index().max(0) as usize)
            .and_then(|target| match target {
                PreviewStageTarget::Document => None,
                PreviewStageTarget::Definition { anchor, .. } => {
                    anchor.resolve(&source.document).ok()
                }
            });
        let mut rows = Vec::new();
        for group in VisualStateGroup::read(&source.document) {
            if target_node.is_some_and(|target| {
                !source
                    .document
                    .ast()
                    .subtree(target)
                    .any(|node| node.id == group.owner)
            }) {
                continue;
            }
            let Ok(owner) = PreviewSourceAnchor::capture(&source.document, group.owner) else {
                continue;
            };
            let owner_name = source
                .document
                .ast()
                .node(group.owner)
                .map(|node| node.xaml_name().unwrap_or(node.local_name()))
                .unwrap_or("Control");
            let control = picker("", &format!("{owner_name} · {} visual state", group.name));
            item(&control, "Authored appearance");
            let mut values = group
                .states
                .iter()
                .map(|state| state.name.clone())
                .collect::<Vec<_>>();
            for value in &values {
                item(&control, value);
            }
            let selection = selected
                .iter()
                .find(|state| state.owner == owner && state.group == group.name);
            let index = if let Some(selected) = selection {
                if let Some(index) = values.iter().position(|value| value == &selected.state) {
                    index + 1
                } else {
                    values.push(selected.state.clone());
                    item(&control, &format!("{} · unavailable", selected.state));
                    values.len()
                }
            } else {
                0
            };
            control.set_selected_index(index as i32);
            row(
                &self.state_host,
                &format!("{owner_name}\n{}", group.name),
                &control,
            );
            rows.push(StateChoice {
                owner,
                group: group.name,
                values,
                picker: control,
            });
        }
        if rows.is_empty() {
            self.state_host.children().add(
                label("This view has no selectable authored visual-state groups.")
                    .upcast::<Control>(),
            );
        }
        if selected.iter().any(|selected| {
            !rows
                .iter()
                .any(|row| row.owner == selected.owner && row.group == selected.group)
        }) {
            self.state_host.children().add(label("Some configured states are unavailable and will be preserved. Choose another view to reset them.").upcast::<Control>());
        }
        *self.states.borrow_mut() = rows;
    }

    pub fn value(&self) -> Result<PreviewStageConfiguration, String> {
        let source_uri = self
            .source_uris
            .get(
                usize::try_from(self.source.selected_index())
                    .map_err(|_| "Choose a source document")?,
            )
            .ok_or("Choose a source document")?
            .clone();
        let mut target = self
            .targets
            .borrow()
            .get(
                usize::try_from(self.target.selected_index())
                    .map_err(|_| "Choose a preview view")?,
            )
            .ok_or("Choose a preview view")?
            .clone();
        if let PreviewStageTarget::Definition { options, .. } = &mut target {
            let target_type = self
                .target_type
                .text()
                .unwrap_or_default()
                .trim()
                .to_owned();
            *options = DefinitionPreviewOptions {
                target_type: (!target_type.is_empty()).then_some(target_type),
                sample_content: self.sample_content.text().unwrap_or_default(),
                width: number(&self.sample_width, "sample width")?,
                height: number(&self.sample_height, "sample height")?,
            };
            options.validate()?;
        }
        let rows = self.states.borrow();
        let mut states = self
            .retained_states
            .borrow()
            .iter()
            .filter(|state| {
                !rows
                    .iter()
                    .any(|row| row.owner == state.owner && row.group == state.group)
            })
            .cloned()
            .collect::<Vec<_>>();
        for row in rows.iter() {
            let index = row.picker.selected_index();
            if index > 0 {
                let state = row
                    .values
                    .get(index as usize - 1)
                    .ok_or("Choose a valid visual state")?;
                states.push(PreviewStageState {
                    owner: row.owner.clone(),
                    group: row.group.clone(),
                    state: state.clone(),
                });
            }
        }
        let value = PreviewStageConfiguration {
            name: self.name.text().unwrap_or_default().trim().to_owned(),
            source_uri,
            target,
            layout: match self.layout.selected_index() {
                0 => PreviewStageLayout::Fixed,
                1 => PreviewStageLayout::SizeThatFits,
                2 => PreviewStageLayout::FixedWidth,
                3 => PreviewStageLayout::FixedHeight,
                _ => return Err("Choose a preview layout".into()),
            },
            environment: PreviewEnvironment {
                width: number(&self.width, "width")?,
                height: number(&self.height, "height")?,
                theme: match self.theme.selected_index() {
                    0 => PreviewTheme::Light,
                    1 => PreviewTheme::Dark,
                    _ => PreviewTheme::Inherit,
                },
                flow_direction: match self.flow.selected_index() {
                    1 => PreviewFlowDirection::LeftToRight,
                    2 => PreviewFlowDirection::RightToLeft,
                    _ => PreviewFlowDirection::Inherit,
                },
            },
            design_time_data: self.design_data.is_checked() == Some(true),
            states,
            pinned: self.pinned.is_checked() == Some(true),
            auto_update: self.auto_update.is_checked() == Some(true),
            mode: if self.mode.selected_index() == 1 {
                PreviewStageMode::Live
            } else {
                PreviewStageMode::Select
            },
        };
        value.validate()?;
        Ok(value)
    }

    pub fn on_apply(
        &self,
        callback: impl Fn(PreviewStageConfiguration) -> Result<(), String> + 'static,
    ) {
        *self.apply.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_close(&self, callback: impl Fn() + 'static) {
        *self.close.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn apply(&self) -> Result<(), String> {
        if self.retired.get() {
            return Err("This stage editor is closed; reopen it to edit the stage".into());
        }
        let result = self.value().and_then(|value| {
            self.apply
                .borrow()
                .clone()
                .ok_or("No stage editor host is attached")?(value)
        });
        self.error.set_text(Some(
            result.as_ref().err().map(String::as_str).unwrap_or(""),
        ));
        if result.is_ok() {
            self.cancel();
        }
        result
    }
    pub fn cancel(&self) {
        self.retire();
        if let Some(callback) = self.close.borrow().clone() {
            callback();
        }
    }
    pub fn retire(&self) {
        self.retired.set(true);
        self.apply_button.set_is_enabled(false);
    }
    pub fn show_error(&self, message: &str) {
        self.error.set_text(Some(message));
    }
}

fn number(control: &TextBox, name: &str) -> Result<f64, String> {
    control
        .text()
        .unwrap_or_default()
        .trim()
        .parse()
        .map_err(|_| format!("Enter a numeric {name}"))
}
fn label(value: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}
fn field(value: &str, name: &str, description: &str) -> Ref<TextBox> {
    let control = TextBox::new();
    control.set_name(Some(name.into()));
    control.set_text(Some(value));
    AutomationProperties::set_name(&control, Some(description));
    control
}
fn picker(name: &str, description: &str) -> Ref<ComboBox> {
    let control = ComboBox::new();
    if !name.is_empty() {
        control.set_name(Some(name.into()));
    }
    control.set_horizontal_alignment(HorizontalAlignment::Stretch);
    AutomationProperties::set_name(&control, Some(description));
    control
}
fn item(control: &ComboBox, text: &str) {
    control.items().add(Some(Rc::new(text.to_owned())));
}
fn action(text: &str, name: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_content(Some(Rc::new(text.to_owned())));
    AutomationProperties::set_name(&button, Some(text));
    button
}
fn check(text: &str, name: &str, value: bool) -> Ref<CheckBox> {
    let control = CheckBox::new();
    control.set_name(Some(name.into()));
    control.set_content(Some(Rc::new(text.to_owned())));
    control.set_is_checked(Some(value));
    AutomationProperties::set_name(&control, Some(text));
    control
}
fn row(panel: &StackPanel, text: &str, control: &Control) {
    captioned_row(panel, text, control);
}
fn captioned_row(panel: &StackPanel, text: &str, control: &Control) -> Ref<TextBlock> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("112,*").unwrap());
    row.set_column_spacing(10.0);
    let caption = label(text);
    caption.classes().add("designer-muted");
    row.children().add(caption.clone().upcast::<Control>());
    Grid::set_column(control, 1);
    row.children().add(control.to_ref());
    panel.children().add(row.upcast::<Control>());
    caption
}
fn short_uri(uri: &str) -> &str {
    uri.rsplit('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(uri)
}
