//! Section boundaries depend on the model. Their delegates, like row delegates,
//! exist only near the viewport; measured label sizes survive scrolling.
use super::*;

#[derive(Clone, Default)]
pub(super) struct Sections {
    role: String,
    first_character: bool,
    factory: Option<ObjectId>,
    inline: bool,
    // A numeric model grouped by its full index needs no per-index allocation.
    indexed: usize,
    groups: Rc<Vec<(usize, String)>>,
    labels: BTreeMap<usize, ObjectId>,
    nominal: Option<f64>,
    sizes: Extents,
}

impl Sections {
    pub(super) fn items(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.labels.values().copied()
    }
    fn len(&self) -> usize {
        self.indexed.max(self.groups.len())
    }
    fn preceding(&self, index: usize) -> usize {
        if self.indexed > 0 {
            index.min(self.indexed)
        } else {
            self.groups
                .partition_point(|(boundary, _)| *boundary < index)
        }
    }
    fn boundary(&self, group: usize) -> usize {
        if self.indexed > 0 {
            group
        } else {
            self.groups[group].0
        }
    }
    fn at(&self, index: usize) -> String {
        self.preceding(index + 1)
            .checked_sub(1)
            .map(|group| self.name(group))
            .unwrap_or_default()
    }
    fn name(&self, group: usize) -> String {
        if self.indexed > 0 {
            group.to_string()
        } else {
            self.groups[group].1.clone()
        }
    }
    fn size(&self, group: usize) -> f64 {
        self.sizes.get(group).or(self.nominal).unwrap_or(0.)
    }
    pub(super) fn before(&self, index: usize) -> f64 {
        if !self.inline {
            return 0.;
        }
        let groups = self.preceding(index);
        self.nominal.unwrap_or(0.) * groups as f64 + self.sizes.before(groups)
    }
    pub(super) fn label_extent(&self, index: usize) -> f64 {
        let group = self.preceding(index);
        if self.inline && group < self.len() && self.boundary(group) == index {
            self.size(group)
        } else {
            0.
        }
    }
    fn section_name(
        &self,
        tx: &mut Transaction<'_>,
        owner: ObjectId,
        source: &Source,
        index: usize,
    ) -> Result<String, Error> {
        let (_, row) = source.at(tx.model, index).expect("valid row");
        let field = match &row {
            Value::ModelRow(row) if self.role == "modelData" => {
                match tx.model.dynamic.repeaters[&owner].roles.as_deref() {
                    Some([role]) => tx.model.model_role(*row, role)?,
                    _ => Value::Undefined,
                }
            }
            Value::ModelRow(row) => tx.model.model_role(*row, &self.role)?,
            _ if self.role == "modelData" => row,
            Value::Record(r) => tx
                .model
                .record_entries(r)?
                .get(&self.role)
                .cloned()
                .unwrap_or(Value::Undefined),
            Value::External(value) => {
                tx.with_external(owner, value, &mut BTreeSet::new(), |engine, ctx| {
                    engine.get_external(ctx, value, &self.role)
                })?
            }
            Value::Object(o) => tx.model.get(*o, &self.role).unwrap_or(Value::Undefined),
            _ => Value::Undefined,
        };
        let name = match field {
            Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                tx.model.value_string(&field)?
            }
            _ => String::new(),
        };
        Ok(if self.first_character {
            name.chars().take(1).collect()
        } else {
            name
        })
    }

    // One role edit can only create, remove or rename a boundary at this row
    // or the next. Structural edits still rebuild against the new row order.
    fn update_boundary_pair(
        &mut self,
        tx: &mut Transaction<'_>,
        owner: ObjectId,
        source: &Source,
        row: RowId,
    ) -> Result<bool, Error> {
        let Source::Rows(model) = source else {
            return Ok(false);
        };
        let Some(index) = tx.model.models[model].row_index(row) else {
            return Ok(false);
        };
        let mut previous = match index.checked_sub(1) {
            Some(i) => self.section_name(tx, owner, source, i)?.to_lowercase(),
            None => String::new(),
        };
        let mut changed = false;
        for i in index..(index + 2).min(source.len(tx.model)) {
            let name = self.section_name(tx, owner, source, i)?;
            let folded = name.to_lowercase();
            let boundary = folded != previous;
            previous = folded;
            match self.groups.binary_search_by_key(&i, |g| g.0) {
                Ok(at) if !boundary => {
                    Rc::make_mut(&mut self.groups).remove(at);
                }
                Ok(at) if self.groups[at].1 != name => {
                    Rc::make_mut(&mut self.groups)[at].1 = name;
                }
                Err(at) if boundary => {
                    Rc::make_mut(&mut self.groups).insert(at, (i, name));
                }
                _ => continue,
            }
            changed = true;
        }
        Ok(changed)
    }

    fn rebuild(
        &mut self,
        tx: &mut Transaction<'_>,
        owner: ObjectId,
        source: &Source,
    ) -> Result<(), Error> {
        let count = source.len(tx.model);
        self.indexed = 0;
        let mut groups = Vec::new();
        if !self.role.is_empty() && count != 0 {
            if matches!(source, Source::Count(_)) {
                if self.role != "modelData" && self.role != "index" {
                    // A leading empty section has no label.
                } else if self.first_character {
                    groups.push((0, "0".into()));
                    let mut power: usize = 1;
                    while power < count {
                        for digit in 1..=9 {
                            let index = power.saturating_mul(digit);
                            if index >= count {
                                break;
                            }
                            groups.push((index, digit.to_string()));
                        }
                        power = power.saturating_mul(10);
                    }
                } else {
                    self.indexed = count;
                }
            } else {
                let mut prev = None;
                for i in 0..count {
                    let name = self.section_name(tx, owner, source, i)?;
                    let folded = name.to_lowercase();
                    if prev.as_deref().unwrap_or("") != folded {
                        prev = Some(folded);
                        groups.push((i, name));
                    }
                }
            }
        }
        self.groups = Rc::new(groups);
        Ok(())
    }
}

