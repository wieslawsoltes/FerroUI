//! Render long resolved type paths once per large Rust module. Only paths in the
//! emitted program participate. In particular a key or URI can itself look like a
//! Rust path. Keep imports in their own module so importing a trait cannot affect
//! method lookup in the generated builders.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::ops::Range;

fn word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// The emitter produces ordinary escaped string/char literals and line comments.
fn paths(source: &str, own_crate: Option<&str>, mut visit: impl FnMut(Range<usize>, bool)) {
    let s = source.as_bytes();
    let mut i = 0;
    while i < s.len() {
        match s[i] {
            b'/' if s.get(i + 1) == Some(&b'/') => {
                i += source[i..].find('\n').unwrap_or(s.len() - i);
            }
            b'"' | b'\'' => {
                let quote = s[i];
                i += 1;
                if quote == b'\''
                    && s.get(i).is_some_and(|&c| word(c))
                    && s.get(i + 1) != Some(&quote)
                {
                    // Named lifetime, e.g. &'static str.
                    continue;
                }
                while i < s.len() && s[i] != quote {
                    i += if s[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b':' if s.get(i + 1) == Some(&b':')
                && (i == 0 || !(word(s[i - 1]) || s[i - 1] == b':')) =>
            {
                let start = i;
                i += 2;
                let root = i;
                while s.get(i).copied().is_some_and(word) {
                    i += 1;
                }
                let name = &source[root..i];
                if s.get(i..i + 2) != Some(b"::") {
                    continue;
                }
                if own_crate == Some(name) {
                    visit(start..i, true);
                    continue;
                }
                if !(name.starts_with("ferroui_") || matches!(name, "core" | "std")) {
                    continue;
                }
                while s.get(i..i + 2) == Some(b"::") {
                    i += 2;
                    let segment = i;
                    while s.get(i).copied().is_some_and(word) {
                        i += 1;
                    }
                    if i == segment {
                        break;
                    }
                    if s[segment].is_ascii_uppercase() {
                        visit(start..i, false);
                        break;
                    }
                }
            }
            _ => i += 1,
        }
    }
}

pub(super) fn finish(source: String, own: Option<&str>) -> String {
    // Keep small documents readable without an import vocabulary. Root qualification
    // of class documents is still required at every size.
    let large = source.len() >= 512 * 1024;
    if !large && own.is_none() {
        return source;
    }
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut sites = Vec::new();
    paths(&source, own, |span, local| {
        if local || large {
            if !local {
                *counts.entry(&source[span.clone()]).or_default() += 1;
            }
            sites.push((span, local));
        }
    });
    let mut imports = String::new();
    let mut names = HashMap::new();
    let mut used = HashSet::new();
    // Stable ordering and unambiguous aliases, including same-named types from
    // separate namespaces. Rust's item order lets us append the private module.
    for (path, count) in counts {
        let (_, short) = path.rsplit_once("::").unwrap();
        let mut alias = short.to_string();
        for n in 2.. {
            if !used.contains(&alias) {
                break;
            }
            alias = format!("{short}_{n}");
        }
        let reference = format!("__t::{alias}");
        let declaration = if alias == short {
            format!("    pub(super) use {path};\n")
        } else {
            format!("    pub(super) use {path} as {alias};\n")
        };
        if count * path.len().saturating_sub(reference.len()) <= declaration.len() {
            continue;
        }
        used.insert(alias);
        imports.push_str(&declaration);
        names.insert(path, reference);
    }
    if names.is_empty() && own.is_none() {
        return source;
    }
    let mut result = String::with_capacity(source.len());
    let mut last = 0;
    for (span, local) in sites {
        let replacement = if local {
            "crate"
        } else if let Some(name) = names.get(&source[span.clone()]) {
            name
        } else {
            continue;
        };
        result.push_str(&source[last..span.start]);
        result.push_str(replacement);
        last = span.end;
    }
    result.push_str(&source[last..]);
    if !imports.is_empty() {
        writeln!(
            result,
            "\n#[allow(non_camel_case_types)]\nmod __t {{\n{imports}}}"
        )
        .unwrap();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualification_keeps_literals_comments_and_lifetimes() {
        let source = r#"// ::custom::C
let s: &'static str = "::custom::T \" ::custom::S";
let c = '"'; let t = ::custom::Widget::new();
"#;
        assert_eq!(
            finish(source.to_string(), Some("custom")),
            source.replace("= ::custom::Widget", "= crate::Widget")
        );
    }

    #[test]
    fn aliases_do_not_import_traits_into_the_builder_scope_or_merge_same_named_types() {
        let statement =
            "    let a = ::ferroui_a::layout::Layout::new(::ferroui_b::layout::Layout::new());\n";
        let source = statement.repeat(8_000);
        let compact = finish(source, None);
        assert!(compact.starts_with("    let a = __t::Layout::new(__t::Layout_2::new());"));
        assert!(compact.contains("mod __t {\n    pub(super) use ::ferroui_a::layout::Layout;"));
        assert!(compact.contains("    pub(super) use ::ferroui_b::layout::Layout as Layout_2;"));
    }
}
