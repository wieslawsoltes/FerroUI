//! State selection and PropertyChanges share the same journalled overlays as Binding.
use super::*;
use crate::transaction::Transaction;

const ACTIVE: &str = "$qml:currentState";
const APPLIED: &str = "$qml:appliedChanges";

impl TypeDefinition {
    /// Add Qt Quick Item's state group to a custom native item definition.
    pub fn with_quick_states(self) -> Self {
        self.property("state", "string", Value::String(String::new()))
            .property("states", "list<State>", Value::List(vec![]))
            .property("transitions", "list<Transition>", Value::List(vec![]))
            .readonly_property(ACTIVE, "QtObject", Value::Null)
            .readonly_property(APPLIED, "list<QtObject>", Value::List(vec![]))
    }
}

impl TypeRegistry {
    pub fn register_quick_states(&mut self) -> Result<(), Error> {
        let object = || TypeDefinition::default().inherits("QtQml", "QtObject");
        crate::animations::register(self)?;
        self.register("QtQuick", "StateGroup", object().with_quick_states())?;
        self.register(
            "QtQuick",
            "State",
            object()
                .property("name", "string", Value::String(String::new()))
                .property("extend", "string", Value::String(String::new()))
                .property("when", "bool", Value::Bool(false))
                .property("changes", "list<QtObject>", Value::List(vec![]))
                .default_property("changes"),
        )?;
        self.register(
            "QtQuick",
            "PropertyChanges",
            object()
                .property("target", "QtObject", Value::Null)
                .property("explicit", "bool", Value::Bool(false))
                .property("restoreEntryValues", "bool", Value::Bool(true)),
        )?;
        self.register(
            "QtQuick",
            "Transition",
            object()
                .property("from", "string", Value::String("*".into()))
                .property("to", "string", Value::String("*".into()))
                .property("enabled", "bool", Value::Bool(true))
                .property("reversible", "bool", Value::Bool(false))
                .readonly_property("running", "bool", Value::Bool(false))
                .property("animations", "list<QtObject>", Value::List(vec![]))
                .default_property("animations"),
        )
    }
}

#[derive(Clone)]
pub(crate) struct Declaration {
    path: String,
    input: SlotId,
    expression: Rc<Binding>,
}

impl Instance {
    pub(crate) fn state_input(
        &mut self,
        owner: ObjectId,
        path: &str,
        binding: &Binding,
        scope: Option<u32>,
    ) -> Result<(), Error> {
        if !matches!(binding, Binding::Expr(_) | Binding::Script(_)) {
            return Err(Error::new("PropertyChanges requires a value expression"));
        }
        let input = self.allocate_property(
            Property {
                owner,
                name: format!("$qml:change:{path}"),
                ty: "var".into(),
                alias: None,
                readonly: true,
                required: false,
            },
            Value::Undefined,
        )?;
        self.slots[input].scope = scope;
        self.dynamic
            .state_changes
            .entry(owner)
            .or_default()
            .push(Declaration {
                input,
                path: path.into(),
                expression: Rc::new(binding.clone()),
            });
        Ok(())
    }

