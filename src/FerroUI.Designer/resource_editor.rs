//! Reusable definition inspector. Native controls stage values locally and send
//! semantic commands to a document-owning host; this view never edits XAML itself.
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{FontWeight, SolidColorBrush, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Border, Button, ColumnDefinitions, ComboBox, Control, Grid, RowDefinitions, ScrollViewer,
    StackPanel, TextBlock, TextBox, ToolTip, WrapPanel,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_tools::ComponentKind;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[derive(Clone, Debug)]
pub struct DefinitionPreviewSettings {
    pub target_type: Option<String>,
    pub sample_content: String,
    pub width: f64,
    pub height: f64,
}
impl Default for DefinitionPreviewSettings {
    fn default() -> Self {
        Self {
            target_type: None,
            sample_content: "Preview content".into(),
            width: 420.0,
            height: 280.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DefinitionReference {
    pub node: NodeId,
    pub title: String,
    pub property: String,
}

/// A host-created typed editor, shared with the regular property inspector.
pub struct DefinitionPropertyField {
    pub name: String,
    pub editor: Ref<Control>,
    pub description: Option<String>,
}

pub struct ResourceDefinitionModel {
    pub node: NodeId,
    pub title: String,
    pub type_name: String,
    pub kind: ComponentKind,
    pub key: Option<String>,
    pub scope: String,
    pub source: String,
    pub writable: bool,
    pub can_preview: bool,
    pub preview: DefinitionPreviewSettings,
    pub apply_property: String,
    pub properties: Vec<DefinitionPropertyField>,
    pub definition_properties: Vec<DefinitionPropertyField>,
    pub references: Vec<DefinitionReference>,
    pub primitive_value: Option<String>,
    pub setter_properties: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum DefinitionEditorCommand {
    Preview(DefinitionPreviewSettings),
    Source,
    Rename(String),
    Duplicate(String),
    CopyTemplate(String),
    Remove,
    Apply {
        property: String,
        dynamic: bool,
    },
    SetSetter {
        property: String,
        value: String,
    },
    SetResourceValue(String),
    ReplaceSource(String),
    SelectReference(NodeId),
    CopyProvided {
        owner: NodeId,
        key: String,
    },
    InsertControl,
    OpenProvidedSource,
    Create {
        owner: NodeId,
        key: String,
        kind: DefinitionCreationKind,
        target_type: String,
        source: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitionCreationKind {
    Resource,
    ControlTheme,
    ControlTemplate,
    DataTemplate,
}

type Command = Rc<dyn Fn(DefinitionEditorCommand) -> Result<(), String>>;

pub struct ResourceDefinitionEditor {
    root: Ref<Grid>,
    panel: Ref<StackPanel>,
    error: Ref<TextBlock>,
    command: RefCell<Option<Command>>,
    generation: Cell<u64>,
}

struct EditorCommandTarget {
    editor: Weak<ResourceDefinitionEditor>,
    generation: u64,
}
impl EditorCommandTarget {
    fn upgrade(&self) -> Option<Rc<ResourceDefinitionEditor>> {
        self.editor
            .upgrade()
            .filter(|editor| editor.generation.get() == self.generation)
    }
}

impl ResourceDefinitionEditor {
    pub fn new() -> Rc<Self> {
        let root = Grid::new();
        root.set_row_definitions(RowDefinitions::parse("*,Auto").unwrap());
        let scroll = ScrollViewer::new();
        scroll.set_horizontal_scroll_bar_visibility(
            ferroui_controls::primitives::ScrollBarVisibility::Disabled,
        );
        let panel = StackPanel::new();
        panel.set_spacing(8.0);
        panel.set_margin(Thickness::new(12.0, 12.0, 12.0, 14.0));
        scroll.set_content(Some(Control::boxed(&panel)));
        root.children().add(scroll.upcast::<Control>());
        let error = TextBlock::new();
        error.set_name(Some("DefinitionEditorError".into()));
        error.set_font_size(11.0);
        error.set_foreground(Some(SolidColorBrush::from_uint32(0xfff48b95).into()));
        error.set_text_wrapping(TextWrapping::Wrap);
        error.set_margin(Thickness::new(12.0, 4.0, 12.0, 10.0));
        error.set_is_visible(false);
        AutomationProperties::set_name(&error, Some("Definition editing status"));
        Grid::set_row(&error, 1);
        root.children().add(error.clone().upcast::<Control>());
        Rc::new(Self {
            root,
            panel,
            error,
            command: RefCell::new(None),
            generation: Cell::new(0),
        })
    }
    fn command_target(self: &Rc<Self>) -> EditorCommandTarget {
        EditorCommandTarget {
            editor: Rc::downgrade(self),
            generation: self.generation.get(),
        }
    }
    pub fn control(&self) -> Ref<Control> {
        self.root.clone().upcast::<Control>()
    }
    pub fn on_command(
        &self,
        callback: impl Fn(DefinitionEditorCommand) -> Result<(), String> + 'static,
    ) {
        *self.command.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn show_error(&self, error: Option<&str>) {
        self.error.set_text(error);
        self.error
            .set_is_visible(error.is_some_and(|error| !error.is_empty()));
    }
    fn invoke(&self, command: DefinitionEditorCommand) {
        let generation = self.generation.get();
        let callback = self.command.borrow().clone();
        let result = callback.map_or_else(
            || Err("The definition editor has no document host".into()),
            |callback| callback(command),
        );
        // A successful navigation or edit may synchronously configure a different
        // definition. A retired command must not overwrite its new editor's status.
        if self.generation.get() == generation {
            self.show_error(result.as_ref().err().map(String::as_str));
        }
    }

    pub fn configure(self: &Rc<Self>, model: ResourceDefinitionModel) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.panel.children().clear();
        self.show_error(None);
        let category = label(kind_title(model.kind), 10.0, 0xffb6a2f6);
        category.set_font_weight(FontWeight::SemiBold);
        self.panel.children().add(category.upcast::<Control>());
        let title = label(&model.title, 18.0, 0xffe8eaf0);
        title.set_font_weight(FontWeight::SemiBold);
        title.set_text_wrapping(TextWrapping::Wrap);
        self.panel.children().add(title.upcast::<Control>());
        self.panel.children().add(
            description(&format!("{} · {}", model.type_name, model.scope)).upcast::<Control>(),
        );

        if !model.writable {
            self.panel.children().add(
                description("Read-only document. Preview and navigation remain available.")
                    .upcast::<Control>(),
            );
        }
        if !model.definition_properties.is_empty() {
            section(&self.panel, "Definition");
            for property in model.definition_properties {
                property_row(&self.panel, property, model.writable);
            }
        }
        if !model.properties.is_empty() {
            section(
                &self.panel,
                if matches!(
                    model.kind,
                    ComponentKind::ControlTheme | ComponentKind::Style
                ) {
                    "Setters"
                } else {
                    "Properties"
                },
            );
            for property in model.properties {
                property_row(&self.panel, property, model.writable);
            }
        }

        if let Some(value) = model.primitive_value {
            section(&self.panel, "Resource value");
            let input = field(&self.panel, "DefinitionPrimitiveValue", "Value", &value);
            input.set_is_enabled(model.writable);
            let save = button("Apply value", "DefinitionSetResourceValue", model.writable);
            let weak = self.command_target();
            let input = input.downgrade();
            save.click(move |_, _| {
                if let (Some(this), Some(input)) = (weak.upgrade(), input.upgrade()) {
                    this.invoke(DefinitionEditorCommand::SetResourceValue(
                        input.text().unwrap_or_default(),
                    ));
                }
            });
            self.panel.children().add(save.upcast::<Control>());
        }

        if matches!(
            model.kind,
            ComponentKind::ControlTheme | ComponentKind::Style
        ) {
            section(&self.panel, "Add a setter");
            let names = model
                .setter_properties
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let property = choice_field(
                &self.panel,
                "DefinitionSetterProperty",
                "Property",
                &names,
                -1,
            );
            property.set_is_editable(true);
            property.set_placeholder_text(Some("Choose or enter a property".into()));
            let value = field(&self.panel, "DefinitionSetterValue", "Value", "");
            value.set_placeholder_text(Some("Literal or resource expression"));
            let add = button("Add / update setter", "DefinitionAddSetter", model.writable);
            let weak = self.command_target();
            add.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.invoke(DefinitionEditorCommand::SetSetter {
                        property: property.text().unwrap_or_default().trim().to_owned(),
                        value: value.text().unwrap_or_default(),
                    });
                }
            });
            self.panel.children().add(add.upcast::<Control>());
        }

        if let Some(key) = &model.key {
            section(&self.panel, "Resource identity");
            let key_field = field(&self.panel, "DefinitionResourceKey", "Key", key);
            key_field.set_is_enabled(model.writable);
            let row = WrapPanel::new();
            let weak = self.command_target();
            let input = key_field.downgrade();
            command_button(
                &row,
                "Rename",
                "DefinitionRename",
                model.writable,
                move || {
                    if let (Some(this), Some(input)) = (weak.upgrade(), input.upgrade()) {
                        this.invoke(DefinitionEditorCommand::Rename(
                            input.text().unwrap_or_default(),
                        ));
                    }
                },
            );
            let weak = self.command_target();
            let duplicate_key = format!("{key}Copy");
            command_button(
                &row,
                "Duplicate",
                "DefinitionDuplicate",
                model.writable,
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.invoke(DefinitionEditorCommand::Duplicate(duplicate_key.clone()));
                    }
                },
            );
            if matches!(
                model.kind,
                ComponentKind::ControlTemplate | ComponentKind::DataTemplate
            ) {
                let weak = self.command_target();
                let key = format!("{key}Copy");
                command_button(
                    &row,
                    "Edit a copy",
                    "DefinitionEditCopy",
                    model.writable,
                    move || {
                        if let Some(this) = weak.upgrade() {
                            this.invoke(DefinitionEditorCommand::CopyTemplate(key.clone()));
                        }
                    },
                );
            }
            self.panel.children().add(row.upcast::<Control>());

            section(&self.panel, "Apply to selection");
            let property = field(
                &self.panel,
                "DefinitionApplyProperty",
                "Property",
                &model.apply_property,
            );
            property.set_placeholder_text(Some("Theme, Template, Background…"));
            let mode = choice_field(
                &self.panel,
                "DefinitionReferenceMode",
                "Reference",
                &["Static resource", "Dynamic resource"],
                0,
            );
            let weak = self.command_target();
            let property = property.downgrade();
            let mode = mode.downgrade();
            let apply = button("Apply resource", "DefinitionApply", model.writable);
            apply.classes().add("primary");
            apply.click(move |_, _| {
                if let (Some(this), Some(property), Some(mode)) =
                    (weak.upgrade(), property.upgrade(), mode.upgrade())
                {
                    this.invoke(DefinitionEditorCommand::Apply {
                        property: property.text().unwrap_or_default().trim().to_owned(),
                        dynamic: mode.selected_index() == 1,
                    });
                }
            });
            self.panel.children().add(apply.upcast::<Control>());
        }

        if model.can_preview {
            section(&self.panel, "Canvas preview");
            let target = field(
                &self.panel,
                "DefinitionPreviewTarget",
                "Target",
                model.preview.target_type.as_deref().unwrap_or(""),
            );
            target.set_placeholder_text(Some("Infer from definition"));
            let sample = field(
                &self.panel,
                "DefinitionPreviewContent",
                "Content",
                &model.preview.sample_content,
            );
            sample.set_placeholder_text(Some("Sample content"));
            let dimensions = Grid::new();
            dimensions.set_column_definitions(ColumnDefinitions::parse("*,8,*").unwrap());
            let width = TextBox::new();
            width.set_name(Some("DefinitionPreviewWidth".into()));
            width.set_text(Some(&model.preview.width.to_string()));
            width.set_placeholder_text(Some("Width"));
            AutomationProperties::set_name(&width, Some("Preview width"));
            let height = TextBox::new();
            height.set_name(Some("DefinitionPreviewHeight".into()));
            height.set_text(Some(&model.preview.height.to_string()));
            height.set_placeholder_text(Some("Height"));
            AutomationProperties::set_name(&height, Some("Preview height"));
            dimensions.children().add(width.clone().upcast::<Control>());
            Grid::set_column(&height, 2);
            dimensions
                .children()
                .add(height.clone().upcast::<Control>());
            self.panel.children().add(dimensions.upcast::<Control>());
            let edit = button("Edit on canvas", "DefinitionEditCanvas", true);
            edit.classes().add("primary");
            let weak = self.command_target();
            edit.click(move |_, _| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                let size = width
                    .text()
                    .unwrap_or_default()
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .zip(height.text().unwrap_or_default().trim().parse::<f64>().ok());
                let Some((width, height)) = size.filter(|(width, height)| {
                    width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0
                }) else {
                    this.show_error(Some("Preview width and height must be positive numbers"));
                    return;
                };
                this.invoke(DefinitionEditorCommand::Preview(
                    DefinitionPreviewSettings {
                        target_type: target
                            .text()
                            .filter(|value| !value.trim().is_empty())
                            .map(|value| value.trim().to_owned()),
                        sample_content: sample.text().unwrap_or_default(),
                        width,
                        height,
                    },
                ));
            });
            self.panel.children().add(edit.upcast::<Control>());
        }

        section(
            &self.panel,
            &format!("Used by · {}", model.references.len()),
        );
        if model.references.is_empty() {
            self.panel
                .children()
                .add(description("No references in this document.").upcast::<Control>());
        }
        for reference in model.references {
            let target = button(
                &format!("{} · {}", reference.title, reference.property),
                "DefinitionReference",
                true,
            );
            target.classes().add("nav");
            target.set_horizontal_content_alignment(HorizontalAlignment::Left);
            let weak = self.command_target();
            target.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.invoke(DefinitionEditorCommand::SelectReference(reference.node));
                }
            });
            self.panel.children().add(target.upcast::<Control>());
        }

        section(&self.panel, "Definition source");
        let source = source_field(&self.panel, "DefinitionSourceDraft", &model.source);
        source.set_is_read_only(!model.writable);
        let actions = WrapPanel::new();
        let weak = self.command_target();
        let source = source.downgrade();
        command_button(
            &actions,
            "Apply XAML",
            "DefinitionApplySource",
            model.writable,
            move || {
                if let (Some(this), Some(source)) = (weak.upgrade(), source.upgrade()) {
                    this.invoke(DefinitionEditorCommand::ReplaceSource(
                        source.text().unwrap_or_default(),
                    ));
                }
            },
        );
        let weak = self.command_target();
        command_button(
            &actions,
            "Show source",
            "DefinitionShowSource",
            true,
            move || {
                if let Some(this) = weak.upgrade() {
                    this.invoke(DefinitionEditorCommand::Source);
                }
            },
        );
        if model.key.is_some() {
            let weak = self.command_target();
            command_button(
                &actions,
                "Delete",
                "DefinitionDelete",
                model.writable,
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.invoke(DefinitionEditorCommand::Remove);
                    }
                },
            );
        }
        self.panel.children().add(actions.upcast::<Control>());
    }

    pub fn configure_create(self: &Rc<Self>, owners: Vec<(NodeId, String)>, writable: bool) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.panel.children().clear();
        self.show_error(None);
        let title = label("New definition", 18.0, 0xffe8eaf0);
        title.set_font_weight(FontWeight::SemiBold);
        self.panel.children().add(title.upcast::<Control>());
        self.panel.children().add(
            description("Choose its scope and key. Creation is one undoable document edit.")
                .upcast::<Control>(),
        );
        let names = owners
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>();
        let scope = choice_field(&self.panel, "NewDefinitionScope", "Scope", &names, 0);
        let kind = choice_field(
            &self.panel,
            "NewDefinitionKind",
            "Kind",
            &[
                "Resource",
                "Control theme",
                "Control template",
                "Data template",
            ],
            0,
        );
        let key = field(&self.panel, "NewDefinitionKey", "Key", "NewBrush");
        let target = field(&self.panel, "NewDefinitionTarget", "Target", "Button");
        target.set_placeholder_text(Some("Control type"));
        let help = description(
            "Resource: provide one XAML object. Templates: provide the visual content. Themes: add setters after creation.",
        );
        self.panel.children().add(help.upcast::<Control>());
        let source = source_field(
            &self.panel,
            "NewDefinitionSource",
            "<SolidColorBrush Color=\"#FF79A8FF\" />",
        );
        let weak_source = source.downgrade();
        let weak_key = key.downgrade();
        let weak_kind = kind.downgrade();
        kind.selection_changed(move |_, _| {
            if let (Some(source), Some(key), Some(kind)) = (weak_source.upgrade(), weak_key.upgrade(), weak_kind.upgrade()) {
                let defaults = ["<SolidColorBrush Color=\"#FF79A8FF\" />", "", "<Border Background=\"{TemplateBinding Background}\" Padding=\"{TemplateBinding Padding}\"><ContentPresenter Content=\"{TemplateBinding Content}\" /></Border>", "<Border Padding=\"12\"><TextBlock Text=\"{Binding}\" /></Border>"];
                let names = ["NewBrush", "NewTheme", "NewTemplate", "NewDataTemplate"];
                let index = kind.selected_index().max(0) as usize;
                if defaults.contains(&source.text().unwrap_or_default().as_str()) { source.set_text(defaults.get(index).copied()); }
                if names.contains(&key.text().unwrap_or_default().as_str()) { key.set_text(names.get(index).copied()); }
                source.set_is_visible(index != 1);
            }
        });
        let create = button(
            "Create definition",
            "CreateDefinition",
            writable && !owners.is_empty(),
        );
        create.classes().add("primary");
        let weak = self.command_target();
        create.click(move |_, _| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let Some((owner, _)) = owners.get(scope.selected_index().max(0) as usize) else {
                this.show_error(Some("Choose a resource scope"));
                return;
            };
            let kind = match kind.selected_index() {
                1 => DefinitionCreationKind::ControlTheme,
                2 => DefinitionCreationKind::ControlTemplate,
                3 => DefinitionCreationKind::DataTemplate,
                _ => DefinitionCreationKind::Resource,
            };
            this.invoke(DefinitionEditorCommand::Create {
                owner: *owner,
                key: key.text().unwrap_or_default().trim().to_owned(),
                kind,
                target_type: target.text().unwrap_or_default().trim().to_owned(),
                source: source.text().unwrap_or_default(),
            });
        });
        self.panel.children().add(create.upcast::<Control>());
    }

    /// Registered library sources are immutable. Copying creates an authored resource
    /// in the active document; the provider's original source is never rewritten.
    pub fn configure_external(
        self: &Rc<Self>,
        title: &str,
        kind: ComponentKind,
        origin: Option<&str>,
        source: &str,
        owners: Vec<(NodeId, String)>,
        writable: bool,
    ) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.panel.children().clear();
        self.show_error(None);
        self.panel
            .children()
            .add(label(kind_title(kind), 10.0, 0xffb6a2f6).upcast::<Control>());
        let heading = label(title, 18.0, 0xffe8eaf0);
        heading.set_font_weight(FontWeight::SemiBold);
        heading.set_text_wrapping(TextWrapping::Wrap);
        self.panel.children().add(heading.upcast::<Control>());
        self.panel
            .children()
            .add(description(origin.unwrap_or("Registered component library")).upcast::<Control>());
        if matches!(
            kind,
            ComponentKind::UserControl | ComponentKind::CustomControl
        ) {
            self.panel.children().add(description("Place an instance on the canvas. Registered templates and themes appear separately in the library for source editing.").upcast::<Control>());
            let insert = button("Place on canvas", "DefinitionInsertControl", writable);
            insert.classes().add("primary");
            let weak = self.command_target();
            insert.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.invoke(DefinitionEditorCommand::InsertControl);
                }
            });
            self.panel.children().add(insert.upcast::<Control>());
            let open = button(
                "Open definition document",
                "DefinitionOpenProvidedSource",
                origin.is_some(),
            );
            let weak = self.command_target();
            open.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.invoke(DefinitionEditorCommand::OpenProvidedSource);
                }
            });
            self.panel.children().add(open.upcast::<Control>());
            section(&self.panel, "Instance source");
            source_field(&self.panel, "ProvidedDefinitionSource", source).set_is_read_only(true);
            return;
        }
        self.panel.children().add(description("Create an editable copy in this document to customize the definition and its canvas. Required local resources receive independent keys; application resources keep their references.").upcast::<Control>());
        section(&self.panel, "Copy into document");
        let names = owners
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>();
        let scope = choice_field(&self.panel, "ProvidedDefinitionScope", "Scope", &names, 0);
        let suggested = title
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect::<String>();
        let key = field(
            &self.panel,
            "ProvidedDefinitionKey",
            "Key",
            &format!(
                "{}Custom",
                if suggested.is_empty() {
                    "Definition"
                } else {
                    &suggested
                }
            ),
        );
        let copy = button(
            "Create editable copy",
            "DefinitionCopyProvided",
            writable && !owners.is_empty(),
        );
        copy.classes().add("primary");
        let weak = self.command_target();
        copy.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                if let Some((owner, _)) = owners.get(scope.selected_index().max(0) as usize) {
                    this.invoke(DefinitionEditorCommand::CopyProvided {
                        owner: *owner,
                        key: key.text().unwrap_or_default().trim().to_owned(),
                    });
                }
            }
        });
        self.panel.children().add(copy.upcast::<Control>());
        section(&self.panel, "Library source");
        source_field(&self.panel, "ProvidedDefinitionSource", source).set_is_read_only(true);
    }
}

