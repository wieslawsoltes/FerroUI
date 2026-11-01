//! Qt Quick Controls button state is shared by native input, bindings and scripts.
//! Membership and selection live in ordinary journaled slots.
use super::*;
use crate::transaction::Transaction;
use std::collections::VecDeque;

const MEMBER: &str = "ButtonGroup.group";
const GROUP: &str = "$qml:buttonGroup";

impl TypeDefinition {
    /// Shared button API for a host that maps buttons to native controls.
    pub fn quick_button(mut self, name: &str) -> Self {
        self = self
            .property("checked", "bool", Value::Bool(false))
            .property("checkable", "bool", Value::Bool(name != "Button"))
            .property("autoExclusive", "bool", Value::Bool(name == "RadioButton"))
            .property_group("ButtonGroup")
            .property(MEMBER, "QtObject", Value::Null)
            .signal("clicked", vec![])
            .signal("toggled", vec![]);
        if name == "CheckBox" {
            self = self
                .property("tristate", "bool", Value::Bool(false))
                .property("checkState", "int", Value::Number(0.));
        }
        if name == "Switch" {
            self = self
                .readonly_property("position", "real", Value::Number(0.))
                .readonly_property("visualPosition", "real", Value::Number(0.));
        }
        for method in ["click", "toggle"] {
            self = self.context_method(method, vec![], "var", move |ctx, args| {
                ctx.quick_button_call(method, args)
            });
        }
        self
    }

    pub fn quick_button_group() -> Self {
        let button = Parameter {
            name: "button".into(),
            ty: Some("QtObject".into()),
        };
        let mut ty = Self::default()
            .inherits("QtQml", "QtObject")
            .readonly_property(GROUP, "bool", Value::Bool(true))
            .property("buttons", "list<QtObject>", Value::List(vec![]))
            .property("exclusive", "bool", Value::Bool(true))
            .property("checkedButton", "QtObject", Value::Null)
            .property("checkState", "int", Value::Number(0.))
            .signal("clicked", vec![button.clone()]);
        for method in ["addButton", "removeButton"] {
            ty = ty.context_method(method, vec![button.clone()], "var", move |ctx, args| {
                ctx.quick_button_call(method, args)
            });
        }
        ty
    }
}

#[derive(Default)]
pub(crate) struct Changes {
    pending: VecDeque<(ObjectId, String, Value)>,
    syncing: bool,
}

