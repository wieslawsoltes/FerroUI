//! Shared declarations, validation and exact source locations for the Styles & Themes dock.
use super::{decode, node_id, schema as s, state, text};
use crate::property_literals::setter_target_type;
use crate::style_composition::{
    compatible_base, setter_metadata, setter_property, supports_palette, supports_styles,
};
use crate::style_sources::StyleSource;
use crate::{DesignerSession, PropertyEditorKind, ScopedPalette};
use ferroui_designer_core::{Document, MarkupValue, NodeId, NodeKind};
use ferroui_designer_language::LineIndex;
use ferroui_designer_tools::component_styles::{authored_setters, base_theme, StyleChange};
use ferroui_designer_tools::{
    component_property_attribute, component_property_elements, ComponentKind,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_style_catalog"
            | "designer_style_definition"
            | "designer_style_resources"
            | "designer_style_edit"
            | "designer_open_style_source"
            | "designer_style_source"
            | "designer_palette"
            | "designer_palette_edit"
    )
}

pub(super) fn tools() -> Vec<Value> {
    vec![
        s::tool(
            "designer_style_catalog",
            "List local XAML style and palette scopes and declarations in the accepted include graph.",
            s::object(json!({"target":s::id()}), &[]),
            false,
        ),
        s::tool(
            "designer_style_definition",
            "Read one authored Style or ControlTheme, including known typed setters, available base themes, and precise source locations.",
            s::object(json!({"node":s::id()}), &["node"]),
            false,
        ),
        s::tool(
            "designer_open_style_source",
            "Open an included Style, ControlTheme or palette in its owning document. Pass the source from the style catalog or palette comparison and the consuming document revision. Opening changes no source.",
            s::object(json!({"source": source_schema(), "expected_revision": s::revision()}), &["source", "expected_revision"]),
            true,
        ),
        s::tool(
            "designer_style_source",
            "Locate an included style or palette in the current owning document. Pass the original source locator. Returns a local definition or palette scope, layer, and exact source range.",
            s::object(json!({"source": source_schema()}), &["source"]),
            false,
        ),
        s::tool(
            "designer_style_resources",
            "List compatible XAML resources visible from this style setter in the accepted native theme.",
            s::object(
                json!({"owner":s::id(),"property":s::string()}),
                &["owner", "property"],
            ),
            false,
        ),
        s::actions(
            "designer_style_edit",
            "Create an XAML Style, edit a selector or compatible base theme, edit or reset a setter, assign a scoped resource, or remove an unkeyed Style. Each accepted edit is one source transaction.",
            vec![
                s::action(
                    "create",
                    json!({"owner":s::id(),"selector":s::string()}),
                    &["owner", "selector"],
                    true,
                ),
                s::action(
                    "selector",
                    json!({"node":s::id(),"selector":s::string()}),
                    &["node", "selector"],
                    true,
                ),
                s::action(
                    "base",
                    json!({"node":s::id(),"base":s::nullable(s::id())}),
                    &["node", "base"],
                    true,
                ),
                s::action(
                    "setter",
                    json!({"owner":s::id(),"property":s::string(),"value":s::nullable(s::string())}),
                    &["owner", "property", "value"],
                    true,
                ),
                s::action(
                    "resource",
                    json!({"owner":s::id(),"property":s::string(),"resource":s::id(),"dynamic":{"type":"boolean"}}),
                    &["owner", "property", "resource", "dynamic"],
                    true,
                ),
                s::action("remove", json!({"node":s::id()}), &["node"], true),
            ],
            true,
        ),
        s::tool(
            "designer_palette",
            "Read the authored palette layers in a scope and compare how each key resolves in Light, Dark, and supported High contrast previews.",
            s::object(json!({"owner":s::id()}), &["owner"]),
            false,
        ),
        s::actions(
            "designer_palette_edit",
            "Edit or add a literal Color or SolidColorBrush. To create a High contrast dictionary, pass a provider from designer_palette in the same add action.",
            vec![
                s::action(
                    "set",
                    json!({"node":s::id(),"color":s::string()}),
                    &["node", "color"],
                    true,
                ),
                s::action(
                    "add",
                    json!({"owner":s::id(),"layer":{"enum":["shared","default","light","dark","high_contrast"]},
                    "provider":{"enum":["winui","material","macos"]},
                    "key":s::string(),"kind":{"enum":["color","solid_color_brush"]},"color":s::string()}),
                    &["owner", "layer", "key", "kind", "color"],
                    true,
                ),
            ],
            true,
        ),
    ]
}

