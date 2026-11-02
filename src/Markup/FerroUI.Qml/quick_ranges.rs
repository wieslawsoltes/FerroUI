//! Range values and normalized thumb positions settle in the same transaction.
use super::*;
use crate::transaction::Transaction;

const KIND: &str = "$qml:rangeKind";
const SLIDER: i32 = 0;
const PROGRESS: i32 = 1;
const SPIN: i32 = 2;

impl TypeDefinition {
    pub fn quick_slider(self) -> Self {
        self.range(SLIDER)
            .property("orientation", "int", Value::Number(1.))
            .property("stepSize", "real", Value::Number(0.))
            .signal("moved", vec![])
            .context_method(
                "valueAt",
                vec![Parameter {
                    name: "position".into(),
                    ty: Some("real".into()),
                }],
                "real",
                |ctx, args| {
                    let [Value::Number(p)] = args else {
                        unreachable!()
                    };
                    let id = ctx.owner();
                    let read = |ctx: &mut ScriptContext<'_, '_>, name| match ctx.get(id, name)? {
                        Value::Number(n) => Ok(n),
                        _ => Err(Error::new("invalid slider range")),
                    };
                    let start = read(ctx, "from")?;
                    let to = read(ctx, "to")?;
                    let step = read(ctx, "stepSize")?.abs();
                    let delta = (to - start) * p;
                    Ok(Value::Number(
                        start
                            + if step == 0. {
                                delta
                            } else {
                                (delta / step).round() * step
                            },
                    ))
                },
            )
    }
    pub fn quick_progress_bar(self) -> Self {
        self.range(PROGRESS)
            .property("indeterminate", "bool", Value::Bool(false))
    }
    pub fn quick_spin_box(self) -> Self {
        self.range(SPIN)
            .property("stepSize", "int", Value::Number(1.))
            .property("editable", "bool", Value::Bool(false))
            .property("wrap", "bool", Value::Bool(false))
            .readonly_property("displayText", "string", Value::String("0".into()))
            .signal("valueModified", vec![])
    }

    fn range(mut self, kind: i32) -> Self {
        let ty = if kind == SPIN { "int" } else { "real" };
        self = self
            .readonly_property(KIND, "int", Value::Number(kind as f64))
            .property("from", ty, Value::Number(0.))
            .property("to", ty, Value::Number(if kind == SPIN { 99. } else { 1. }))
            .property("value", ty, Value::Number(0.));
        if kind != SPIN {
            self = self
                .readonly_property("position", "real", Value::Number(0.))
                .readonly_property("visualPosition", "real", Value::Number(0.));
        }
        if kind != PROGRESS {
            let ty = if kind == SPIN { "int" } else { "real" };
            self = self.context_method(
                "$qml:rangeInput",
                vec![Parameter {
                    name: "value".into(),
                    ty: Some(ty.into()),
                }],
                "var",
                move |ctx, args| {
                    let id = ctx.owner();
                    ctx.set(id, "value", args[0].clone())?;
                    ctx.call(
                        id,
                        if kind == SPIN {
                            "valueModified"
                        } else {
                            "moved"
                        },
                        vec![],
                    )
                },
            );
            for method in ["increase", "decrease"] {
                self = self.context_method(method, vec![], "var", move |ctx, _| {
                    ctx.step_range(method == "increase")
                });
            }
        }
        self
    }
}
fn num(model: &Instance, id: ObjectId, name: &str) -> f64 {
    match model.key(id, name).ok().map(|key| &model.slots[key].value) {
        Some(Value::Number(n)) => *n,
        _ => 0.,
    }
}
impl Transaction<'_> {
    pub(crate) fn observe_range(&mut self, key: SlotId) {
        let p = &self.model.properties[key];
        if matches!(
            p.name.as_str(),
            "value" | "from" | "to" | "orientation" | "LayoutMirroring.enabled"
        ) && self.model.nodes[p.owner.index]
            .properties
            .contains_key(KIND)
        {
            self.range_pending.insert(p.owner);
        }
    }
    pub(crate) fn initialize_ranges(&mut self, first: usize) {
        self.range_pending.extend(
            self.model.nodes[first..]
                .iter()
                .filter(|n| n.properties.contains_key(KIND))
                .map(|n| n.id),
        );
    }
    pub(crate) fn stage_range(&mut self) -> Result<bool, Error> {
        let Some(id) = self.range_pending.pop_first() else {
            return Ok(false);
        };
        if self.model.node(id).is_none() {
            return Ok(true);
        }
        let start = num(self.model, id, "from");
        let end = num(self.model, id, "to");
        let old = num(self.model, id, "value");
        if !start.is_finite() || !end.is_finite() || !old.is_finite() {
            return Err(Error::new("range values must be finite"));
        }
        let value = old.clamp(start.min(end), start.max(end));
        self.set_value(self.model.key(id, "value")?, Value::Number(value));
        if num(self.model, id, KIND) as i32 == SPIN {
            self.set_value(
                self.model.key(id, "displayText")?,
                Value::String(format!("{}", value as i32)),
            );
        } else {
            let p = if start == end {
                0.
            } else {
                (value - start) / (end - start)
            };
            let reverse = num(self.model, id, "orientation") == 2.
                || self.model.get(id, "LayoutMirroring.enabled") == Ok(Value::Bool(true));
            self.set_value(self.model.key(id, "position")?, Value::Number(p));
            self.set_value(
                self.model.key(id, "visualPosition")?,
                Value::Number(if reverse { 1. - p } else { p }),
            );
        }
        Ok(true)
    }
    pub(crate) fn step_quick_range(
        &mut self,
        id: ObjectId,
        increase: bool,
    ) -> Result<Value, Error> {
        let start = num(self.model, id, "from");
        let end = num(self.model, id, "to");
        let spin = num(self.model, id, KIND) as i32 == SPIN;
        let mut step = num(self.model, id, "stepSize");
        if !spin && step == 0. {
            step = 0.1;
        }
        if !increase {
            step = -step;
        }
        if spin && start > end {
            step = -step;
        }
        let min = start.min(end);
        let max = start.max(end);
        let mut next = num(self.model, id, "value") + step;
        if spin && self.model.get(id, "wrap") == Ok(Value::Bool(true)) {
            if next > max {
                next = min;
            } else if next < min {
                next = max;
            }
        }
        let value = self.model.key(id, "value")?;
        self.write(value, Value::Number(next.clamp(min, max)))?;
        Ok(Value::Undefined)
    }
}