pub(crate) fn kind_title(kind: ComponentKind) -> &'static str {
    match kind {
        ComponentKind::UserControl => "USER CONTROL",
        ComponentKind::CustomControl => "CUSTOM CONTROL",
        ComponentKind::ControlTemplate => "CONTROL TEMPLATE",
        ComponentKind::DataTemplate => "DATA TEMPLATE",
        ComponentKind::ControlTheme => "CONTROL THEME",
        ComponentKind::Style => "STYLE",
        ComponentKind::Resource => "RESOURCE",
        ComponentKind::ResourceDictionary => "RESOURCE DICTIONARY",
    }
}
fn label(text: &str, size: f64, color: u32) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(text));
    label.set_font_size(size);
    label.set_foreground(Some(SolidColorBrush::from_uint32(color).into()));
    label
}
fn property_row(panel: &StackPanel, property: DefinitionPropertyField, writable: bool) {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("92,*").unwrap());
    let title = label(&property.name, 11.0, 0xffa9adba);
    title.set_vertical_alignment(VerticalAlignment::Center);
    title.set_text_wrapping(TextWrapping::Wrap);
    title.set_margin(Thickness::new(0.0, 0.0, 7.0, 0.0));
    row.children().add(title.upcast::<Control>());
    // The editor factory may already have disabled a read-only or unsupported
    // property. Document permissions can restrict it further, never re-enable it.
    if !writable {
        property.editor.set_is_enabled(false);
    }
    Grid::set_column(&property.editor, 1);
    row.children().add(property.editor);
    if let Some(description) = property.description {
        ToolTip::set_tip(&row, Some(Rc::new(description)));
    }
    panel.children().add(row.upcast::<Control>());
}
fn description(text: &str) -> Ref<TextBlock> {
    let label = label(text, 11.0, 0xffa9adba);
    label.set_text_wrapping(TextWrapping::Wrap);
    label
}
fn section(panel: &StackPanel, title: &str) {
    let border = Border::new();
    border.set_border_brush(Some(SolidColorBrush::from_uint32(0xff3a3d46).into()));
    border.set_border_thickness(Thickness::new(0.0, 1.0, 0.0, 0.0));
    border.set_margin(Thickness::new(0.0, 7.0, 0.0, 0.0));
    border.set_padding(Thickness::new(0.0, 10.0, 0.0, 1.0));
    let text = label(title, 11.0, 0xffe8eaf0);
    text.set_font_weight(FontWeight::SemiBold);
    border.set_child(Some(text.upcast::<Control>()));
    panel.children().add(border.upcast::<Control>());
}
fn field(panel: &StackPanel, name: &str, title: &str, value: &str) -> Ref<TextBox> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("72,*").unwrap());
    let text = label(title, 11.0, 0xffa9adba);
    text.set_vertical_alignment(VerticalAlignment::Center);
    row.children().add(text.upcast::<Control>());
    let input = TextBox::new();
    input.set_name(Some(name.into()));
    input.set_text(Some(value));
    input.set_min_width(0.0);
    AutomationProperties::set_name(&input, Some(title));
    Grid::set_column(&input, 1);
    row.children().add(input.clone().upcast::<Control>());
    panel.children().add(row.upcast::<Control>());
    input
}
fn choice_field(
    panel: &StackPanel,
    name: &str,
    title: &str,
    choices: &[&str],
    index: i32,
) -> Ref<ComboBox> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("72,*").unwrap());
    let text = label(title, 11.0, 0xffa9adba);
    text.set_vertical_alignment(VerticalAlignment::Center);
    row.children().add(text.upcast::<Control>());
    let picker = ComboBox::new();
    picker.set_name(Some(name.into()));
    picker.set_horizontal_alignment(HorizontalAlignment::Stretch);
    for choice in choices {
        picker.items().add(Some(Rc::new((*choice).to_owned())));
    }
    picker.set_selected_index(index);
    AutomationProperties::set_name(&picker, Some(title));
    Grid::set_column(&picker, 1);
    row.children().add(picker.clone().upcast::<Control>());
    panel.children().add(row.upcast::<Control>());
    picker
}
fn button(title: &str, name: &str, enabled: bool) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.into()));
    button.set_content(Some(Rc::new(title.to_owned())));
    button.set_is_enabled(enabled);
    button.set_horizontal_alignment(HorizontalAlignment::Stretch);
    button.set_horizontal_content_alignment(HorizontalAlignment::Center);
    AutomationProperties::set_name(&button, Some(title));
    button
}
fn command_button(
    panel: &WrapPanel,
    title: &str,
    name: &str,
    enabled: bool,
    action: impl Fn() + 'static,
) {
    let button = button(title, name, enabled);
    button.set_margin(Thickness::new(0.0, 0.0, 4.0, 4.0));
    button.click(move |_, _| action());
    panel.children().add(button.upcast::<Control>());
}
fn source_field(panel: &StackPanel, name: &str, value: &str) -> Ref<TextBox> {
    let input = TextBox::new();
    input.set_name(Some(name.into()));
    input.set_text(Some(value));
    input.set_font_family(ferroui_base::media::FontFamily::from("monospace"));
    input.set_font_size(11.0);
    input.set_accepts_return(true);
    input.set_accepts_tab(true);
    input.set_text_wrapping(TextWrapping::Wrap);
    input.set_min_height(108.0);
    input.set_max_height(220.0);
    AutomationProperties::set_name(&input, Some("Definition XAML draft"));
    let original = value.to_owned();
    let weak = input.downgrade();
    input.add_handler(InputElement::key_down_event(), move |_, event| {
        if event.key == Key::Escape {
            if let Some(input) = weak.upgrade() {
                input.set_text(Some(&original));
                event.set_handled(true);
            }
        }
    });
    panel.children().add(input.clone().upcast::<Control>());
    input
}

#[cfg(test)]
#[path = "resource_editor_tests.rs"]
mod tests;
