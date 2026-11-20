//! Stable naming rules shared by the XAML compiler and project tools.

/// Composition of a generated XAML module, which determines which names it reserves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "compiled-index",
    derive(serde::Deserialize, serde::Serialize)
)]
#[cfg_attr(feature = "compiled-index", serde(rename_all = "snake_case"))]
pub enum CompiledGroupKind {
    Documents,
    /// The first document populates a class. The remaining documents have build functions.
    Class,
}

impl CompiledGroupKind {
    /// Allocate names from the compiler's ordered logical document keys.
    /// A class exports a stable `populate` entry point. Its first allocated name only
    /// determines the names of internal functions used to populate the class.
    pub fn function_names<'a>(self, keys: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let requests: Vec<_> = keys
            .into_iter()
            .enumerate()
            .map(|(index, key)| {
                let mut name = function_name_of(key);
                if self == Self::Class && index == 0 {
                    name.replace_range(..6, "populate_");
                }
                (name, key)
            })
            .collect();
        let borrowed: Vec<_> = requests
            .iter()
            .map(|(name, key)| (name.as_str(), *key))
            .collect();
        let collision = match self {
            Self::Documents => functions_collide,
            Self::Class => collides,
        };
        allocate_names(&borrowed, collision)
    }
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
        .map(|index| {
            (0..requests.len())
                .any(|other| other != index && collides(requests[index].0, requests[other].0))
        })
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
            if !(0..names.len()).any(|other| other != index && collides(&candidate, &names[other]))
            {
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
    text.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

/// The name of the untyped build function that wraps the build function `function_name`.
pub fn untyped_function_name(function_name: &str) -> String {
    format!("{function_name}_untyped")
}

/// Whether the items `a` and `b` of two documents would have the same name:
/// the names are equal, or one is the build function of deferred content
/// (`<function>_deferred_<n>`) or a part of a split function
/// (`<function>_part_<n>`) of the other.
fn collides(a: &str, b: &str) -> bool {
    let is_deferred_of = |deferred: &str, function: &str| {
        deferred
            .strip_prefix(function)
            .and_then(|rest| {
                rest.strip_prefix("_deferred_")
                    .or_else(|| rest.strip_prefix("_part_"))
                    .or_else(|| rest.strip_prefix("_unit_"))
            })
            .is_some_and(|index| !index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
    };
    a == b || is_deferred_of(a, b) || is_deferred_of(b, a)
}

/// Whether the build functions `a` and `b` of two documents of a group would give two items one
/// name ([`collides`]), with the untyped build functions of the loader table
/// ([`untyped_function_name`]).
fn functions_collide(a: &str, b: &str) -> bool {
    let (untyped_a, untyped_b) = (untyped_function_name(a), untyped_function_name(b));
    collides(a, b)
        || collides(&untyped_a, b)
        || collides(a, &untyped_b)
        || collides(&untyped_a, &untyped_b)
}

/// Whether two names of documents below one root URI are one URI for the loader table of the
/// compiled markup: equal, or equal when each character is mapped to its upper case where that is
/// one character, as `rt::uri_equals` compares a requested URI with a document (upstream's
/// `OrdinalIgnoreCase`).
pub fn same_document_uri(a: &str, b: &str) -> bool {
    fn fold(character: char) -> char {
        let mut upper = character.to_uppercase();
        match (upper.next(), upper.next()) {
            (Some(single), None) => single,
            _ => character,
        }
    }
    a.chars().count() == b.chars().count()
        && a.chars()
            .zip(b.chars())
            .all(|(a, b)| a == b || fold(a) == fold(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_one_key_can_rename_multiple_build_functions() {
        let kind = CompiledGroupKind::Documents;
        let before = kind.function_names(["Views/A-B.xaml", "Views/A_B.xaml", "Views/Other.xaml"]);
        let after =
            kind.function_names(["Views/Customers.xaml", "Views/A_B.xaml", "Views/Other.xaml"]);
        assert_eq!(
            before[0],
            format!("build_views_a_b_xaml_{:08x}", fnv1a("Views/A-B.xaml"))
        );
        assert_eq!(
            before[1],
            format!("build_views_a_b_xaml_{:08x}", fnv1a("Views/A_B.xaml"))
        );
        assert_eq!(
            after,
            [
                "build_views_customers_xaml",
                "build_views_a_b_xaml",
                "build_views_other_xaml"
            ]
        );
        assert_eq!(before[2], after[2]);
    }

    #[test]
    fn groups_reserve_their_generated_helpers() {
        let keys = [
            "Views/Main.xaml",
            "Views/Main.xaml.untyped",
            "Views/Main.xaml.part.0",
        ];
        let documents = CompiledGroupKind::Documents.function_names(keys);
        assert!(
            documents
                .iter()
                .zip(keys)
                .all(|(name, key)| *name != function_name_of(key))
        );
        assert_eq!(
            CompiledGroupKind::Class.function_names(keys),
            [
                "populate_views_main_xaml",
                "build_views_main_xaml_untyped",
                "build_views_main_xaml_part_0",
            ]
        );
    }

    #[test]
    fn function_names_are_identifiers() {
        assert_eq!(function_name_of("Views/Main.xaml"), "build_views_main_xaml");
        assert_eq!(function_name_of("border-padding"), "build_border_padding");
        assert_eq!(
            function_name_of("Dot__Syntax/_Root.xaml"),
            "build_dot_syntax_root_xaml"
        );
        assert_eq!(
            function_name_of("\u{30b9}\u{30bf}\u{30a4}\u{30eb}.xaml"),
            "build_u30b9_u30bf_u30a4_u30eb_xaml"
        );
    }

    /// Not from upstream (#849): unique established names are kept; every name of a collision
    /// gets the hash of its text, in the case of the name, whatever the order of the requests.
    #[test]
    fn allocated_names_keep_the_unique_ones_and_disambiguate_the_others() {
        let equal: fn(&str, &str) -> bool = |a, b| a == b;
        let names = allocate_names(
            &[
                ("build_a_b_xaml", "A-B.xaml"),
                ("build_main_xaml", "Main.xaml"),
                ("build_a_b_xaml", "A_B.xaml"),
            ],
            equal,
        );
        assert_eq!(names[1], "build_main_xaml");
        assert_eq!(
            names[0],
            format!("build_a_b_xaml_{:08x}", fnv1a("A-B.xaml"))
        );
        assert_eq!(
            names[2],
            format!("build_a_b_xaml_{:08x}", fnv1a("A_B.xaml"))
        );
        assert_ne!(names[0], names[2]);
        let reversed = allocate_names(
            &[
                ("build_a_b_xaml", "A_B.xaml"),
                ("build_a_b_xaml", "A-B.xaml"),
            ],
            equal,
        );
        assert_eq!(reversed, [names[2].clone(), names[0].clone()]);

        let statics = allocate_names(
            &[
                ("THEME_PARTS_A_B_C", "A.B_C"),
                ("THEME_PARTS_A_B_C", "A_B.C"),
            ],
            equal,
        );
        assert_eq!(
            statics[0],
            format!("THEME_PARTS_A_B_C_{:08X}", fnv1a("A.B_C"))
        );
        assert_eq!(
            statics[1],
            format!("THEME_PARTS_A_B_C_{:08X}", fnv1a("A_B.C"))
        );
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
