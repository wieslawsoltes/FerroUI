use super::{valid_token_name, Issue};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy)]
pub(super) struct Token<'a> {
    pub node: &'a Value,
    pub kind: Option<&'a str>,
}

pub(super) struct Tokens<'a> {
    root: &'a Value,
    pub entries: BTreeMap<String, Token<'a>>,
    kinds: HashMap<*const Value, &'a str>,
}

pub(super) struct Resolved<'a> {
    pub value: &'a Value,
    pub kind: Option<&'a str>,
    pub reference: Option<&'a str>,
}

type Result<T> = std::result::Result<T, String>;

impl<'a> Tokens<'a> {
    pub fn read(root: &'a Value, issues: &mut Vec<Issue>) -> Self {
        let mut tokens = Self {
            root,
            entries: BTreeMap::new(),
            kinds: HashMap::new(),
        };
        tokens.group(root, "", None, issues);
        tokens
    }

    fn group(
        &mut self,
        node: &'a Value,
        path: &str,
        parent_kind: Option<&'a str>,
        issues: &mut Vec<Issue>,
    ) {
        let Some(children) = node.as_object() else {
            issues.push(Issue {
                path: path.into(),
                message: "Expected a token or group object".into(),
            });
            return;
        };
        if children.contains_key("$extends") || children.contains_key("$ref") {
            issues.push(Issue {
                path: path.into(),
                message: "Group inheritance must be resolved before this group can be imported"
                    .into(),
            });
            return;
        }
        let kind = node["$type"].as_str().or(parent_kind);
        for (name, child) in children {
            if name.starts_with('$') && name != "$root" {
                continue;
            }
            let path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if (!valid_token_name(name) && name != "$root") || (name == "$root" && !is_token(child))
            {
                issues.push(Issue {
                    path,
                    message: "Invalid token or group name".into(),
                });
            } else if is_token(child) {
                let kind = child["$type"].as_str().or(kind);
                self.entries.insert(path, Token { node: child, kind });
                if let Some(kind) = kind {
                    self.kinds.insert(child, kind);
                    if let Some(value) = child.get("$value") {
                        self.kinds.insert(value, kind);
                    }
                }
            } else {
                self.group(child, &path, kind, issues);
            }
        }
    }

    pub fn resolve(&self, token: Token<'a>) -> Result<Resolved<'a>> {
        self.follow(token.node, token.kind, &mut Vec::new())
    }

    pub fn property(&self, property: &'a Value) -> Result<&'a Value> {
        Ok(self.follow(property, None, &mut Vec::new())?.value)
    }

    fn follow(
        &self,
        node: &'a Value,
        expected: Option<&'a str>,
        seen: &mut Vec<*const Value>,
    ) -> Result<Resolved<'a>> {
        let identity = node as *const Value;
        if seen.contains(&identity) {
            return Err("Circular design token reference".into());
        }
        if seen.len() >= 128 {
            return Err("Design token reference chain exceeds 128 values".into());
        }
        seen.push(identity);
        let kind = self
            .kinds
            .get(&identity)
            .copied()
            .or_else(|| node["$type"].as_str());
        if matches!((expected, kind), (Some(a), Some(b)) if a != b) {
            return Err(format!(
                "Reference type {} does not match {}",
                kind.unwrap_or_default(),
                expected.unwrap_or_default()
            ));
        }
        let kind = expected.or(kind);
        if let Some(reference) = node.get("$ref") {
            if node.get("$value").is_some() {
                return Err("A token cannot combine $ref and $value".into());
            }
            let reference = reference
                .as_str()
                .ok_or("$ref must be a JSON Pointer string")?;
            let pointer = pointer(reference)?;
            let target = self
                .root
                .pointer(&pointer)
                .ok_or_else(|| format!("Missing JSON Pointer target {reference}"))?;
            let mut value = self.follow(target, kind, seen)?;
            value.reference = Some(reference);
            Ok(value)
        } else if let Some(value) = node.get("$value") {
            self.follow(value, kind, seen)
        } else if let Some(raw) = node
            .as_str()
            .filter(|raw| raw.starts_with('{') || raw.ends_with('}'))
        {
            let path = raw
                .strip_prefix('{')
                .and_then(|path| path.strip_suffix('}'))
                .filter(|path| !path.is_empty() && !path.contains(['{', '}']))
                .ok_or("Malformed token alias")?;
            let target = self
                .entries
                .get(path)
                .ok_or_else(|| format!("Missing token alias {raw}"))?;
            let mut value = self.follow(target.node, kind, seen)?;
            value.reference = Some(raw);
            Ok(value)
        } else {
            Ok(Resolved {
                value: node,
                kind,
                reference: None,
            })
        }
    }
}

fn is_token(node: &Value) -> bool {
    node.get("$value").is_some()
        || node.as_object().is_some_and(|children| {
            children.contains_key("$ref")
                && children
                    .keys()
                    .all(|key| key.starts_with('$') && key != "$root")
        })
}

fn pointer(reference: &str) -> Result<String> {
    let fragment = reference
        .strip_prefix('#')
        .ok_or("Only same-document JSON Pointers are supported")?;
    let mut bytes = fragment.bytes();
    let mut decoded = Vec::with_capacity(fragment.len());
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = bytes
                .next()
                .and_then(|byte| (byte as char).to_digit(16))
                .ok_or("Malformed JSON Pointer percent escape")?;
            let low = bytes
                .next()
                .and_then(|byte| (byte as char).to_digit(16))
                .ok_or("Malformed JSON Pointer percent escape")?;
            decoded.push((high * 16 + low) as u8);
        } else {
            decoded.push(byte);
        }
    }
    let decoded = String::from_utf8(decoded).map_err(|_| "JSON Pointer must decode to UTF-8")?;
    if !decoded.is_empty() && !decoded.starts_with('/') {
        return Err("Malformed JSON Pointer path".into());
    }
    for (index, byte) in decoded.bytes().enumerate() {
        if byte == b'~' && !matches!(decoded.as_bytes().get(index + 1), Some(b'0' | b'1')) {
            return Err("Malformed JSON Pointer path escape".into());
        }
    }
    Ok(decoded)
}