fn yes(model: &Instance, id: ObjectId, prop: &str) -> bool {
    model.get(id, prop) == Ok(Value::Bool(true))
}
fn object(value: Value) -> Option<ObjectId> {
    if let Value::Object(o) = value {
        Some(o)
    } else {
        None
    }
}
fn group(model: &Instance, id: ObjectId) -> Option<ObjectId> {
    model.get(id, MEMBER).ok().and_then(object)
}
fn members(model: &Instance, id: ObjectId) -> Vec<ObjectId> {
    match model.get(id, "buttons") {
        Ok(Value::List(list)) => list
            .into_iter()
            .filter_map(object)
            .filter(|o| model.node(*o).is_some())
            .collect(),
        _ => Vec::new(),
    }
}
impl Transaction<'_> {
    pub(crate) fn initialize_quick_buttons(&mut self, first: usize) -> Result<(), Error> {
        // Literal assignments have already populated their slots. Seed derived
        // properties before bindings or completion handlers observe these buttons.
        for index in first..self.model.nodes.len() {
            let id = self.model.nodes[index].id;
            if !self.model.nodes[index].properties.contains_key(MEMBER) {
                continue;
            }
            let key = if self.model.is_assigned(id, "checkState") {
                self.model.key(id, "checkState")?
            } else if self.model.key(id, "position").is_ok()
                || self.model.key(id, "checkState").is_ok()
            {
                self.model.key(id, "checked")?
            } else {
                continue;
            };
            self.observe_quick_button(key);
        }
        Ok(())
    }

    pub(crate) fn observe_quick_button(&mut self, key: SlotId) {
        if self.quick_buttons.syncing {
            return;
        }
        let p = &self.model.properties[key];
        if matches!(
            p.name.as_str(),
            "checked"
                | "checkState"
                | "buttons"
                | "exclusive"
                | "checkedButton"
                | MEMBER
                | "LayoutMirroring.enabled"
        ) {
            let props = &self.model.nodes[p.owner.index].properties;
            if props.contains_key(MEMBER) || props.contains_key(GROUP) {
                self.quick_buttons.pending.push_back((
                    p.owner,
                    p.name.clone(),
                    self.model.slots[key].value.clone(),
                ));
            }
        }
    }
    fn button_set(&mut self, id: ObjectId, name: &str, value: Value) -> Result<(), Error> {
        let key = self.model.key(id, name)?;
        self.set_value(key, value);
        Ok(())
    }
    fn set_checked(&mut self, id: ObjectId, checked: bool) -> Result<(), Error> {
        self.button_set(id, "checked", Value::Bool(checked))?;
        if self.model.key(id, "checkState").is_ok() {
            self.button_set(
                id,
                "checkState",
                Value::Number(if checked { 2. } else { 0. }),
            )?;
        }
        if self.model.key(id, "position").is_ok() {
            self.button_set(id, "position", Value::Number(f64::from(checked)))?;
            self.button_set(
                id,
                "visualPosition",
                Value::Number(f64::from(
                    checked ^ yes(self.model, id, "LayoutMirroring.enabled"),
                )),
            )?;
        }
        Ok(())
    }

    fn group_summary(&mut self, g: ObjectId, winner: Option<ObjectId>) -> Result<(), Error> {
        let list = members(self.model, g);
        let exclusive = yes(self.model, g, "exclusive");
        let selected = winner
            .filter(|o| list.contains(o) && yes(self.model, *o, "checked"))
            .or_else(|| {
                object(self.model.get(g, "checkedButton").ok()?)
                    .filter(|o| list.contains(o) && yes(self.model, *o, "checked"))
            })
            .or_else(|| {
                list.iter()
                    .copied()
                    .find(|o| yes(self.model, *o, "checked"))
            });
        if exclusive {
            for &id in &list {
                if Some(id) != selected && yes(self.model, id, "checked") {
                    self.set_checked(id, false)?;
                }
            }
            self.button_set(
                g,
                "checkedButton",
                selected.map_or(Value::Null, Value::Object),
            )?;
        }
        let checked = list
            .iter()
            .filter(|o| yes(self.model, **o, "checked"))
            .count();
        let partial = list
            .iter()
            .any(|o| self.model.get(*o, "checkState") == Ok(Value::Number(1.)));
        let state = if checked == 0 && !partial {
            0.
        } else if checked == list.len() {
            2.
        } else {
            1.
        };
        self.button_set(g, "checkState", Value::Number(state))
    }

    fn move_button(
        &mut self,
        id: ObjectId,
        old: Option<ObjectId>,
        next: Option<ObjectId>,
    ) -> Result<(), Error> {
        if self.model.key(id, MEMBER).is_err() {
            return Err(Error::new("ButtonGroup expects a button"));
        }
        if let Some(g) = next {
            if self.model.key(g, GROUP).is_err() {
                return Err(Error::new("invalid ButtonGroup"));
            }
        }
        if let Some(g) = old.filter(|g| Some(*g) != next && self.model.node(*g).is_some()) {
            let list = members(self.model, g)
                .into_iter()
                .filter(|o| *o != id)
                .map(Value::Object)
                .collect();
            self.button_set(g, "buttons", Value::List(list))?;
            self.group_summary(g, None)?;
        }
        self.button_set(id, MEMBER, next.map_or(Value::Null, Value::Object))?;
        if let Some(g) = next {
            let mut list = members(self.model, g);
            if !list.contains(&id) {
                list.push(id);
                self.button_set(
                    g,
                    "buttons",
                    Value::List(list.into_iter().map(Value::Object).collect()),
                )?;
            }
            self.group_summary(g, Some(id))?;
        }
        Ok(())
    }

    fn update_button(&mut self, id: ObjectId, name: &str, old: Value) -> Result<(), Error> {
        let is_group = self.model.key(id, GROUP).is_ok();
        match (is_group, name) {
            (true, "buttons") => {
                let list = members(self.model, id);
                if let Value::List(previous) = old {
                    for o in previous
                        .into_iter()
                        .filter_map(object)
                        .filter(|o| !list.contains(o))
                    {
                        if group(self.model, o) == Some(id) {
                            self.move_button(o, Some(id), None)?;
                        }
                    }
                }
                let mut unique = BTreeSet::new();
                for o in list {
                    if unique.insert(o) {
                        self.move_button(o, group(self.model, o), Some(id))?;
                    }
                }
                // Remove retired handles and entries repeated in a declaration.
                let mut seen = BTreeSet::new();
                let list = members(self.model, id)
                    .into_iter()
                    .filter(|o| seen.insert(*o))
                    .map(Value::Object)
                    .collect();
                self.button_set(id, "buttons", Value::List(list))?;
                self.group_summary(id, None)
            }
            (true, "exclusive") => self.group_summary(id, None),
            (true, "checkState") => {
                let value = self.model.get(id, "checkState")?;
                if !yes(self.model, id, "exclusive")
                    && (value == Value::Number(0.) || value == Value::Number(2.))
                {
                    for o in members(self.model, id) {
                        self.set_checked(o, value == Value::Number(2.))?;
                    }
                } else if yes(self.model, id, "exclusive") && value == Value::Number(0.) {
                    if let Some(o) = object(self.model.get(id, "checkedButton")?) {
                        self.set_checked(o, false)?;
                    }
                }
                self.group_summary(id, None)
            }
            (true, "checkedButton") => {
                let selected = object(self.model.get(id, "checkedButton")?);
                if let Some(o) = selected.filter(|o| members(self.model, id).contains(o)) {
                    self.set_checked(o, true)?;
                }
                if selected.is_none() {
                    if let Some(o) = object(old) {
                        if self.model.node(o).is_some() {
                            self.set_checked(o, false)?;
                        }
                    }
                }
                self.group_summary(id, selected)
            }
            (false, MEMBER) => self.move_button(id, object(old), group(self.model, id)),
            (false, "checked" | "checkState" | "LayoutMirroring.enabled") => {
                if name == "checkState" {
                    let state = self.model.get(id, "checkState")?;
                    if !matches!(state, Value::Number(0. | 1. | 2.)) {
                        return Err(Error::new("invalid check state"));
                    }
                    self.button_set(id, "checked", Value::Bool(state == Value::Number(2.)))?;
                } else if name == "checked" || self.model.key(id, "position").is_ok() {
                    self.set_checked(id, yes(self.model, id, "checked"))?;
                }
                if let Some(g) = group(self.model, id) {
                    self.group_summary(g, Some(id))?;
                } else if yes(self.model, id, "checked") && yes(self.model, id, "autoExclusive") {
                    if let Some(parent) = self.model.parent(id) {
                        for other in self.model.children(parent).to_vec() {
                            if other != id
                                && group(self.model, other).is_none()
                                && yes(self.model, other, "autoExclusive")
                                && yes(self.model, other, "checked")
                            {
                                self.set_checked(other, false)?;
                            }
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn stage_button(&mut self) -> Result<bool, Error> {
        let Some((id, name, old)) = self.quick_buttons.pending.pop_front() else {
            return Ok(false);
        };
        if self.model.node(id).is_some() {
            self.quick_buttons.syncing = true;
            let result = self.update_button(id, &name, old);
            self.quick_buttons.syncing = false;
            result?;
        }
        Ok(true)
    }

    pub(crate) fn button_call(
        &mut self,
        id: ObjectId,
        name: &str,
        args: &[Value],
    ) -> Result<Value, Error> {
        if self.in_binding() {
            return Err(Error::new("a binding cannot activate a button"));
        }
        if matches!(name, "addButton" | "removeButton") {
            if let [Value::Object(button)] = args {
                let key = self.model.key(*button, MEMBER)?;
                if name == "addButton" {
                    self.write(key, Value::Object(id))?;
                } else if group(self.model, *button) == Some(id) {
                    self.write(key, Value::Null)?;
                }
            }
        } else {
            let click = name == "click";
            if click && !yes(self.model, id, "enabled") {
                return Ok(Value::Undefined);
            }
            let exclusive = group(self.model, id).map_or_else(
                || yes(self.model, id, "autoExclusive"),
                |g| yes(self.model, g, "exclusive"),
            );
            let checked = yes(self.model, id, "checked");
            if yes(self.model, id, "checkable") && !(click && checked && exclusive) {
                if click && yes(self.model, id, "tristate") {
                    let Value::Number(n) = self.model.get(id, "checkState")? else {
                        unreachable!()
                    };
                    self.write(
                        self.model.key(id, "checkState")?,
                        Value::Number((n + 1.) % 3.),
                    )?;
                } else {
                    self.write(self.model.key(id, "checked")?, Value::Bool(!checked))?;
                    self.settle()?;
                    if click {
                        self.dispatch(id, "toggled", vec![])?;
                    }
                }
            }
            self.settle()?;
            if click {
                self.dispatch(id, "clicked", vec![])?;
                if let Some(g) = group(self.model, id) {
                    self.dispatch(g, "clicked", vec![Value::Object(id)])?;
                }
            }
        }
        self.settle()?;
        Ok(Value::Undefined)
    }
}
