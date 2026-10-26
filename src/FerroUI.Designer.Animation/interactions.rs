//! Typed authoring of the existing Xaml.Behaviors object model. Unknown behavior/action types
//! and nested payloads round-trip through Custom variants instead of being discarded.
use crate::programs::{core_error, escape, fragment, native_fragment, validate_fragment};
use crate::{require, AnimationError, Result};
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorTrigger {
    Event {
        event: String,
        source: Option<String>,
    },
    Data {
        binding: String,
        comparison: String,
        value: String,
        revert_on_false: bool,
    },
    Custom {
        type_name: String,
        properties: BTreeMap<String, String>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorAction {
    ChangeProperty {
        target: Option<String>,
        property: String,
        value: String,
    },
    InvokeCommand {
        command: String,
        parameter: Option<String>,
    },
    CallMethod {
        target: Option<String>,
        method: String,
    },
    AddClass {
        target: Option<String>,
        class: String,
    },
    RemoveClass {
        target: Option<String>,
        class: String,
    },
    Custom {
        type_name: String,
        properties: BTreeMap<String, String>,
        children: String,
    },
    /// Opaque property elements, comments or text retained verbatim inside a behavior.
    Markup { source: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorModel {
    pub id: Option<NodeId>,
    pub owner: NodeId,
    pub trigger: BehaviorTrigger,
    pub actions: Vec<BehaviorAction>,
}
impl BehaviorAction {
    pub fn to_xaml(&self) -> Result<String> {
        if let Self::Markup { source } = self {
            require(
                source.len() <= 16 * 1024 * 1024,
                "Behavior payload exceeds the source limit",
            )?;
            return Ok(source.clone());
        }
        let mut properties = BTreeMap::new();
        let (name, children) = match self {
            Self::ChangeProperty {
                target,
                property,
                value,
            } => {
                require(
                    !property.trim().is_empty(),
                    "ChangePropertyAction requires a property",
                )?;
                optional(&mut properties, "TargetObject", target);
                properties.insert("PropertyName".into(), property.clone());
                properties.insert("Value".into(), value.clone());
                ("ChangePropertyAction", String::new())
            }
            Self::InvokeCommand { command, parameter } => {
                require(
                    !command.trim().is_empty(),
                    "InvokeCommandAction requires a command binding",
                )?;
                properties.insert("Command".into(), command.clone());
                optional(&mut properties, "CommandParameter", parameter);
                ("InvokeCommandAction", String::new())
            }
            Self::CallMethod { target, method } => {
                require(
                    !method.trim().is_empty(),
                    "CallMethodAction requires a method",
                )?;
                optional(&mut properties, "TargetObject", target);
                properties.insert("MethodName".into(), method.clone());
                ("CallMethodAction", String::new())
            }
            Self::AddClass { target, class } | Self::RemoveClass { target, class } => {
                validate_class(class)?;
                optional(&mut properties, "StyledElement", target);
                properties.insert("ClassName".into(), class.clone());
                (
                    if matches!(self, Self::AddClass { .. }) {
                        "AddClassAction"
                    } else {
                        "RemoveClassAction"
                    },
                    String::new(),
                )
            }
            Self::Custom {
                type_name,
                properties: values,
                children,
            } => {
                properties = values.clone();
                (type_name.as_str(), children.clone())
            }
            Self::Markup { .. } => unreachable!("handled raw markup"),
        };
        object_xml(name, &properties, &children)
    }
}
impl BehaviorModel {
    pub fn to_xaml(&self) -> Result<String> {
        require(self.actions.len() <= 4096, "Too many behavior actions")?;
        let mut properties = BTreeMap::new();
        let name = match &self.trigger {
            BehaviorTrigger::Event { event, source } => {
                require(
                    !event.trim().is_empty(),
                    "Event trigger requires an event name",
                )?;
                properties.insert("EventName".into(), event.clone());
                optional(&mut properties, "SourceObject", source);
                "EventTriggerBehavior"
            }
            BehaviorTrigger::Data {
                binding,
                comparison,
                value,
                revert_on_false,
            } => {
                require(
                    !binding.trim().is_empty(),
                    "Data trigger requires a binding",
                )?;
                require(
                    matches!(
                        comparison.as_str(),
                        "Equal"
                            | "NotEqual"
                            | "LessThan"
                            | "LessThanOrEqual"
                            | "GreaterThan"
                            | "GreaterThanOrEqual"
                    ),
                    "Unknown data comparison",
                )?;
                properties.insert("Binding".into(), binding.clone());
                properties.insert("ComparisonCondition".into(), comparison.clone());
                properties.insert("Value".into(), value.clone());
                properties.insert(
                    "RevertOnFalse".into(),
                    if *revert_on_false { "True" } else { "False" }.into(),
                );
                "DataTriggerBehavior"
            }
            BehaviorTrigger::Custom {
                type_name,
                properties: values,
            } => {
                properties = values.clone();
                type_name
            }
        };
        object_xml(
            name,
            &properties,
            &self
                .actions
                .iter()
                .map(BehaviorAction::to_xaml)
                .collect::<Result<Vec<_>>>()?
                .join("\n"),
        )
    }
    pub fn read(document: &Document, owner: NodeId) -> Result<Vec<Self>> {
        let node = document
            .ast()
            .node(owner)
            .ok_or_else(|| AnimationError("Unknown behavior owner".into()))?;
        let Some(collection) = node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find(|node| node.local_name() == "Interaction.Behaviors")
        else {
            return Ok(Vec::new());
        };
        let container = document
            .ast()
            .element_children(collection.id)
            .into_iter()
            .find(|id| {
                document
                    .ast()
                    .node(*id)
                    .is_some_and(|node| node.local_name() == "BehaviorCollection")
            })
            .unwrap_or(collection.id);
        document
            .ast()
            .element_children(container)
            .into_iter()
            .map(|id| Self::from_source(&fragment(document, id)?, owner, Some(id)))
            .collect()
    }
    pub fn from_source(source: &str, owner: NodeId, id: Option<NodeId>) -> Result<Self> {
        let document = Document::new(source).map_err(core_error)?;
        let root = document.ast().node(document.ast().root()).expect("root");
        let properties = root
            .attributes
            .iter()
            .filter(|attribute| !attribute.is_namespace())
            .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
            .collect::<BTreeMap<_, _>>();
        let exact = |allowed: &[&str]| {
            native_behavior_namespace(root.namespace_uri.as_deref())
                && properties
                    .keys()
                    .all(|name| allowed.contains(&name.as_str()))
        };
        let trigger = match root.local_name() {
            "EventTriggerBehavior" if exact(&["EventName", "SourceObject"]) => {
                BehaviorTrigger::Event {
                    event: properties
                        .get("EventName")
                        .cloned()
                        .unwrap_or_else(|| "AttachedToVisualTree".into()),
                    source: properties.get("SourceObject").cloned(),
                }
            }
            "DataTriggerBehavior"
                if exact(&["Binding", "ComparisonCondition", "Value", "RevertOnFalse"])
                    && properties.contains_key("Binding")
                    && properties.contains_key("Value")
                    && properties.get("RevertOnFalse").is_none_or(|value| {
                        value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false")
                    })
                    && properties.get("ComparisonCondition").is_none_or(|value| {
                        matches!(
                            value.as_str(),
                            "Equal"
                                | "NotEqual"
                                | "LessThan"
                                | "LessThanOrEqual"
                                | "GreaterThan"
                                | "GreaterThanOrEqual"
                        )
                    }) =>
            {
                BehaviorTrigger::Data {
                    binding: properties.get("Binding").cloned().unwrap_or_default(),
                    comparison: properties
                        .get("ComparisonCondition")
                        .cloned()
                        .unwrap_or_else(|| "Equal".into()),
                    value: properties.get("Value").cloned().unwrap_or_default(),
                    revert_on_false: properties
                        .get("RevertOnFalse")
                        .is_some_and(|value| value.eq_ignore_ascii_case("true")),
                }
            }
            _ => BehaviorTrigger::Custom {
                type_name: root.name.clone(),
                properties: root
                    .attributes
                    .iter()
                    .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
                    .collect(),
            },
        };
        let mut actions = Vec::new();
        if matches!(trigger, BehaviorTrigger::Custom { .. }) {
            if let Some(closing) = root.closing_span {
                actions.push(BehaviorAction::Markup {
                    source: source[root.opening_span.end..closing.start].to_owned(),
                });
            }
        } else {
            read_actions(&document, root.id, &mut actions)?;
        }
        Ok(Self {
            id,
            owner,
            trigger,
            actions,
        })
    }
    pub fn plan(&self, document: &Document) -> Result<Vec<Edit>> {
        let owner = document
            .ast()
            .node(self.owner)
            .ok_or_else(|| AnimationError("Unknown behavior owner".into()))?;
        require(
            owner.attribute("Interaction.Behaviors").is_none(),
            "The behavior collection is bound; edit that expression explicitly",
        )?;
        let source = self.to_xaml()?;
        if let Some(id) = self.id {
            require(
                Self::read(document, self.owner)?
                    .iter()
                    .any(|behavior| behavior.id == Some(id)),
                "Behavior does not belong to this owner",
            )?;
            let old = document.ast().node(id).expect("existing behavior");
            let source = native_fragment(document, self.owner, &source)?;
            validate_fragment(document, self.owner, &source, None)?;
            return Ok(vec![Edit::Text {
                span: old.span,
                replacement: source,
            }]);
        }
        let source = native_fragment(document, self.owner, &source)?;
        validate_fragment(document, self.owner, &source, None)?;
        if let Some(collection) = owner
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find(|node| node.local_name() == "Interaction.Behaviors")
        {
            let container = document
                .ast()
                .element_children(collection.id)
                .into_iter()
                .find(|id| {
                    document
                        .ast()
                        .node(*id)
                        .is_some_and(|node| node.local_name() == "BehaviorCollection")
                })
                .unwrap_or(collection.id);
            Ok(vec![Edit::InsertChild {
                parent: container,
                index: document.ast().element_children(container).len(),
                source,
            }])
        } else {
            let source = native_fragment(
                document,
                self.owner,
                &format!("<Interaction.Behaviors>{source}</Interaction.Behaviors>"),
            )?;
            // Behavior parameters can use the owner's StaticResource values too.
            let index = document
                .ast()
                .element_children(self.owner)
                .iter()
                .rposition(|id| {
                    document
                        .ast()
                        .node(*id)
                        .is_some_and(|child| child.local_name().ends_with(".Resources"))
                })
                .map_or(0, |index| index + 1);
            Ok(vec![Edit::InsertChild {
                parent: self.owner,
                index,
                source,
            }])
        }
    }
    pub fn remove(document: &Document, id: NodeId) -> Result<Vec<Edit>> {
        let node = document
            .ast()
            .node(id)
            .ok_or_else(|| AnimationError("Unknown behavior".into()))?;
        let parent = node
            .parent
            .and_then(|id| document.ast().node(id))
            .ok_or_else(|| AnimationError("Behavior has no collection".into()))?;
        require(
            matches!(
                parent.local_name(),
                "Interaction.Behaviors" | "BehaviorCollection"
            ),
            "Select a behavior in an Interaction.Behaviors collection",
        )?;
        Ok(vec![Edit::RemoveNode { node: id }])
    }
}
fn read_actions(
    document: &Document,
    parent: NodeId,
    actions: &mut Vec<BehaviorAction>,
) -> Result<()> {
    for id in &document.ast().node(parent).expect("action parent").children {
        let node = document.ast().node(*id).expect("action child");
        if (node.local_name().ends_with(".Actions") || node.local_name() == "ActionCollection")
            && native_behavior_namespace(node.namespace_uri.as_deref())
            && node
                .attributes
                .iter()
                .all(|attribute| attribute.is_namespace())
        {
            read_actions(document, *id, actions)?;
        } else if node.is_element() && node.kind != NodeKind::PropertyElement {
            actions.push(parse_action(document, *id)?);
        } else if !(node.kind == NodeKind::Text
            && node
                .text
                .as_deref()
                .is_some_and(|text| text.trim().is_empty()))
        {
            actions.push(BehaviorAction::Markup {
                source: if node.is_element() {
                    fragment(document, *id)?
                } else {
                    document.valid_source()[node.span.start..node.span.end].into()
                },
            });
        }
    }
    Ok(())
}
fn parse_action(document: &Document, id: NodeId) -> Result<BehaviorAction> {
    let node = document.ast().node(id).expect("action");
    let attrs = node
        .attributes
        .iter()
        .filter(|attribute| !attribute.is_namespace())
        .map(|attribute| (attribute.name.as_str(), attribute.value.as_str()))
        .collect::<BTreeMap<_, _>>();
    let exact = |allowed: &[&str]| {
        native_behavior_namespace(node.namespace_uri.as_deref())
            && attrs.keys().all(|name| allowed.contains(name))
            && node
                .children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .all(|child| {
                    child.kind == NodeKind::Text
                        && child
                            .text
                            .as_deref()
                            .is_some_and(|text| text.trim().is_empty())
                })
    };
    let get = |name| attrs.get(name).map(|value| (*value).to_owned());
    Ok(match node.local_name() {
        "ChangePropertyAction"
            if exact(&["TargetObject", "PropertyName", "Value"])
                && attrs.contains_key("PropertyName")
                && attrs.contains_key("Value") =>
        {
            BehaviorAction::ChangeProperty {
                target: get("TargetObject"),
                property: get("PropertyName").unwrap_or_default(),
                value: get("Value").unwrap_or_default(),
            }
        }
        "InvokeCommandAction"
            if exact(&["Command", "CommandParameter"]) && attrs.contains_key("Command") =>
        {
            BehaviorAction::InvokeCommand {
                command: get("Command").unwrap_or_default(),
                parameter: get("CommandParameter"),
            }
        }
        "CallMethodAction"
            if exact(&["TargetObject", "MethodName"]) && attrs.contains_key("MethodName") =>
        {
            BehaviorAction::CallMethod {
                target: get("TargetObject"),
                method: get("MethodName").unwrap_or_default(),
            }
        }
        "AddClassAction"
            if exact(&["StyledElement", "ClassName"]) && attrs.contains_key("ClassName") =>
        {
            BehaviorAction::AddClass {
                target: get("StyledElement"),
                class: get("ClassName").unwrap_or_default(),
            }
        }
        "RemoveClassAction"
            if exact(&["StyledElement", "ClassName"]) && attrs.contains_key("ClassName") =>
        {
            BehaviorAction::RemoveClass {
                target: get("StyledElement"),
                class: get("ClassName").unwrap_or_default(),
            }
        }
        _ => {
            let source = fragment(document, id)?;
            let standalone = Document::new(source.clone()).map_err(core_error)?;
            let root = standalone
                .ast()
                .node(standalone.ast().root())
                .expect("root");
            BehaviorAction::Custom {
                type_name: root.name.clone(),
                properties: root
                    .attributes
                    .iter()
                    .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
                    .collect(),
                children: root
                    .closing_span
                    .map(|closing| source[root.opening_span.end..closing.start].to_owned())
                    .unwrap_or_default(),
            }
        }
    })
}
fn optional(properties: &mut BTreeMap<String, String>, name: &str, value: &Option<String>) {
    if let Some(value) = value {
        properties.insert(name.into(), value.clone());
    }
}
fn object_xml(name: &str, properties: &BTreeMap<String, String>, children: &str) -> Result<String> {
    require(
        !name.is_empty()
            && !name
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '/' | '=')),
        "Invalid behavior object type",
    )?;
    let mut source = format!("<{name}");
    for (name, value) in properties {
        require(
            !name.is_empty()
                && !name
                    .chars()
                    .any(|c| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '/' | '=')),
            "Invalid behavior property name",
        )?;
        source.push_str(&format!(" {name}=\"{}\"", escape(value)));
    }
    source.push('>');
    source.push_str(children);
    source.push_str(&format!("</{name}>"));
    Ok(source)
}
pub(crate) fn validate_class(class: &str) -> Result<()> {
    if class.trim_start().starts_with('{') && class.trim_end().ends_with('}') {
        return Ok(());
    }
    require(
        !class.is_empty()
            && class
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-')),
        "Class names must contain only letters, numbers, underscores or hyphens",
    )
}

fn native_behavior_namespace(namespace: Option<&str>) -> bool {
    namespace.is_none_or(|namespace| {
        namespace == "https://github.com/ferroui"
            || namespace
                .strip_prefix("using:")
                .or_else(|| {
                    namespace
                        .strip_prefix("clr-namespace:")
                        .map(|value| value.split(';').next().unwrap_or(value))
                })
                .is_some_and(|namespace| {
                    namespace.starts_with("FerroUI.Xaml.Interactions")
                        || namespace == "FerroUI.Xaml.Interactivity"
                })
    })
}
