//! Visual-state authoring expressed as real FerroUI style classes, setters and animations.
//! Metadata comments identify editable groups; the application consumes only native markup.
use crate::programs::{
    core_error, decode_text, encode_text, escape, fragment, insert_styles, native_fragment,
    validate_name,
};
use crate::{
    require, AnimationError, AnimationValue, BehaviorAction, Easing, FillMode, Result, Timeline,
};
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
const GROUP_MARKER: &str = "ferroui-designer-state-group:";
const STATE_MARKER: &str = "ferroui-designer-state:";
const TRANSITION_MARKER: &str = "ferroui-designer-transition:";
pub(crate) const STATE_PROGRAM_MARKER: &str = "ferroui-designer-state-program:";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSetter {
    pub target: String,
    pub property: String,
    pub value: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualState {
    pub name: String,
    pub setters: Vec<StateSetter>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: Option<String>,
    pub to: String,
    pub duration: f64,
    pub easing: Easing,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VisualStateGroup {
    pub name: String,
    pub owner: NodeId,
    pub states: Vec<VisualState>,
    pub transitions: Vec<StateTransition>,
}
impl VisualStateGroup {
    pub fn validate(&self) -> Result<()> {
        validate_name(&self.name)?;
        require(self.owner > 0, "State group owner must be nonzero")?;
        require(
            self.states.len() <= 1024 && self.transitions.len() <= 4096,
            "State group exceeds its model limits",
        )?;
        let mut names = BTreeSet::new();
        for state in &self.states {
            validate_name(&state.name)?;
            require(
                names.insert(&state.name),
                "State names must be unique in a group",
            )?;
            let mut properties = BTreeSet::new();
            for setter in &state.setters {
                require(
                    !setter.property.trim().is_empty(),
                    "A state setter requires a property",
                )?;
                require(
                    setter.target.is_empty()
                        || setter
                            .target
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_'),
                    "State setter targets use a control Name, or empty for the owner",
                )?;
                require(
                    properties.insert((&setter.target, &setter.property)),
                    "Duplicate state target/property setter",
                )?;
            }
        }
        let mut transitions = BTreeSet::new();
        for transition in &self.transitions {
            require(
                names.contains(&transition.to)
                    && transition
                        .from
                        .as_ref()
                        .is_none_or(|from| names.contains(from)),
                "Transition references an unknown state",
            )?;
            require(
                transition.duration.is_finite() && transition.duration >= 0.0,
                "Transition duration must be finite and nonnegative",
            )?;
            transition.easing.validate()?;
            require(
                transitions.insert((&transition.from, &transition.to)),
                "Duplicate from/to transition",
            )?;
        }
        Ok(())
    }
    pub fn class_name(&self, state: &str) -> String {
        format!("ds_{}_{}", encode_text(&self.name), encode_text(state))
    }
    pub fn transition_class(&self, from: &str, to: &str) -> String {
        format!(
            "dst_{}_{}_{}",
            encode_text(&self.name),
            encode_text(from),
            encode_text(to)
        )
    }
    pub fn classes(&self) -> Vec<String> {
        self.states
            .iter()
            .map(|state| self.class_name(&state.name))
            .chain(self.transitions.iter().filter_map(|transition| {
                transition
                    .from
                    .as_ref()
                    .map(|from| self.transition_class(from, &transition.to))
            }))
            .collect()
    }
    /// Emit ordinary class actions, routing directional transitions through ConditionalAction.
    /// The group's generated styles set each route condition from the current state class,
    /// so a single event action chooses the edge that is active when the event actually fires.
    pub fn actions(
        &self,
        owner_name: &str,
        to: &str,
        from: Option<&str>,
    ) -> Result<Vec<BehaviorAction>> {
        self.validate()?;
        require(
            self.states.iter().any(|state| state.name == to),
            "Unknown destination state",
        )?;
        require(
            !owner_name.is_empty(),
            "State actions require a named owner",
        )?;
        if let Some(from) = from {
            require(
                self.states.iter().any(|state| state.name == from),
                "Unknown source state",
            )?;
            return Ok(self.direct_actions(owner_name, to, Some(from)));
        }
        let mut routed = self.direct_actions(owner_name, to, None);
        for transition in self
            .transitions
            .iter()
            .rev()
            .filter(|transition| transition.to == to && transition.from.is_some())
        {
            let from = transition.from.as_deref().expect("directional edge");
            let branch = self
                .direct_actions(owner_name, to, Some(from))
                .iter()
                .map(BehaviorAction::to_xaml)
                .collect::<Result<Vec<_>>>()?
                .join("");
            let fallback = routed
                .iter()
                .map(BehaviorAction::to_xaml)
                .collect::<Result<Vec<_>>>()?
                .join("");
            routed = vec![BehaviorAction::Custom {
                type_name: "ConditionalAction".into(),
                properties: BTreeMap::from([("Classes".into(), self.route_class(from, to))]),
                children: format!(
                    "<ConditionalAction.Actions>{branch}</ConditionalAction.Actions><ConditionalAction.ElseActions>{fallback}</ConditionalAction.ElseActions>"
                ),
            }];
        }
        Ok(routed)
    }
    fn route_class(&self, from: &str, to: &str) -> String {
        format!(
            "dsr_{}_{}_{}",
            encode_text(&self.name),
            encode_text(from),
            encode_text(to)
        )
    }
    fn direct_actions(
        &self,
        owner_name: &str,
        to: &str,
        from: Option<&str>,
    ) -> Vec<BehaviorAction> {
        let target = Some(owner_name.to_owned());
        let mut actions = self
            .classes()
            .into_iter()
            .map(|class| BehaviorAction::RemoveClass {
                target: target.clone(),
                class,
            })
            .collect::<Vec<_>>();
        if let Some(from) = from.filter(|from| {
            self.transitions
                .iter()
                .any(|transition| transition.from.as_deref() == Some(from) && transition.to == to)
        }) {
            actions.push(BehaviorAction::AddClass {
                target: target.clone(),
                class: self.transition_class(from, to),
            });
        }
        actions.push(BehaviorAction::AddClass {
            target,
            class: self.class_name(to),
        });
        actions
    }
    /// Native nested styles own their class scope. Literal state properties also use an
    /// animation layer so an authored local Width/Opacity does not mask the selected state.
    pub fn to_xaml(&self, owner_name: &str) -> Result<String> {
        self.validate()?;
        require(!owner_name.is_empty(), "State styles require a named owner")?;
        let metadata = serde_json::to_string(self).map_err(core_error)?;
        let mut result = format!(
            "<Style Selector=\"#{}\"><!--{GROUP_MARKER}{}-->",
            escape(owner_name),
            encode_text(&metadata)
        );
        for state in &self.states {
            let transition = self
                .transitions
                .iter()
                .find(|transition| transition.from.is_none() && transition.to == state.name);
            let (duration, easing) =
                transition.map_or((0.000_000_1, Easing::Linear), |transition| {
                    (
                        transition.duration.max(0.000_000_1),
                        transition.easing.clone(),
                    )
                });
            result.push_str(&self.state_styles(state, None, duration, easing)?);
        }
        for transition in self
            .transitions
            .iter()
            .filter(|transition| transition.from.is_some())
        {
            let to = self
                .states
                .iter()
                .find(|state| state.name == transition.to)
                .expect("validated state");
            result.push_str(&self.state_styles(
                to,
                transition.from.as_deref(),
                transition.duration.max(0.000_000_1),
                transition.easing.clone(),
            )?);
        }
        for transition in self
            .transitions
            .iter()
            .filter(|transition| transition.from.is_some())
        {
            let from = transition.from.as_deref().expect("directional edge");
            result.push_str(&format!("<Style Selector=\"^.{} ConditionalAction.{}\"><!--{TRANSITION_MARKER}route--><Setter Property=\"Condition\" Value=\"True\"/></Style>",self.class_name(from),self.route_class(from,&transition.to)));
        }
        result.push_str("</Style>");
        Ok(result)
    }
    fn state_styles(
        &self,
        state: &VisualState,
        from: Option<&str>,
        duration: f64,
        easing: Easing,
    ) -> Result<String> {
        let mut targets = BTreeMap::<&str, Vec<&StateSetter>>::new();
        for setter in &state.setters {
            targets.entry(&setter.target).or_default().push(setter);
        }
        let mut result = String::new();
        for (target, setters) in targets {
            let class = self.class_name(&state.name);
            let transition = from
                .map(|from| format!(".{}", self.transition_class(from, &state.name)))
                .unwrap_or_default();
            let selector = format!(
                "^.{class}{transition}{}",
                if target.is_empty() {
                    String::new()
                } else {
                    format!(" #{target}")
                }
            );
            let marker = if let Some(from) = from {
                format!(
                    "{TRANSITION_MARKER}{}:{}:{}",
                    encode_text(from),
                    encode_text(&state.name),
                    encode_text(target)
                )
            } else {
                format!(
                    "{STATE_MARKER}{}:{}",
                    encode_text(&state.name),
                    encode_text(target)
                )
            };
            result.push_str(&format!(
                "<Style Selector=\"{}\"><!--{marker}-->",
                escape(&selector)
            ));
            let mut bound_setters = Vec::new();
            let mut timeline = Timeline::new(duration)?;
            timeline.fill = FillMode::Forward;
            timeline.easing = easing.clone();
            for setter in setters {
                result.push_str(&format!(
                    "<Setter Property=\"{}\" Value=\"{}\"/>",
                    escape(&setter.property),
                    escape(&setter.value)
                ));
                if setter.value.trim_start().starts_with('{') {
                    bound_setters.push(format!(
                        "<Setter Property=\"{}\" Value=\"{}\"/>",
                        escape(&setter.property),
                        escape(&setter.value)
                    ));
                    continue;
                }
                let value = state_value(&setter.property, &setter.value)?;
                if let Some(previous) = from
                    .and_then(|from| self.states.iter().find(|state| state.name == from))
                    .and_then(|state| {
                        state.setters.iter().find(|previous| {
                            previous.target == setter.target && previous.property == setter.property
                        })
                    })
                    .filter(|previous| !previous.value.trim_start().starts_with('{'))
                {
                    let start = state_value(&previous.property, &previous.value)?;
                    require(
                        start.kind() == value.kind(),
                        "Transition endpoints have inconsistent types",
                    )?;
                    timeline.record(self.owner, &setter.property, 0.0, start)?;
                }
                timeline.record(self.owner, &setter.property, duration, value.clone())?;
                timeline
                    .track_mut(self.owner, &setter.property)
                    .expect("recorded track")
                    .base_value = Some(value);
            }
            if !timeline.tracks.is_empty() {
                result.push_str("<Style.Animations>");
                result.push_str(&state_program(
                    &timeline.to_xaml(self.owner)?,
                    "literal",
                    &format!(
                        "{} / {} / {}",
                        self.name,
                        state.name,
                        if target.is_empty() { "owner" } else { target }
                    ),
                )?);
                result.push_str("</Style.Animations>");
            }
            if !bound_setters.is_empty() {
                let mut binding_timeline = timeline.clone();
                binding_timeline.tracks.clear();
                let animation = binding_timeline.to_xaml(self.owner)?.replace(
                    "</Animation>",
                    &format!(
                        "<KeyFrame Cue=\"100%\">{}</KeyFrame></Animation>",
                        bound_setters.join("")
                    ),
                );
                let animation = state_program(
                    &animation,
                    "expression",
                    &format!(
                        "{} / {} / {} expressions",
                        self.name,
                        state.name,
                        if target.is_empty() { "owner" } else { target }
                    ),
                )?;
                // Merge into an existing property collection when literals and bindings coexist.
                if let Some(end) = result
                    .rfind("</Style.Animations>")
                    .filter(|end| result[*end..].len() == "</Style.Animations>".len())
                {
                    result.insert_str(end, &animation);
                } else {
                    result.push_str(&format!("<Style.Animations>{animation}</Style.Animations>"));
                }
            }
            result.push_str("</Style>");
        }
        Ok(result)
    }
    pub fn read(document: &Document) -> Vec<Self> {
        group_nodes(document)
            .into_iter()
            .filter_map(|(style, mut group)| {
                document.ast().node(group.owner)?;
                // Native keyframes/setters are authoritative after direct source edits.
                if let Some(style) = document.ast().node(style).map(|node| node.id) {
                    let mut seen_transitions = BTreeSet::new();
                    for child in document.ast().element_children(style) {
                        sync_transition(document, child, &mut group, &mut seen_transitions);
                        let Some(node) = document.ast().node(child) else {
                            continue;
                        };
                        let marker = node
                            .children
                            .iter()
                            .filter_map(|id| document.ast().node(*id))
                            .filter_map(|node| node.text.as_deref())
                            .find_map(|text| text.trim().strip_prefix(STATE_MARKER));
                        let Some((state, target)) =
                            marker.and_then(|marker| marker.split_once(':')).and_then(
                                |(state, target)| Some((decode_text(state)?, decode_text(target)?)),
                            )
                        else {
                            continue;
                        };
                        let original_target = target.clone();
                        let target = if !target.is_empty() {
                            node.attribute("Selector")
                                .and_then(|selector| {
                                    selector.rsplit_once('#').map(|(_, name)| {
                                        name.chars()
                                            .take_while(|character| {
                                                character.is_alphanumeric() || *character == '_'
                                            })
                                            .collect::<String>()
                                    })
                                })
                                .filter(|name| !name.is_empty())
                                .unwrap_or(target)
                        } else {
                            target
                        };
                        let Some(state) = group
                            .states
                            .iter_mut()
                            .find(|existing| existing.name == state)
                        else {
                            continue;
                        };
                        let mut values = BTreeMap::new();
                        for id in document.ast().element_children(child) {
                            if let Some(setter) = document
                                .ast()
                                .node(id)
                                .filter(|node| node.local_name() == "Setter")
                            {
                                if let (Some(property), Some(value)) =
                                    (setter.attribute("Property"), setter.attribute("Value"))
                                {
                                    values.insert(property.to_owned(), value.to_owned());
                                }
                            }
                        }
                        let animations = crate::programs::subtree(document, child)
                            .into_iter()
                            .filter(|node| node.local_name() == "Animation")
                            .collect::<Vec<_>>();
                        let has_owned = animations
                            .iter()
                            .any(|node| state_program_role(document, node.id).is_some());
                        for animation in animations.into_iter().filter(|node| {
                            !has_owned || state_program_role(document, node.id).is_some()
                        }) {
                            if let Ok(source) = fragment(document, animation.id)
                                .and_then(|source| Document::new(source).map_err(core_error))
                            {
                                for (property, points) in endpoints(&source) {
                                    if let Some(value) = points
                                        .last()
                                        .and_then(|point| source.ast().node(point.setter))
                                        .and_then(|setter| setter.attribute("Value"))
                                    {
                                        values.insert(property, value.to_owned());
                                    }
                                }
                            }
                        }
                        state.setters.retain(|setter| {
                            setter.target != target && setter.target != original_target
                        });
                        state
                            .setters
                            .extend(values.into_iter().map(|(property, value)| StateSetter {
                                target: target.clone(),
                                property,
                                value,
                            }));
                    }
                }
                Some(group)
            })
            .collect()
    }
    pub fn plan(&self, document: &Document) -> Result<Vec<Edit>> {
        self.validate()?;
        let owner = document
            .ast()
            .node(self.owner)
            .ok_or_else(|| AnimationError("Unknown state-group owner".into()))?;
        let mut name = owner
            .xaml_name()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("DesignerStateOwner{}", self.owner));
        let mut edits = Vec::new();
        if owner.xaml_name().is_none() {
            let names = document
                .ast()
                .elements()
                .filter_map(|node| node.xaml_name())
                .collect::<BTreeSet<_>>();
            while names.contains(name.as_str()) {
                name.push('_');
            }
            edits.push(Edit::SetAttribute {
                node: self.owner,
                name: "Name".into(),
                value: name.clone(),
            });
        }
        let mut authored = self.clone();
        for setter in authored
            .states
            .iter_mut()
            .flat_map(|state| state.setters.iter_mut())
        {
            if setter.target == name {
                setter.target.clear();
            }
        }
        let mut source = authored.to_xaml(&name)?;
        source = source.replacen(
            &format!("Selector=\"#{}\"", escape(&name)),
            &format!(
                "Selector=\"{}#{}\"",
                owner.name.replace(':', "|"),
                escape(&name)
            ),
            1,
        );
        let owner_namespaces = document.ast().namespaces(self.owner);
        let mut namespaces = group_node(document, self.owner, &self.name)
            .map(|id| document.ast().namespaces(id))
            .unwrap_or_else(|| owner_namespaces.clone());
        let mut added_namespaces = BTreeMap::new();
        if let Some((prefix, local)) = owner.name.split_once(':') {
            if let Some(uri) = owner_namespaces
                .get(prefix)
                .filter(|uri| namespaces.get(prefix) != Some(*uri))
            {
                let replacement = reserve_namespace(&mut namespaces, "designerowner", uri);
                added_namespaces.insert(replacement.clone(), uri.clone());
                source = source.replacen(
                    &format!("{prefix}|{local}#{}", escape(&name)),
                    &format!("{replacement}|{local}#{}", escape(&name)),
                    1,
                );
            }
        }
        for target in authored
            .states
            .iter()
            .flat_map(|state| state.setters.iter())
            .map(|setter| setter.target.as_str())
            .filter(|target| !target.is_empty())
            .collect::<BTreeSet<_>>()
        {
            let target_id = resolve_state_target(document, self.owner, target)?;
            let node = document.ast().node(target_id).expect("resolved target");
            let mut type_name = node.name.replace(':', "|");
            if let Some((prefix, local)) = node.name.split_once(':') {
                if let Some(uri) = document.ast().namespaces(node.id).get(prefix) {
                    if namespaces.get(prefix) != Some(uri) {
                        let replacement = reserve_namespace(&mut namespaces, "designertarget", uri);
                        added_namespaces.insert(replacement.clone(), uri.clone());
                        type_name = format!("{replacement}|{local}");
                    }
                }
            }
            source = source.replace(
                &format!(" #{target}\""),
                &format!(" {type_name}#{target}\""),
            );
        }
        if !added_namespaces.is_empty() {
            let declarations = added_namespaces
                .iter()
                .map(|(prefix, uri)| format!(" xmlns:{prefix}=\"{}\"", escape(uri)))
                .collect::<String>();
            source.insert_str("<Style".len(), &declarations);
        }
        if let Some(id) = group_node(document, self.owner, &self.name) {
            let old_group = Self::read(document)
                .into_iter()
                .find(|group| group.owner == self.owner && group.name == self.name)
                .ok_or_else(|| AnimationError("Existing state group metadata is invalid".into()))?;
            source = merge_generated_animations(document, id, &source, &old_group, &authored)?;
            // Preserve extension styles/resources and ordinary comments not generated by us.
            let node = document.ast().node(id).expect("group style");
            let mut retained = String::new();
            for child in &node.children {
                let child = document.ast().node(*child).expect("child");
                let generated = child.kind == NodeKind::Comment
                    && child
                        .text
                        .as_deref()
                        .is_some_and(|text| text.trim().starts_with(GROUP_MARKER))
                    || child
                        .children
                        .iter()
                        .filter_map(|id| document.ast().node(*id))
                        .filter_map(|node| node.text.as_deref())
                        .any(|text| {
                            text.trim().starts_with(STATE_MARKER)
                                || text.trim().starts_with(TRANSITION_MARKER)
                        });
                if !generated {
                    retained.push_str(&document.valid_source()[child.span.start..child.span.end]);
                }
            }
            let closing = source.rfind("</Style>").expect("generated style");
            source.insert_str(closing, &retained);
            let source = native_fragment(document, self.owner, &source)?;
            edits.insert(
                0,
                Edit::Text {
                    span: node.span,
                    replacement: source,
                },
            );
        } else {
            edits.extend(insert_styles(document, self.owner, &source)?);
        }
        Ok(edits)
    }
    pub fn remove(document: &Document, owner: NodeId, name: &str) -> Result<Vec<Edit>> {
        let id = group_node(document, owner, name)
            .ok_or_else(|| AnimationError("Unknown state group".into()))?;
        Ok(vec![Edit::RemoveNode { node: id }])
    }
}
fn group_node(document: &Document, owner: NodeId, name: &str) -> Option<NodeId> {
    group_nodes(document)
        .into_iter()
        .find(|(_, group)| group.owner == owner && group.name == name)
        .map(|(id, _)| id)
}
fn group_nodes(document: &Document) -> Vec<(NodeId, VisualStateGroup)> {
    document
        .ast()
        .nodes()
        .filter(|node| node.kind == NodeKind::Comment)
        .filter_map(|node| {
            let text = node.text.as_deref()?.trim().strip_prefix(GROUP_MARKER)?;
            let mut group: VisualStateGroup = serde_json::from_str(&decode_text(text)?).ok()?;
            let style = node.parent?;
            let mut current = Some(style);
            while let Some(id) = current {
                let parent = document.ast().node(id)?;
                if parent.local_name().ends_with(".Styles") {
                    group.owner = parent.parent?;
                    break;
                }
                current = parent.parent;
            }
            Some((style, group))
        })
        .collect()
}
/// Resolve an authored state target without crossing a template name-scope boundary.
pub fn resolve_state_target(document: &Document, owner: NodeId, name: &str) -> Result<NodeId> {
    require(document.ast().node(owner).is_some(), "Unknown state owner")?;
    if name.is_empty() {
        return Ok(owner);
    }
    let scope = document.ast().name_scope(owner);
    let mut matches = crate::programs::subtree(document, owner)
        .into_iter()
        .filter(|node| {
            node.xaml_name() == Some(name) && document.ast().name_scope(node.id) == scope
        });
    let target = matches
        .next()
        .ok_or_else(|| {
            AnimationError(format!(
                "Unknown state target {name} in the owner's name scope"
            ))
        })?
        .id;
    require(
        matches.next().is_none(),
        format!("Ambiguous state target {name} in the owner's name scope"),
    )?;
    Ok(target)
}

pub fn state_value(property: &str, value: &str) -> Result<AnimationValue> {
    AnimationValue::parse(value, crate::xaml::infer_kind(property, value))
}

/// Update designer metadata after a scope-aware source rename/copy. Real named references
/// must be changed by the language service first; this updates only encoded state targets.
/// Source must be a standalone XML subtree including its inherited namespace declarations.
pub fn rewrite_design_metadata(source: &str, renames: &BTreeMap<String, String>) -> Result<String> {
    let mut document = Document::new(source).map_err(core_error)?;
    let mut edits = Vec::new();
    for node in document
        .ast()
        .nodes()
        .filter(|node| node.kind == NodeKind::Comment)
    {
        let text = node.text.as_deref().unwrap_or("").trim();
        let replacement = if let Some(encoded) = text.strip_prefix(GROUP_MARKER) {
            let Some(metadata) = decode_text(encoded) else {
                continue;
            };
            let Ok(mut group) = serde_json::from_str::<VisualStateGroup>(&metadata) else {
                continue;
            };
            let mut changed = false;
            for setter in group
                .states
                .iter_mut()
                .flat_map(|state| state.setters.iter_mut())
            {
                if let Some(name) = renames.get(&setter.target) {
                    if setter.target != *name {
                        setter.target = name.clone();
                        changed = true;
                    }
                }
            }
            if !changed {
                continue;
            }
            Some(format!(
                "{GROUP_MARKER}{}",
                encode_text(&serde_json::to_string(&group).map_err(core_error)?)
            ))
        } else if let Some((marker, encoded)) = text
            .strip_prefix(STATE_MARKER)
            .map(|encoded| (STATE_MARKER, encoded))
            .or_else(|| {
                text.strip_prefix(TRANSITION_MARKER)
                    .map(|encoded| (TRANSITION_MARKER, encoded))
            })
        {
            let mut parts = encoded.split(':').map(str::to_owned).collect::<Vec<_>>();
            let expected = if marker == STATE_MARKER { 2 } else { 3 };
            if parts.len() != expected {
                continue;
            }
            let Some(target) = parts.last().and_then(|value| decode_text(value)) else {
                continue;
            };
            if let Some(name) = renames.get(&target) {
                if *name == target {
                    continue;
                }
                *parts.last_mut().expect("target") = encode_text(name);
            } else {
                continue;
            }
            Some(format!("{marker}{}", parts.join(":")))
        } else {
            None
        };
        if let Some(replacement) = replacement.filter(|replacement| replacement != text) {
            edits.push(Edit::Text {
                span: node.span,
                replacement: format!("<!--{replacement}-->"),
            });
        }
    }
    edits.sort_by_key(|edit| match edit {
        Edit::Text { span, .. } => std::cmp::Reverse(span.start),
        _ => std::cmp::Reverse(0),
    });
    document
        .apply("Rewrite copied designer metadata", edits)
        .map_err(core_error)?;
    Ok(document.source().into())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StateStyleKey {
    from: Option<String>,
    to: String,
    target: String,
}
fn state_style_key(document: &Document, id: NodeId) -> Option<StateStyleKey> {
    let node = document.ast().node(id)?;
    let mut key = node
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter_map(|node| node.text.as_deref())
        .find_map(|text| {
            let text = text.trim();
            if let Some(marker) = text.strip_prefix(STATE_MARKER) {
                let (to, target) = marker.split_once(':')?;
                Some(StateStyleKey {
                    from: None,
                    to: decode_text(to)?,
                    target: decode_text(target)?,
                })
            } else if let Some(marker) = text.strip_prefix(TRANSITION_MARKER) {
                let parts = marker.split(':').collect::<Vec<_>>();
                if parts.len() != 3 {
                    return None;
                }
                Some(StateStyleKey {
                    from: Some(decode_text(parts[0])?),
                    to: decode_text(parts[1])?,
                    target: decode_text(parts[2])?,
                })
            } else {
                None
            }
        })?;
    if !key.target.is_empty() {
        if let Some(name) = node
            .attribute("Selector")
            .and_then(|selector| {
                selector.rsplit_once('#').map(|(_, name)| {
                    name.chars()
                        .take_while(|character| character.is_alphanumeric() || *character == '_')
                        .collect::<String>()
                })
            })
            .filter(|name| !name.is_empty())
        {
            key.target = name;
        }
    }
    Some(key)
}
fn sync_transition(
    document: &Document,
    style: NodeId,
    group: &mut VisualStateGroup,
    seen: &mut BTreeSet<(Option<String>, String)>,
) {
    let Some(key) = state_style_key(document, style) else {
        return;
    };
    let edge = (key.from.clone(), key.to.clone());
    if !seen.insert(edge.clone()) {
        return;
    }
    let animations = crate::programs::subtree(document, style)
        .into_iter()
        .filter(|node| node.local_name() == "Animation")
        .collect::<Vec<_>>();
    let Some(animation) = animations
        .iter()
        .find(|node| state_program_role(document, node.id).is_some())
        .copied()
        .or_else(|| animations.first().copied())
    else {
        return;
    };
    let Ok(duration) = crate::xaml::parse_time(animation.attribute("Duration").unwrap_or("0:0:0"))
    else {
        return;
    };
    let Ok(easing) = fragment(document, animation.id)
        .and_then(|source| crate::xaml::easing_from_source(&source))
    else {
        return;
    };
    if let Some(transition) = group
        .transitions
        .iter_mut()
        .find(|transition| transition.from == edge.0 && transition.to == edge.1)
    {
        transition.duration = duration;
        transition.easing = easing;
    } else if key.from.is_some() || duration > 0.000_000_1 || easing != Easing::Linear {
        group.transitions.push(StateTransition {
            from: key.from,
            to: key.to,
            duration,
            easing,
        });
    }
}
fn state_properties(
    group: &VisualStateGroup,
    state: &str,
    target: &str,
) -> BTreeMap<String, String> {
    group
        .states
        .iter()
        .find(|entry| entry.name == state)
        .into_iter()
        .flat_map(|state| state.setters.iter())
        .filter(|setter| setter.target == target)
        .map(|setter| (setter.property.clone(), setter.value.clone()))
        .collect()
}
fn transition_timing(group: &VisualStateGroup, key: &StateStyleKey) -> (f64, Easing) {
    group
        .transitions
        .iter()
        .find(|transition| transition.from == key.from && transition.to == key.to)
        .map(|transition| {
            (
                transition.duration.max(0.000_000_1),
                transition.easing.clone(),
            )
        })
        .unwrap_or((0.000_000_1, Easing::Linear))
}
/// Keep custom animator payloads, authored intermediate keys, comments and persistent IDs when
/// editing a state. Only changed state endpoints and explicitly changed transition timing are
/// patched; an unrelated state edit does not regenerate another program's source.
fn merge_generated_animations(
    document: &Document,
    old_style: NodeId,
    generated: &str,
    old: &VisualStateGroup,
    new: &VisualStateGroup,
) -> Result<String> {
    let mut output = Document::new(generated).map_err(core_error)?;
    preserve_style_attributes(document, old_style, &mut output)?;
    let old_styles = document
        .ast()
        .element_children(old_style)
        .into_iter()
        .filter_map(|id| state_style_key(document, id).map(|key| (key, id)))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut edits = Vec::new();
    for child in output.ast().element_children(output.ast().root()) {
        let Some(key) = state_style_key(&output, child) else {
            continue;
        };
        seen.insert(key.clone());
        if let Some(old_child) = old_styles.get(&key) {
            let source = merge_state_style(
                document,
                *old_child,
                &fragment(&output, child)?,
                &key,
                old,
                new,
            )?;
            edits.push(Edit::Text {
                span: output.ast().node(child).expect("state style").span,
                replacement: source,
            });
        }
    }
    edits.sort_by_key(|edit| match edit {
        Edit::Text { span, .. } => std::cmp::Reverse(span.start),
        _ => std::cmp::Reverse(0),
    });
    output
        .apply("Preserve authored state programs", edits)
        .map_err(core_error)?;
    // An empty state remains a valid class scope. Retain independent extension programs and
    // payloads while removing its former owned property tracks; removing the state/edge itself
    // deliberately removes that state/edge's style.
    for (key, original) in old_styles
        .into_iter()
        .filter(|(key, _)| !seen.contains(key))
    {
        if !new.states.iter().any(|state| state.name == key.to)
            || key.from.as_ref().is_some_and(|from| {
                !new.transitions
                    .iter()
                    .any(|edge| edge.from.as_ref() == Some(from) && edge.to == key.to)
            })
        {
            continue;
        }
        let node = document.ast().node(original).expect("state style");
        let marker = node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find(|node| {
                node.kind == NodeKind::Comment
                    && node.text.as_deref().is_some_and(|text| {
                        text.trim().starts_with(STATE_MARKER)
                            || text.trim().starts_with(TRANSITION_MARKER)
                    })
            })
            .expect("state marker");
        let source = format!(
            "<Style Selector=\"{}\">{}</Style>",
            escape(node.attribute("Selector").unwrap_or("")),
            &document.valid_source()[marker.span.start..marker.span.end]
        );
        let source = merge_state_style(document, original, &source, &key, old, new)?;
        let parent = output.ast().root();
        output
            .apply(
                "Retain empty state extensions",
                vec![Edit::InsertChild {
                    parent,
                    index: output.ast().element_children(parent).len(),
                    source,
                }],
            )
            .map_err(core_error)?;
    }
    Ok(output.source().into())
}
fn preserve_style_attributes(original: &Document, id: NodeId, output: &mut Document) -> Result<()> {
    let root = output.ast().root();
    let edits = original
        .ast()
        .node(id)
        .expect("style")
        .attributes
        .iter()
        .filter(|attribute| attribute.name != "Selector")
        .map(|attribute| Edit::SetAttribute {
            node: root,
            name: attribute.name.clone(),
            value: attribute.value.clone(),
        })
        .collect();
    output
        .apply(
            "Preserve style attributes and namespace declarations",
            edits,
        )
        .map_err(core_error)?;
    Ok(())
}
fn state_collection(document: &Document, style: NodeId) -> Option<NodeId> {
    document
        .ast()
        .element_children(style)
        .into_iter()
        .find(|id| {
            document
                .ast()
                .node(*id)
                .is_some_and(|node| node.local_name() == "Style.Animations")
        })
}
fn merge_state_style(
    document: &Document,
    original: NodeId,
    generated: &str,
    key: &StateStyleKey,
    old: &VisualStateGroup,
    new: &VisualStateGroup,
) -> Result<String> {
    let mut output = Document::new(generated).map_err(core_error)?;
    preserve_style_attributes(document, original, &mut output)?;
    let root = output.ast().root();
    let before = state_properties(old, &key.to, &key.target);
    let after = state_properties(new, &key.to, &key.target);
    if let Some(collection) = state_collection(document, original) {
        let replacement = state_collection(&output, root);
        let desired = replacement
            .map(|id| fragment(&output, id))
            .transpose()?
            .unwrap_or_else(|| "<Style.Animations/>".into());
        let merged =
            merge_state_collection(&fragment(document, collection)?, &desired, key, old, new)?;
        let edit = if let Some(id) = replacement {
            Edit::Text {
                span: output.ast().node(id).expect("collection").span,
                replacement: merged,
            }
        } else {
            Edit::InsertChild {
                parent: root,
                index: output.ast().element_children(root).len(),
                source: merged,
            }
        };
        output
            .apply("Preserve state animation collection", vec![edit])
            .map_err(core_error)?;
    }
    for child in &document.ast().node(original).expect("style").children {
        let node = document.ast().node(*child).expect("child");
        if node.kind == NodeKind::Text
            || node.local_name() == "Style.Animations"
            || node.text.as_deref().is_some_and(|text| {
                text.trim().starts_with(STATE_MARKER) || text.trim().starts_with(TRANSITION_MARKER)
            })
        {
            continue;
        }
        if node.local_name() == "Setter" {
            if let Some(property) = node.attribute("Property") {
                let complex = node.attribute("Value").is_none()
                    || node
                        .children
                        .iter()
                        .filter_map(|id| document.ast().node(*id))
                        .any(|node| node.is_element())
                    || node.attributes.iter().any(|attribute| {
                        !attribute.is_namespace()
                            && !matches!(attribute.name.as_str(), "Property" | "Value")
                    });
                if complex && before.get(property) == after.get(property) {
                    let duplicates = output
                        .ast()
                        .element_children(root)
                        .into_iter()
                        .filter(|id| {
                            output.ast().node(*id).is_some_and(|node| {
                                node.local_name() == "Setter"
                                    && node.attribute("Property") == Some(property)
                            })
                        })
                        .map(|node| Edit::RemoveNode { node })
                        .collect();
                    output
                        .apply("Retain authored complex setter", duplicates)
                        .map_err(core_error)?;
                } else if before.contains_key(property) || after.contains_key(property) {
                    continue;
                }
            }
        }
        let source = if node.is_element() {
            fragment(document, node.id)?
        } else {
            document.valid_source()[node.span.start..node.span.end].into()
        };
        output
            .apply(
                "Retain state extension payload",
                vec![Edit::InsertChild {
                    parent: root,
                    index: output.ast().element_children(root).len(),
                    source,
                }],
            )
            .map_err(core_error)?;
    }
    Ok(output.source().into())
}
fn merge_state_collection(
    original: &str,
    generated: &str,
    key: &StateStyleKey,
    old: &VisualStateGroup,
    new: &VisualStateGroup,
) -> Result<String> {
    let mut output = Document::new(original).map_err(core_error)?;
    let desired = Document::new(generated).map_err(core_error)?;
    let before = state_properties(old, &key.to, &key.target);
    let after = state_properties(new, &key.to, &key.target);
    let before_start = key
        .from
        .as_deref()
        .map(|state| state_properties(old, state, &key.target))
        .unwrap_or_default();
    let after_start = key
        .from
        .as_deref()
        .map(|state| state_properties(new, state, &key.target))
        .unwrap_or_default();
    let old_timing = transition_timing(old, key);
    let new_timing = transition_timing(new, key);
    let mut wanted = BTreeMap::new();
    for node in desired.ast().element_children(desired.ast().root()) {
        if let Some(role) = state_program_role(&desired, node) {
            require(
                wanted.insert(role, fragment(&desired, node)?).is_none(),
                "Duplicate generated state program ownership",
            )?;
        }
    }
    let programs = output
        .ast()
        .element_children(output.ast().root())
        .into_iter()
        .filter(|id| {
            output
                .ast()
                .node(*id)
                .is_some_and(|node| node.local_name() == "Animation")
        })
        .collect::<Vec<_>>();
    let mut ownership = BTreeMap::new();
    for id in &programs {
        if let Some(role) = state_program_role(&output, *id) {
            require(!ownership.values().any(|existing| existing == &role), "Multiple programs claim the same state ownership; remove the duplicate ownership comment before editing the state")?;
            ownership.insert(*id, role);
        }
    }
    // Legacy generated programs predate explicit ownership. Adopt only an unambiguous
    // program whose properties all belong to this state's recorded metadata.
    if ownership.is_empty() {
        for id in &programs {
            let source = Document::new(fragment(&output, *id)?).map_err(core_error)?;
            let values = endpoints(&source);
            if !values.is_empty() && values.keys().all(|property| before.contains_key(property)) {
                let role = if values
                    .values()
                    .flatten()
                    .all(|endpoint| endpoint.value.trim_start().starts_with('{'))
                {
                    "expression"
                } else {
                    "literal"
                }
                .to_owned();
                require(!ownership.values().any(|existing| existing == &role), "Legacy state animations are ambiguous; identify the owned program before editing this state")?;
                ownership.insert(*id, role);
            }
        }
    }
    let mut empty = Timeline::new(new_timing.0)?;
    empty.fill = FillMode::Forward;
    empty.easing = new_timing.1.clone();
    let empty = empty.to_xaml(new.owner)?;
    let mut edits = Vec::new();
    for id in programs {
        let Some(role) = ownership.get(&id) else {
            continue;
        };
        let desired = wanted.remove(role).unwrap_or_else(|| empty.clone());
        let merged = merge_state_animation(
            &fragment(&output, id)?,
            &desired,
            &before,
            &after,
            &before_start,
            &after_start,
            old_timing.0 != new_timing.0,
            old_timing.1 != new_timing.1,
        )?;
        let merged = if state_program_role(&output, id).is_none() {
            state_program(&merged, role, "State animation")?
        } else {
            merged
        };
        edits.push(Edit::Text {
            span: output.ast().node(id).expect("program").span,
            replacement: merged,
        });
    }
    edits.sort_by_key(|edit| match edit {
        Edit::Text { span, .. } => std::cmp::Reverse(span.start),
        _ => std::cmp::Reverse(0),
    });
    output
        .apply("Patch owned programs in source order", edits)
        .map_err(core_error)?;
    let parent = output.ast().root();
    for source in wanted.into_values() {
        output
            .apply(
                "Add state program",
                vec![Edit::InsertChild {
                    parent,
                    index: output.ast().element_children(parent).len(),
                    source,
                }],
            )
            .map_err(core_error)?;
    }
    Ok(output.source().into())
}
#[derive(Clone)]
struct Endpoint {
    setter: NodeId,
    time: f64,
    value: String,
}
fn endpoints(document: &Document) -> BTreeMap<String, Vec<Endpoint>> {
    let root = document
        .ast()
        .node(document.ast().root())
        .expect("animation");
    let duration = root
        .attribute("Duration")
        .and_then(|value| crate::xaml::parse_time(value).ok())
        .unwrap_or(0.0);
    let mut result = BTreeMap::<String, Vec<Endpoint>>::new();
    for frame in document
        .ast()
        .element_children(root.id)
        .into_iter()
        .filter_map(|id| document.ast().node(id))
        .filter(|node| node.local_name() == "KeyFrame")
    {
        let time = frame
            .attribute("KeyTime")
            .and_then(|value| crate::xaml::parse_time(value).ok())
            .or_else(|| {
                frame
                    .attribute("Cue")
                    .and_then(|value| {
                        let value = value.trim();
                        if let Some(percent) = value.strip_suffix('%') {
                            percent.parse::<f64>().ok().map(|value| value / 100.0)
                        } else {
                            value.parse::<f64>().ok()
                        }
                    })
                    .map(|cue| cue * duration)
            });
        let Some(time) = time else {
            continue;
        };
        for setter in document
            .ast()
            .element_children(frame.id)
            .into_iter()
            .filter_map(|id| document.ast().node(id))
            .filter(|node| node.local_name() == "Setter")
        {
            if let Some(property) = setter.attribute("Property") {
                result.entry(property.into()).or_default().push(Endpoint {
                    setter: setter.id,
                    time,
                    value: setter.attribute("Value").unwrap_or("").into(),
                });
            }
        }
    }
    for values in result.values_mut() {
        values.sort_by(|left, right| left.time.total_cmp(&right.time));
    }
    result
}
#[allow(clippy::too_many_arguments)]
fn merge_state_animation(
    original: &str,
    generated: &str,
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
    before_start: &BTreeMap<String, String>,
    after_start: &BTreeMap<String, String>,
    retime: bool,
    reease: bool,
) -> Result<String> {
    let mut output = Document::new(original).map_err(core_error)?;
    let desired = Document::new(generated).map_err(core_error)?;
    let desired_root = desired.ast().node(desired.ast().root()).expect("root");
    let root = output.ast().root();
    let current = endpoints(&output);
    let wanted = endpoints(&desired);
    let mut edits = Vec::new();
    if retime {
        let old_duration = output
            .ast()
            .node(root)
            .and_then(|node| node.attribute("Duration"))
            .and_then(|value| crate::xaml::parse_time(value).ok())
            .unwrap_or(0.0);
        if let Some(value) = desired_root.attribute("Duration") {
            let new_duration = crate::xaml::parse_time(value)?;
            edits.push(Edit::SetAttribute {
                node: root,
                name: "Duration".into(),
                value: value.into(),
            });
            if old_duration > 0.0 {
                for frame in output
                    .ast()
                    .element_children(root)
                    .into_iter()
                    .filter_map(|id| output.ast().node(id))
                    .filter(|node| node.local_name() == "KeyFrame")
                {
                    if let Some(time) = frame
                        .attribute("KeyTime")
                        .and_then(|value| crate::xaml::parse_time(value).ok())
                    {
                        let time = time * (new_duration / old_duration);
                        crate::xaml::native_ticks(time)?;
                        edits.push(Edit::SetAttribute {
                            node: frame.id,
                            name: "KeyTime".into(),
                            value: crate::xaml::time_string(time),
                        });
                    }
                }
            }
        }
    }
    if reease {
        for child in output
            .ast()
            .element_children(root)
            .into_iter()
            .filter_map(|id| output.ast().node(id))
            .filter(|node| node.local_name() == "Animation.Easing")
        {
            edits.push(Edit::RemoveNode { node: child.id });
        }
        if let Some(value) = desired_root.attribute("Easing") {
            edits.push(Edit::SetAttribute {
                node: root,
                name: "Easing".into(),
                value: value.into(),
            });
        } else {
            edits.push(Edit::RemoveAttribute {
                node: root,
                name: "Easing".into(),
            });
            if let Some(child) = desired
                .ast()
                .element_children(desired_root.id)
                .into_iter()
                .filter_map(|id| desired.ast().node(id))
                .find(|node| node.local_name() == "Animation.Easing")
            {
                edits.push(Edit::InsertChild {
                    parent: root,
                    index: 0,
                    source: native_fragment(&output, root, &fragment(&desired, child.id)?)?,
                });
            }
        }
    }
    for (property, values) in &current {
        if before.contains_key(property) && !wanted.contains_key(property) {
            for value in values {
                edits.push(Edit::RemoveNode { node: value.setter });
            }
        }
    }
    let mut appended = Vec::new();
    for (property, values) in &wanted {
        let final_value = values.last().expect("endpoint");
        if let Some(existing) = current.get(property).and_then(|values| values.last()) {
            let desired_start = values
                .first()
                .filter(|value| value.time == 0.0 && values.len() > 1);
            let obsolete_start = current
                .get(property)
                .and_then(|values| values.first())
                .filter(|value| {
                    value.time == 0.0
                        && before_start.contains_key(property)
                        && before_start.get(property) != after_start.get(property)
                        && desired_start.is_none()
                });
            if let Some(start) = obsolete_start {
                edits.push(Edit::RemoveNode { node: start.setter });
                if start.setter == existing.setter {
                    appended.push(format!(
                        "<KeyFrame Cue=\"100%\"><Setter Property=\"{}\" Value=\"{}\"/></KeyFrame>",
                        escape(property),
                        escape(&final_value.value)
                    ));
                    continue;
                }
            }
            if before.get(property) != after.get(property) {
                replace_endpoint(&output, existing.setter, &final_value.value, &mut edits);
            }
            if before_start.get(property) != after_start.get(property) {
                if let Some(start) = values
                    .first()
                    .filter(|value| value.time == 0.0 && values.len() > 1)
                {
                    if let Some(existing) = current
                        .get(property)
                        .and_then(|values| values.first())
                        .filter(|value| value.time == 0.0)
                    {
                        replace_endpoint(&output, existing.setter, &start.value, &mut edits);
                    } else {
                        appended.push(format!("<KeyFrame Cue=\"0%\"><Setter Property=\"{}\" Value=\"{}\"/></KeyFrame>",escape(property),escape(&start.value)));
                    }
                }
            }
        } else {
            for value in values {
                let cue = if value.time == 0.0 && values.len() > 1 {
                    "0%"
                } else {
                    "100%"
                };
                appended.push(format!(
                    "<KeyFrame Cue=\"{cue}\"><Setter Property=\"{}\" Value=\"{}\"/></KeyFrame>",
                    escape(property),
                    escape(&value.value)
                ));
            }
        }
    }
    output
        .apply("Update state endpoints", edits)
        .map_err(core_error)?;
    for source in appended {
        let source = native_fragment(&output, root, &source)?;
        output
            .apply(
                "Add state endpoint",
                vec![Edit::InsertChild {
                    parent: root,
                    index: output.ast().element_children(root).len(),
                    source,
                }],
            )
            .map_err(core_error)?;
    }
    Ok(output.source().into())
}
fn replace_endpoint(document: &Document, setter: NodeId, value: &str, edits: &mut Vec<Edit>) {
    if let Some(node) = document.ast().node(setter) {
        for child in node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .filter(|node| node.local_name() == "Setter.Value")
        {
            edits.push(Edit::RemoveNode { node: child.id });
        }
    }
    edits.push(Edit::SetAttribute {
        node: setter,
        name: "Value".into(),
        value: value.into(),
    });
}

fn state_program(source: &str, role: &str, name: &str) -> Result<String> {
    let mut document =
        Document::new(crate::programs::named_source(source, name)?).map_err(core_error)?;
    document
        .apply(
            "Identify state-owned program",
            vec![Edit::InsertChild {
                parent: document.ast().root(),
                index: 0,
                source: format!("<!--{STATE_PROGRAM_MARKER}{role}-->"),
            }],
        )
        .map_err(core_error)?;
    Ok(document.source().into())
}
pub(crate) fn state_program_role(document: &Document, node: NodeId) -> Option<String> {
    document
        .ast()
        .node(node)?
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| node.kind == NodeKind::Comment)
        .filter_map(|node| node.text.as_deref())
        .find_map(|text| {
            text.trim()
                .strip_prefix(STATE_PROGRAM_MARKER)
                .map(str::to_owned)
        })
}

fn reserve_namespace(namespaces: &mut BTreeMap<String, String>, stem: &str, uri: &str) -> String {
    let mut suffix = 0;
    let name = loop {
        let name = format!("{stem}{suffix}");
        if !namespaces.contains_key(&name) {
            break name;
        }
        suffix += 1;
    };
    namespaces.insert(name.clone(), uri.to_owned());
    name
}
