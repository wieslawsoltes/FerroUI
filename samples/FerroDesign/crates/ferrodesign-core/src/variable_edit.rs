//! Typed local-variable authoring. Collection and mode controls share these
//! validated transactions with bindings; aliases cannot introduce cycles or
//! change a variable's effective type between modes.
use crate::{Color, Document, Editor, NodeId, NodeKind, Stroke, Variable, VariableId, VariableValue};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableKind { Color, Number, String, Boolean }

impl VariableKind {
    pub fn label(self) -> &'static str {
        match self { Self::Color => "Color", Self::Number => "Number", Self::String => "String", Self::Boolean => "Boolean" }
    }
    pub fn default_value(self) -> VariableValue {
        match self {
            Self::Color => VariableValue::Color(Color::BLACK),
            Self::Number => VariableValue::Number(0.0),
            Self::String => VariableValue::String(String::new()),
            Self::Boolean => VariableValue::Boolean(true),
        }
    }
}

pub fn variable_value_kind(value: &VariableValue) -> Option<VariableKind> {
    match value {
        VariableValue::Color(_) => Some(VariableKind::Color),
        VariableValue::Number(_) => Some(VariableKind::Number),
        VariableValue::String(_) => Some(VariableKind::String),
        VariableValue::Boolean(_) => Some(VariableKind::Boolean),
        VariableValue::Alias(_) => None,
    }
}

pub fn variable_binding_kind(property: &str) -> Option<VariableKind> {
    match property {
        "fill" | "stroke" => Some(VariableKind::Color),
        "width" | "height" | "opacity" | "corner_radius" | "font_size" => Some(VariableKind::Number),
        "text" => Some(VariableKind::String),
        "visible" => Some(VariableKind::Boolean),
        _ => crate::parse_layout_variable_binding(property).map(|_| VariableKind::Number)
            .or_else(|| crate::parse_layout_guide_binding(property).map(|(_, field)| field.kind())),
    }
}

#[derive(Clone, Debug)]
pub enum VariableEdit {
    /// A type change intentionally initializes every mode with the supplied
    /// value. Ordinary mode edits preserve every other mode unchanged.
    Upsert {
        id: Option<VariableId>, name: String, collection: String, mode: String,
        value: VariableValue, replace_all_modes: bool,
    },
    Delete { id: VariableId },
    RenameCollection { collection: String, name: String },
    DeleteCollection { collection: String },
    AddMode { name: String, copy_from: String },
    RenameMode { mode: String, name: String },
    DeleteMode { mode: String },
    ActivateMode { mode: String },
    Bind { nodes: Vec<NodeId>, property: String, variable: Option<VariableId> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableEditError(pub String);
impl std::fmt::Display for VariableEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) }
}
impl std::error::Error for VariableEditError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VariableEditResult { pub changed: bool, pub variable: Option<VariableId> }

fn error(message: impl Into<String>) -> VariableEditError { VariableEditError(message.into()) }
fn name(value: &str, subject: &str) -> Result<String, VariableEditError> {
    let value = value.trim();
    if value.is_empty() { return Err(error(format!("{subject} cannot be empty."))); }
    if value.chars().count() > 256 { return Err(error(format!("{subject} must be at most 256 characters."))); }
    Ok(value.to_string())
}

impl Document {
    pub fn variable_collections(&self) -> Vec<String> {
        self.variables.values().map(|variable| variable.collection.clone()).collect::<BTreeSet<_>>().into_iter().collect()
    }

    /// The current model has one global active mode. A mode operation therefore
    /// updates all collections together, and never pretends to store per-
    /// collection active modes that the document cannot represent.
    pub fn variable_modes(&self) -> Vec<String> {
        let mut modes = BTreeSet::from(["Default".to_string(), self.variable_mode.clone()]);
        for variable in self.variables.values() { modes.extend(variable.values.keys().cloned()); }
        modes.remove("");
        let mut result = vec!["Default".to_string()];
        modes.remove("Default");
        result.extend(modes);
        result
    }

    pub fn variable_kind(&self, variable: VariableId, mode: &str) -> Option<VariableKind> {
        self.resolve_variable(variable, mode).and_then(variable_value_kind)
    }

