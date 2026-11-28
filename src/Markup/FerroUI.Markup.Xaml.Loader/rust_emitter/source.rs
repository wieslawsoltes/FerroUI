//! Text helpers of the emitter: literals and identifiers.

/// `text` as a Rust string literal (with the quotes). Printable ASCII is
/// written as it is; the quote, the backslash and the usual control
/// characters use their short escapes; everything else is written as
/// `\u{..}`, so the literal is plain ASCII whatever the document holds.
pub fn rust_string_literal(text: &str) -> String {
    let mut literal = String::with_capacity(text.len() + 2);
    literal.push('"');
    for character in text.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\0' => literal.push_str("\\0"),
            ' '..='~' => literal.push(character),
            other => literal.push_str(&format!("\\u{{{:x}}}", u32::from(other))),
        }
    }
    literal.push('"');
    literal
}

/// The name of the build function of a document: `build_` followed by the
/// document name in lower case with every ASCII character that is not a
/// letter or digit replaced by `_` and every other character by `u` and its
/// code point in hexadecimal between underscores, a run of underscores
/// written once, so that the name is a snake case identifier
/// (`Views/Main.xaml` -> `build_views_main_xaml`, `スタイル.xaml` ->
/// `build_u30b9_u30bf_u30a4_u30eb_xaml`).
pub fn function_name_of(document_name: &str) -> String {
    let mut name = String::with_capacity(document_name.len() + 6);
    name.push_str("build_");
    let push_separator = |name: &mut String| {
        if !name.ends_with('_') {
            name.push('_');
        }
    };
    for character in document_name.chars() {
        if character.is_ascii_alphanumeric() {
            name.push(character.to_ascii_lowercase());
        } else if character.is_ascii() {
            push_separator(&mut name);
        } else {
            push_separator(&mut name);
            name.push_str(&format!("u{:x}", character as u32));
            push_separator(&mut name);
        }
    }
    name
}

/// The names of the items of one generated module (#849): the build and populate functions of
/// the documents of a group, the parts tables of the themes of an application. Each request is the
/// name the item is established by (what [`function_name_of`] or `parts_static_name` of
/// `ferroui-build` derive) and the text it is told apart by when that name is not unique (the
/// path of the document below the root URI of its assembly, the full name of a class).
///
/// An established name that collides with no other one (by `collides`, which knows the names an
/// item derives from its name: the functions of the deferred content and of the parts of a build
/// function) is kept, so the names a file had stay what they were. Every name of a collision is
/// disambiguated, whatever the order of the requests: the established name, `_` and the 32-bit
/// FNV-1a hash of the text, in eight hexadecimal digits in the case of the name
/// (`build_a_b_xaml_1f0e2d3c`, `THEME_PARTS_A_B_C_1F0E2D3C`). A hash that collides as well is
/// taken again over the text and a counter. The result has one name per request, in order.
///
/// A name that is public and cannot change (the URI of a document in the loader table, the
/// parts table a build script states for the code of its crate) is checked by the caller before:
/// its collision is an error of the build that names both places, not a name this function
/// changes.
pub fn allocate_names(requests: &[(&str, &str)], collides: fn(&str, &str) -> bool) -> Vec<String> {
    let mut names: Vec<String> = requests.iter().map(|(name, _)| name.to_string()).collect();
    let colliding: Vec<bool> = (0..requests.len())
        .map(|index| (0..requests.len()).any(|other| other != index && collides(requests[index].0, requests[other].0)))
        .collect();
    for index in 0..requests.len() {
        if !colliding[index] {
            continue;
        }
        let (established, text) = requests[index];
        let upper = !established.bytes().any(|byte| byte.is_ascii_lowercase());
        let mut attempt = 0u32;
        loop {
            let hashed = match attempt {
                0 => fnv1a(text),
                _ => fnv1a(&format!("{text}#{attempt}")),
            };
            let candidate = match upper {
                true => format!("{established}_{hashed:08X}"),
                false => format!("{established}_{hashed:08x}"),
            };
            if !(0..names.len()).any(|other| other != index && collides(&candidate, &names[other])) {
                names[index] = candidate;
                break;
            }
            attempt += 1;
        }
    }
    names
}

/// The 32-bit FNV-1a hash of the UTF-8 bytes of `text`.
fn fnv1a(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5_u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
}