fn source_schema() -> Value {
    let position = s::object(json!({"line":s::revision(),"character":s::revision()}), &["line", "character"]);
    s::object(json!({
        "uri":s::string(), "range":s::object(json!({"start":position,"end":position}), &["start", "end"]),
        "sourceStamp":s::string(),"kind":{"enum":["style", "control_theme", "palette", "color"]}
    }), &["uri", "range", "sourceStamp", "kind"])
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    session.ensure_style_document()?;
    match name {
        "designer_style_catalog" => return catalog(session, args["target"].as_u64()),
        "designer_style_definition" => return definition(session, node_id(args, "node")?),
        "designer_open_style_source" => {
            let navigation = session.open_style_source(decode(args, "source")?)?;
            return Ok(json!({"document":state(session),"openDefinition":navigation}));
        }
        "designer_style_source" => {
            let source: StyleSource = decode(args, "source")?;
            let document = session.document();
            let (editor, span) = source.editor_in(document, session.document_uri())?;
            let range = LineIndex::new(document.source()).range(span).map_err(|error| error.to_string())?;
            return Ok(json!({"editor":editor,"range":range}));
        }
        "designer_style_resources" => {
            return Ok(
                json!({"resources":session.style_resources(node_id(args, "owner")?, &text(args, "property")?)?}),
            );
        }
        "designer_palette" => return palette(session, node_id(args, "owner")?),
        "designer_style_edit" => edit_style(session, args)?,
        "designer_palette_edit" => match args["action"].as_str() {
            Some("set") => {
                session.change_palette_color(node_id(args, "node")?, &text(args, "color")?)?
            }
            Some("add") => session.add_palette_color(
                node_id(args, "owner")?,
                decode(args, "layer")?,
                &text(args, "key")?,
                decode(args, "kind")?,
                &text(args, "color")?,
                args.get("provider").map(|_| decode(args, "provider")).transpose()?,
            )?,
            _ => return Err("Choose a palette action".into()),
        },
        _ => return Err("Unknown style composer tool".into()),
    }
    Ok(json!({"document":state(session)}))
}

fn edit_style(session: &mut DesignerSession, args: &Value) -> Result<(), String> {
    match args["action"].as_str() {
        Some("create") => session.change_style(StyleChange::Create {
            owner: node_id(args, "owner")?,
            selector: text(args, "selector")?.trim(),
        }),
        Some("selector") => session.change_style(StyleChange::SetSelector {
            node: node_id(args, "node")?,
            selector: text(args, "selector")?.trim(),
        }),
        Some("base") => session.change_style(StyleChange::SetBase {
            theme: node_id(args, "node")?,
            base: args["base"].as_u64(),
        }),
        Some("setter") => session.set_style_value(
            node_id(args, "owner")?,
            &text(args, "property")?,
            args["value"].as_str(),
        ),
        Some("resource") => session.set_style_resource(
            node_id(args, "owner")?,
            &text(args, "property")?,
            node_id(args, "resource")?,
            args["dynamic"] == true,
        ),
        Some("remove") => session.change_style(StyleChange::Remove {
            node: node_id(args, "node")?,
        }),
        _ => Err("Choose a style action".into()),
    }
}

fn catalog(session: &DesignerSession, target: Option<NodeId>) -> Result<Value, String> {
    let document = session.document();
    let target = target
        .or_else(|| session.selection().last().copied())
        .unwrap_or(document.ast().root());
    let mut current = Some(target);
    let mut ancestors = Vec::new();
    while let Some(node) = current {
        let node = document
            .ast()
            .node(node)
            .ok_or("Choose an authored style scope")?;
        if node.kind == NodeKind::Element {
            ancestors.push(node.id);
        }
        current = node.parent;
    }
    let style_hosts = ancestors
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| supports_styles(document, node))
        .map(|node| {
            json!({"node":node.id,"label":label(document,node.id),"type_name":node.name,
                "nested":matches!(node.local_name(),"Style" | "ControlTheme")})
        })
        .collect::<Vec<_>>();
    let eligible_resources = document
        .ast()
        .nodes_in_source_order()
        .filter(|node| {
            node.kind == NodeKind::Element
                && (ancestors.contains(&node.id) || has_resources(document, node.id))
                && supports_palette(document, node)
        })
        .map(|node| json!({"node":node.id,"label":label(document,node.id)}))
        .collect::<Vec<_>>();
    let closest_scope = ancestors
        .iter()
        .copied()
        .find(|id| has_resources(document, *id))
        .or_else(|| {
            eligible_resources
                .first()
                .and_then(|owner| owner["node"].as_u64())
        });
    let definitions = session.component_catalog()?.entries.into_iter()
        .filter(|entry| matches!(entry.kind, ComponentKind::Style | ComponentKind::ControlTheme))
        .map(|entry| {
            let parent = entry.owner.or_else(|| lexical_style_owner(document, entry.node));
            json!({"node":entry.node,"name":entry.display_name,"kind":entry.kind,"target":entry.target_type,
                "scope":parent.map(|id| label(document,id))})
        }).collect::<Vec<_>>();
    Ok(
        json!({"definitions":definitions,"style_hosts":style_hosts,"palette_scopes":eligible_resources,
        "included_sources":session.style_source_choices()?,
        "palette_owner":closest_scope,"theme":session.component_resource_context(target).and_then(|value| value.theme)}),
    )
}