impl State {
    pub(super) fn attached_values(
        &self,
        id: ObjectId,
        index: usize,
        count: usize,
        selected: Option<usize>,
    ) -> [(&'static str, Value); 5] {
        [
            ("view", Value::Object(id)),
            ("isCurrentItem", Value::Bool(Some(index) == selected)),
            ("section", Value::String(self.sections.at(index))),
            (
                "previousSection",
                Value::String(
                    index
                        .checked_sub(1)
                        .map(|i| self.sections.at(i))
                        .unwrap_or_default(),
                ),
            ),
            (
                "nextSection",
                Value::String(if index + 1 < count {
                    self.sections.at(index + 1)
                } else {
                    String::new()
                }),
            ),
        ]
    }
    pub(crate) fn section_role<'a>(&'a self, roles: Option<&'a [String]>) -> Option<&'a str> {
        match self.sections.role.as_str() {
            "" => None,
            // A single-role ListModel projects that role as modelData.
            "modelData" => match roles {
                Some([role]) => Some(role),
                _ => None,
            },
            named => Some(named),
        }
    }
}

pub(super) struct SectionLayout<'a> {
    pub reversed: bool,
    pub axis: &'a str,
    pub coord: &'a str,
    pub spacing: f64,
    pub offset: f64,
    pub viewport: f64,
    pub cache: f64,
    pub count: usize,
}

fn factory(model: &Instance, id: ObjectId, name: &str) -> Result<Option<ObjectId>, Error> {
    Ok(match model.get(id, name)? {
        Value::Object(o) if model.dynamic.templates.contains_key(&o) => Some(o),
        _ => None,
    })
}
fn item_size(model: &Instance, item: ObjectId, axis: &str) -> f64 {
    if let Ok(Value::Number(n)) = model.get(item, axis) {
        if n > 0. {
            return n;
        }
    }
    real(
        model,
        item,
        if axis == "width" {
            "implicitWidth"
        } else {
            "implicitHeight"
        },
    )
    .max(0.)
}

