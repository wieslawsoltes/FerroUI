//! JavaScript arrays share object identity, sparse indices and ordinary fields.
//! Conversion at a typed QML list boundary produces a separate typed sequence.
use super::*;
use crate::transaction::Transaction;

fn index(key: &str) -> Option<u32> {
    let n = key.parse::<u32>().ok()?;
    (n != u32::MAX && n.to_string() == key).then_some(n)
}

fn offset(value: &Value, len: u32) -> Result<u32, Error> {
    let number = value.number()?.trunc();
    if number.is_nan() {
        return Ok(0);
    }
    Ok(if number < 0. {
        (f64::from(len) + number).clamp(0., f64::from(len)) as u32
    } else {
        number.min(f64::from(len)) as u32
    })
}

impl Instance {
    pub(crate) fn array_length(&self, array: &Record) -> Result<u32, Error> {
        match self.record_entries(array)?.get("length") {
            Some(Value::Number(n)) if array.is_array() => Ok(*n as u32),
            _ => Err(Error::new("expected a JavaScript array")),
        }
    }

    /// Copy a dense JavaScript array to a Rust sequence. Sparse arrays can be
    /// inspected with record_entries without allocating missing elements.
    pub fn array_values(&self, array: &Record) -> Result<Vec<Value>, Error> {
        let fields = self.record_entries(array)?;
        let len = self.array_length(array)?;
        if len as usize >= fields.len() {
            return Err(Error::new(
                "sparse array cannot be copied to a typed sequence",
            ));
        }
        (0..len)
            .map(|i| {
                fields
                    .get(&i.to_string())
                    .cloned()
                    .ok_or_else(|| Error::new("sparse array cannot be copied to a typed sequence"))
            })
            .collect()
    }

    pub(crate) fn value_string(&self, v: &Value) -> Result<String, Error> {
        match v {
            Value::Record(r) if r.is_array() => self.join_array(r, ",", &mut Vec::new()),
            _ => Ok(v.js_string()),
        }
    }

    fn join_array(
        &self,
        array: &Record,
        separator: &str,
        path: &mut Vec<Record>,
    ) -> Result<String, Error> {
        if path.contains(array) {
            return Ok(String::new());
        }
        if path.len() >= 64 {
            return Err(Error::new("nested array conversion limit exceeded"));
        }
        path.push(array.clone());
        let fields = self.record_entries(array)?;
        let len = self.array_length(array)?;
        const MAX_TEXT: usize = 16 * 1024 * 1024;
        let separators = (len.saturating_sub(1) as usize).saturating_mul(separator.len());
        if separators > MAX_TEXT {
            return Err(Error::new("array string exceeds the text limit"));
        }
        let mut result = String::new();
        let mut text_bytes = 0usize;
        // Sorting populated indices visits sparse arrays in numeric order
        // without iterating over all holes, including when separator is empty.
        let mut present: Vec<_> = fields
            .iter()
            .filter_map(|(key, v)| index(key).map(|i| (i, v)))
            .collect();
        present.sort_unstable_by_key(|(i, _)| *i);
        let mut previous = 0u32;
        for (i, value) in present {
            if !separator.is_empty() {
                result.push_str(&separator.repeat((i - previous) as usize));
            }
            previous = i;
            let text = match value {
                Value::Undefined | Value::Null => continue,
                Value::Record(r) if r.is_array() => self.join_array(r, ",", path)?,
                other => other.js_string(),
            };
            text_bytes = text_bytes.saturating_add(text.len());
            if text_bytes.saturating_add(separators) > MAX_TEXT {
                return Err(Error::new("array string exceeds the text limit"));
            }
            result.push_str(&text);
        }
        if !separator.is_empty() {
            result.push_str(&separator.repeat(len.saturating_sub(previous + 1) as usize));
        }
        path.pop();
        Ok(result)
    }

    pub(crate) fn js_binary(&self, op: &str, mut a: Value, mut b: Value) -> Result<Value, Error> {
        let same_class = matches!((&a, &b), (Value::Record(_), Value::Record(_)));
        let equality = matches!(op, "==" | "!=");
        let nullish = matches!(a, Value::Null | Value::Undefined)
            || matches!(b, Value::Null | Value::Undefined);
        if !matches!(op, "===" | "!==") && !(equality && (same_class || nullish)) {
            if matches!(a, Value::Record(_)) {
                a = Value::String(self.value_string(&a)?);
            }
            if matches!(b, Value::Record(_)) {
                b = Value::String(self.value_string(&b)?);
            }
        }
        evaluation::binary(op, a, b)
    }

