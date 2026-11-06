//! Derived from ScreenKeyboard.Uno/Icons/IconFactory.cs (Apache-2.0).
use super::icon_data::{NAMES, PATHS};
use ferroboard_core::{keys::KeyIcon, text::Utf16Text};
use ferroui_base::{
    Point, Ref,
    media::{
        BezierSegment, FillRule, Geometry, IBrush, LineSegment, PathFigure, PathGeometry, Stretch,
    },
    utilities::span_helpers::parse_double,
};
use ferroui_controls::shapes::Path;
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconError {
    NullPath,
    MissingNumber { token: usize },
    InvalidNumber { token: usize },
}
impl std::fmt::Display for IconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for IconError {}

#[derive(Clone, Copy, Debug)]
struct Command {
    kind: u16,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x: f64,
    y: f64,
}
#[derive(Default)]
struct Cache {
    parsed: HashMap<i32, Arc<[Command]>>,
    custom: HashMap<i32, Option<Utf16Text>>,
}
fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Cache::default()))
}
fn builtin(icon: i32) -> Option<&'static str> {
    usize::try_from(icon)
        .ok()
        .and_then(|i| PATHS.get(i).copied().flatten())
}

/// Thread-safe lazy command cache and native vector construction. Numeric-value overloads bridge
/// unnamed source enum values; nullable UTF-16 registration retains source null/error behavior.
pub struct IconFactory;
impl IconFactory {
    pub(crate) fn name_value(icon: i32) -> String {
        usize::try_from(icon)
            .ok()
            .and_then(|i| NAMES.get(i))
            .map_or_else(|| icon.to_string(), |name| (*name).to_owned())
    }
    pub fn register(icon: KeyIcon, normalized_path: impl Into<Utf16Text>) {
        Self::register_nullable(icon, Some(normalized_path.into()));
    }
    /// Explicit nullable source-string bridge. Registration itself does not parse or reject null.
    pub fn register_nullable(icon: KeyIcon, normalized_path: Option<Utf16Text>) {
        Self::register_value(icon.0, normalized_path);
    }
    pub fn register_value(icon: i32, normalized_path: Option<Utf16Text>) {
        // A source monitor is released, not poisoned, if a prior call throws.
        let mut cache = cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.custom.insert(icon, normalized_path);
        cache.parsed.remove(&icon);
    }
    pub fn has_icon(icon: KeyIcon) -> bool {
        Self::has_icon_value(icon.0)
    }
    pub fn has_icon_value(icon: i32) -> bool {
        if icon == KeyIcon::None.0 {
            return false;
        }
        let cache = cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.custom.contains_key(&icon) || builtin(icon).is_some()
    }
    pub fn create(
        icon: KeyIcon,
        size: f64,
        fill: Option<Rc<dyn IBrush>>,
    ) -> Result<Ref<Path>, IconError> {
        Self::create_value(icon.0, size, fill)
    }
    pub fn create_value(
        icon: i32,
        size: f64,
        fill: Option<Rc<dyn IBrush>>,
    ) -> Result<Ref<Path>, IconError> {
        let path = Path::new();
        path.set_data(Self::create_geometry_value(icon, size)?);
        path.set_fill(fill);
        path.set_width(size);
        path.set_height(size);
        path.set_stretch(Stretch::None);
        path.set_is_hit_test_visible(false);
        Ok(path)
    }
    pub fn create_geometry(icon: KeyIcon, size: f64) -> Result<Ref<Geometry>, IconError> {
        Self::create_geometry_value(icon.0, size)
    }
    pub fn create_geometry_value(icon: i32, size: f64) -> Result<Ref<Geometry>, IconError> {
        let commands = get_commands(icon)?;
        let geometry = PathGeometry::new();
        geometry.set_fill_rule(FillRule::NonZero);
        let figures = geometry.figures().expect("new native geometry has figures");
        let mut figure: Option<Ref<PathFigure>> = None;
        let p = |x, y| Point::new(x * size, y * size);
        for c in commands.iter() {
            match c.kind {
                77 => {
                    let next = PathFigure::new();
                    next.set_start_point(p(c.x, c.y));
                    next.set_is_closed(false);
                    next.set_is_filled(true);
                    figures.add(next.clone());
                    figure = Some(next);
                }
                76 => {
                    if let Some(figure) = &figure {
                        let line = LineSegment::new();
                        line.set_point(p(c.x, c.y));
                        figure
                            .segments()
                            .expect("new figure has segments")
                            .add(line.upcast());
                    }
                }
                67 => {
                    if let Some(figure) = &figure {
                        let curve = BezierSegment::new();
                        curve.set_point1(p(c.x1, c.y1));
                        curve.set_point2(p(c.x2, c.y2));
                        curve.set_point3(p(c.x, c.y));
                        figure
                            .segments()
                            .expect("new figure has segments")
                            .add(curve.upcast());
                    }
                }
                90 => {
                    if let Some(figure) = &figure {
                        figure.set_is_closed(true);
                    }
                }
                _ => unreachable!("parser only emits M/L/C/Z"),
            }
        }
        Ok(geometry.upcast())
    }
}
fn get_commands(icon: i32) -> Result<Arc<[Command]>, IconError> {
    let mut cache = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(commands) = cache.parsed.get(&icon) {
        return Ok(commands.clone());
    }
    let fallback;
    let data = if let Some(custom) = cache.custom.get(&icon) {
        custom.as_ref().ok_or(IconError::NullPath)?
    } else {
        fallback = Utf16Text::from(builtin(icon).unwrap_or(""));
        &fallback
    };
    let commands: Arc<[Command]> = parse(data)?.into();
    cache.parsed.insert(icon, commands.clone());
    Ok(commands)
}
fn invariant_number(token: &str) -> Option<f64> {
    if let Some(value) = parse_double(token) {
        return Some(value);
    }
    // The source runtime's numeric parser accepts trailing NULs after numeric syntax (including
    // overflow), but not after special symbols such as "NaN" or "Infinity". The pinned framework
    // helper doesn't implement that legacy rule. Do not trim whitespace after a NUL.
    let body = token.trim_end_matches('\0');
    if body.len() == token.len() {
        return None;
    }
    let unsigned = body.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let unsigned = unsigned.strip_prefix(['+', '-']).unwrap_or(unsigned);
    if !unsigned
        .as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
    {
        return None;
    }
    parse_double(body)
}
fn parse(data: &Utf16Text) -> Result<Vec<Command>, IconError> {
    // Split only U+0020, RemoveEmptyEntries; unknown tokens (including command case) are ignored.
    let tokens: Vec<_> = data
        .units()
        .split(|&unit| unit == 32)
        .filter(|s| !s.is_empty())
        .collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        i += 1;
        let kind = match t {
            [kind @ (77 | 76 | 67 | 90)] => *kind,
            _ => continue,
        };
        let mut c = Command {
            kind,
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
            x: 0.0,
            y: 0.0,
        };
        let mut number = || {
            let index = i;
            i += 1;
            let token = tokens
                .get(index)
                .ok_or(IconError::MissingNumber { token: index })?;
            let token =
                String::from_utf16(token).map_err(|_| IconError::InvalidNumber { token: index })?;
            let value =
                invariant_number(&token).ok_or(IconError::InvalidNumber { token: index })?;
            // System.Double.NaN uses a negative canonical NaN even for Parse("+NaN").
            Ok::<_, IconError>(if value.is_nan() {
                f64::from_bits(0xfff8_0000_0000_0000)
            } else {
                value
            })
        };
        match kind {
            77 | 76 => {
                c.x = number()?;
                c.y = number()?;
            }
            67 => {
                c.x1 = number()?;
                c.y1 = number()?;
                c.x2 = number()?;
                c.y2 = number()?;
                c.x = number()?;
                c.y = number()?;
            }
            90 => (),
            _ => unreachable!(),
        }
        result.push(c);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn bits(value: f64) -> String {
        format!("{:016x}", value.to_bits())
    }
    /// `value` with every number written as its bits that is a NaN replaced by the canonical
    /// quiet NaN. The sign and payload of a NaN depend on the processor that computed it (x86-64
    /// makes negative NaNs where ARM makes positive ones), on either side of the comparison.
    fn canonical_nan(value: Value) -> Value {
        match value {
            Value::String(text) => match u64::from_str_radix(&text, 16) {
                Ok(bits) if text.len() == 16 && f64::from_bits(bits).is_nan() => {
                    Value::String(format!("{:016x}", f64::NAN.to_bits()))
                }
                _ => Value::String(text),
            },
            Value::Array(items) => Value::Array(items.into_iter().map(canonical_nan).collect()),
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .map(|(key, item)| (key, canonical_nan(item)))
                    .collect(),
            ),
            other => other,
        }
    }
    fn point(p: Point) -> Value {
        json!([bits(p.x), bits(p.y)])
    }
    fn geometry_data(geometry: Ref<Geometry>) -> Value {
        let geometry = geometry.downcast::<PathGeometry>().unwrap();
        let figures: Vec<_> = geometry.figures().unwrap().iter().map(|f| {
            let segments: Vec<_> = f.segments().unwrap().iter().map(|s| {
                if let Some(line) = s.downcast_ref::<LineSegment>() {
                    json!({"kind":"L","points":[point(line.point())]})
                } else {
                    let curve = s.downcast_ref::<BezierSegment>().unwrap();
                    json!({"kind":"C","points":[point(curve.point1()),point(curve.point2()),point(curve.point3())]})
                }
            }).collect();
            json!({"start":point(f.start_point()),"closed":f.is_closed(),"filled":f.is_filled(),"segments":segments})
        }).collect();
        json!({"fillRule":geometry.fill_rule() as i32,"figures":figures})
    }
    #[test]
    fn unchanged_csharp_parser_cache_and_native_geometry_reference() {
        crate::test_support::run(icon_reference);
    }
    fn icon_reference() {
        // Only commands are cached; public geometry and its mutable figures must be fresh.
        let first = IconFactory::create_geometry(KeyIcon::None, 1.0).unwrap();
        let second = IconFactory::create_geometry(KeyIcon::None, 1.0).unwrap();
        assert_ne!(first, second);
        let first = first.downcast::<PathGeometry>().unwrap();
        let second = second.downcast::<PathGeometry>().unwrap();
        assert!(first.figures() != second.figures());
        first.figures().unwrap().add(PathFigure::new());
        assert_eq!(second.figures().unwrap().len(), 0);
        let mut rows = 0;
        for line in include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/icons.jsonl"
        ))
        .lines()
        {
            let row: Value = serde_json::from_str(line).unwrap();
            let id = row["index"].as_u64().unwrap();
            let icon = row["icon"].as_i64().unwrap() as i32;
            let size =
                f64::from_bits(u64::from_str_radix(row["size"].as_str().unwrap(), 16).unwrap());
            if row["register"] == true {
                let path = row["path"].as_array().map(|units| {
                    Utf16Text::from_units(
                        units
                            .iter()
                            .map(|u| u.as_u64().unwrap() as u16)
                            .collect::<Vec<_>>(),
                    )
                });
                IconFactory::register_value(icon, path);
            }
            assert_eq!(
                IconFactory::has_icon_value(icon),
                row["hasIcon"].as_bool().unwrap(),
                "HasIcon row {id}"
            );
            match get_commands(icon) {
                Ok(commands) => {
                    assert!(
                        row["error"].is_null(),
                        "row {id} should fail: {}",
                        row["error"]
                    );
                    assert_eq!(
                        Arc::ptr_eq(&commands, &get_commands(icon).unwrap()),
                        row["same"].as_bool().unwrap()
                    );
                    let actual: Vec<_> = commands.iter().map(|c| json!({
                        "kind":char::from_u32(c.kind as u32).unwrap().to_string(),
                        "x1":bits(c.x1),"y1":bits(c.y1),"x2":bits(c.x2),"y2":bits(c.y2),"x":bits(c.x),"y":bits(c.y)
                    })).collect();
                    assert_eq!(
                        canonical_nan(json!(actual)),
                        canonical_nan(row["commands"].clone()),
                        "commands row {id}"
                    );
                    let geometry = IconFactory::create_geometry_value(icon, size).unwrap();
                    assert_eq!(
                        canonical_nan(geometry_data(geometry)),
                        canonical_nan(row["geometry"].clone()),
                        "geometry row {id}"
                    );
                }
                Err(error) => {
                    let expected = match error {
                        IconError::NullPath => "System.NullReferenceException",
                        IconError::MissingNumber { .. } => "System.IndexOutOfRangeException",
                        IconError::InvalidNumber { .. } => "System.FormatException",
                    };
                    assert_eq!(row["error"].as_str(), Some(expected), "error row {id}");
                    assert_eq!(
                        IconFactory::create_geometry_value(icon, size)
                            .unwrap_err()
                            .to_string(),
                        error.to_string()
                    );
                }
            }
            rows += 1;
        }
        assert_eq!(rows, 918);
    }
    #[test]
    fn parsed_cache_identity_invalidation_failure_retry_and_thread_sharing() {
        let id = 97631;
        IconFactory::register_value(id, Some("M 0 0 L 1 1".into()));
        let first = get_commands(id).unwrap();
        assert!(Arc::ptr_eq(&first, &get_commands(id).unwrap()));
        let worker = std::thread::spawn(move || get_commands(id).unwrap())
            .join()
            .unwrap();
        assert!(Arc::ptr_eq(&first, &worker));
        IconFactory::register_value(id, Some("M bad 0".into()));
        assert!(get_commands(id).is_err());
        assert!(get_commands(id).is_err());
        IconFactory::register_value(id, Some("M 0 0 L 1 1".into()));
        assert!(!Arc::ptr_eq(&first, &get_commands(id).unwrap()));
        IconFactory::register_value(id, None);
        assert_eq!(get_commands(id).unwrap_err(), IconError::NullPath);
        assert!(IconFactory::has_icon_value(id));
    }
}

#[cfg(test)]
#[path = "key_icon_tests.rs"]
mod key_icon_tests;