/// Whether two names of documents below one root URI are one URI for the loader table of the
/// compiled markup: equal, or equal when each character is mapped to its upper case where that is
/// one character, as `rt::uri_equals` compares a requested URI with a document (upstream's
/// `OrdinalIgnoreCase`).
pub(crate) fn same_document_uri(a: &str, b: &str) -> bool {
    fn fold(character: char) -> char {
        let mut upper = character.to_uppercase();
        match (upper.next(), upper.next()) {
            (Some(single), None) => single,
            _ => character,
        }
    }
    a.chars().count() == b.chars().count() && a.chars().zip(b.chars()).all(|(a, b)| a == b || fold(a) == fold(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_quoted() {
        assert_eq!(rust_string_literal("Hello"), "\"Hello\"");
        assert_eq!(rust_string_literal(""), "\"\"");
        assert_eq!(rust_string_literal("a b~{}"), "\"a b~{}\"");
    }

    #[test]
    fn quotes_and_backslashes_are_escaped() {
        assert_eq!(rust_string_literal("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(rust_string_literal("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn control_characters_are_escaped() {
        assert_eq!(rust_string_literal("a\nb\r\tc\0"), "\"a\\nb\\r\\tc\\0\"");
        assert_eq!(rust_string_literal("\u{1}"), "\"\\u{1}\"");
        assert_eq!(rust_string_literal("\u{7f}"), "\"\\u{7f}\"");
    }

    #[test]
    fn other_characters_use_the_unicode_escape() {
        assert_eq!(rust_string_literal("\u{e9}"), "\"\\u{e9}\"");
        assert_eq!(rust_string_literal("\u{1f600}"), "\"\\u{1f600}\"");
    }

    #[test]
    fn function_names_are_identifiers() {
        assert_eq!(function_name_of("Views/Main.xaml"), "build_views_main_xaml");
        assert_eq!(function_name_of("border-padding"), "build_border_padding");
        assert_eq!(function_name_of("Dot__Syntax/_Root.xaml"), "build_dot_syntax_root_xaml");
        assert_eq!(function_name_of("\u{30b9}\u{30bf}\u{30a4}\u{30eb}.xaml"), "build_u30b9_u30bf_u30a4_u30eb_xaml");
    }

    /// Not from upstream (#849): unique established names are kept; every name of a collision
    /// gets the hash of its text, in the case of the name, whatever the order of the requests.
    #[test]
    fn allocated_names_keep_the_unique_ones_and_disambiguate_the_others() {
        let equal: fn(&str, &str) -> bool = |a, b| a == b;
        let names = allocate_names(
            &[("build_a_b_xaml", "A-B.xaml"), ("build_main_xaml", "Main.xaml"), ("build_a_b_xaml", "A_B.xaml")],
            equal,
        );
        assert_eq!(names[1], "build_main_xaml");
        assert_eq!(names[0], format!("build_a_b_xaml_{:08x}", fnv1a("A-B.xaml")));
        assert_eq!(names[2], format!("build_a_b_xaml_{:08x}", fnv1a("A_B.xaml")));
        assert_ne!(names[0], names[2]);
        let reversed = allocate_names(&[("build_a_b_xaml", "A_B.xaml"), ("build_a_b_xaml", "A-B.xaml")], equal);
        assert_eq!(reversed, [names[2].clone(), names[0].clone()]);

        let statics = allocate_names(&[("THEME_PARTS_A_B_C", "A.B_C"), ("THEME_PARTS_A_B_C", "A_B.C")], equal);
        assert_eq!(statics[0], format!("THEME_PARTS_A_B_C_{:08X}", fnv1a("A.B_C")));
        assert_eq!(statics[1], format!("THEME_PARTS_A_B_C_{:08X}", fnv1a("A_B.C")));
        assert_ne!(statics[0], statics[1]);

        // The same text twice: the counter tells them apart.
        let twice = allocate_names(&[("x", "same"), ("x", "same")], equal);
        assert_ne!(twice[0], twice[1]);
        // A known value of the hash.
        assert_eq!(fnv1a(""), 0x811c_9dc5);
        assert_eq!(fnv1a("a"), 0xe40c_292c);
    }

    /// Not from upstream (#849): two names are one URI for the loader table when they are equal
    /// without regard to case, as `rt::uri_equals` compares them.
    #[test]
    fn document_uris_are_compared_without_regard_to_case() {
        assert!(same_document_uri("Views/Main.xaml", "views/main.XAML"));
        assert!(same_document_uri("\u{c4}.xaml", "\u{e4}.xaml"));
        assert!(!same_document_uri("A-B.xaml", "A_B.xaml"));
        assert!(!same_document_uri("A.xaml", "AB.xaml"));
    }
}