    pub(crate) fn js_unary(&self, op: &str, mut value: Value) -> Result<Value, Error> {
        if matches!(op, "+" | "-" | "~") && matches!(value, Value::Record(_)) {
            value = Value::String(self.value_string(&value)?);
        }
        evaluation::unary(op, value)
    }
}

impl Transaction<'_> {
    pub(crate) fn new_array(&mut self, items: Vec<Value>) -> Value {
        let length = items.len() as u32;
        let fields = items
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect();
        self.sparse_array(length, fields)
    }

    fn sparse_array(&mut self, length: u32, mut fields: BTreeMap<String, Value>) -> Value {
        fields.insert("length".into(), Value::Number(length as f64));
        Value::Record(self.model.records.create(self.model.identity, true, fields))
    }

    pub(crate) fn write_array(
        &mut self,
        array: &Record,
        key: &str,
        value: Value,
    ) -> Result<(), Error> {
        let length = self.model.array_length(array)?;
        if key == "length" {
            let num = value.number()?;
            if num.fract() != 0. || !(0.0..=u32::MAX as f64).contains(&num) {
                return Err(Error::new("invalid array length"));
            }
            let new_length = num as u32;
            if new_length < length {
                let removed: Vec<_> = self
                    .model
                    .record_entries(array)?
                    .keys()
                    .filter(|k| index(k).is_some_and(|i| i >= new_length))
                    .cloned()
                    .collect();
                for k in removed {
                    self.store_field(array, &k, None)?;
                }
            }
            return self.store_field(array, key, Some(Value::Number(num)));
        }
        if let Some(i) = index(key).filter(|i| *i >= length) {
            self.store_field(array, "length", Some(Value::Number(f64::from(i) + 1.)))?;
        }
        self.store_field(array, key, Some(value))
    }

    pub(crate) fn array_method(
        &mut self,
        array: &Record,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        let len = self.model.array_length(array)?;
        if self.model.record_entries(array)?.contains_key(name) {
            return Err(Error::new(format!("array field '{name}' is not callable")));
        }
        match name {
            "push" => {
                let count = u32::try_from(args.len())
                    .ok()
                    .and_then(|n| len.checked_add(n))
                    .ok_or_else(|| Error::new("invalid array length"))?;
                for (i, value) in args.into_iter().enumerate() {
                    self.set_record_field(array, &(len + i as u32).to_string(), value)?;
                }
                Ok(Value::Number(count as f64))
            }
            "pop" if len == 0 => Ok(Value::Undefined),
            "pop" => {
                let key = (len - 1).to_string();
                let value = self
                    .model
                    .record_entries(array)?
                    .get(&key)
                    .cloned()
                    .unwrap_or(Value::Undefined);
                self.store_field(array, &key, None)?;
                self.store_field(array, "length", Some(Value::Number((len - 1) as f64)))?;
                Ok(value)
            }
            "join" | "toString" => {
                let separator = match args.first() {
                    Some(value) if *value != Value::Undefined && name == "join" => {
                        self.model.value_string(value)?
                    }
                    _ => ",".into(),
                };
                Ok(Value::String(self.model.join_array(
                    array,
                    &separator,
                    &mut Vec::new(),
                )?))
            }
            "at" => {
                let n = args.first().unwrap_or(&Value::Undefined).number()?.trunc();
                let idx = if n < 0. {
                    f64::from(len) + n
                } else if n.is_nan() {
                    0.
                } else {
                    n
                };
                if idx < 0. || idx >= f64::from(len) {
                    return Ok(Value::Undefined);
                }
                Ok(self
                    .model
                    .record_entries(array)?
                    .get(&(idx as u32).to_string())
                    .cloned()
                    .unwrap_or(Value::Undefined))
            }
            "slice" => {
                let start = args
                    .first()
                    .map(|v| offset(v, len))
                    .transpose()?
                    .unwrap_or(0);
                let end = args
                    .get(1)
                    .filter(|v| **v != Value::Undefined)
                    .map(|v| offset(v, len))
                    .transpose()?
                    .unwrap_or(len);
                let mut fields = BTreeMap::new();
                let mut length = 0;
                // Qt 6.12.0 QJSEngine trims trailing holes in a slice. Retain
                // leading/interior holes by preserving the relative indices.
                for (key, value) in self.model.record_entries(array)? {
                    if let Some(i) = index(key).filter(|i| (start..end).contains(i)) {
                        let next = i - start;
                        length = length.max(next + 1);
                        fields.insert(next.to_string(), value.clone());
                    }
                }
                Ok(self.sparse_array(length, fields))
            }
            _ => Err(Error::new(format!("unsupported array method '{name}'"))),
        }
    }
}
