//! Viewport-bounded Qt Quick list delegates. Unknown sizes use the first
//! measured extent, and realized size differences refine the content geometry.
use super::*;
#[path = "list_extents.rs"]
mod extents;
use extents::Extents;
#[path = "list_sections.rs"]
mod sections;
use sections::{SectionLayout, Sections};

impl TypeDefinition {
    pub fn quick_list_view() -> Self {
        let number = Value::Number;
        let mut ty = Self::default()
            .inherits("QtQuick", "Item")
            .property("model", "var", Value::Undefined)
            .property("delegate", "Component", Value::Null)
            .property("delegateModelAccess", "int", number(0.))
            .property("orientation", "int", number(2.))
            .property("reuseItems", "bool", Value::Bool(false))
            .property("layoutDirection", "int", number(0.))
            .readonly_property("effectiveLayoutDirection", "int", number(0.))
            .property("verticalLayoutDirection", "int", number(0.))
            .property("headerPositioning", "int", number(0.))
            .property("footerPositioning", "int", number(0.))
            .enumeration(
                "HeaderPositioning",
                &[
                    ("InlineHeader", 0),
                    ("OverlayHeader", 1),
                    ("PullBackHeader", 2),
                ],
            )
            .expect("Qt header modes")
            .enumeration(
                "FooterPositioning",
                &[
                    ("InlineFooter", 0),
                    ("OverlayFooter", 1),
                    ("PullBackFooter", 2),
                ],
            )
            .expect("Qt footer modes")
            .property("header", "Component", Value::Null)
            .property("footer", "Component", Value::Null)
            .property_group("section")
            .property("section.property", "string", Value::String(String::new()))
            .property("section.criteria", "int", number(0.))
            .property("section.delegate", "Component", Value::Null)
            .property("section.labelPositioning", "int", number(1.))
            .readonly_property("headerItem", "QtQuick.Item", Value::Null)
            .readonly_property("footerItem", "QtQuick.Item", Value::Null)
            .readonly_property("currentSection", "string", Value::String(String::new()))
            .readonly_property("originX", "real", number(0.))
            .readonly_property("originY", "real", number(0.))
            .enumeration(
                "VerticalLayoutDirection",
                &[("TopToBottom", 0), ("BottomToTop", 1)],
            )
            .expect("Qt vertical directions")
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
    reversed: bool,
    header: Decoration,
    footer: Decoration,
    sections: Sections,
    pool: Vec<ObjectId>,
    // Width/height deviations are retained independently of the live delegates.
    nominal: Option<f64>,
    sizes: Extents,
    realized: BTreeMap<usize, (Key, ObjectId)>,
    current: i32,
    current_row: Option<Key>,
}
#[derive(Clone, Default)]
struct Decoration {
    factory: Option<ObjectId>,
    item: Option<ObjectId>,
    extent: f64,
    position: Option<f64>,
}

fn axis_position(reversed: bool, start: f64, extent: f64) -> f64 {
    if reversed {
        -start - extent
    } else {
        start
    }
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
        self.visual_items().chain(self.pool.iter().copied())
    }
    fn visual_items(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.realized
            .values()
            .map(|entry| entry.1)
            .chain(self.sections.items())
            .chain(self.header.item)
            .chain(self.footer.item)
    }
    fn bounds(&self, count: usize, spacing: f64, viewport: f64) -> (f64, f64) {
        let begin = -self.header.extent;
        (
            begin,
            (self.full_size(count, spacing) + self.footer.extent - viewport).max(begin),
        )
    }
    fn extent(&self, index: usize) -> f64 {
        self.sizes.get(index).unwrap_or(self.nominal.unwrap_or(32.))
    }
    fn start(&self, index: usize, spacing: f64) -> f64 {
        index as f64 * (self.nominal.unwrap_or(32.) + spacing)
            + self.sizes.before(index)
            + self.sections.before(index.saturating_add(1))
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
        self.sizes.set(
            index,
            extent,
            self.nominal.expect("measured nominal extent"),
        );
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
        let offset = axis_position(state.reversed, real(self.model, id, offset_axis), viewport);
        let count = state.source.len(self.model);
        let pos = if horizontal { arg(0) } else { arg(1) };
        let cross = if horizontal { arg(1) } else { arg(0) };
        let search = if state.reversed { -pos } else { pos };
        let at = if state.reversed {
            // The leading visual edge is inclusive even when the logical axis runs backwards.
            let (mut lo, mut hi) = (0, count);
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                if state.start(mid, spacing) + state.extent(mid) < search {
                    lo = mid + 1
                } else {
                    hi = mid
                }
            }
            lo
        } else {
            state.lower_index(search, count, spacing)
        };
        let hits = || {
            state.realized.get(&at).filter(|(_, item)| {
                let cross_size = real(
                    self.model,
                    *item,
                    if horizontal { "height" } else { "width" },
                );
                let start =
                    axis_position(state.reversed, state.start(at, spacing), state.extent(at));
                pos >= start && pos < start + state.extent(at) && cross >= 0. && cross < cross_size
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
                let (min, max) = state.bounds(count, spacing, viewport);
                let next = match name {
                    "positionViewAtBeginning" => min,
                    "positionViewAtEnd" => max,
                    "positionViewAtIndex" if arg(0) >= 0. && (arg(0) as usize) < count => {
                        let i = arg(0) as usize;
                        let head = if real(self.model, id, "headerPositioning") != 0. {
                            state.header.extent
                        } else {
                            0.
                        };
                        let foot = if real(self.model, id, "footerPositioning") != 0. {
                            state.footer.extent
                        } else {
                            0.
                        };
                        let mode = arg(1) as i32;
                        if mode == 0 || mode == 5 {
                            self.repeater_changes.reveal_header.insert(id);
                        }
                        let label = state.sections.label_extent(i);
                        let aligned = aligned(
                            state.start(i, spacing) - label,
                            state.extent(i) + label,
                            offset + head,
                            (viewport - head - foot).max(0.),
                            mode,
                        );
                        (aligned - head).clamp(min, max)
                    }
                    _ => return Ok(Value::Undefined),
                };
                let next = axis_position(state.reversed, next, viewport);
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
        let mirrored = self.model.get(id, "LayoutMirroring.enabled") == Ok(Value::Bool(true));
        let rtl = (real(self.model, id, "layoutDirection") == 1.) ^ mirrored;
        self.list_property(
            id,
            "effectiveLayoutDirection",
            Value::Number(f64::from(rtl)),
        )?;
        let reversed = if horizontal {
            rtl
        } else {
            real(self.model, id, "verticalLayoutDirection") == 1.
        };
        let old_header = state.header.extent;
        self.update_decoration(id, "header", axis, &mut state.header)?;
        self.update_decoration(id, "footer", axis, &mut state.footer)?;
        let edited_section = self.repeater_changes.section_changes.remove(&id);
        let retained_horizontal = state.horizontal;
        let sections_changed = self.prepare_sections(
            id,
            &mut state,
            reset || structural || horizontal != retained_horizontal,
            edited_section,
        )?;
        let mut offset = axis_position(reversed, real(self.model, id, offset_prop), viewport);
        if (!state.initialized && !self.model.is_assigned(id, offset_prop))
            || reversed != state.reversed
            || horizontal != state.horizontal
            || (old_header != state.header.extent && offset == -old_header)
        {
            offset = -state.header.extent;
        }
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
        let (min, max) = state.bounds(count, spacing, viewport);
        let offset = offset.clamp(min, max);
        let mut wanted = BTreeSet::new();
        if let Some(i) = selected {
            wanted.insert(i);
        }
        if viewport > 0. {
            let mut i = state.lower_index((offset - cache).max(0.), count, spacing);
            while i < count
                && state.start(i, spacing) - state.sections.label_extent(i)
                    < offset + viewport + cache
            {
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
        let previous = std::mem::take(&mut state.realized);
        let mut retained = BTreeMap::new();
        let can_reuse =
            self.model.get(id, "reuseItems")? == Value::Bool(true) && !reset && !structural;
        let mut removed = if can_reuse {
            Vec::new()
        } else {
            std::mem::take(&mut state.pool)
        };
        for &(key, item) in previous.values() {
            if !reset && keys.contains(&key) {
                retained.insert(key, item);
            } else if can_reuse {
                self.dispatch(item, "ListView.pooled", vec![])?;
                state.pool.push(item);
            } else {
                removed.push(item);
            }
        }
        self.destroy_subtrees(&removed)?;
        let mut measured = false;
        if let Some(f) = factory {
            let template = self.model.dynamic.templates[&f].clone();
            for (index, (key, value)) in targets {
                let kept = retained.get(&key).copied();
                let recycled = if kept.is_none() {
                    state.pool.pop()
                } else {
                    None
                };
                let owner = kept
                    .or(recycled)
                    .unwrap_or_else(|| self.model.next_object());
                let item = if let Some(item) = kept {
                    let moved = !previous.get(&index).is_some_and(|e| e.1 == item);
                    if structural || moved {
                        let inputs = self.model.delegate_inputs(id, index, &value, item)?;
                        self.update_delegate(item, inputs)?;
                    }
                    if sections_changed || moved {
                        for (name, value) in state.attached_values(id, index, count, selected) {
                            self.list_property(item, &format!("ListView.{name}"), value)?;
                        }
                    } else if current != state.current {
                        self.list_property(
                            item,
                            "ListView.isCurrentItem",
                            Value::Bool(Some(index) == selected),
                        )?;
                    }
                    item
                } else {
                    let mut inputs = self.model.delegate_inputs(id, index, &value, owner)?;
                    let attached = state.attached_values(id, index, count, selected);
                    if let Some(item) = recycled {
                        for (name, value) in attached {
                            self.list_property(item, &format!("ListView.{name}"), value)?;
                        }
                        self.recycle_delegate(item, inputs)?;
                        self.dispatch(item, "ListView.reused", vec![])?;
                        item
                    } else {
                        inputs.attached = Some(
                            attached
                                .into_iter()
                                .map(|(k, v)| (k.to_owned(), v))
                                .collect(),
                        );
                        self.construct_object(
                            f,
                            Some(id),
                            &template,
                            Default::default(),
                            Some(&inputs),
                            Default::default(),
                        )?
                    }
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
                self.list_property(
                    item,
                    coord,
                    Value::Number(axis_position(
                        reversed,
                        state.start(index, spacing),
                        state.extent(index),
                    )),
                )?;
                state.realized.insert(index, (key, item));
            }
        }
        let excess = state.pool.len().saturating_sub(state.realized.len().max(1));
        self.destroy_subtrees(&state.pool.drain(..excess).collect::<Vec<_>>())?;
        measured |= self.layout_sections(
            id,
            &mut state,
            SectionLayout {
                reversed,
                axis,
                coord,
                spacing,
                offset,
                viewport,
                cache,
                count,
            },
        )?;
        let total = state.full_size(count, spacing);
        let origin = if reversed {
            -(total + state.footer.extent)
        } else {
            -state.header.extent
        };
        let reveal_header = self.repeater_changes.reveal_header.remove(&id);
        let boundary = offset + viewport;
        let header = &mut state.header;
        let header_start = match real(self.model, id, "headerPositioning") as i32 {
            1 => offset,
            2 if reveal_header => offset,
            2 => header
                .position
                .unwrap_or(offset)
                .clamp(offset - header.extent, offset)
                .max(-header.extent),
            _ => -header.extent,
        };
        header.position = Some(header_start);
        if let Some(item) = header.item {
            self.list_property(
                item,
                coord,
                Value::Number(axis_position(reversed, header_start, header.extent)),
            )?;
        }
        let footer = &mut state.footer;
        let footer_start = match real(self.model, id, "footerPositioning") as i32 {
            1 => boundary - footer.extent,
            2 => footer
                .position
                .unwrap_or(boundary)
                .clamp(boundary - footer.extent, boundary)
                .min(total),
            _ => total,
        };
        footer.position = Some(footer_start);
        if let Some(item) = footer.item {
            self.list_property(
                item,
                coord,
                Value::Number(axis_position(reversed, footer_start, footer.extent)),
            )?;
        }
        self.list_property(
            id,
            if horizontal { "originX" } else { "originY" },
            Value::Number(origin),
        )?;
        self.list_property(
            id,
            if horizontal { "originY" } else { "originX" },
            Value::Number(0.),
        )?;
        if state.horizontal != horizontal {
            self.list_property(
                id,
                if horizontal { "contentY" } else { "contentX" },
                Value::Number(0.),
            )?;
            for item in state.visual_items() {
                self.list_property(item, if horizontal { "y" } else { "x" }, Value::Number(0.))?;
            }
        }
        let data = self.model.key(id, "data")?;
        let Value::List(children) = &self.model.slots[data].value else {
            return Err(Error::new("ListView data must be a list"));
        };
        if !state
            .visual_items()
            .map(Value::Object)
            .eq(children.iter().cloned())
        {
            let old_items: BTreeSet<_> = self.model.dynamic.repeaters[&id]
                .list
                .as_ref()
                .expect("registered view")
                .items()
                .collect();
            let mut children = children.clone();
            children.retain(|v| !matches!(v,Value::Object(o) if old_items.contains(o)));
            children.extend(state.visual_items().map(Value::Object));
            self.set_value(data, Value::List(children));
        }
        self.list_property(id, "count", Value::Number(count as f64))?;
        self.list_property(id, "currentIndex", Value::Number(current as f64))?;
        self.list_property(
            id,
            "currentItem",
            selected
                .and_then(|i| state.realized.get(&i))
                .map_or(Value::Null, |e| Value::Object(e.1)),
        )?;
        self.list_property(
            id,
            offset_prop,
            Value::Number(axis_position(reversed, offset, viewport)),
        )?;
        self.list_property(
            id,
            if horizontal {
                "contentWidth"
            } else {
                "contentHeight"
            },
            Value::Number(total + state.header.extent + state.footer.extent),
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
        let changed = previous != state.realized;
        if changed {
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
        state.reversed = reversed;
        let out = self.model.dynamic.repeaters.get_mut(&id).unwrap();
        out.factory = factory;
        if changed {
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
        }
        out.list = Some(Box::new(state));
        if measured {
            self.repeater_changes.pending.insert(id);
        }
        Ok(())
    }
}