fn label(document: &Document, id: NodeId) -> &str {
    document.ast().node(id).map_or("Document", |node| {
        node.xaml_name().unwrap_or(node.local_name())
    })
}

fn has_resources(document: &Document, id: NodeId) -> bool {
    document.ast().node(id).is_some_and(|node| {
        node.local_name() == "ResourceDictionary"
            || component_property_elements(document, id, "Resources")
                .is_ok_and(|entries| !entries.is_empty())
    })
}

fn lexical_style_owner(document: &Document, id: NodeId) -> Option<NodeId> {
    let node = document.ast().node(document.ast().node(id)?.parent?)?;
    if node.kind == NodeKind::PropertyElement {
        node.parent
    } else {
        Some(node.id)
    }
}

fn definition(session: &DesignerSession, node: NodeId) -> Result<Value, String> {
    let document = session.document();
    let index = LineIndex::new(document.source());
    let catalog = session.component_catalog()?;
    let entry = catalog
        .entries
        .iter()
        .find(|entry| {
            entry.node == node
                && matches!(
                    entry.kind,
                    ComponentKind::Style | ComponentKind::ControlTheme
                )
        })
        .ok_or("Choose one authored Style or ControlTheme")?;
    let declaration = document.ast().node(node).ok_or("The definition changed")?;
    let (identity, member) = match entry.kind {
        ComponentKind::Style => ("Selector", "selector"),
        _ => ("BasedOn", "base"),
    };
    let field = component_property_attribute(document, node, identity)
        .map_err(|error| error.to_string())?;
    let editable = component_property_elements(document, node, identity)
        .map_err(|error| error.to_string())?
        .is_empty()
        && (member != "selector"
            || field.is_none_or(|value| matches!(value.markup(), Ok(MarkupValue::Literal(_)))));
    let mut value = json!({
        "node":node,"kind":entry.kind,"name":entry.display_name,"target":entry.target_type,
        "range":index.range(declaration.name_span).map_err(|error| error.to_string())?,
        "identity":{"kind":member,"value":field.map(|attribute| &attribute.value),"editable":editable},
        "can_remove":entry.kind == ComponentKind::Style && entry.key.is_none()
    });
    if member == "base" {
        value["base"] = json!(base_theme(document, node).map_err(|error| error.to_string())?);
        value["base_choices"] = json!(catalog.entries.iter().filter(|entry| compatible_base(document,node,entry))
            .map(|entry| json!({"node":entry.node,"name":entry.display_name,"type_name":entry.target_type})).collect::<Vec<_>>());
    }
    let mut existing = BTreeSet::new();
    let setters = authored_setters(document, node).map_err(|error| error.to_string())?
        .into_iter().map(|setter| {
            existing.insert(setter.property.clone());
            let field = editor(document, node, &setter.property);
            let expression = setter.value.as_ref().is_some_and(|value| matches!(MarkupValue::parse(value), Ok(MarkupValue::Extension(_))));
            let editable = !expression && setter.object.is_none();
            Ok(json!({"node":setter.node,"property":setter.property,"value":setter.value,"object":setter.object.is_some(),
                "range":index.range(setter.span).map_err(|error| error.to_string())?, "field":field,"editable":editable,"expression":expression}))
        }).collect::<Result<Vec<_>, String>>()?;
    let available =
        setter_target_type(document, node)
            .map(|type_| {
                let mut names = type_
                    .properties()
                    .iter()
                    .filter(|property| {
                        !property.is_direct()
                            && !property.is_read_only()
                            && !existing.contains(property.name())
                    })
                    .map(|property| property.name())
                    .collect::<Vec<_>>();
                names.sort_unstable();
                names.dedup();
                names.into_iter().map(|property|
            json!({"property":property,"field":editor(document,node,property)})).collect::<Vec<_>>()
            })
            .unwrap_or_default();
    value["setters"] = json!(setters);
    value["available"] = json!(available);
    Ok(value)
}

fn editor(document: &Document, node: NodeId, name: &str) -> Value {
    let Some(property) = setter_property(document, node, name) else {
        return json!({"kind":"text","options":[],"resources":false});
    };
    let metadata = setter_metadata(node, name, property).1;
    let (kind, options) = match metadata.kind {
        PropertyEditorKind::Boolean => ("boolean", vec!["True".into(), "False".into()]),
        PropertyEditorKind::Enumeration {
            values,
            flags: false,
        } => ("choice", values),
        PropertyEditorKind::Color | PropertyEditorKind::Brush => ("color", vec![]),
        PropertyEditorKind::Number => ("number", vec![]),
        _ => ("text", vec![]),
    };
    json!({"kind":kind,"options":options,"type_name":property.property_type_name(),"resources":true})
}

fn palette(session: &DesignerSession, owner: NodeId) -> Result<Value, String> {
    let document = session.document();
    let root = document
        .ast()
        .node(owner)
        .ok_or("Choose an authored resource owner")?;
    if !supports_palette(document, root) {
        return Err("Choose an authored resource owner".into());
    }
    ScopedPalette::for_session(session).report(owner)
}
