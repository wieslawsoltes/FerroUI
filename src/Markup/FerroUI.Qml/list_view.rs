//! Viewport-bounded Qt Quick list delegates. Unknown sizes use the first
//! measured extent, and realized size differences refine the content geometry.
use super::*;

impl TypeDefinition {
    pub fn quick_list_view() -> Self {
        let number = Value::Number;
        let mut ty = Self::default()
            .inherits("QtQuick", "Item")
            .property("model", "var", Value::Undefined)
            .property("delegate", "Component", Value::Null)
            .property("delegateModelAccess", "int", number(0.))
            .property("orientation", "int", number(2.))
            .property("spacing", "real", number(0.))
            .property("cacheBuffer", "int", number(0.))
            .property("currentIndex", "int", number(-1.))
            .property("contentX", "real", number(0.))
            .property("contentY", "real", number(0.))
            .readonly_property("count", "int", number(0.))
            .readonly_property("contentWidth", "real", number(0.))
            .readonly_property("contentHeight", "real", number(0.))
            .readonly_property("currentItem", "QtQuick.Item", Value::Null)
            .readonly_property(REVISION, "real", number(0.))
            .enumeration("Orientation", &[("Horizontal", 1), ("Vertical", 2)])
            .expect("Qt orientations")
            .enumeration(
                "PositionMode",
                &[
                    ("Beginning", 0),
                    ("Center", 1),
                    ("End", 2),
                    ("Visible", 3),
                    ("Contain", 4),
                    ("SnapPosition", 5),
                ],
            )
            .expect("Qt positioning modes");
        for (name, signature, result) in [
            ("itemAtIndex", &[("index", "int")][..], "QtQuick.Item"),
            (
                "itemAt",
                &[("x", "real"), ("y", "real")][..],
                "QtQuick.Item",
            ),
            ("indexAt", &[("x", "real"), ("y", "real")][..], "int"),
            (
                "positionViewAtIndex",
                &[("index", "int"), ("mode", "int")][..],
                "var",
            ),
            ("positionViewAtBeginning", &[][..], "var"),
            ("positionViewAtEnd", &[][..], "var"),
            ("incrementCurrentIndex", &[][..], "var"),
            ("decrementCurrentIndex", &[][..], "var"),
            ("forceLayout", &[][..], "var"),
        ] {
            ty = ty.context_method(
                name,
                signature
                    .iter()
                    .map(|&(name, ty)| Parameter {
                        name: name.into(),
                        ty: Some(ty.into()),
                    })
                    .collect(),
                result,
                move |ctx, args| ctx.list_view_call(name, args),
            );
        }
        ty
    }
}