    /// Resolve each alias edge once for a table or a validation pass. Long
    /// shared alias chains do not require one complete walk per variable.
    pub fn variable_kinds(&self, mode: &str) -> Result<BTreeMap<VariableId, VariableKind>, VariableEditError> {
        let mut resolved = BTreeMap::new();
        let mut path = Vec::new();
        let mut visiting = BTreeSet::new();
        for &start in self.variables.keys() {
            if resolved.contains_key(&start) { continue; }
            path.clear();
            visiting.clear();
            let mut current = start;
            let kind = loop {
                if let Some(&kind) = resolved.get(&current) { break kind; }
                let variable = self.variables.get(&current).ok_or_else(|| error(format!("Alias target {current} is missing.")))?;
                if !visiting.insert(current) {
                    return Err(error(format!("{} has an alias cycle in {mode}.", variable.name)));
                }
                path.push(current);
                let value = raw_mode_value(variable, mode).ok_or_else(|| error(format!("{} has no value in {mode}.", variable.name)))?;
                match value {
                    VariableValue::Alias(target) => current = *target,
                    VariableValue::Number(value) if !value.is_finite() => {
                        return Err(error(format!("{} has a nonfinite value in {mode}.", variable.name)));
                    }
                    value => break variable_value_kind(value).unwrap(),
                }
            };
            for &id in &path { resolved.insert(id, kind); }
        }
        Ok(resolved)
    }

    /// Validate every globally addressable mode, including fallback through
    /// Default. This catches cycles created only by cross-mode alias fallback.
    pub fn validate_variables(&self) -> Result<(), VariableEditError> {
        name(&self.variable_mode, "Active mode")?;
        let modes = self.variable_modes();
        let mut names = BTreeSet::new();
        for (&id, variable) in &self.variables {
            if id == 0 || variable.id != id { return Err(error("A variable ID does not match its map key.")); }
            name(&variable.name, "Variable name")?;
            name(&variable.collection, "Collection name")?;
            if !names.insert((&variable.collection, &variable.name)) {
                return Err(error(format!("{} already contains a variable named {}.", variable.collection, variable.name)));
            }
            if variable.values.is_empty() { return Err(error(format!("{} needs at least one mode value.", variable.name))); }
            for (mode, value) in &variable.values {
                name(mode, "Mode name")?;
                if matches!(value, VariableValue::Number(value) if !value.is_finite()) {
                    return Err(error(format!("{} has a nonfinite value in {mode}.", variable.name)));
                }
                if let VariableValue::Alias(target) = value {
                    if !self.variables.contains_key(target) {
                        return Err(error(format!("{} refers to missing variable {target} in {mode}.", variable.name)));
                    }
                }
            }
        }
        let default_kinds = self.variable_kinds("Default")?;
        for mode in &modes {
            if mode == "Default" { continue; }
            for (id, kind) in self.variable_kinds(mode)? {
                if default_kinds.get(&id) != Some(&kind) {
                    return Err(error(format!("{} must resolve to the same type in every mode. Use Change type to reset all modes.", self.variables[&id].name)));
                }
            }
        }
        for node in self.nodes.values() {
            for (property, &id) in &node.variable_bindings {
                let expected = variable_binding_kind(property).ok_or_else(||
                    error(format!("{} has an unsupported variable binding: {property}.", node.name)))?;
                let actual = default_kinds.get(&id).copied().ok_or_else(||
                    error(format!("{} is bound to missing or unresolved variable {id}.", node.name)))?;
                if expected != actual {
                    return Err(error(format!("{} → {property} requires a {} variable. Unbind it before changing the variable's type.", node.name, expected.label())));
                }
                if matches!(property.as_str(), "text" | "font_size") && node.kind != NodeKind::Text {
                    return Err(error(format!("Only text layers can bind {property}.")));
                }
                if crate::parse_layout_guide_binding(property).is_some() {
                    for mode in &modes {
                        let value = self.resolve_variable(id, mode).ok_or_else(|| error("The guide variable no longer resolves."))?;
                        crate::validate_layout_guide_variable(node, property, value).map_err(VariableEditError)?;
                    }
                }
                if crate::parse_layout_variable_binding(property).is_some() {
                    for mode in &modes {
                        let value = self.resolve_variable(id, mode).ok_or_else(|| error("The layout variable no longer resolves."))?;
                        crate::validate_layout_variable(node, property, value).map_err(VariableEditError)?;
                    }
                }
            }
            for interaction in &node.interactions {
                if let crate::PrototypeAction::SetVariable { variable, value } = &interaction.action {
                    let expected = default_kinds.get(variable).ok_or_else(||
                        error(format!("{} has a prototype interaction using missing variable {variable}.", node.name)))?;
                    let actual = if let VariableValue::Alias(target) = value {
                        default_kinds.get(target).copied()
                    } else { variable_value_kind(value) };
                    if actual != Some(*expected) {
                        return Err(error(format!("{} has a prototype interaction whose variable value needs type {}.", node.name, expected.label())));
                    }
                    crate::validate_prototype_interaction(self, node.id, interaction).map_err(VariableEditError)?;
                }
            }
        }
        Ok(())
    }
}

