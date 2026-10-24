//! The snapshot of the property defaults of the public types of `ferroui-base` and
//! `ferroui-controls`: the API gate of the default values (docs/release.md, "API stability").
//!
//! `cargo-semver-checks` compares the signatures of the public API with the last release; it
//! cannot see a default value, which lives in the metadata of a property. This test records, for
//! every styled and attached property of the public types (the types with a Rust path, in the
//! `FerroUI` namespaces):
//!
//! - one line per property: the owner, the name, the value type, the default for the owner, the
//!   default binding mode, and whether it inherits or is attached;
//! - one line per type whose default or default binding mode of a property differs from the one
//!   of its base type (a metadata override, or a registration with `add_owner`).
//!
//! and compares them with `property_defaults.snapshot`. A changed or removed line is a change of
//! the public API that 0.4 does not allow; an added line (a new property or type) is fine and is
//! recorded by regenerating the snapshot:
//!
//! ```text
//! FERROUI_UPDATE_PROPERTY_DEFAULTS=1 cargo test -p ferroui-controls --lib property_defaults
//! ```

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{BoxedValue, FerroProperty, FerroPropertyRegistry, TypeInfo};
use std::collections::{BTreeMap, BTreeSet};

const SNAPSHOT: &str = "property_defaults.snapshot";
const UPDATE: &str = "FERROUI_UPDATE_PROPERTY_DEFAULTS";

/// The Rust type name with each path reduced to its last segment
/// (`core::option::Option<ferroui_base::media::Brush>` is `Option<Brush>`): stable across
/// moves of modules and across compilers, whose `type_name` is not specified.
fn short_type_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String| {
        out.push_str(token.rsplit("::").next().unwrap_or(token));
        token.clear();
    };
    for c in name.chars() {
        if c.is_alphanumeric() || c == '_' || c == ':' {
            token.push(c);
        } else {
            flush(&mut token, &mut out);
            out.push(c);
        }
    }
    flush(&mut token, &mut out);
    out
}

/// The text of a default value: a string quoted, null as `(null)`, otherwise the display form
/// the bindings use (the member name of an enumeration, the type name of an object).
fn display(value: &BoxedValue) -> String {
    let Some(value) = ValueTypes::normalize(value.clone()) else {
        return "(null)".to_string();
    };
    if let Some(text) = value.downcast_ref::<String>() {
        return format!("{text:?}");
    }
    ValueTypes::to_display_string(Some(&value)).replace('\n', "\\n")
}

fn public_types() -> Vec<&'static TypeInfo> {
    let mut types: Vec<&'static TypeInfo> = TypeInfo::registered_types()
        .into_iter()
        .filter(|t| t.rust_path().is_some() && t.namespace().starts_with("FerroUI"))
        .collect();
    types.sort_by_key(|t| t.full_name());
    types.dedup_by(|a, b| std::ptr::eq(*a, *b));
    types
}

/// The lines of the snapshot, keyed by everything before ` = `.
fn snapshot_lines() -> BTreeMap<String, String> {
    crate::register_types();
    let types = public_types();
    for t in types.iter().copied() {
        t.ensure_class_init();
    }

    let registry = FerroPropertyRegistry::instance();
    let mut properties: Vec<&'static FerroProperty> = Vec::new();
    let mut seen = BTreeSet::new();
    for t in types.iter().copied() {
        for property in registry.get_registered(t).iter().chain(registry.get_registered_attached(t).iter()) {
            if !property.is_direct() && seen.insert(property.id()) {
                properties.push(property);
            }
        }
    }

    let mut lines = BTreeMap::new();
    for property in properties.iter().copied() {
        let owner = property.owner_type();
        let at = property.host_type().filter(|_| property.is_attached()).unwrap_or(owner);
        let mut flags = String::new();
        if property.inherits() {
            flags.push_str("; inherits");
        }
        if property.is_attached() {
            flags.push_str("; attached");
        }
        lines.insert(
            format!("{}.{}: {}", owner.full_name(), property.name(), short_type_name(property.property_type_name())),
            format!(
                "{}; {:?}{}",
                display(&property.default_value_boxed(at)),
                property.get_metadata(at).default_binding_mode(),
                flags
            ),
        );
    }

    for t in types.iter().copied() {
        let Some(base) = t.base_type() else { continue };
        for property in properties.iter().copied() {
            let value = property.default_value_boxed(t);
            let mode = property.get_metadata(t).default_binding_mode();
            let base_value = property.default_value_boxed(base);
            let base_mode = property.get_metadata(base).default_binding_mode();
            if mode != base_mode || !value.value_eq(&*base_value) {
                let text = format!("{}; {:?}", display(&value), mode);
                // An object default compares by identity: a type that registers the same text
                // again (a new brush of the same colour) is not an observable change of the text.
                if mode == base_mode && text == format!("{}; {:?}", display(&base_value), base_mode) {
                    continue;
                }
                lines.insert(
                    format!("{} overrides {}.{}", t.full_name(), property.owner_type().full_name(), property.name()),
                    text,
                );
            }
        }
    }
    lines
}