impl Transaction<'_> {
    fn create_decoration(
        &mut self,
        id: ObjectId,
        factory: ObjectId,
        section: Option<String>,
    ) -> Result<ObjectId, Error> {
        let template = self.model.dynamic.templates[&factory].clone();
        let values = section
            .into_iter()
            .map(|s| ("section".to_owned(), Value::String(s)))
            .collect();
        let inputs = Inputs {
            view: id,
            roles: None,
            values,
            attached: Some(BTreeMap::from([("view".into(), Value::Object(id))])),
        };
        self.construct_object(
            factory,
            Some(id),
            &template,
            Default::default(),
            Some(&inputs),
            Default::default(),
        )
    }
    pub(super) fn update_decoration(
        &mut self,
        id: ObjectId,
        name: &str,
        axis: &str,
        deco: &mut Decoration,
    ) -> Result<(), Error> {
        let next = factory(self.model, id, name)?;
        if next != deco.factory {
            if let Some(old) = deco.item {
                self.destroy_subtrees(&[old])?;
            }
            deco.item = next
                .map(|f| self.create_decoration(id, f, None))
                .transpose()?;
            deco.factory = next;
            self.list_property(
                id,
                &format!("{name}Item"),
                deco.item.map_or(Value::Null, Value::Object),
            )?;
        }
        deco.extent = deco.item.map_or(0., |o| item_size(self.model, o, axis));
        Ok(())
    }

    pub(super) fn prepare_sections(
        &mut self,
        id: ObjectId,
        state: &mut State,
        dirty: bool,
        edited: Option<repeater::SectionEdit>,
    ) -> Result<bool, Error> {
        let sections = &mut state.sections;
        let Value::String(role) = self.model.get(id, "section.property")? else {
            unreachable!()
        };
        let first = real(self.model, id, "section.criteria") == 1.;
        let next = factory(self.model, id, "section.delegate")?;
        sections.inline =
            real(self.model, id, "section.labelPositioning") as i32 & 1 != 0 && next.is_some();
        let rebuild = dirty
            || role != sections.role
            || first != sections.first_character
            || next != sections.factory
            || edited == Some(repeater::SectionEdit::MultipleRows);
        if rebuild {
            sections.role = role;
            sections.first_character = first;
            sections.factory = next;
            sections.rebuild(self, id, &state.source)?;
        } else if let Some(repeater::SectionEdit::Row(row)) = edited {
            if !sections.update_boundary_pair(self, id, &state.source, row)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
        self.destroy_subtrees(&sections.items().collect::<Vec<_>>())?;
        sections.labels.clear();
        sections.nominal = None;
        sections.sizes = Default::default();
        Ok(true)
    }

    pub(super) fn layout_sections(
        &mut self,
        id: ObjectId,
        state: &mut State,
        layout: SectionLayout<'_>,
    ) -> Result<bool, Error> {
        let SectionLayout {
            reversed,
            axis,
            coord,
            spacing,
            offset,
            viewport,
            cache,
            count,
        } = layout;
        let first_row = state
            .lower_index(offset, count, spacing)
            .min(count.saturating_sub(1));
        let current = state.sections.preceding(first_row + 1).checked_sub(1);
        self.list_property(
            id,
            "currentSection",
            Value::String(current.map(|g| state.sections.name(g)).unwrap_or_default()),
        )?;
        let Some(factory) = state.sections.factory else {
            return Ok(false);
        };
        let flags = real(self.model, id, "section.labelPositioning") as i32;
        let mut wanted = BTreeSet::new();
        if state.sections.inline {
            let first = state.lower_index(offset - cache, count, spacing);
            let mut g = state.sections.preceding(first);
            while g < state.sections.len() {
                let index = state.sections.boundary(g);
                let start = state.start(index, spacing) - state.sections.size(g);
                if start >= offset + viewport + cache {
                    break;
                }
                if start + state.sections.size(g) >= offset - cache
                    || state.sections.nominal.is_none()
                {
                    wanted.insert(g);
                }
                g += 1;
            }
        }
        let next = current.and_then(|g| (g + 1 < state.sections.len()).then_some(g + 1));
        if flags & 2 != 0 {
            wanted.extend(current);
        }
        // Pin the group following the last row that fits above the trailing
        // label. Several other section headers can still fit in the viewport.
        let trailing = if flags & 4 != 0 {
            let bottom = offset + viewport - state.sections.nominal.unwrap_or(0.);
            let last_row = state.lower_index(bottom, count, spacing);
            let group = state.sections.preceding(last_row.saturating_add(1));
            (group < state.sections.len()).then_some(group)
        } else {
            None
        };
        wanted.extend(trailing);
        let removed: Vec<_> = state
            .sections
            .labels
            .iter()
            .filter_map(|(g, o)| (!wanted.contains(g)).then_some(*o))
            .collect();
        self.destroy_subtrees(&removed)?;
        state.sections.labels.retain(|g, _| wanted.contains(g));
        let mut measured = false;
        for g in wanted {
            let item = if let Some(&item) = state.sections.labels.get(&g) {
                item
            } else {
                let item = self.create_decoration(id, factory, Some(state.sections.name(g)))?;
                state.sections.labels.insert(g, item);
                item
            };
            let extent = item_size(self.model, item, axis);
            if state.sections.nominal.is_none() || state.sections.size(g) != extent {
                let nominal = *state.sections.nominal.get_or_insert(extent);
                state.sections.sizes.set(g, extent, nominal);
                measured = true;
            }
            let boundary = state.sections.boundary(g);
            let start = state.start(boundary, spacing) - state.sections.label_extent(boundary);
            let mut position = start;
            if current == Some(g) && flags & 2 != 0 {
                position = position.max(offset);
                if let Some(n) = next {
                    let next = state.sections.boundary(n);
                    position = position.min(
                        state.start(next, spacing) - state.sections.label_extent(next) - extent,
                    );
                }
            } else if trailing == Some(g) {
                position = position.min(offset + viewport - extent);
                if state.sections.inline && g > 0 {
                    position = position.max(state.start(state.sections.boundary(g - 1), spacing));
                }
            }
            self.list_property(
                item,
                coord,
                Value::Number(axis_position(reversed, position, extent)),
            )?;
        }
        Ok(measured)
    }
}