impl Editor {
    /// A rejected edit leaves metadata, bound nodes, allocation and history
    /// untouched. Bound layout/text changes join the same undo transaction.
    pub fn edit_variables(&mut self, edit: VariableEdit) -> Result<VariableEditResult, VariableEditError> {
        if self.has_active_transaction() {
            return Err(error("Finish the current edit before changing variables."));
        }
        let mut candidate = self.document.clone();
        let (label, variable) = apply_edit(&mut candidate, edit)?;
        candidate.validate_variables()?;
        if candidate == self.document { return Ok(VariableEditResult { changed: false, variable }); }
        candidate.apply_variables();
        let layout = candidate.layout_all();
        if !layout.errors.is_empty() { return Err(error(layout.errors.join("\n"))); }
        candidate.validate().map_err(|error| VariableEditError(error.to_string()))?;
        let changed = candidate != self.document;
        if changed { self.transact(label, move |document| *document = candidate); }
        Ok(VariableEditResult { changed, variable })
    }
}

fn apply_edit(document: &mut Document, edit: VariableEdit) -> Result<(&'static str, Option<VariableId>), VariableEditError> {
    match edit {
        VariableEdit::Upsert { id, name: variable_name, collection, mode, value, replace_all_modes } => {
            let variable_name = name(&variable_name, "Variable name")?;
            let collection = name(&collection, "Collection name")?;
            let mode = name(&mode, "Mode name")?;
            if !document.variable_modes().contains(&mode) { return Err(error("Add this mode before assigning a value.")); }
            let modes = document.variable_modes();
            let id = if let Some(id) = id {
                let variable = document.variables.get_mut(&id).ok_or_else(|| error("The variable no longer exists."))?;
                variable.name = variable_name;
                variable.collection = collection;
                if replace_all_modes {
                    variable.values = modes.into_iter().map(|mode| (mode, value.clone())).collect();
                } else { variable.values.insert(mode, value); }
                id
            } else {
                let id = document.allocate_id();
                let values = modes.into_iter().map(|mode| (mode, value.clone())).collect();
                document.variables.insert(id, Variable { id, name: variable_name, collection, values });
                id
            };
            Ok(("Edit local variable", Some(id)))
        }
        VariableEdit::Delete { id } => {
            if !document.variables.contains_key(&id) { return Err(error("The variable no longer exists.")); }
            remove_variables(document, &BTreeSet::from([id]))?;
            Ok(("Delete local variable", None))
        }
        VariableEdit::RenameCollection { collection, name: new_name } => {
            let new_name = name(&new_name, "Collection name")?;
            if !document.variable_collections().contains(&collection) { return Err(error("The collection no longer exists.")); }
            if collection != new_name && document.variable_collections().contains(&new_name) {
                return Err(error("A collection with this name already exists."));
            }
            for variable in document.variables.values_mut().filter(|variable| variable.collection == collection) {
                variable.collection = new_name.clone();
            }
            Ok(("Rename variable collection", None))
        }
        VariableEdit::DeleteCollection { collection } => {
            let ids: BTreeSet<_> = document.variables.values().filter(|variable| variable.collection == collection).map(|variable| variable.id).collect();
            if ids.is_empty() { return Err(error("The collection no longer exists.")); }
            remove_variables(document, &ids)?;
            Ok(("Delete variable collection", None))
        }
        VariableEdit::AddMode { name: new_name, copy_from } => {
            let new_name = name(&new_name, "Mode name")?;
            if document.variable_modes().contains(&new_name) { return Err(error("A mode with this name already exists.")); }
            if !document.variable_modes().contains(&copy_from) { return Err(error("Choose an existing mode to copy.")); }
            for variable in document.variables.values_mut() {
                let value = raw_mode_value(variable, &copy_from).cloned().ok_or_else(|| error("A variable has no value to copy."))?;
                variable.values.insert(new_name.clone(), value);
            }
            document.variable_mode = new_name;
            Ok(("Add variable mode", None))
        }
        VariableEdit::RenameMode { mode, name: new_name } => {
            if mode == "Default" { return Err(error("Default is the fallback mode and cannot be renamed.")); }
            let new_name = name(&new_name, "Mode name")?;
            if !document.variable_modes().contains(&mode) { return Err(error("The mode no longer exists.")); }
            if mode != new_name && document.variable_modes().contains(&new_name) { return Err(error("A mode with this name already exists.")); }
            for variable in document.variables.values_mut() {
                if let Some(value) = variable.values.remove(&mode) { variable.values.insert(new_name.clone(), value); }
            }
            if document.variable_mode == mode { document.variable_mode = new_name; }
            Ok(("Rename variable mode", None))
        }
        VariableEdit::DeleteMode { mode } => {
            if mode == "Default" { return Err(error("Default is the fallback mode and cannot be deleted.")); }
            if !document.variable_modes().contains(&mode) { return Err(error("The mode no longer exists.")); }
            for variable in document.variables.values_mut() {
                // Imported variables may have no Default entry. Preserve their
                // current fallback before removing the only stored value.
                if variable.values.len() == 1 && variable.values.contains_key(&mode) {
                    let value = variable.values[&mode].clone();
                    variable.values.insert("Default".into(), value);
                }
                variable.values.remove(&mode);
            }
            if document.variable_mode == mode { document.variable_mode = "Default".into(); }
            Ok(("Delete variable mode", None))
        }
        VariableEdit::ActivateMode { mode } => {
            if !document.variable_modes().contains(&mode) { return Err(error("Choose an existing variable mode.")); }
            document.variable_mode = mode;
            Ok(("Change variable mode", None))
        }
        VariableEdit::Bind { nodes, property, variable } => {
            let expected = variable_binding_kind(&property).ok_or_else(|| error("This property does not support variable bindings."))?;
            if nodes.is_empty() { return Err(error("Select at least one layer to bind.")); }
            if let Some(id) = variable {
                if document.variable_kind(id, &document.variable_mode) != Some(expected) {
                    return Err(error(format!("{property} requires a {} variable.", expected.label())));
                }
            }
            for &id in &nodes {
                let node = document.nodes.get(&id).ok_or_else(|| error("A selected layer no longer exists."))?;
                if document.effective_locked(id) { return Err(error("Unlock the selected layers and their parents before binding.")); }
                if matches!(property.as_str(), "text" | "font_size") && node.kind != NodeKind::Text {
                    return Err(error(format!("Only text layers can bind {property}.")));
                }
                if crate::parse_layout_guide_binding(&property).is_some() {
                    crate::validate_layout_guide_binding_target(node, &property).map_err(VariableEditError)?;
                }
                if crate::parse_layout_variable_binding(&property).is_some() {
                    crate::validate_layout_variable_target(node, &property).map_err(VariableEditError)?;
                }
            }
            if nodes.iter().all(|id| document.nodes[id].variable_bindings.get(&property).copied() == variable) {
                return Ok(("Edit variable binding", variable));
            }
            // Unbinding freezes the currently resolved appearance.
            document.apply_variables();
            for id in nodes.into_iter().collect::<BTreeSet<_>>() {
                let node = document.nodes.get_mut(&id).unwrap();
                let layout_binding_changed = crate::parse_layout_variable_binding(&property).is_some()
                    && node.variable_bindings.get(&property).copied() != variable;
                if let Some(variable) = variable {
                    crate::style_edit::detach_style_for_variable(node, &property);
                    if property == "stroke" && node.strokes.is_empty() { node.strokes.push(Stroke::new(Color::BLACK, 1.0)); }
                    node.variable_bindings.insert(property.clone(), variable);
                } else { node.variable_bindings.remove(&property); }
                if layout_binding_changed && node.component_source.is_some() {
                    node.overridden_properties.insert("layout".into());
                }
            }
            Ok(("Edit variable binding", variable))
        }
    }
}