#[derive(Clone, Default)]
enum Source {
    #[default]
    Empty,
    Count(usize),
    Array(Rc<[Value]>),
    Rows(ObjectId),
}
impl Source {
    fn len(&self, model: &Instance) -> usize {
        match self {
            Self::Empty => 0,
            Self::Count(n) => *n,
            Self::Array(v) => v.len(),
            Self::Rows(id) => model.models.get(id).map_or(0, |m| m.len()),
        }
    }
    fn at(&self, model: &Instance, index: usize) -> Option<(Key, Value)> {
        match self {
            Self::Count(n) if index < *n => {
                Some((Key::Position(index), Value::Number(index as f64)))
            }
            Self::Array(v) => Some((Key::Position(index), v.get(index)?.clone())),
            Self::Rows(id) => {
                let row = model.models.get(id)?.row_at(*id, index)?;
                Some((Key::Row(row), Value::ModelRow(row)))
            }
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct State {
    source: Source,
    initialized: bool,
    viewport: Option<(f64, f64)>,
    horizontal: bool,
    // Width/height deviations are retained independently of the live delegates.
    nominal: Option<f64>,
    sizes: Rc<BTreeMap<usize, f64>>,
    realized: BTreeMap<usize, (Key, ObjectId)>,
    current: i32,
    current_row: Option<Key>,
}
impl Instance {
    /// Supply an arranged native viewport without replacing QML width or height bindings.
    pub fn set_list_view_viewport(
        &mut self,
        id: ObjectId,
        width: f64,
        height: f64,
    ) -> Result<(), Error> {
        if !width.is_finite() || !height.is_finite() || width < 0. || height < 0. {
            return Err(Error::new("invalid ListView viewport"));
        }
        let size = Some((width, height));
        let old = self
            .dynamic
            .repeaters
            .get(&id)
            .filter(|s| s.list.is_some())
            .ok_or_else(|| Error::new("not a live ListView"))?;
        if old.list.as_ref().unwrap().viewport == size {
            return Ok(());
        }
        self.transaction(|tx| {
            let state = tx.model.dynamic.repeaters.get_mut(&id).unwrap();
            tx.repeater_changes
                .before
                .entry(id)
                .or_insert_with(|| state.clone());
            state.list.as_mut().unwrap().viewport = size;
            tx.repeater_changes.pending.insert(id);
            Ok(())
        })
    }
}

impl State {
    fn dimension(&self, model: &Instance, owner: ObjectId, axis: &str) -> f64 {
        self.viewport.map_or_else(
            || real(model, owner, axis).max(0.),
            |(width, height)| if axis == "width" { width } else { height },
        )
    }

    pub(super) fn items(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.realized.values().map(|entry| entry.1)
    }
    fn extent(&self, index: usize) -> f64 {
        self.sizes
            .get(&index)
            .copied()
            .unwrap_or(self.nominal.unwrap_or(32.))
    }
    fn start(&self, index: usize, spacing: f64) -> f64 {
        let base = self.nominal.unwrap_or(32.);
        index as f64 * (base + spacing)
            + self
                .sizes
                .range(..index)
                .map(|(_, extent)| *extent - base)
                .sum::<f64>()
    }
    fn full_size(&self, count: usize, spacing: f64) -> f64 {
        if count == 0 {
            0.
        } else {
            self.start(count, spacing) - spacing
        }
    }
    fn lower_index(&self, position: f64, count: usize, spacing: f64) -> usize {
        let (mut lo, mut hi) = (0, count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.start(mid, spacing) + self.extent(mid) <= position {
                lo = mid + 1
            } else {
                hi = mid
            }
        }
        lo
    }
    fn measure(&mut self, index: usize, extent: f64) -> bool {
        if extent <= 0. || !extent.is_finite() {
            return false;
        }
        if self.nominal.is_none() {
            self.nominal = Some(extent);
            return true;
        }
        if self.extent(index) == extent {
            return false;
        }
        if Some(extent) == self.nominal {
            Rc::make_mut(&mut self.sizes).remove(&index);
        } else {
            Rc::make_mut(&mut self.sizes).insert(index, extent);
        }
        true
    }
}

fn real(model: &Instance, id: ObjectId, name: &str) -> f64 {
    match model.get(id, name) {
        Ok(Value::Number(n)) if n.is_finite() => n,
        _ => 0.,
    }
}
fn aligned(begin: f64, extent: f64, offset: f64, viewport: f64, mode: i32) -> f64 {
    let end = begin + extent;
    match mode {
        0 | 5 => begin,
        1 => begin + (extent - viewport) / 2.,
        2 => end - viewport,
        3 if end > offset && begin < offset + viewport => offset,
        3 | 4 if begin < offset => begin,
        3 | 4 if end > offset + viewport => {
            if extent > viewport {
                begin
            } else {
                end - viewport
            }
        }
        _ => offset,
    }
}

impl Transaction<'_> {
    fn list_source(&mut self, id: ObjectId, value: &Value) -> Result<Source, Error> {
        Ok(match value {
            Value::Number(n) if n.is_finite() && (0.0..=i32::MAX as f64).contains(n) => {
                Source::Count(*n as usize)
            }
            Value::List(values) => Source::Array(values.clone().into()),
            Value::Record(r) if r.is_array() => Source::Array(self.model.array_values(r)?.into()),
            Value::Object(model) if self.model.models.contains_key(model) => Source::Rows(*model),
            Value::External(array) => Source::Array(
                self.with_external(id, array, &mut BTreeSet::new(), |engine, ctx| {
                    engine.array_elements(ctx, array, MAX_ROWS)
                })?
                .unwrap_or_default()
                .into(),
            ),
            _ => Source::Empty,
        })
    }

    fn list_property(&mut self, id: ObjectId, name: &str, value: Value) -> Result<(), Error> {
        let key = self.model.key(id, name)?;
        self.set_value(key, value);
        Ok(())
    }

    pub(crate) fn list_view_call(
        &mut self,
        id: ObjectId,
        name: &str,
        args: &[Value],
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        self.read(self.model.key(id, REVISION)?, reads)?;
        let state = self
            .model
            .dynamic
            .repeaters
            .get(&id)
            .and_then(|s| s.list.as_ref())
            .ok_or_else(|| Error::new("not a ListView"))?;
        let arg = |i| match args.get(i) {
            Some(Value::Number(n)) => *n,
            _ => 0.,
        };
        let horizontal = state.horizontal;
        let (axis, offset_axis) = if horizontal {
            ("width", "contentX")
        } else {
            ("height", "contentY")
        };
        let viewport = state.dimension(self.model, id, axis);
        let spacing = real(self.model, id, "spacing").max(0.);
        let offset = real(self.model, id, offset_axis);
        let count = state.source.len(self.model);
        let pos = if horizontal { arg(0) } else { arg(1) };
        let cross = if horizontal { arg(1) } else { arg(0) };
        let at = state.lower_index(pos, count, spacing);
        let hits = || {
            state.realized.get(&at).filter(|(_, item)| {
                let cross_size = real(
                    self.model,
                    *item,
                    if horizontal { "height" } else { "width" },
                );
                pos >= state.start(at, spacing) && cross >= 0. && cross < cross_size
            })
        };
        match name {
            "itemAtIndex" => {
                return Ok(state
                    .realized
                    .get(&(arg(0) as i32 as usize))
                    .map_or(Value::Null, |e| Value::Object(e.1)))
            }
            "indexAt" => {
                return Ok(Value::Number(if hits().is_some() {
                    at as f64
                } else {
                    -1.
                }))
            }
            "itemAt" => return Ok(hits().map_or(Value::Null, |e| Value::Object(e.1))),
            _ if self.in_binding() => return Err(Error::new("a binding cannot scroll a ListView")),
            "incrementCurrentIndex" | "decrementCurrentIndex" => {
                if count > 0 {
                    let next = (state.current
                        + if name == "incrementCurrentIndex" {
                            1
                        } else {
                            -1
                        })
                    .clamp(0, count as i32 - 1);
                    let key = self.model.key(id, "currentIndex")?;
                    self.write(key, Value::Number(next as f64))?;
                }
            }
            "forceLayout" => {
                self.repeater_changes.pending.insert(id);
            }
            _ => {
                let max = (state.full_size(count, spacing) - viewport).max(0.);
                let next = match name {
                    "positionViewAtBeginning" => 0.,
                    "positionViewAtEnd" => max,
                    "positionViewAtIndex" if arg(0) >= 0. && (arg(0) as usize) < count => {
                        let i = arg(0) as usize;
                        aligned(
                            state.start(i, spacing),
                            state.extent(i),
                            offset,
                            viewport,
                            arg(1) as i32,
                        )
                        .clamp(0., max)
                    }
                    _ => return Ok(Value::Undefined),
                };
                let key = self.model.key(id, offset_axis)?;
                self.write(key, Value::Number(next))?;
            }
        }
        self.settle()?;
        Ok(Value::Undefined)
    }

    pub(super) fn refresh_list_view(&mut self, id: ObjectId) -> Result<(), Error> {
        self.budget()?;
        let retained = &self.model.dynamic.repeaters[&id];
        let mut state = retained.list.as_deref().expect("registered view").clone();
        let model_changed = self.repeater_changes.list_models.remove(&id) || !state.initialized;
        let structural = self.repeater_changes.list_structure.remove(&id);
        let factory = match self.model.get(id, "delegate")? {
            Value::Object(o) if self.model.dynamic.templates.contains_key(&o) => Some(o),
            _ => None,
        };
        let reset = model_changed || factory != retained.factory;
        let horizontal = real(self.model, id, "orientation") == 1.;
        if reset || structural || horizontal != state.horizontal {
            state.sizes = Default::default();
            if reset || horizontal != state.horizontal {
                state.nominal = None;
            }
        }
        if model_changed {
            let model = self.model.get(id, "model")?;
            state.source = self.list_source(id, &model)?;
            let s = self.model.dynamic.repeaters.get_mut(&id).unwrap();
            s.model = Some(model);
            s.roles = None;
        }
        let count = state.source.len(self.model);
        if count > 0 {
            if let Source::Rows(model) = state.source {
                let s = self.model.dynamic.repeaters.get_mut(&id).unwrap();
                if s.roles.is_none() {
                    s.roles = Some(self.model.models[&model].view_roles());
                }
            }
        }
        let previous_count = real(self.model, id, "count") as usize;
        let requested = real(self.model, id, "currentIndex") as i32;
        let mut current = requested;
        if count == 0 {
            current = -1;
        } else if previous_count == 0 && !self.model.is_assigned(id, "currentIndex") {
            current = 0;
        } else if structural && requested == state.current {
            if let (Source::Rows(model), Some(Key::Row(row))) = (&state.source, state.current_row) {
                current = self.model.models[model]
                    .row_index(row)
                    .map_or(requested.min(count as i32 - 1), |i| i as i32);
            }
        }
        let selected = usize::try_from(current).ok().filter(|i| *i < count);
        let (axis, cross, coord, offset_prop) = if horizontal {
            ("width", "height", "x", "contentX")
        } else {
            ("height", "width", "y", "contentY")
        };
        let viewport = state.dimension(self.model, id, axis);
        let spacing = real(self.model, id, "spacing").max(0.);
        let cache = real(self.model, id, "cacheBuffer").max(0.);
        let mut offset = real(self.model, id, offset_prop);
        if let Some(i) =
            selected.filter(|_| state.initialized && current != state.current && !structural)
        {
            offset = aligned(
                state.start(i, spacing),
                state.extent(i),
                offset,
                viewport,
                4,
            );
        }
        let offset = offset.clamp(0., (state.full_size(count, spacing) - viewport).max(0.));
        let mut wanted = BTreeSet::new();
        if let Some(i) = selected {
            wanted.insert(i);
        }
        if viewport > 0. {
            let mut i = state.lower_index((offset - cache).max(0.), count, spacing);
            while i < count && state.start(i, spacing) < offset + viewport + cache {
                if wanted.len() >= MAX_ROWS {
                    return Err(Error::new(
                        "ListView viewport exceeds the realization limit",
                    ));
                }
                wanted.insert(i);
                i += 1;
            }
        }
        let targets: BTreeMap<_, _> = if factory.is_some() {
            wanted
                .into_iter()
                .filter_map(|i| Some((i, state.source.at(self.model, i)?)))
                .collect()
        } else {
            Default::default()
        };
        let keys: BTreeSet<_> = targets.values().map(|r| r.0).collect();
        let previous = state.realized.clone();
        let mut retained = BTreeMap::new();
        let mut removed = vec![];
        for &(key, item) in previous.values() {
            if !reset && keys.contains(&key) {
                retained.insert(key, item);
            } else {
                removed.push(item);
            }
        }
        state.realized.clear();
        self.destroy_subtrees(&removed)?;
        let mut measured = false;
        if let Some(f) = factory {
            let template = self.model.dynamic.templates[&f].clone();
            for (index, (key, value)) in targets {
                let owner = retained
                    .get(&key)
                    .copied()
                    .unwrap_or_else(|| self.model.next_object());
                let inputs = self.model.delegate_inputs(id, index, &value, owner)?;
                let item = if let Some(&old) = retained.get(&key) {
                    if structural || !previous.get(&index).is_some_and(|e| e.1 == old) {
                        self.update_delegate(old, inputs)?;
                    }
                    old
                } else {
                    self.construct_object(
                        f,
                        Some(id),
                        &template,
                        Default::default(),
                        Some(&inputs),
                        Default::default(),
                    )?
                };
                let size = match self.model.get(item, axis)? {
                    Value::Number(n) if n > 0. => n,
                    _ => real(
                        self.model,
                        item,
                        if horizontal {
                            "implicitWidth"
                        } else {
                            "implicitHeight"
                        },
                    ),
                };
                measured |= state.measure(index, size);
                self.list_property(item, coord, Value::Number(state.start(index, spacing)))?;
                state.realized.insert(index, (key, item));
            }
        }
        let old_items: BTreeSet<_> = previous.values().map(|e| e.1).collect();
        let data = self.model.key(id, "data")?;
        let Value::List(mut children) = self.model.slots[data].value.clone() else {
            return Err(Error::new("ListView data must be a list"));
        };
        children.retain(|v| !matches!(v,Value::Object(o) if old_items.contains(o)));
        children.extend(state.items().map(Value::Object));
        self.set_value(data, Value::List(children));
        self.list_property(id, "count", Value::Number(count as f64))?;
        self.list_property(id, "currentIndex", Value::Number(current as f64))?;
        self.list_property(
            id,
            "currentItem",
            selected
                .and_then(|i| state.realized.get(&i))
                .map_or(Value::Null, |e| Value::Object(e.1)),
        )?;
        self.list_property(id, offset_prop, Value::Number(offset))?;
        self.list_property(
            id,
            if horizontal {
                "contentWidth"
            } else {
                "contentHeight"
            },
            Value::Number(state.full_size(count, spacing)),
        )?;
        self.list_property(
            id,
            if horizontal {
                "contentHeight"
            } else {
                "contentWidth"
            },
            Value::Number(state.dimension(self.model, id, cross)),
        )?;
        if previous != state.realized {
            // The revision slot is private. Resolve directly instead of via the public getter.
            let key = self.model.key(id, REVISION)?;
            let Value::Number(rev) = self.model.slots[key].value else {
                unreachable!()
            };
            self.set_value(key, Value::Number(rev + 1.));
        }
        state.current = current;
        state.current_row = selected.and_then(|i| state.realized.get(&i)).map(|e| e.0);
        state.initialized = true;
        state.horizontal = horizontal;
        let out = self.model.dynamic.repeaters.get_mut(&id).unwrap();
        out.factory = factory;
        out.rows = state
            .realized
            .iter()
            .filter_map(|(&index, &(key, item))| {
                if let Key::Row(row) = key {
                    Some((row, (index, item)))
                } else {
                    None
                }
            })
            .collect();
        out.list = Some(Box::new(state));
        if measured {
            self.repeater_changes.pending.insert(id);
        }
        Ok(())
    }
}
