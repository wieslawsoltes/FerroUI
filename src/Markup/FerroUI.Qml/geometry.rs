//! Owned geometry crosses Rust and JavaScript boundaries by value. A script
//! reading a typed QML property gets a reference that can write back to its slot.
use super::*;
use crate::transaction::Transaction;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Geometry {
    Point {
        x: f64,
        y: f64,
    },
    Size {
        width: f64,
        height: f64,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}

impl Geometry {
    pub fn from_components(kind: &str, values: &[f64]) -> Result<Self, Error> {
        match (kind, values) {
            ("point", [x, y]) => Ok(Self::Point { x: *x, y: *y }),
            ("size", [width, height]) => Ok(Self::Size {
                width: *width,
                height: *height,
            }),
            ("rect", [x, y, width, height]) => Ok(Self::Rect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            }),
            _ => Err(Error::new(format!("invalid {kind} components"))),
        }
    }
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Point { .. } => "point",
            Self::Size { .. } => "size",
            Self::Rect { .. } => "rect",
        }
    }
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Point { .. } => &["x", "y"],
            Self::Size { .. } => &["width", "height"],
            Self::Rect { .. } => &[
                "x", "y", "width", "height", "left", "right", "top", "bottom",
            ],
        }
    }
    pub fn components(self) -> Vec<f64> {
        match self {
            Self::Point { x, y } => vec![x, y],
            Self::Size { width, height } => vec![width, height],
            Self::Rect {
                x,
                y,
                width,
                height,
            } => vec![x, y, width, height],
        }
    }
    pub fn field(self, name: &str) -> Option<f64> {
        match (self, name) {
            (Self::Point { x, .. } | Self::Rect { x, .. }, "x")
            | (Self::Rect { x, .. }, "left") => Some(x),
            (Self::Point { y, .. } | Self::Rect { y, .. }, "y") | (Self::Rect { y, .. }, "top") => {
                Some(y)
            }
            (Self::Size { width, .. } | Self::Rect { width, .. }, "width") => Some(width),
            (Self::Size { height, .. } | Self::Rect { height, .. }, "height") => Some(height),
            (Self::Rect { x, width, .. }, "right") => Some(x + width),
            (Self::Rect { y, height, .. }, "bottom") => Some(y + height),
            _ => None,
        }
    }
    pub(crate) fn with(mut self, name: &str, n: f64) -> Result<Self, Error> {
        match (&mut self, name) {
            (Self::Point { x, .. } | Self::Rect { x, .. }, "x") => *x = n,
            (Self::Point { y, .. } | Self::Rect { y, .. }, "y") => *y = n,
            (Self::Size { width, .. } | Self::Rect { width, .. }, "width") => *width = n,
            (Self::Size { height, .. } | Self::Rect { height, .. }, "height") => *height = n,
            _ => {
                return Err(Error::new(format!(
                    "unknown or readonly {} field '{name}'",
                    self.type_name()
                )))
            }
        }
        Ok(self)
    }
    pub(crate) fn default_for(name: &str) -> Option<Self> {
        match name {
            "point" => Some(Self::Point { x: 0., y: 0. }),
            "size" => Some(Self::Size {
                width: -1.,
                height: -1.,
            }),
            "rect" => Some(Self::Rect {
                x: 0.,
                y: 0.,
                width: 0.,
                height: 0.,
            }),
            _ => None,
        }
    }
    pub(crate) fn parse(kind: &str, text: &str) -> Result<Self, Error> {
        let invalid = || Error::new(format!("invalid {kind} literal '{text}'"));
        let parts: Vec<_> = match kind {
            "point" => text.split(',').collect(),
            "size" => text.split('x').collect(),
            "rect" => {
                let (x, rest) = text.split_once(',').ok_or_else(invalid)?;
                let (y, wh) = rest.split_once(',').ok_or_else(invalid)?;
                let (w, h) = wh.split_once('x').ok_or_else(invalid)?;
                vec![x, y, w, h]
            }
            _ => return Err(invalid()),
        };
        let values = parts
            .into_iter()
            .map(|v| v.trim().parse::<f64>().map_err(|_| invalid()))
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_components(kind, &values)
    }
}
impl std::fmt::Display for Geometry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Point { x, y } => write!(f, "QPointF({x}, {y})"),
            Self::Size { width, height } => write!(f, "QSizeF({width}, {height})"),
            Self::Rect {
                x,
                y,
                width,
                height,
            } => write!(f, "QRectF({x}, {y}, {width}, {height})"),
        }
    }
}