    pub(crate) fn state_objects(&self, owner: ObjectId, name: &str) -> Vec<ObjectId> {
        match self.key(owner, name).ok().map(|key| &self.slots[key].value) {
            Some(Value::List(items)) => items
                .iter()
                .filter_map(|v| match v {
                    Value::Object(id) if self.node(*id).is_some() => Some(*id),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        }
    }

    fn state_name(&self, state: ObjectId) -> String {
        match self.get(state, "name") {
            Ok(Value::String(name)) => name,
            _ => String::new(),
        }
    }

    fn state_operations(
        &self,
        states: &[ObjectId],
        selected: Option<ObjectId>,
    ) -> Result<Vec<ObjectId>, Error> {
        let mut current = selected;
        let mut chain = vec![];
        while let Some(state) = current {
            if chain.contains(&state) {
                return Err(Error::new("cyclic State.extend"));
            }
            chain.push(state);
            let Value::String(base) = self.get(state, "extend")? else {
                unreachable!()
            };
            if base.is_empty() {
                break;
            }
            current = Some(
                *states
                    .iter()
                    .find(|&&s| self.state_name(s) == base)
                    .ok_or_else(|| Error::new(format!("unknown base state '{base}'")))?,
            );
        }
        Ok(chain
            .into_iter()
            .rev()
            .flat_map(|s| self.state_objects(s, "changes"))
            .collect())
    }

    fn change_target(
        &self,
        change: ObjectId,
        decl: &Declaration,
    ) -> Result<(ObjectId, SlotId), Error> {
        let grouped = decl
            .path
            .split_once('.')
            .and_then(|(id, path)| self.lexical_id(change, id).map(|target| (target, path)));
        let (receiver, target, path) = if let Some((target, path)) = grouped {
            (change, target, path)
        } else {
            let Value::Object(target) = self.get(change, "target")? else {
                return Err(Error::new(format!(
                    "PropertyChanges has no target for '{}'",
                    decl.path
                )));
            };
            (target, target, decl.path.as_str())
        };
        let key = self.key_in(target, path, self.slots[decl.input].scope)?;
        Ok((receiver, self.write_target(key)?))
    }
}

impl Transaction<'_> {
    pub(crate) fn initialize_states(&mut self, first: usize) {
        for entry in &self.model.nodes[first..] {
            if entry.properties.contains_key(ACTIVE)
                && !self.model.state_objects(entry.id, "states").is_empty()
            {
                self.state_pending.insert(entry.id, true);
            }
        }
    }

    pub(crate) fn observe_states(&mut self, key: SlotId) {
        let prop = &self.model.properties[key];
        let node = &self.model.nodes[prop.owner.index];
        if matches!(prop.name.as_str(), "state" | "states") && node.properties.contains_key(ACTIVE)
        {
            self.state_pending.insert(prop.owner, prop.name == "states");
        } else if node.module == "QtQuick"
            && node.name == "State"
            && matches!(prop.name.as_str(), "when" | "changes" | "extend" | "name")
        {
            if let Some(group) = node
                .parent
                .filter(|g| self.model.nodes[g.index].properties.contains_key(ACTIVE))
            {
                self.state_pending.entry(group).or_insert(true);
            }
        }
    }

    pub(crate) fn stage_state(&mut self) -> Result<bool, Error> {
        let Some((owner, auto)) = self.state_pending.pop_first() else {
            return Ok(false);
        };
        self.budget()?;
        if self.model.node(owner).is_none() {
            return Ok(true);
        }
        let states = self.model.state_objects(owner, "states");
        let state_key = self.model.key(owner, "state")?;
        let Value::String(mut name) = self.model.slots[state_key].value.clone() else {
            unreachable!()
        };
        for (index, &state) in states.iter().enumerate() {
            if self.model.state_name(state).is_empty() {
                let key = self.model.key(state, "name")?;
                self.set_value(key, Value::String(format!("anonymousState{}", index + 1)));
            }
        }
        let find =
            |m: &Instance, name: &str| states.iter().copied().find(|&s| m.state_name(s) == name);
        let mut selected = find(self.model, &name);
        if auto {
            if let Some(s) = states
                .iter()
                .copied()
                .find(|&s| self.model.get(s, "when") == Ok(Value::Bool(true)))
            {
                selected = Some(s);
                name = self.model.state_name(s);
            } else if selected.is_some_and(|s| self.model.is_assigned(s, "when")) {
                selected = None;
                name.clear();
            }
            self.set_value(state_key, Value::String(name));
        }
        let active = self.model.key(owner, ACTIVE)?;
        let applied = self.model.key(owner, APPLIED)?;
        let old = self.model.state_objects(owner, APPLIED);
        let next = self.model.state_operations(&states, selected)?;
        let selected_value = selected.map_or(Value::Null, Value::Object);
        if self.model.slots[active].value == selected_value && old == next {
            return Ok(true);
        }
        let mut targets = BTreeSet::new();
        let mut reactive = BTreeSet::new();
        for &change in &old {
            for decl in self
                .model
                .dynamic
                .state_changes
                .get(&change)
                .into_iter()
                .flatten()
            {
                if let Some(target) = self.model.slots[decl.input].active_target {
                    targets.insert(target);
                    if self.model.slots[target]
                        .overlay
                        .as_ref()
                        .is_some_and(|o| o.restores_expression())
                    {
                        reactive.insert(target);
                    }
                }
            }
        }
        for &change in &next {
            for decl in self
                .model
                .dynamic
                .state_changes
                .get(&change)
                .into_iter()
                .flatten()
            {
                let target = self.model.change_target(change, decl)?.1;
                targets.insert(target);
                if !matches!(decl.expression.as_ref(),Binding::Expr(e) if e.as_literal().is_some())
                    && self.model.get(change, "explicit")? != Value::Bool(true)
                {
                    reactive.insert(target);
                } else {
                    reactive.remove(&target);
                }
            }
        }
        let old_name = match self.model.slots[active].value {
            Value::Object(id) => self.model.state_name(id),
            _ => String::new(),
        };
        let new_name = selected
            .map(|s| self.model.state_name(s))
            .unwrap_or_default();
        self.transition(owner, &old_name, &new_name, &targets, &reactive);
        let shared = old.iter().zip(&next).take_while(|(a, b)| a == b).count();
        for &change in old[shared..].iter().rev() {
            self.exit_change(change)?;
        }
        for &change in &next[shared..] {
            self.enter_change(change)?;
        }
        self.set_value(active, selected_value);
        self.set_value(
            applied,
            Value::List(next.into_iter().map(Value::Object).collect()),
        );
        Ok(true)
    }

    pub(crate) fn exit_change(&mut self, change: ObjectId) -> Result<(), Error> {
        let mode = if self.model.get(change, "restoreEntryValues")? == Value::Bool(true) {
            3
        } else {
            0
        };
        for decl in self
            .model
            .dynamic
            .state_changes
            .get(&change)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .rev()
        {
            let input = &self.model.slots[decl.input];
            let target = input.active_target;
            let scope = input.scope;
            let retained = (mode == 0 && !matches!(decl.expression.as_ref(), Binding::Expr(expr) if expr.as_literal().is_some()))
                .then(|| input.binding.clone()).flatten();
            self.update_overlay(decl.input, None, false, mode);
            if let (Some(target), Some(binding)) = (target, retained) {
                if self.model.slots[target].overlay.is_none() {
                    // Keep the expression as an ordinary, replaceable target binding.
                    let slot = self.touched(target);
                    slot.scope = scope;
                    slot.binding = Some(binding);
                    slot.dirty = true;
                    self.pending.insert(target);
                }
            }
            self.replace_reads(decl.input, BTreeSet::new());
            self.touched(decl.input).binding = None;
            self.touched(decl.input).dirty = false;
            self.pending.remove(&decl.input);
        }
        Ok(())
    }

    fn enter_change(&mut self, change: ObjectId) -> Result<(), Error> {
        if self.model.nodes[change.index].name != "PropertyChanges" {
            return Err(Error::new("unsupported state operation"));
        }
        let explicit = self.model.get(change, "explicit")? == Value::Bool(true);
        for decl in self
            .model
            .dynamic
            .state_changes
            .get(&change)
            .cloned()
            .unwrap_or_default()
        {
            let (receiver, target) = self.model.change_target(change, &decl)?;
            let slot = self.touched(decl.input);
            slot.binding = Some(InstalledBinding::StateExpression {
                declaration: change,
                receiver,
                expression: decl.expression,
            });
            slot.dirty = true;
            self.pending.insert(decl.input);
            if explicit {
                self.read(decl.input, &mut BTreeSet::new())?;
                self.replace_reads(decl.input, BTreeSet::new());
                self.touched(decl.input).binding = None;
            }
            self.update_overlay(decl.input, Some(target), false, 3);
        }
        Ok(())
    }
}