fn parse(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once(" = ").map(|(k, v)| (k.to_string(), v.to_string())))
        .collect()
}

fn render(lines: &BTreeMap<String, String>) -> String {
    let mut text = String::from(
        "# The defaults of the styled and attached properties of the public types of ferroui-base and\n\
         # ferroui-controls (property_defaults_tests.rs). Generated: regenerate with\n\
         # FERROUI_UPDATE_PROPERTY_DEFAULTS=1 cargo test -p ferroui-controls --lib property_defaults\n",
    );
    for (key, value) in lines {
        text.push_str(key);
        text.push_str(" = ");
        text.push_str(value);
        text.push('\n');
    }
    text
}

#[test]
fn property_defaults_match_the_snapshot() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT);
    let actual = snapshot_lines();
    assert!(actual.len() > 1000, "the walk found only {} properties", actual.len());
    if std::env::var_os(UPDATE).is_some() {
        std::fs::write(&path, render(&actual)).expect("the snapshot is writable");
        return;
    }
    let expected = parse(&std::fs::read_to_string(&path).unwrap_or_default());

    let mut changed = Vec::new();
    let mut removed = Vec::new();
    let mut added = Vec::new();
    for (key, value) in &expected {
        match actual.get(key) {
            None => removed.push(format!("{key} = {value}")),
            Some(now) if now != value => changed.push(format!("{key} = {value}  ->  {now}")),
            Some(_) => {}
        }
    }
    for (key, value) in &actual {
        if !expected.contains_key(key) {
            added.push(format!("{key} = {value}"));
        }
    }
    if changed.is_empty() && removed.is_empty() && added.is_empty() {
        return;
    }
    let mut message = String::new();
    if !changed.is_empty() || !removed.is_empty() {
        message.push_str(
            "Property defaults changed or properties were removed: within 0.4 the public API only grows \
             (docs/release.md, \"API stability\"). Restore them, or record an intended change.\n",
        );
        for line in changed.iter().chain(removed.iter()) {
            message.push_str("  ");
            message.push_str(line);
            message.push('\n');
        }
    }
    if !added.is_empty() {
        message.push_str("Properties or overrides were added; record them in the snapshot:\n");
        for line in added.iter().take(50) {
            message.push_str("  + ");
            message.push_str(line);
            message.push('\n');
        }
        if added.len() > 50 {
            message.push_str(&format!("  ... and {} more\n", added.len() - 50));
        }
    }
    message.push_str(&format!("Regenerate with {UPDATE}=1 cargo test -p ferroui-controls --lib property_defaults"));
    panic!("{message}");
}

#[test]
fn short_type_names_drop_the_paths() {
    assert_eq!(
        "Option<Ref<Brush>>",
        short_type_name("core::option::Option<ferroui_base::type_system::Ref<ferroui_base::media::brush::Brush>>")
    );
    assert_eq!("(f64, bool)", short_type_name("(f64, bool)"));
}