impl Instance {
    pub(crate) fn geometry_snapshot(&self, value: &Value) -> Result<Option<Geometry>, Error> {
        match value {
            Value::Geometry(shape) => Ok(Some(*shape)),
            Value::Record(record) if record.is_geometry() => {
                let value = if let Some((owner, key)) = record.geometry_slot() {
                    self.node(owner)
                        .ok_or_else(|| Error::new("expired geometry property reference"))?;
                    &self.slots[key].value
                } else {
                    self.record_entries(record)?
                        .get("$qml:geometry")
                        .ok_or_else(|| Error::new("missing geometry value"))?
                };
                match value {
                    Value::Geometry(shape) => Ok(Some(*shape)),
                    _ => Err(Error::new("geometry property no longer contains a value")),
                }
            }
            _ => Ok(None),
        }
    }
    pub(crate) fn structured_geometry(
        &self,
        mut shape: Geometry,
        value: &Value,
    ) -> Result<Value, Error> {
        for name in shape.fields() {
            let field = match value {
                Value::Record(record) => self.record_entries(record)?.get(*name).cloned(),
                Value::Object(id) => self.get(*id, name).ok(),
                Value::Geometry(input) => input.field(name).map(Value::Number),
                Value::Null | Value::List(_) => None,
                _ => {
                    return Err(Error::new(format!(
                        "cannot assign {value:?} to '{}'",
                        shape.type_name()
                    )))
                }
            };
            if let Some(field) = field.filter(|v| *v != Value::Undefined) {
                let number = match &field {
                    Value::Record(_) | Value::List(_) => {
                        Value::String(self.value_string(&field)?).number()?
                    }
                    _ => field.number()?,
                };
                shape = shape.with(name, number)?;
            }
        }
        Ok(Value::Geometry(shape))
    }

    pub(crate) fn geometry_path(
        &self,
        object: ObjectId,
        path: &str,
    ) -> Option<(SlotId, &'static str)> {
        let (name, field) = path.rsplit_once('.')?;
        if self.public_key(object, path).is_ok() {
            return None;
        }
        let key = self.public_key(object, name).ok()?;
        let Value::Geometry(shape) = &self.slots[key].value else {
            return None;
        };
        Some((key, *shape.fields().iter().find(|n| **n == field)?))
    }
}

impl Transaction<'_> {
    pub(crate) fn geometry_object(&mut self, shape: Geometry, source: Option<SlotId>) -> Value {
        Value::Record(self.model.records.geometry(
            self.model.identity,
            shape,
            source.map(|key| (self.model.properties[key].owner, key)),
        ))
    }
    pub(crate) fn script_read(
        &mut self,
        key: SlotId,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        let value = self.read(key, reads)?;
        Ok(if let Value::Geometry(shape) = value {
            self.geometry_object(shape, Some(key))
        } else {
            value
        })
    }
    pub(crate) fn geometry_member(
        &mut self,
        r: &Record,
        name: &str,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        let shape = if let Some((owner, key)) = r.geometry_slot() {
            self.model
                .node(owner)
                .ok_or_else(|| Error::new("expired geometry property reference"))?;
            self.read(key, reads)?
        } else {
            self.model
                .record_entries(r)?
                .get("$qml:geometry")
                .cloned()
                .unwrap_or(Value::Undefined)
        };
        Ok(if let Value::Geometry(shape) = shape {
            shape.field(name).map_or(Value::Undefined, Value::Number)
        } else {
            Value::Undefined
        })
    }
    pub(crate) fn write_geometry_slot(
        &mut self,
        key: SlotId,
        name: &str,
        v: Value,
    ) -> Result<(), Error> {
        let key = self.model.write_target(key)?;
        if let Some(InstalledBinding::GeometryFields { base, fields }) =
            self.model.slots[key].binding.clone()
        {
            if let Some((_, input)) = fields.iter().find(|(field, _)| *field == name) {
                return self.write(*input, v);
            }
            if self.in_binding() {
                return Err(Error::new("a binding cannot modify a geometry property"));
            }
            let base = base.with(name, v.number()?)?;
            let slot = self.touched(key);
            slot.binding = Some(InstalledBinding::GeometryFields { base, fields });
            slot.dirty = true;
            self.pending.insert(key);
            return Ok(());
        }
        let Value::Geometry(shape) = self.read(key, &mut BTreeSet::new())? else {
            return Err(Error::new("expected a geometry property"));
        };
        self.write(key, Value::Geometry(shape.with(name, v.number()?)?))
    }
    pub(crate) fn write_geometry(&mut self, r: &Record, name: &str, v: Value) -> Result<(), Error> {
        if let Some((id, key)) = r.geometry_slot() {
            self.model
                .node(id)
                .ok_or_else(|| Error::new("expired geometry property reference"))?;
            self.write_geometry_slot(key, name, v)
        } else {
            let shape = self
                .model
                .geometry_snapshot(&Value::Record(r.clone()))?
                .expect("geometry object");
            self.store_field(
                r,
                "$qml:geometry",
                Some(Value::Geometry(shape.with(name, v.number()?)?)),
            )
        }
    }
}
