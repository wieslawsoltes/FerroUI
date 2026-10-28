//! Deferred, staged value editor shared by inspector rows and definition editors.
use super::{
    PropertyEditorContext, PropertyEditorKind, PropertyEditorMetadata, PropertyObjectDraft,
    PropertyValueEdit,
};
use ferroui_base::input::{InputElement, Key};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::{SolidColorBrush, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, CheckBox, ColumnDefinitions, ComboBox, Control, Flyout, Grid, ScrollViewer, StackPanel,
    TextBlock, TextBox, ToolTip,
};
use ferroui_designer_core::{MarkupArgument, MarkupExtension, MarkupValue, NodeId};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) fn attach(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
    inline: Ref<Control>,
    factories: &[Rc<dyn super::PropertyEditorFactory>],
) -> Ref<Control> {
    let root = Grid::new();
    root.set_column_definitions(ColumnDefinitions::parse("*,22").unwrap());
    root.children().add(inline);
    let button = icon_action(
        crate::icons::MORE,
        "Edit value, binding, resource or object",
    );
    button.classes().add("property-value-actions");
    button.set_min_width(0.0);
    button.set_width(21.0);
    button.set_padding(Thickness::uniform(1.0));
    let flyout = Flyout::new();
    flyout.flyout_presenter_classes().add("designer-chrome");
    button.set_flyout(Some(flyout.clone().upcast()));
    let weak = flyout.downgrade();
    let context = context.clone();
    let metadata = metadata.clone();
    let factories = factories.to_vec();
    // Large object editors are allocated only when requested. Reopening starts from the
    // immutable row snapshot, so light dismissal and Escape cannot leak a draft.
    flyout.opening(move |_| {
        if let Some(flyout) = weak.upgrade() {
            let content = build_with_factories(&context, &metadata, &flyout, &factories);
            flyout.set_content(Some(Control::boxed(&content)));
        }
    });
    Grid::set_column(&button, 1);
    root.children().add(button.upcast::<Control>());
    root.upcast::<Control>()
}

fn action(label: &str, help: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_content(Some(Rc::new(label.to_owned())));
    button.set_min_height(26.0);
    AutomationProperties::set_name(&button, Some(help));
    ToolTip::set_tip(&button, Some(Rc::new(help.to_owned())));
    button
}
fn icon_action(path: &str, help: &str) -> Ref<Button> {
    let button = action("", help);
    button.classes().add("icon");
    button.set_width(24.0);
    button.set_content(Some(Control::boxed(crate::icons::vector_icon(path, 14.0))));
    button
}
fn label(value: &str) -> Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(value));
    text.set_font_size(11.0);
    text
}
fn text(value: Option<&str>, help: &str) -> Ref<TextBox> {
    let field = TextBox::new();
    field.classes().add("designer-property-field");
    field.set_text(value);
    field.set_min_height(28.0);
    field.set_font_size(12.0);
    AutomationProperties::set_name(&field, Some(help));
    field
}
fn choices(values: &[&str], help: &str) -> Ref<ComboBox> {
    let field = ComboBox::new();
    field.classes().add("designer-property-field");
    field.set_min_width(0.0);
    for value in values {
        field.items().add(Some(Rc::new((*value).to_owned())));
    }
    field.set_selected_index(0);
    AutomationProperties::set_name(&field, Some(help));
    field
}
fn row(body: &StackPanel, name: &str, field: Ref<Control>) {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("85,*").unwrap());
    let title = label(name);
    title.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    row.children().add(title.upcast::<Control>());
    Grid::set_column(&field, 1);
    row.children().add(field);
    body.children().add(row.upcast::<Control>());
}

#[cfg(test)]
fn build(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
    flyout: &Ref<Flyout>,
) -> Ref<Control> {
    build_with_factories(context, metadata, flyout, &[])
}