fn raw_mode_value<'a>(variable: &'a Variable, mode: &str) -> Option<&'a VariableValue> {
    variable.values.get(mode).or_else(|| variable.values.get("Default")).or_else(|| variable.values.values().next())
}

fn remove_variables(document: &mut Document, ids: &BTreeSet<VariableId>) -> Result<(), VariableEditError> {
    for variable in document.variables.values().filter(|variable| !ids.contains(&variable.id)) {
        for (mode, value) in &variable.values {
            if matches!(value, VariableValue::Alias(target) if ids.contains(target)) {
                return Err(error(format!("{} still aliases this variable in {mode}. Change or delete that alias first.", variable.name)));
            }
        }
    }
    for node in document.nodes.values() {
        if node.interactions.iter().any(|interaction| match &interaction.action {
            crate::PrototypeAction::SetVariable { variable, value } => ids.contains(variable)
                || matches!(value, VariableValue::Alias(target) if ids.contains(target)),
            _ => false,
        }) {
            return Err(error(format!("{} uses this variable in a prototype interaction. Update that interaction before deleting the variable.", node.name)));
        }
    }
    document.apply_variables();
    for node in document.nodes.values_mut() { node.variable_bindings.retain(|_, id| !ids.contains(id)); }
    document.variables.retain(|id, _| !ids.contains(id));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fill, Node, TextRun};
    fn add(editor: &mut Editor, name: &str, collection: &str, value: VariableValue) -> VariableId {
        editor.edit_variables(VariableEdit::Upsert { id: None, name: name.into(), collection: collection.into(),
            mode: editor.document.variable_mode.clone(), value, replace_all_modes: false }).unwrap().variable.unwrap()
    }
    fn save(editor: &mut Editor, id: VariableId, mode: &str, value: VariableValue, replace_all_modes: bool) -> Result<VariableEditResult, VariableEditError> {
        let variable = editor.document.variables[&id].clone();
        editor.edit_variables(VariableEdit::Upsert { id: Some(id), name: variable.name, collection: variable.collection,
            mode: mode.into(), value, replace_all_modes })
    }

    #[test]
    fn mode_values_and_aliases_apply_atomically_to_bindings_with_undo() {
        let mut doc = Document::new("Modes");
        let node = doc.insert_node(doc.pages[0].id, Node::new(NodeKind::Rectangle, "Shape", 0.0, 0.0, 10.0, 10.0)).unwrap();
        let mut editor = Editor::new(doc);
        let base = add(&mut editor, "Brand", "Colors", VariableValue::Color(Color::BLACK));
        let alias = add(&mut editor, "Primary", "Colors", VariableValue::Alias(base));
        editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "fill".into(), variable: Some(alias) }).unwrap();
        editor.edit_variables(VariableEdit::AddMode { name: "Dark".into(), copy_from: "Default".into() }).unwrap();
        save(&mut editor, base, "Dark", VariableValue::Color(Color::WHITE), false).unwrap();
        assert_eq!(editor.document.nodes[&node].fills, [Fill::solid(Color::WHITE)]);
        assert_eq!(editor.document.resolve_variable(alias, "Default"), Some(&VariableValue::Color(Color::BLACK)));
        assert!(editor.undo());
        assert_eq!(editor.document.nodes[&node].fills, [Fill::solid(Color::BLACK)]);
        assert!(editor.redo());
        assert_eq!(editor.document.nodes[&node].fills, [Fill::solid(Color::WHITE)]);
    }

    #[test]
    fn cyclic_fallback_aliases_and_type_changes_are_rejected_without_allocating_or_history() {
        let mut editor = Editor::new(Document::new("Aliases"));
        let a = add(&mut editor, "A", "Tokens", VariableValue::Number(1.0));
        let b = add(&mut editor, "B", "Tokens", VariableValue::Alias(a));
        editor.edit_variables(VariableEdit::AddMode { name: "Dark".into(), copy_from: "Default".into() }).unwrap();
        let before = editor.document.clone();
        let history = editor.history_len();
        assert!(save(&mut editor, a, "Dark", VariableValue::Alias(b), false).is_err());
        assert!(save(&mut editor, a, "Dark", VariableValue::String("wrong type".into()), false).is_err());
        assert!(editor.edit_variables(VariableEdit::Upsert { id: None, name: "Missing".into(), collection: "Tokens".into(),
            mode: "Dark".into(), value: VariableValue::Alias(u64::MAX), replace_all_modes: false }).is_err());
        assert_eq!(editor.document, before);
        assert_eq!(editor.history_len(), history);
    }

    #[test]
    fn changing_type_resets_all_modes_and_bound_incompatible_properties_block_it() {
        let mut editor = Editor::new(Document::new("Types"));
        let value = add(&mut editor, "Value", "Tokens", VariableValue::Number(1.0));
        editor.edit_variables(VariableEdit::AddMode { name: "Dark".into(), copy_from: "Default".into() }).unwrap();
        save(&mut editor, value, "Dark", VariableValue::Boolean(false), true).unwrap();
        assert!(editor.document.variables[&value].values.values().all(|value| value == &VariableValue::Boolean(false)));
        let node = editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 10.0, 10.0);
        editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "visible".into(), variable: Some(value) }).unwrap();
        assert!(save(&mut editor, value, "Dark", VariableValue::String("new".into()), true).is_err());
        assert_eq!(editor.document.variable_kind(value, "Dark"), Some(VariableKind::Boolean));
    }

    #[test]
    fn deletion_detaches_bindings_preserves_appearance_and_blocks_external_aliases() {
        let mut editor = Editor::new(Document::new("Delete"));
        let node = editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 10.0, 10.0);
        let base = add(&mut editor, "Brand", "Colors", VariableValue::Color(Color::WHITE));
        let alias = add(&mut editor, "Alias", "External", VariableValue::Alias(base));
        editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "fill".into(), variable: Some(base) }).unwrap();
        assert!(editor.edit_variables(VariableEdit::DeleteCollection { collection: "Colors".into() }).is_err());
        editor.edit_variables(VariableEdit::Delete { id: alias }).unwrap();
        editor.edit_variables(VariableEdit::Delete { id: base }).unwrap();
        assert!(editor.document.nodes[&node].variable_bindings.is_empty());
        assert_eq!(editor.document.nodes[&node].fills, [Fill::solid(Color::WHITE)]);
        assert!(editor.undo());
        assert_eq!(editor.document.nodes[&node].variable_bindings["fill"], base);
        assert!(editor.document.variables.contains_key(&base));
    }

    #[test]
    fn collection_and_global_mode_renames_preserve_ids_and_default_fallback() {
        let mut editor = Editor::new(Document::new("Collections"));
        let a = add(&mut editor, "Spacing", "Layout", VariableValue::Number(8.0));
        let b = add(&mut editor, "Label", "Content", VariableValue::String("Hi".into()));
        editor.edit_variables(VariableEdit::AddMode { name: "Compact".into(), copy_from: "Default".into() }).unwrap();
        editor.edit_variables(VariableEdit::RenameCollection { collection: "Layout".into(), name: "Dimensions".into() }).unwrap();
        editor.edit_variables(VariableEdit::RenameMode { mode: "Compact".into(), name: "Small".into() }).unwrap();
        assert_eq!(editor.document.variables[&a].collection, "Dimensions");
        assert!(editor.document.variables[&a].values.contains_key("Small"));
        assert!(editor.document.variables[&b].values.contains_key("Small"));
        assert_eq!(editor.document.variable_mode, "Small");
        editor.edit_variables(VariableEdit::DeleteMode { mode: "Small".into() }).unwrap();
        assert_eq!(editor.document.variable_mode, "Default");
        assert_eq!(editor.document.variable_modes(), ["Default"]);
        assert!(editor.edit_variables(VariableEdit::DeleteMode { mode: "Default".into() }).is_err());
    }

    #[test]
    fn binding_text_replacements_preserves_rich_ranges_and_unbinding_freezes_content() {
        let mut doc = Document::new("Content");
        let mut text = Node::new(NodeKind::Text, "Label", 0.0, 0.0, 100.0, 30.0);
        text.text = "A🦀B".into();
        text.text_runs = vec![TextRun { start: 1, end: 5, font_family: None, font_size: None,
            font_weight: Some(700), color: None, italic: None, decoration: None }];
        let node = doc.insert_node(doc.pages[0].id, text).unwrap();
        let mut editor = Editor::new(doc);
        let label = add(&mut editor, "Label", "Content", VariableValue::String("AéB".into()));
        editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "text".into(), variable: Some(label) }).unwrap();
        assert_eq!((editor.document.nodes[&node].text_runs[0].start, editor.document.nodes[&node].text_runs[0].end), (1, 3));
        editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "text".into(), variable: None }).unwrap();
        save(&mut editor, label, "Default", VariableValue::String("Changed".into()), false).unwrap();
        assert_eq!(editor.document.nodes[&node].text, "AéB");
        editor.document.validate().unwrap();
    }

    #[test]
    fn binding_validation_handles_new_strokes_locked_nodes_and_wrong_types_atomically() {
        let mut editor = Editor::new(Document::new("Bindings"));
        let node = editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 10.0, 10.0);
        let color = add(&mut editor, "Color", "Tokens", VariableValue::Color(Color::WHITE));
        assert!(editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "width".into(), variable: Some(color) }).is_err());
        editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "stroke".into(), variable: Some(color) }).unwrap();
        assert_eq!(editor.document.nodes[&node].strokes, [Stroke::new(Color::WHITE, 1.0)]);
        editor.document.nodes.get_mut(&node).unwrap().locked = true;
        let before = editor.document.clone();
        assert!(editor.edit_variables(VariableEdit::Bind { nodes: vec![node], property: "fill".into(), variable: Some(color) }).is_err());
        assert_eq!(editor.document, before);
    }

    #[test]
    fn duplicate_names_nonfinite_values_and_repeated_noops_do_not_create_history() {
        let mut editor = Editor::new(Document::new("Validation"));
        let id = add(&mut editor, "Value", "Tokens", VariableValue::Number(2.0));
        let history = editor.history_len();
        assert!(!save(&mut editor, id, "Default", VariableValue::Number(2.0), false).unwrap().changed);
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(save(&mut editor, id, "Default", VariableValue::Number(value), false).is_err());
        }
        assert!(editor.edit_variables(VariableEdit::Upsert { id: None, name: "Value".into(), collection: "Tokens".into(),
            mode: "Default".into(), value: VariableValue::Number(3.0), replace_all_modes: false }).is_err());
        assert_eq!(editor.history_len(), history);
        assert_eq!(editor.document.variables.len(), 1);
    }

    #[test]
    fn shared_long_alias_chains_resolve_iteratively_and_fallback_cycles_are_detected() {
        let mut document = Document::new("Shared aliases");
        for index in 0..5000 {
            let id = document.allocate_id();
            document.variables.insert(id, Variable { id, name: format!("Value {id}"), collection: "Tokens".into(),
                values: BTreeMap::from([("Default".into(), if index == 4999 {
                    VariableValue::Number(8.0)
                } else { VariableValue::Alias(id + 1) })]) });
        }
        let first = *document.variables.keys().next().unwrap();
        let last = *document.variables.keys().next_back().unwrap();
        document.variable_mode = "Compact".into();
        let kinds = document.variable_kinds("Compact").unwrap();
        assert_eq!(kinds.len(), 5000);
        assert!(kinds.values().all(|kind| *kind == VariableKind::Number));
        document.validate_variables().unwrap();
        document.variables.get_mut(&last).unwrap().values.insert("Compact".into(), VariableValue::Alias(first));
        assert!(document.variable_kinds("Compact").is_err());
        assert!(document.validate_variables().is_err());
        assert!(document.variable_kinds("Default").is_ok());
    }

    #[test]
    fn deleting_variables_used_by_prototype_actions_is_atomic() {
        let mut editor = Editor::new(Document::new("Prototype variables"));
        let id = add(&mut editor, "Visible", "Tokens", VariableValue::Boolean(true));
        let node = editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 10.0, 10.0);
        editor.document.nodes.get_mut(&node).unwrap().interactions = vec![crate::Interaction {
            trigger: crate::Trigger::Click,
            action: crate::PrototypeAction::SetVariable { variable: id, value: VariableValue::Boolean(false) },
            transition: crate::Transition::default(), delay_ms: 0, key: None,
        }];
        let before = editor.document.clone();
        let history = editor.history_len();
        assert!(editor.edit_variables(VariableEdit::Delete { id }).unwrap_err().to_string().contains("prototype interaction"));
        assert_eq!(editor.document, before);
        assert_eq!(editor.history_len(), history);
    }
}