fn build_with_factories(
    context: &PropertyEditorContext,
    metadata: &PropertyEditorMetadata,
    flyout: &Ref<Flyout>,
    factories: &[Rc<dyn super::PropertyEditorFactory>],
) -> Ref<Control> {
    let current_resource =
        resource_reference(context.value.as_deref(), &metadata.binding_extension);
    let body = StackPanel::new();
    body.classes().add("designer-property-workbench");
    body.classes().add("designer-chrome");
    body.set_width(410.0);
    body.set_spacing(8.0);
    let title = label(&format!("{} · value editor", context.property));
    title.set_font_size(13.0);
    body.children().add(title.upcast::<Control>());
    let tabs = choices(
        &[
            "Value",
            "Binding / expression",
            "Resource",
            "Object / collection",
        ],
        "Value source",
    );
    tabs.set_name(Some("PropertyValueSourceMode".into()));
    tabs.set_selected_index(if metadata.object_source.is_some() {
        3
    } else if current_resource.is_some() {
        2
    } else if context
        .value
        .as_deref()
        .is_some_and(super::metadata::is_expression)
    {
        1
    } else {
        0
    });
    body.children().add(tabs.clone().upcast::<Control>());
    let error = label("");
    error.set_text_wrapping(TextWrapping::Wrap);
    error.set_foreground(Some(SolidColorBrush::from_uint32(0xffffaaa0).into()));
    let value_body = StackPanel::new();
    value_body.set_spacing(6.0);
    let value = text(context.value.as_deref(), "Literal value");
    value.set_name(Some("PropertyLiteralValue".into()));
    row(&value_body, "Value", value.clone().upcast::<Control>());
    if let PropertyEditorKind::Enumeration {
        values,
        flags: true,
    } = &metadata.kind
    {
        let original = context
            .value
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        let flags = values
            .iter()
            .map(|name| {
                let field = CheckBox::new();
                field.set_content(Some(Rc::new(name.clone())));
                field.set_is_checked(Some(
                    original
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(name)),
                ));
                value_body.children().add(field.clone().upcast::<Control>());
                (field, name.clone())
            })
            .collect::<Vec<_>>();
        for (field, _) in &flags {
            let weak = value.downgrade();
            let fields = flags
                .iter()
                .map(|(field, name)| (field.downgrade(), name.clone()))
                .collect::<Vec<_>>();
            field.is_checked_changed(move |_, _| {
                if let Some(value) = weak.upgrade() {
                    value.set_text(Some(
                        &fields
                            .iter()
                            .filter(|(field, _)| {
                                field
                                    .upgrade()
                                    .is_some_and(|field| field.is_checked() == Some(true))
                            })
                            .map(|(_, name)| name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ));
                }
            });
        }
    }
    let binding_body = StackPanel::new();
    binding_body.set_spacing(6.0);
    let expression = text(
        context
            .value
            .as_deref()
            .filter(|value| super::metadata::is_expression(value)),
        "Binding or markup expression",
    );
    expression.set_name(Some("PropertyExpression".into()));
    expression.set_accepts_return(true);
    expression.set_min_height(58.0);
    row(
        &binding_body,
        "Expression",
        expression.clone().upcast::<Control>(),
    );
    let existing = context
        .value
        .as_deref()
        .and_then(|source| MarkupValue::parse(source).ok())
        .and_then(|value| match value {
            MarkupValue::Extension(extension) if extension.name == metadata.binding_extension => {
                Some(extension)
            }
            _ => None,
        });
    let argument = |name: &str| {
        existing
            .as_ref()
            .and_then(|binding| {
                binding.arguments.iter().find(|argument| {
                    argument.name.as_deref() == Some(name)
                        || name == "Path" && argument.name.is_none()
                })
            })
            .and_then(|argument| match &argument.value {
                MarkupValue::Literal(value) => Some(value.as_str()),
                _ => None,
            })
    };
    let path = text(argument("Path"), "Binding property path");
    row(&binding_body, "Path", path.clone().upcast::<Control>());
    let source = choices(
        &std::iter::once("DataContext")
            .chain(metadata.named_elements.iter().map(String::as_str))
            .collect::<Vec<_>>(),
        "Binding source element",
    );
    if let Some(name) = argument("ElementName") {
        if let Some(index) = metadata
            .named_elements
            .iter()
            .position(|value| value == name)
        {
            source.set_selected_index(index as i32 + 1);
        }
    }
    row(&binding_body, "Source", source.clone().upcast::<Control>());
    let modes = ["Default", "OneWay", "TwoWay", "OneTime", "OneWayToSource"];
    let mode = choices(&modes, "Binding mode");
    mode.set_selected_index(
        argument("Mode")
            .and_then(|mode| modes.iter().position(|value| *value == mode))
            .unwrap_or(0) as i32,
    );
    row(&binding_body, "Mode", mode.clone().upcast::<Control>());
    let build_binding = action(
        "Update binding expression",
        "Update the expression from source, path and mode; retain other binding arguments",
    );
    binding_body
        .children()
        .add(build_binding.clone().upcast::<Control>());
    let weak_expression = expression.downgrade();
    let weak_path = path.downgrade();
    let weak_source = source.downgrade();
    let weak_mode = mode.downgrade();
    let names = metadata.named_elements.clone();
    let binding_extension = metadata.binding_extension.clone();
    build_binding.click(move |_, _| {
        let Some((expression, path)) = weak_expression.upgrade().zip(weak_path.upgrade()) else {
            return;
        };
        let Some((source, mode)) = weak_source.upgrade().zip(weak_mode.upgrade()) else {
            return;
        };
        let existing = expression
            .text()
            .and_then(|value| MarkupValue::parse(&value).ok())
            .and_then(|value| match value {
                MarkupValue::Extension(value) if value.name == binding_extension => Some(value),
                _ => None,
            })
            .unwrap_or_else(|| MarkupExtension {
                name: binding_extension.clone(),
                arguments: Vec::new(),
            });
        let source = source
            .selected_index()
            .checked_sub(1)
            .and_then(|index| names.get(index as usize))
            .map(String::as_str);
        expression.set_text(Some(&binding_expression(
            Some(&existing),
            &path.text().unwrap_or_default(),
            source,
            modes
                .get(mode.selected_index() as usize)
                .copied()
                .unwrap_or("Default"),
        )));
    });
    let resource_body = StackPanel::new();
    resource_body.set_spacing(6.0);
    let resources = choices(
        &metadata
            .resources
            .iter()
            .map(|resource| resource.label.as_str())
            .collect::<Vec<_>>(),
        "Visible resources",
    );
    resources.set_name(Some("PropertyResourceChoice".into()));
    if let Some((key, _)) = &current_resource {
        resources.set_selected_index(
            metadata
                .resources
                .iter()
                .position(|resource| &resource.key == key)
                .map_or(-1, |index| index as i32),
        );
    }
    row(
        &resource_body,
        "Resource",
        resources.clone().upcast::<Control>(),
    );
    let resource_mode = choices(
        &["StaticResource", "DynamicResource"],
        "Resource lookup mode",
    );
    resource_mode.set_name(Some("PropertyResourceLookup".into()));
    resource_mode.set_selected_index(i32::from(
        current_resource
            .as_ref()
            .is_some_and(|(_, dynamic)| *dynamic),
    ));
    row(
        &resource_body,
        "Lookup",
        resource_mode.clone().upcast::<Control>(),
    );
    let hint = label(if metadata.resources.is_empty() {
        "No string-key resources are visible in this scope."
    } else {
        "Only resources visible to this selection are listed. The exact definition is checked again when applying."
    });
    hint.set_text_wrapping(TextWrapping::Wrap);
    resource_body.children().add(hint.upcast::<Control>());
    if !metadata.resource_owners.is_empty() {
        let extraction_hint = label(
            "Create a resource from the current authored value. Unapplied value drafts are not included.",
        );
        extraction_hint.set_text_wrapping(TextWrapping::Wrap);
        resource_body
            .children()
            .add(extraction_hint.upcast::<Control>());
        let key = text(None, "New resource key");
        key.set_name(Some("PropertyResourceKey".into()));
        key.set_placeholder_text(Some("Resource key"));
        row(&resource_body, "Key", key.clone().upcast::<Control>());
        let owners = choices(
            &metadata
                .resource_owners
                .iter()
                .map(|(_, label)| label.as_str())
                .collect::<Vec<_>>(),
            "New resource owner scope",
        );
        owners.set_name(Some("PropertyResourceOwner".into()));
        row(&resource_body, "Scope", owners.clone().upcast::<Control>());
        let extract = action(
            "Create resource",
            "Extract the accepted authored value into a shared resource",
        );
        let weak_key = key.downgrade();
        let weak_owners = owners.downgrade();
        let weak_error = error.downgrade();
        let weak_flyout = flyout.downgrade();
        let metadata = metadata.clone();
        let context = context.clone();
        extract.click(move |_, _| {
            let result = weak_key
                .upgrade()
                .zip(weak_owners.upgrade())
                .ok_or_else(|| "The resource editor was closed".to_owned())
                .and_then(|(key, owners)| {
                    let key = key.text().unwrap_or_default();
                    if key.trim().is_empty() {
                        return Err("Enter a resource key".into());
                    }
                    let owner = metadata
                        .resource_owners
                        .get(owners.selected_index() as usize)
                        .ok_or_else(|| "Choose a resource scope".to_owned())?
                        .0;
                    metadata.apply(&context, PropertyValueEdit::ExtractResource { owner, key })
                });
            match result {
                Ok(()) => {
                    if let Some(flyout) = weak_flyout.upgrade() {
                        flyout.hide();
                    }
                }
                Err(message) => {
                    if let Some(error) = weak_error.upgrade() {
                        error.set_text(Some(&message));
                    }
                }
            }
        });
        resource_body.children().add(extract.upcast::<Control>());
    }
    let (object_body, object_source) = object_editor(metadata, &error, factories);
    let pages = [
        value_body.upcast::<Control>(),
        binding_body.upcast::<Control>(),
        resource_body.upcast::<Control>(),
        object_body,
    ];
    for (index, page) in pages.iter().enumerate() {
        page.set_is_visible(index as i32 == tabs.selected_index());
        body.children().add(page.clone());
    }
    let weak = tabs.downgrade();
    let pages = pages.iter().map(Ref::downgrade).collect::<Vec<_>>();
    tabs.selection_changed(move |_, _| {
        if let Some(tabs) = weak.upgrade() {
            for (index, page) in pages.iter().enumerate() {
                if let Some(page) = page.upgrade() {
                    page.set_is_visible(index as i32 == tabs.selected_index());
                }
            }
        }
    });
    body.children().add(error.clone().upcast::<Control>());
    let actions = StackPanel::new();
    actions.set_orientation(ferroui_base::layout::Orientation::Horizontal);
    actions.set_spacing(8.0);
    actions.set_horizontal_alignment(HorizontalAlignment::Right);
    let cancel = action("Cancel", "Discard the draft");
    let apply = action(
        "Apply value",
        "Apply the complete value in one undo transaction",
    );
    apply.classes().add("primary");
    actions.children().add(cancel.clone().upcast::<Control>());
    actions.children().add(apply.clone().upcast::<Control>());
    body.children().add(actions.upcast::<Control>());
    let weak_flyout = flyout.downgrade();
    cancel.click(move |_, _| {
        if let Some(flyout) = weak_flyout.upgrade() {
            flyout.hide();
        }
    });
    let weak_flyout = flyout.downgrade();
    let weak_error = error.downgrade();
    let weak_tabs = tabs.downgrade();
    let weak_value = value.downgrade();
    let weak_expression = expression.downgrade();
    let weak_resource = resources.downgrade();
    let weak_mode = resource_mode.downgrade();
    let weak_object = object_source.downgrade();
    let context = context.clone();
    let metadata = metadata.clone();
    apply.click(move |_, _| {
        let Some(tabs) = weak_tabs.upgrade() else {
            return;
        };
        let edit = match tabs.selected_index() {
            1 => weak_expression
                .upgrade()
                .map(|field| PropertyValueEdit::Expression(field.text().unwrap_or_default()))
                .ok_or_else(|| "Expression editor was closed".into()),
            2 => weak_resource
                .upgrade()
                .and_then(|field| metadata.resources.get(field.selected_index() as usize))
                .map(|resource| PropertyValueEdit::Resource {
                    node: resource.node,
                    dynamic: weak_mode
                        .upgrade()
                        .is_some_and(|mode| mode.selected_index() == 1),
                })
                .ok_or_else(|| "Choose a visible resource".into()),
            3 => weak_object
                .upgrade()
                .ok_or_else(|| "Object editor was closed".to_owned())
                .and_then(|field| {
                    let value = field.text().unwrap_or_default();
                    PropertyObjectDraft::new(&value)?;
                    Ok(PropertyValueEdit::Object(value))
                }),
            _ => weak_value
                .upgrade()
                .map(|field| PropertyValueEdit::Literal(field.text().unwrap_or_default()))
                .ok_or_else(|| "Value editor was closed".into()),
        };
        match edit.and_then(|edit| metadata.apply(&context, edit)) {
            Ok(()) => {
                if let Some(flyout) = weak_flyout.upgrade() {
                    flyout.hide();
                }
            }
            Err(message) => {
                if let Some(error) = weak_error.upgrade() {
                    error.set_text(Some(&message));
                }
            }
        }
    });
    let weak = flyout.downgrade();
    body.add_handler(InputElement::key_down_event(), move |_, event| {
        if event.key == Key::Escape {
            if let Some(flyout) = weak.upgrade() {
                flyout.hide();
            }
            event.set_handled(true);
        }
    });
    let scroll = ScrollViewer::new();
    scroll.set_max_height(650.0);
    scroll.set_content(Some(Control::boxed(&body)));
    scroll.upcast::<Control>()
}

fn resource_reference(source: Option<&str>, binding_extension: &str) -> Option<(String, bool)> {
    let MarkupValue::Extension(extension) = MarkupValue::parse(source?).ok()? else {
        return None;
    };
    let (prefix, local) = extension
        .name
        .rsplit_once(':')
        .unwrap_or(("", extension.name.as_str()));
    let framework_prefix = binding_extension
        .rsplit_once(':')
        .map_or("", |(prefix, _)| prefix);
    if prefix != framework_prefix || !matches!(local, "StaticResource" | "DynamicResource") {
        return None;
    }
    let value = extension.arguments.iter().find(|argument| {
        argument.name.is_none() || argument.name.as_deref() == Some("ResourceKey")
    })?;
    let MarkupValue::Literal(key) = &value.value else {
        return None;
    };
    Some((key.clone(), local == "DynamicResource"))
}

/// Build a binding without interpolating path/key strings into markup syntax. Existing
/// converter, fallback, validation and extension arguments survive structured changes.
pub fn binding_expression(
    existing: Option<&MarkupExtension>,
    path: &str,
    element: Option<&str>,
    mode: &str,
) -> String {
    let mut arguments = existing
        .map(|value| value.arguments.clone())
        .unwrap_or_default();
    arguments.retain(|argument| {
        argument.name.as_deref().is_some_and(|name| {
            !matches!(
                name,
                "Path" | "ElementName" | "Source" | "RelativeSource" | "Mode"
            )
        })
    });
    arguments.insert(
        0,
        MarkupArgument {
            name: Some("Path".into()),
            value: MarkupValue::Literal(path.into()),
        },
    );
    if let Some(element) = element {
        arguments.push(MarkupArgument {
            name: Some("ElementName".into()),
            value: MarkupValue::Literal(element.into()),
        });
    }
    if mode != "Default" {
        arguments.push(MarkupArgument {
            name: Some("Mode".into()),
            value: MarkupValue::Literal(mode.into()),
        });
    }
    expression_source(&MarkupValue::Extension(MarkupExtension {
        name: existing.map_or_else(|| "Binding".into(), |extension| extension.name.clone()),
        arguments,
    }))
}
fn expression_source(value: &MarkupValue) -> String {
    match value {
        MarkupValue::Literal(value) => {
            format!(
                "'{}{}'",
                if value.starts_with('{') { "{}" } else { "" },
                value.replace('\\', "\\\\").replace('\'', "\\'")
            )
        }
        MarkupValue::Extension(extension) => format!(
            "{{{}{}}}",
            extension.name,
            if extension.arguments.is_empty() {
                String::new()
            } else {
                format!(
                    " {}",
                    extension
                        .arguments
                        .iter()
                        .map(|argument| format!(
                            "{}{}",
                            argument
                                .name
                                .as_ref()
                                .map(|name| format!("{name}="))
                                .unwrap_or_default(),
                            expression_source(&argument.value)
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        ),
    }
}

fn object_editor(
    metadata: &PropertyEditorMetadata,
    error: &Ref<TextBlock>,
    factories: &[Rc<dyn super::PropertyEditorFactory>],
) -> (Ref<Control>, Ref<TextBox>) {
    use super::object_draft::{OBJECT_PRESETS, object_attributes};
    let body = StackPanel::new();
    body.set_spacing(6.0);
    let source = text(metadata.object_source.as_deref(), "Object XAML draft");
    source.set_name(Some("PropertyObjectSource".into()));
    source.set_accepts_return(true);
    source.set_min_height(100.0);
    source.set_max_height(190.0);
    let presets = choices(
        &OBJECT_PRESETS
            .iter()
            .map(|(label, _)| *label)
            .collect::<Vec<_>>(),
        "New object type",
    );
    if matches!(metadata.kind, PropertyEditorKind::Transform) {
        presets.set_selected_index(4);
    }
    let create = action(
        "Create value",
        "Explicitly replace the local draft with a new object",
    );
    let creation = Grid::new();
    creation.set_column_definitions(ColumnDefinitions::parse("*,112").unwrap());
    creation.children().add(presets.clone().upcast::<Control>());
    Grid::set_column(&create, 1);
    creation.children().add(create.clone().upcast::<Control>());
    body.children().add(creation.upcast::<Control>());
    let nodes = choices(&[], "Objects and collection items");
    nodes.set_name(Some("PropertyObjectNodes".into()));
    body.children().add(nodes.clone().upcast::<Control>());
    let fields = StackPanel::new();
    fields.set_spacing(4.0);
    body.children().add(fields.clone().upcast::<Control>());
    let tools = StackPanel::new();
    tools.set_orientation(ferroui_base::layout::Orientation::Horizontal);
    tools.set_spacing(4.0);
    let add = action(
        "Add child",
        "Add the selected type to the selected object or collection",
    );
    let remove = action("Remove", "Remove the selected object from this draft");
    let up = icon_action(crate::icons::MOVE_UP, "Move object earlier");
    let down = icon_action(crate::icons::MOVE_DOWN, "Move object later");
    for button in [&add, &remove, &up, &down] {
        tools.children().add(button.clone().upcast::<Control>());
    }
    body.children().add(tools.upcast::<Control>());
    let custom = Grid::new();
    custom.set_column_definitions(ColumnDefinitions::parse("*,*,70").unwrap());
    let custom_name = text(None, "Custom object property name");
    custom_name.set_name(Some("PropertyObjectAttributeName".into()));
    custom_name.set_placeholder_text(Some("Property name"));
    let custom_value = text(None, "Custom object property value");
    custom_value.set_name(Some("PropertyObjectAttributeValue".into()));
    custom_value.set_placeholder_text(Some("Value"));
    let custom_add = action("Set", "Set a custom property on the selected draft object");
    custom
        .children()
        .add(custom_name.clone().upcast::<Control>());
    Grid::set_column(&custom_value, 1);
    custom
        .children()
        .add(custom_value.clone().upcast::<Control>());
    Grid::set_column(&custom_add, 2);
    custom
        .children()
        .add(custom_add.clone().upcast::<Control>());
    body.children().add(custom.upcast::<Control>());
    body.children()
        .add(label("XAML · source and structured fields stay synchronized").upcast::<Control>());
    body.children().add(source.clone().upcast::<Control>());
    let ids = Rc::new(RefCell::new(Vec::<NodeId>::new()));
    let weak_nodes = nodes.downgrade();
    let weak_source = source.downgrade();
    let ids_refresh = ids.clone();
    let refresh = Rc::new(move || {
        let Some((nodes, source)) = weak_nodes.upgrade().zip(weak_source.upgrade()) else {
            return;
        };
        let selected = nodes.selected_index();
        let draft = PropertyObjectDraft::new(&source.text().unwrap_or_default());
        let (new_ids, labels) = draft
            .as_ref()
            .map(|draft| {
                let ids = draft.nodes();
                let labels = ids
                    .iter()
                    .map(|id| {
                        let node = draft.document().ast().node(*id).unwrap();
                        let detail = node
                            .attribute("Color")
                            .or_else(|| node.attribute("Angle"))
                            .or_else(|| node.attribute("Matrix"))
                            .unwrap_or("");
                        format!("{} {}", node.name, detail)
                    })
                    .collect::<Vec<_>>();
                (ids, labels)
            })
            .unwrap_or_default();
        *ids_refresh.borrow_mut() = new_ids;
        nodes.items().clear();
        for label in labels {
            nodes.items().add(Some(Rc::new(label)));
        }
        nodes.set_selected_index(if ids_refresh.borrow().is_empty() {
            -1
        } else {
            selected.max(0).min(ids_refresh.borrow().len() as i32 - 1)
        });
    });
    let weak_fields = fields.downgrade();
    let weak_nodes = nodes.downgrade();
    let weak_source = source.downgrade();
    let ids_fields = ids.clone();
    let factories = factories.to_vec();
    nodes.selection_changed(move |_, _| {
        let Some((fields, nodes)) = weak_fields.upgrade().zip(weak_nodes.upgrade()) else {
            return;
        };
        fields.children().clear();
        let Some(source) = weak_source.upgrade() else {
            return;
        };
        let Some(id) = ids_fields
            .borrow()
            .get(nodes.selected_index() as usize)
            .copied()
        else {
            return;
        };
        let Ok(draft) = PropertyObjectDraft::new(&source.text().unwrap_or_default()) else {
            return;
        };
        let Some(node) = draft.document().ast().node(id) else {
            return;
        };
        let mut attributes = super::metadata::object_property_types(node);
        let known = if node.namespace_uri.as_deref() == Some("https://github.com/ferroui") {
            object_attributes(node.local_name())
        } else {
            &[]
        };
        for (name, type_) in known {
            if !attributes.iter().any(|property| property.name == *name) {
                attributes.push(super::metadata::ObjectPropertyType {
                    name: (*name).into(),
                    runtime_type: (*type_).into(),
                    property: None,
                });
            }
        }
        for attribute in &node.attributes {
            if !attribute.is_namespace()
                && !attributes.iter().any(|property| {
                    property.name == attribute.name
                        || ferroui_designer_tools::component_property_attribute(
                            draft.document(),
                            id,
                            &property.name,
                        )
                        .ok()
                        .flatten()
                        .is_some_and(|known| known.name == attribute.name)
                })
            {
                attributes.push(super::metadata::ObjectPropertyType {
                    name: attribute.name.clone(),
                    runtime_type: "String".into(),
                    property: None,
                });
            }
        }
        for property_type in attributes {
            let weak = source.downgrade();
            let expected_source = source.text().unwrap_or_default();
            let property = property_type.name.clone();
            let mut context = PropertyEditorContext {
                property: property.clone(),
                targets: vec![],
                value: ferroui_designer_tools::component_property_attribute(
                    draft.document(),
                    id,
                    &property,
                )
                .ok()
                .flatten()
                .map(|attribute| attribute.value.clone()),
                mixed: false,
                runtime_type: Some(property_type.runtime_type),
                apply: Rc::new(|_| {}),
            };
            let mut metadata =
                super::resolve_property_editor_metadata(&context, property_type.property);
            let namespaces = draft.document().ast().namespaces(id);
            let prefix = context
                .value
                .as_deref()
                .and_then(|value| MarkupValue::parse(value).ok())
                .and_then(|value| match value {
                    MarkupValue::Extension(extension) => {
                        let (prefix, local) = extension
                            .name
                            .rsplit_once(':')
                            .unwrap_or(("", extension.name.as_str()));
                        (matches!(local, "Binding" | "StaticResource" | "DynamicResource")
                            && namespaces
                                .get(prefix)
                                .is_some_and(|uri| uri == "https://github.com/ferroui"))
                        .then(|| prefix.to_owned())
                    }
                    _ => None,
                })
                .or_else(|| {
                    namespaces
                        .iter()
                        .find(|(_, uri)| uri.as_str() == "https://github.com/ferroui")
                        .map(|(prefix, _)| prefix.clone())
                });
            if let Some(prefix) = prefix {
                metadata.binding_extension = if prefix.is_empty() {
                    "Binding".into()
                } else {
                    format!("{prefix}:Binding")
                };
            }
            metadata.object_source =
                ferroui_designer_tools::component_property_object(draft.document(), id, &property)
                    .ok()
                    .flatten();
            if metadata.object_source.is_some() {
                metadata.source = super::PropertyValueSource::Object;
            }
            let commit: super::PropertyValueCommit = Rc::new(move |edit| {
                let source = weak.upgrade().ok_or("The object editor was closed")?;
                if source.text().unwrap_or_default() != expected_source {
                    return Err("The object draft changed; use its refreshed fields".into());
                }
                let mut draft = PropertyObjectDraft::new(&source.text().unwrap_or_default())?;
                match edit {
                    PropertyValueEdit::Literal(value) | PropertyValueEdit::Expression(value) => {
                        draft.set_attribute(id, &property, Some(&value))?
                    }
                    PropertyValueEdit::Reset => draft.set_attribute(id, &property, None)?,
                    PropertyValueEdit::Object(value) => {
                        draft.set_object(id, &property, Some(&value))?
                    }
                    PropertyValueEdit::Resource { .. }
                    | PropertyValueEdit::ExtractResource { .. } => {
                        return Err(
                            "Use an explicit resource expression inside this local object draft"
                                .into(),
                        );
                    }
                }
                source.set_text(Some(&draft.source()));
                Ok(())
            });
            metadata.commit = Some(commit.clone());
            context.apply = Rc::new(move |value| {
                let edit = match value {
                    Some(value) if super::metadata::is_expression(&value) => {
                        PropertyValueEdit::Expression(value)
                    }
                    Some(value) => PropertyValueEdit::Literal(value),
                    None => PropertyValueEdit::Reset,
                };
                let _ = commit(edit);
            });
            // Factories receive the same immutable metadata and a draft-bound commit.
            // Their changes remain local until the outer property's Apply command.
            let editor = super::create_property_editor(&context, &metadata, &factories);
            row(&fields, &property_type.name, editor);
        }
    });
    let update = refresh.clone();
    source.text_changing(move |_, _| update());
    let weak = source.downgrade();
    let weak_presets = presets.downgrade();
    create.click(move |_, _| {
        if let Some((source, presets)) = weak.upgrade().zip(weak_presets.upgrade()) {
            if let Some((_, preset)) = OBJECT_PRESETS.get(presets.selected_index() as usize) {
                source.set_text(Some(preset));
            }
        }
    });
    for (button, operation) in [
        (&add, 0),
        (&remove, 1),
        (&up, 2),
        (&down, 3),
        (&custom_add, 4),
    ] {
        let weak = source.downgrade();
        let weak_nodes = nodes.downgrade();
        let weak_presets = presets.downgrade();
        let weak_error = error.downgrade();
        let ids = ids.clone();
        let weak_name = custom_name.downgrade();
        let weak_value = custom_value.downgrade();
        button.click(move |_, _| {
            let Some((source, nodes)) = weak.upgrade().zip(weak_nodes.upgrade()) else {
                return;
            };
            let selected = ids.borrow().get(nodes.selected_index() as usize).copied();
            let result = PropertyObjectDraft::new(&source.text().unwrap_or_default()).and_then(
                |mut draft| {
                    match operation {
                        0 => {
                            let preset = weak_presets
                                .upgrade()
                                .and_then(|field| {
                                    OBJECT_PRESETS.get(field.selected_index() as usize)
                                })
                                .ok_or("Choose an object type")?;
                            draft.insert(selected, preset.1)?;
                        }
                        1 => draft.remove(selected.ok_or("Choose an object")?)?,
                        2 | 3 => draft.move_by(
                            selected.ok_or("Choose an object")?,
                            if operation == 2 { -1 } else { 1 },
                        )?,
                        _ => {
                            let name = weak_name
                                .upgrade()
                                .and_then(|field| field.text())
                                .unwrap_or_default();
                            if name.is_empty() {
                                return Err("Enter a property name".into());
                            }
                            let value = weak_value
                                .upgrade()
                                .and_then(|field| field.text())
                                .unwrap_or_default();
                            draft.set_attribute(
                                selected.ok_or("Choose an object")?,
                                &name,
                                Some(&value),
                            )?;
                        }
                    }
                    Ok(draft.source())
                },
            );
            match result {
                Ok(value) => {
                    source.set_text(Some(&value));
                    if let Some(error) = weak_error.upgrade() {
                        error.set_text(Some(""));
                    }
                }
                Err(message) => {
                    if let Some(error) = weak_error.upgrade() {
                        error.set_text(Some(&message));
                    }
                }
            }
        });
    }
    refresh();
    (body.upcast::<Control>(), source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};

    fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
        root.get_logical_descendants()
            .find(|control| control.name().as_deref() == Some(name))
            .and_then(|control| control.cast::<T>())
            .unwrap_or_else(|| panic!("Missing authored editor control {name}"))
    }

    fn click(root: &Control, title: &str) {
        let button = root
            .get_logical_descendants()
            .filter_map(|control| control.cast::<Button>())
            .find(|button| {
                button
                    .content()
                    .as_ref()
                    .and_then(|value| value.downcast_ref::<String>())
                    .is_some_and(|value| value == title)
            })
            .unwrap();
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    }

    #[test]
    fn object_workbench_stages_attributes_and_applies_the_complete_graph_once() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let context = PropertyEditorContext {
            property: "Background".into(),
            targets: vec![1],
            value: None,
            mixed: false,
            runtime_type: None,
            apply: Rc::new(|_| {}),
        };
        let seen = Rc::new(RefCell::new(Vec::new()));
        let captured = seen.clone();
        let mut metadata = PropertyEditorMetadata::new(PropertyEditorKind::Brush);
        metadata.object_source = Some("<LinearGradientBrush><!--Keep--><GradientStop Color='Red' Offset='0'/><GradientStop Color='Blue' Offset='1'/></LinearGradientBrush>".into());
        metadata.commit = Some(Rc::new(move |edit| {
            captured.borrow_mut().push(edit);
            Ok(())
        }));
        let body = build(&context, &metadata, &Flyout::new());
        named::<ComboBox>(&body, "PropertyObjectNodes").set_selected_index(1);
        named::<TextBox>(&body, "PropertyObjectAttributeName").set_text(Some("Color"));
        named::<TextBox>(&body, "PropertyObjectAttributeValue").set_text(Some("Green"));
        click(&body, "Set");
        assert!(
            seen.borrow().is_empty(),
            "Structured edits only change the local draft"
        );
        let source = named::<TextBox>(&body, "PropertyObjectSource")
            .text()
            .unwrap();
        assert!(source.contains("Color='Green'"));
        assert!(source.contains("<!--Keep-->"));
        click(&body, "Apply value");
        assert_eq!(*seen.borrow(), vec![PropertyValueEdit::Object(source)]);
    }

    #[test]
    fn workbench_preserves_expression_until_explicit_binding_update_and_reports_rejection() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let context = PropertyEditorContext {
            property: "Text".into(),
            targets: vec![1],
            value: Some("{app:CustomSource Token='a,b'}".into()),
            mixed: false,
            runtime_type: Some("String".into()),
            apply: Rc::new(|_| {}),
        };
        let mut metadata = super::super::resolve_property_editor_metadata(&context, None);
        metadata.commit = Some(Rc::new(|_| Err("The document changed".into())));
        let body = build(&context, &metadata, &Flyout::new());
        assert_eq!(
            named::<TextBox>(&body, "PropertyExpression").text(),
            context.value
        );
        click(&body, "Apply value");
        assert!(
            body.get_logical_descendants()
                .filter_map(|control| control.cast::<TextBlock>())
                .any(|text| text.text().as_deref() == Some("The document changed"))
        );
        assert_eq!(
            named::<TextBox>(&body, "PropertyExpression").text(),
            context.value
        );
    }
    #[test]
    fn binding_fields_quote_paths_and_preserve_converter_arguments() {
        let MarkupValue::Extension(existing) = MarkupValue::parse(
            "{Binding Old, Converter={StaticResource Converter}, FallbackValue='a,b'}",
        )
        .unwrap() else {
            panic!()
        };
        let source = binding_expression(
            Some(&existing),
            "Items['a,b'].Value",
            Some("Source"),
            "TwoWay",
        );
        let MarkupValue::Extension(binding) = MarkupValue::parse(&source).unwrap() else {
            panic!()
        };
        assert_eq!(
            binding
                .arguments
                .iter()
                .find(|arg| arg.name.as_deref() == Some("Path"))
                .unwrap()
                .value,
            MarkupValue::Literal("Items['a,b'].Value".into())
        );
        assert!(
            binding
                .arguments
                .iter()
                .any(|arg| arg.name.as_deref() == Some("Converter"))
        );
        assert!(
            binding
                .arguments
                .iter()
                .any(|arg| arg.name.as_deref() == Some("FallbackValue"))
        );
    }

    #[test]
    fn binding_serialization_retains_leading_braces_as_literal_arguments() {
        let existing = MarkupExtension {
            name: "f:Binding".into(),
            arguments: vec![MarkupArgument {
                name: Some("FallbackValue".into()),
                value: MarkupValue::Literal("{not an extension}, \\ and 'quote'".into()),
            }],
        };
        let source = binding_expression(Some(&existing), "{literal path}", None, "Default");
        let MarkupValue::Extension(parsed) = MarkupValue::parse(&source).unwrap() else {
            panic!()
        };
        assert_eq!(
            parsed.arguments[0].value,
            MarkupValue::Literal("{literal path}".into())
        );
        assert_eq!(parsed.arguments[1], existing.arguments[0]);
    }

    #[test]
    fn resource_extraction_uses_explicit_key_and_owner_and_keeps_value_drafts_local() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let context = PropertyEditorContext {
            property: "Background".into(),
            targets: vec![1],
            value: Some("Red".into()),
            mixed: false,
            runtime_type: None,
            apply: Rc::new(|_| {}),
        };
        let seen = Rc::new(RefCell::new(Vec::new()));
        let captured = seen.clone();
        let mut metadata = PropertyEditorMetadata::new(PropertyEditorKind::Brush);
        metadata.resource_owners = vec![
            (2, "Window resources".into()),
            (3, "Local resources".into()),
        ];
        metadata.commit = Some(Rc::new(move |edit| {
            captured.borrow_mut().push(edit);
            Ok(())
        }));
        let body = build(&context, &metadata, &Flyout::new());
        named::<ComboBox>(&body, "PropertyValueSourceMode").set_selected_index(2);
        named::<TextBox>(&body, "PropertyLiteralValue").set_text(Some("Blue"));
        click(&body, "Create resource");
        assert!(
            seen.borrow().is_empty(),
            "Empty resource keys cannot be committed"
        );
        named::<TextBox>(&body, "PropertyResourceKey").set_text(Some("Accent"));
        named::<ComboBox>(&body, "PropertyResourceOwner").set_selected_index(1);
        assert!(seen.borrow().is_empty());
        click(&body, "Create resource");
        assert_eq!(
            *seen.borrow(),
            vec![PropertyValueEdit::ExtractResource {
                owner: 3,
                key: "Accent".into()
            }]
        );
    }

    #[test]
    fn application_binding_named_extension_is_preserved_until_explicit_replacement() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let context = PropertyEditorContext {
            property: "Text".into(),
            targets: vec![1],
            value: Some("{app:Binding Token='keep', Path='custom'}".into()),
            mixed: false,
            runtime_type: Some("String".into()),
            apply: Rc::new(|_| {}),
        };
        let metadata = PropertyEditorMetadata::new(PropertyEditorKind::Text);
        let body = build(&context, &metadata, &Flyout::new());
        assert_eq!(
            named::<TextBox>(&body, "PropertyExpression").text(),
            context.value
        );
        click(&body, "Update binding expression");
        let MarkupValue::Extension(binding) = MarkupValue::parse(
            &named::<TextBox>(&body, "PropertyExpression")
                .text()
                .unwrap(),
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(binding.name, "Binding");
        assert!(
            !binding
                .arguments
                .iter()
                .any(|argument| argument.name.as_deref() == Some("Token"))
        );
    }

    #[test]
    fn binding_source_changes_preserve_alias_and_remove_conflicting_source_arguments() {
        let MarkupValue::Extension(existing) = MarkupValue::parse(
            "{f:Binding Old, Source={f:StaticResource Model}, RelativeSource={f:RelativeSource Self}, Converter={f:StaticResource Converter}}"
        ).unwrap() else { panic!() };
        let source = binding_expression(Some(&existing), "Value", Some("Owner"), "OneWay");
        let MarkupValue::Extension(binding) = MarkupValue::parse(&source).unwrap() else {
            panic!()
        };
        assert_eq!(binding.name, "f:Binding");
        assert!(
            !binding.arguments.iter().any(|argument| matches!(
                argument.name.as_deref(),
                Some("Source" | "RelativeSource")
            ))
        );
        assert!(
            binding
                .arguments
                .iter()
                .any(|argument| argument.name.as_deref() == Some("ElementName"))
        );
        assert!(
            binding
                .arguments
                .iter()
                .any(|argument| argument.name.as_deref() == Some("Converter"))
        );
    }

    #[test]
    fn registered_factories_edit_reflected_nested_properties_inside_the_local_draft() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        ferroui_base::register_types();
        struct StopEditor;
        impl super::super::PropertyEditorFactory for StopEditor {
            fn create_with_metadata(
                &self,
                context: &PropertyEditorContext,
                metadata: &PropertyEditorMetadata,
            ) -> Option<Ref<Control>> {
                if context.property != "Offset" {
                    return None;
                }
                assert_eq!(context.runtime_type.as_deref(), Some("f64"));
                let button = Button::new();
                button.set_name(Some("CustomStopOffset".into()));
                button.set_content(Some(Rc::new("Apply custom offset".to_owned())));
                let context = context.clone();
                let metadata = metadata.clone();
                button.click(move |_, _| {
                    metadata
                        .apply(&context, PropertyValueEdit::Literal("0.25".into()))
                        .unwrap();
                });
                Some(button.upcast::<Control>())
            }
        }
        let context = PropertyEditorContext {
            property: "Background".into(),
            targets: vec![1],
            value: None,
            mixed: false,
            runtime_type: None,
            apply: Rc::new(|_| {}),
        };
        let seen = Rc::new(RefCell::new(Vec::new()));
        let captured = seen.clone();
        let mut metadata = PropertyEditorMetadata::new(PropertyEditorKind::Brush);
        metadata.object_source = Some("<LinearGradientBrush><!-- keep --><GradientStop Color='Red' Offset='0'/><GradientStop Color='Blue' Offset='1'/></LinearGradientBrush>".into());
        metadata.commit = Some(Rc::new(move |edit| {
            captured.borrow_mut().push(edit);
            Ok(())
        }));
        let body =
            build_with_factories(&context, &metadata, &Flyout::new(), &[Rc::new(StopEditor)]);
        named::<ComboBox>(&body, "PropertyObjectNodes").set_selected_index(1);
        named::<Button>(&body, "CustomStopOffset")
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(seen.borrow().is_empty());
        let source = named::<TextBox>(&body, "PropertyObjectSource")
            .text()
            .unwrap();
        assert!(source.contains("Offset='0.25'"));
        assert!(source.contains("<!-- keep -->"));
        click(&body, "Apply value");
        assert_eq!(*seen.borrow(), vec![PropertyValueEdit::Object(source)]);
    }

    #[test]
    fn nested_typed_fields_resolve_owner_qualified_alias_without_duplicate_properties() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        ferroui_base::register_types();
        struct ColorField(Rc<RefCell<Vec<String>>>);
        impl super::super::PropertyEditorFactory for ColorField {
            fn create_with_metadata(
                &self,
                context: &PropertyEditorContext,
                metadata: &PropertyEditorMetadata,
            ) -> Option<Ref<Control>> {
                self.0.borrow_mut().push(context.property.clone());
                if context.property != "Color" {
                    return None;
                }
                assert!(matches!(context.value.as_deref(), Some("Red" | "Blue")));
                let context = context.clone();
                let metadata = metadata.clone();
                let button = action("Set color through alias", "Change the draft color");
                button.click(move |_, _| {
                    metadata
                        .apply(&context, PropertyValueEdit::Literal("Blue".into()))
                        .unwrap();
                });
                Some(button.upcast::<Control>())
            }
        }
        let context = PropertyEditorContext {
            property: "Background".into(),
            targets: vec![1],
            value: None,
            mixed: false,
            runtime_type: None,
            apply: Rc::new(|_| {}),
        };
        let observed = Rc::new(RefCell::new(Vec::new()));
        let mut metadata = PropertyEditorMetadata::new(PropertyEditorKind::Brush);
        metadata.object_source = Some("<SolidColorBrush xmlns:ui='https://github.com/ferroui' ui:SolidColorBrush.Color='Red'/>".into());
        let body = build_with_factories(
            &context,
            &metadata,
            &Flyout::new(),
            &[Rc::new(ColorField(observed.clone()))],
        );
        assert_eq!(
            observed
                .borrow()
                .iter()
                .filter(|name| name.as_str() == "Color")
                .count(),
            1
        );
        assert!(
            !observed
                .borrow()
                .iter()
                .any(|name| name == "ui:SolidColorBrush.Color")
        );
        click(&body, "Set color through alias");
        let source = named::<TextBox>(&body, "PropertyObjectSource")
            .text()
            .unwrap();
        assert!(source.contains("ui:SolidColorBrush.Color='Blue'"));
        assert!(!source.contains(" Color="));
    }

    #[test]
    fn resource_workbench_starts_from_current_definition_and_lookup_mode() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let context = PropertyEditorContext {
            property: "Background".into(),
            targets: vec![1],
            value: Some("{f:DynamicResource 'Brand Accent'}".into()),
            mixed: false,
            runtime_type: None,
            apply: Rc::new(|_| {}),
        };
        let mut metadata = PropertyEditorMetadata::new(PropertyEditorKind::Brush);
        metadata.binding_extension = "f:Binding".into();
        metadata.resources = vec![
            super::super::PropertyResourceChoice {
                node: 2,
                key: "Other".into(),
                label: "Other".into(),
            },
            super::super::PropertyResourceChoice {
                node: 3,
                key: "Brand Accent".into(),
                label: "Brand Accent".into(),
            },
        ];
        let body = build(&context, &metadata, &Flyout::new());
        assert_eq!(
            named::<ComboBox>(&body, "PropertyValueSourceMode").selected_index(),
            2
        );
        assert_eq!(
            named::<ComboBox>(&body, "PropertyResourceChoice").selected_index(),
            1
        );
        assert_eq!(
            named::<ComboBox>(&body, "PropertyResourceLookup").selected_index(),
            1
        );
    }
}
