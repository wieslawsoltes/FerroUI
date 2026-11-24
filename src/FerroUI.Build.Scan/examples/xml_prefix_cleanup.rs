//! Removes the `xmlns:` prefixes that the default XML namespace makes redundant, in the product
//! markup and docs of the repository (#1282 themes, #1283 samples and components, #1284 docs
//! and templates; ground rule 7 of docs/porting/PORTING-GUIDE.md).
//!
//! ```text
//! cargo run -p ferroui-build-scan --example xml_prefix_cleanup -- --dry-run [repository]
//! cargo run -p ferroui-build-scan --example xml_prefix_cleanup -- --write [repository]
//! ```
//!
//! It is meant to run on `main` once the namespaces are mapped, and to be run again whenever
//! more are: what it removes follows from what the crates map at that moment.
//!
//! A prefix is removed only when every use of it in the document names a type that resolves to
//! the same type without it: the type is in the namespace of the prefix, that namespace is in the
//! default XML namespace, and no other namespace there has a type of the same name. A prefix
//! for the default XML namespace itself is always redundant. Kept, whatever they name:
//!
//! - prefixes of `*.Internal` namespaces, of namespaces that are not mapped (an application's own
//!   types, a third-party library), and of any other XML namespace;
//! - a prefix the document declares more than once, or in a document whose default XML
//!   namespace is not the framework's;
//! - a prefix with a use this cannot read;
//! - tests, copies of other projects and generated references, which are not product markup,
//!   among them the tests of namespace resolution;
//! - what `scripts/conventions/xml-prefix-cleanup-keep.txt` lists, with the reason.
//!
//! In a Markdown file only fenced `xml` and `xaml` blocks are read, each as a document of its own.

use ferroui_build_scan::scanner::{scan_crate, ScanOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The XML namespace of the framework.
const DEFAULT_XML_NAMESPACE: &str = "https://github.com/ferroui";
/// The prefixes that stay, by hand.
const KEEP_FILE: &str = "scripts/conventions/xml-prefix-cleanup-keep.txt";
/// The folders that hold the built-in crates.
const CRATE_FOLDERS: [&str; 2] = ["src/", "components/"];
/// What is not product markup: tests, copies of other projects and generated references.
const NOT_PRODUCT: [&str; 4] = ["tests/", "external/", "third_party/", "reference/"];

/// What the crates put in the default XML namespace.
#[derive(Default)]
struct Inventory {
    /// The namespaces mapped to the default XML namespace.
    mapped: BTreeSet<String>,
    /// The types of each namespace, by markup name without the arity of a generic type.
    types: BTreeMap<String, BTreeSet<String>>,
}

impl Inventory {
    fn add_type(&mut self, namespace: &str, name: &str) {
        let name = name.split('`').next().unwrap_or(name);
        self.types.entry(namespace.to_owned()).or_default().insert(name.to_owned());
    }

    /// The mapped namespaces that have a type of this name.
    fn homes(&self, name: &str) -> Vec<&str> {
        self.mapped
            .iter()
            .filter(|namespace| self.types.get(*namespace).is_some_and(|names| names.contains(name)))
            .map(String::as_str)
            .collect()
    }

    /// Whether `name` resolves without a prefix to the type it names in `namespace`.
    fn resolves_alone(&self, namespace: &str, name: &str) -> bool {
        self.homes(name) == [namespace]
    }
}

/// What a prefix of a document stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    /// The default XML namespace itself.
    Default,
    /// A dotted namespace, by `using:` or `clr-namespace:`.
    Namespace(String),
    /// Anything else.
    Other(String),
}

impl Target {
    fn of(value: &str) -> Self {
        if value == DEFAULT_XML_NAMESPACE {
            return Target::Default;
        }
        let namespace = value.strip_prefix("using:").or_else(|| value.strip_prefix("clr-namespace:"));
        match namespace {
            Some(namespace) => {
                Target::Namespace(namespace.split(';').next().unwrap_or(namespace).trim().to_owned())
            }
            None => Target::Other(value.to_owned()),
        }
    }

    fn describe(&self) -> &str {
        match self {
            Target::Default => DEFAULT_XML_NAMESPACE,
            Target::Namespace(namespace) | Target::Other(namespace) => namespace,
        }
    }
}

/// An `xmlns` attribute of a document.
#[derive(Debug)]
struct Declaration {
    /// The prefix; empty for the default XML namespace.
    prefix: String,
    value: String,
    /// The attribute in the document, with the white space before it.
    range: Range<usize>,
}

/// The `xmlns` and `xmlns:prefix` attributes of a document.
fn declarations(text: &str) -> Vec<Declaration> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find("xmlns").map(|at| at + from) {
        from = at + 5;
        if at > 0 && is_name_byte(bytes[at - 1]) {
            continue;
        }
        let mut cursor = at + 5;
        let mut prefix = String::new();
        if bytes.get(cursor) == Some(&b':') {
            let start = cursor + 1;
            cursor = start;
            while bytes.get(cursor).is_some_and(|byte| is_name_byte(*byte)) {
                cursor += 1;
            }
            prefix = text[start..cursor].to_owned();
            if prefix.is_empty() {
                continue;
            }
        }
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            continue;
        }
        cursor += 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        let Some(&quote) = bytes.get(cursor).filter(|byte| **byte == b'"' || **byte == b'\'') else {
            continue;
        };
        let Some(length) = text[cursor + 1..].find(quote as char) else {
            continue;
        };
        let value = text[cursor + 1..cursor + 1 + length].to_owned();
        let end = cursor + 1 + length + 1;
        let mut start = at;
        while start > 0 && bytes[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        found.push(Declaration { prefix, value, range: start..end });
        from = end;
    }
    found
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.'
}

/// A use of a prefix: where `prefix:` or `prefix|` starts, and the name that follows.
#[derive(Debug)]
struct Use {
    at: usize,
    name: Option<String>,
}

/// Every use of `prefix` in a document, outside its declarations.
fn uses(text: &str, prefix: &str, declarations: &[Range<usize>]) -> Vec<Use> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find(prefix).map(|at| at + from) {
        from = at + prefix.len();
        let before = at.checked_sub(1).map(|index| bytes[index]);
        if before.is_some_and(|byte| is_name_byte(byte) || byte == b':') {
            continue;
        }
        let separator = bytes.get(at + prefix.len()).copied();
        if !matches!(separator, Some(b':') | Some(b'|')) {
            continue;
        }
        if declarations.iter().any(|range| range.contains(&at)) {
            continue;
        }
        let start = at + prefix.len() + 1;
        let mut end = start;
        while bytes.get(end).is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_') {
            end += 1;
        }
        let name = (end > start && !bytes[start].is_ascii_digit()).then(|| text[start..end].to_owned());
        found.push(Use { at, name });
    }
    found
}

/// What happens to one prefix of a document.
#[derive(Debug, PartialEq, Eq)]
enum Decision {
    Remove,
    Keep(String),
}

/// One prefix of a document, and what is done with it.
#[derive(Debug)]
struct Outcome {
    prefix: String,
    target: Target,
    uses: usize,
    decision: Decision,
}

/// The decisions for the prefixes of one document, and the document without the redundant ones.
fn clean_document(text: &str, inventory: &Inventory, kept: &BTreeSet<String>) -> (Vec<Outcome>, String) {
    let declarations = declarations(text);
    let defaults: BTreeSet<&str> = declarations
        .iter()
        .filter(|declaration| declaration.prefix.is_empty())
        .map(|declaration| declaration.value.as_str())
        .collect();
    let ranges: Vec<Range<usize>> = declarations.iter().map(|declaration| declaration.range.clone()).collect();
    let mut outcomes = Vec::new();
    let mut edits: Vec<(Range<usize>, &str)> = Vec::new();
    for declaration in declarations.iter().filter(|declaration| !declaration.prefix.is_empty()) {
        let prefix = &declaration.prefix;
        let target = Target::of(&declaration.value);
        let found = uses(text, prefix, &ranges);
        let declared = declarations.iter().filter(|other| other.prefix == *prefix).count();
        let keep = |reason: &str| Decision::Keep(reason.to_owned());
        let decision = if kept.contains(prefix) {
            keep("listed in the keep file")
        } else if defaults.len() != 1 || !defaults.contains(DEFAULT_XML_NAMESPACE) {
            keep("the default XML namespace of the document is not the framework's")
        } else if declared > 1 {
            keep("declared more than once")
        } else {
            match &target {
                Target::Other(_) => keep("another XML namespace"),
                Target::Namespace(namespace) if namespace.split('.').any(|part| part == "Internal") => {
                    keep("an internal namespace")
                }
                Target::Namespace(namespace) if !inventory.mapped.contains(namespace) => {
                    keep("not in the default XML namespace")
                }
                _ => match found.iter().find(|use_| use_.name.is_none()) {
                    Some(_) => keep("a use that is not a type name"),
                    None => {
                        let names = found.iter().filter_map(|use_| use_.name.as_deref());
                        let unresolved: BTreeSet<&str> = names
                            .filter(|name| match &target {
                                Target::Namespace(namespace) => !inventory.resolves_alone(namespace, name),
                                _ => false,
                            })
                            .collect();
                        if unresolved.is_empty() {
                            Decision::Remove
                        } else {
                            let names: Vec<&str> = unresolved.into_iter().collect();
                            Decision::Keep(format!("resolves differently without it: {}", names.join(", ")))
                        }
                    }
                },
            }
        };
        if decision == Decision::Remove {
            edits.push((declaration.range.clone(), ""));
            for use_ in &found {
                edits.push((use_.at..use_.at + prefix.len() + 1, ""));
            }
        }
        outcomes.push(Outcome { prefix: prefix.clone(), target, uses: found.len(), decision });
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut cleaned = text.to_owned();
    for (range, replacement) in edits {
        cleaned.replace_range(range, replacement);
    }
    (outcomes, cleaned)
}

/// The documents of a file: the whole file, or the fenced `xml` and `xaml` blocks of Markdown.
fn documents(file: &str, text: &str) -> Vec<Range<usize>> {
    if !file.ends_with(".md") {
        return vec![0..text.len()];
    }
    let mut found = Vec::new();
    let mut offset = 0;
    let mut open: Option<usize> = None;
    for line in text.split_inclusive('\n') {
        let fence = line.trim_start();
        match open {
            None if fence.starts_with("```") => {
                let language = fence.trim_start_matches('`').trim().to_ascii_lowercase();
                if language == "xml" || language == "xaml" {
                    open = Some(offset + line.len());
                } else {
                    open = Some(usize::MAX);
                }
            }
            Some(start) if fence.starts_with("```") => {
                if start != usize::MAX {
                    found.push(start..offset);
                }
                open = None;
            }
            _ => {}
        }
        offset += line.len();
    }
    found
}

/// The prefixes that stay by hand, by file: `path` for every prefix of a file, or `path prefix`,
/// each with `#` and why.
fn parse_keep(text: &str) -> Result<BTreeMap<String, Option<BTreeSet<String>>>, String> {
    let mut keep: BTreeMap<String, Option<BTreeSet<String>>> = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let entry = line.split('#').next().unwrap_or("").trim();
        if entry.is_empty() {
            continue;
        }
        if !line.contains('#') {
            return Err(format!("{KEEP_FILE}:{}: an entry says why after `#`", index + 1));
        }
        let mut fields = entry.split_whitespace();
        let path = fields.next().unwrap_or_default().to_owned();
        match fields.next() {
            Some(prefix) => {
                if let Some(prefixes) = keep.entry(path).or_insert_with(|| Some(BTreeSet::new())) {
                    prefixes.insert(prefix.to_owned());
                }
            }
            None => {
                keep.insert(path, None);
            }
        }
    }
    Ok(keep)
}

fn manifest_value(manifest: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == section;
        } else if inside {
            let Some((name, value)) = line.split_once('=') else { continue };
            if name.trim() == key {
                return Some(value.trim().trim_matches('"').to_owned());
            }
        }
    }
    None
}

fn tracked_files(repository: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|error| format!("git ls-files: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files failed".to_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.split('\0').filter(|path| !path.is_empty()).map(str::to_owned).collect())
}

fn is_outside_product(path: &str) -> bool {
    NOT_PRODUCT.iter().any(|part| path.starts_with(part) || path.contains(&format!("/{part}")))
}

/// What the built-in crates put in the default XML namespace, as their source states it.
fn inventory(repository: &Path, files: &[String]) -> Inventory {
    let mut inventory = Inventory::default();
    for file in files {
        let is_built_in = CRATE_FOLDERS.iter().any(|folder| file.starts_with(folder));
        if !is_built_in || !file.ends_with("/Cargo.toml") || is_outside_product(file) {
            continue;
        }
        let path = repository.join(file);
        let Ok(manifest) = fs::read_to_string(&path) else { continue };
        let Some(name) = manifest_value(&manifest, "[package]", "name") else { continue };
        let library = manifest_value(&manifest, "[lib]", "path").unwrap_or_else(|| "src/lib.rs".to_owned());
        let root: PathBuf = path.with_file_name(library);
        if !root.is_file() || !root.with_file_name("register_types.rs").is_file() {
            continue;
        }
        let model = scan_crate(&ScanOptions::new(&name.replace('-', "_"), root)).model;
        for definition in &model.xmlns_definitions {
            if definition.xml_namespace == DEFAULT_XML_NAMESPACE {
                inventory.mapped.insert(definition.namespace.clone());
            }
        }
        for type_ in &model.types {
            if !type_.namespace.is_empty() {
                inventory.add_type(&type_.namespace, &type_.name);
            }
        }
    }
    inventory
}

fn run(write: bool, repository: &Path) -> Result<(), String> {
    let files = tracked_files(repository)?;
    let inventory = inventory(repository, &files);
    let keep = match fs::read_to_string(repository.join(KEEP_FILE)) {
        Ok(text) => parse_keep(&text)?,
        Err(_) => BTreeMap::new(),
    };
    let (mut removed, mut kept, mut changed) = (0, 0, 0);
    println!("# {} mapped namespaces", inventory.mapped.len());
    println!("# file\tprefix\ttarget\tuses\tdecision");
    for file in &files {
        let is_markup = [".xaml", ".md"].iter().any(|extension| file.ends_with(extension));
        if !is_markup || is_outside_product(file) {
            continue;
        }
        if matches!(keep.get(file), Some(None)) {
            continue;
        }
        let Ok(text) = fs::read_to_string(repository.join(file)) else { continue };
        let listed = keep.get(file).cloned().flatten().unwrap_or_default();
        let mut cleaned = String::with_capacity(text.len());
        let mut last = 0;
        let mut file_changed = false;
        for range in documents(file, &text) {
            let (outcomes, document) = clean_document(&text[range.clone()], &inventory, &listed);
            for outcome in &outcomes {
                let decision = match &outcome.decision {
                    Decision::Remove => {
                        removed += 1;
                        "removed".to_owned()
                    }
                    Decision::Keep(reason) => {
                        kept += 1;
                        format!("kept: {reason}")
                    }
                };
                println!("{file}\t{}\t{}\t{}\t{decision}", outcome.prefix, outcome.target.describe(), outcome.uses);
            }
            cleaned.push_str(&text[last..range.start]);
            file_changed |= document != text[range.clone()];
            cleaned.push_str(&document);
            last = range.end;
        }
        cleaned.push_str(&text[last..]);
        if file_changed {
            changed += 1;
            if write {
                fs::write(repository.join(file), cleaned).map_err(|error| format!("{file}: {error}"))?;
            }
        }
    }
    let verb = if write { "changed" } else { "would change" };
    println!("# {removed} prefixes removed, {kept} kept; {verb} {changed} files");
    Ok(())
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let mode = arguments.next().unwrap_or_else(|| "--dry-run".to_owned());
    let repository = PathBuf::from(arguments.next().unwrap_or_else(|| ".".to_owned()));
    let write = match mode.as_str() {
        "--dry-run" => false,
        "--write" => true,
        other => {
            eprintln!("error: unknown mode `{other}`: --dry-run or --write");
            return ExitCode::FAILURE;
        }
    };
    match run(write, &repository) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory() -> Inventory {
        let mut inventory = Inventory::default();
        for namespace in ["FerroUI.Controls", "FerroUI.Controls.Converters", "FerroUI.Themes.Fluent"] {
            inventory.mapped.insert(namespace.to_owned());
        }
        inventory.add_type("FerroUI.Controls", "Button");
        inventory.add_type("FerroUI.Controls", "Grid");
        inventory.add_type("FerroUI.Controls.Converters", "EnumToBoolConverter");
        inventory.add_type("FerroUI.Controls.Converters", "Clash");
        inventory.add_type("FerroUI.Themes.Fluent", "Clash");
        inventory.add_type("FerroUI.Controls.Internal", "Helper");
        inventory.add_type("FerroUI.Collections", "FerroList`1");
        inventory
    }

    fn clean(text: &str) -> (Vec<Outcome>, String) {
        clean_document(text, &inventory(), &BTreeSet::new())
    }

    const HEAD: &str =
        r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml""#;

    #[test]
    fn a_prefix_of_a_mapped_namespace_goes_with_every_kind_of_use() {
        let text = format!(
            "{HEAD}\n      xmlns:c=\"using:FerroUI.Controls.Converters\">\n  <Grid.Resources>\n    \
             <c:EnumToBoolConverter x:Key=\"k\" />\n  </Grid.Resources>\n  \
             <Button Tag=\"{{x:Type c:EnumToBoolConverter}}\" Content=\"{{c:EnumToBoolConverter}}\" />\n  \
             <Style Selector=\"c|EnumToBoolConverter\" />\n  <c:EnumToBoolConverter></c:EnumToBoolConverter>\n</Grid>"
        );
        let (outcomes, cleaned) = clean(&text);
        assert_eq!(outcomes.len(), 2, "{outcomes:?}");
        let converters = outcomes.iter().find(|outcome| outcome.prefix == "c").unwrap();
        assert_eq!((converters.uses, &converters.decision), (6, &Decision::Remove));
        let gone = !cleaned.contains("c:") && !cleaned.contains("c|") && !cleaned.contains("xmlns:c");
        assert!(gone, "{cleaned}");
        assert!(cleaned.contains("<EnumToBoolConverter x:Key=\"k\" />"));
        assert!(cleaned.contains("{x:Type EnumToBoolConverter}"));
        assert!(cleaned.contains("Selector=\"EnumToBoolConverter\""));
        assert!(cleaned.contains("xmlns:x="), "the language prefix stays");
        assert!(cleaned.starts_with(&format!("{HEAD}>")), "{cleaned}");
    }

    #[test]
    fn a_name_two_mapped_namespaces_have_keeps_its_prefix() {
        let text = format!("{HEAD} xmlns:c=\"using:FerroUI.Controls.Converters\"><c:Clash /></Grid>");
        let (outcomes, cleaned) = clean(&text);
        let converters = outcomes.iter().find(|outcome| outcome.prefix == "c").unwrap();
        assert_eq!(converters.decision, Decision::Keep("resolves differently without it: Clash".into()));
        assert_eq!(cleaned, text);
    }

    #[test]
    fn a_name_the_namespace_does_not_have_keeps_its_prefix() {
        let text = format!("{HEAD} xmlns:c=\"using:FerroUI.Controls.Converters\"><c:Unknown /></Grid>");
        assert_eq!(clean(&text).1, text);
    }

    #[test]
    fn internal_unmapped_and_other_namespaces_keep_their_prefix() {
        let values =
            ["using:FerroUI.Controls.Internal", "using:MyApp.Views", "clr-namespace:Other;assembly=Other", "urn:x"];
        for value in values {
            let text = format!("{HEAD} xmlns:p=\"{value}\"><p:Helper /></Grid>");
            let (outcomes, cleaned) = clean(&text);
            let decision = &outcomes.iter().find(|outcome| outcome.prefix == "p").unwrap().decision;
            assert!(matches!(decision, Decision::Keep(_)), "{value}");
            assert_eq!(cleaned, text, "{value}");
        }
    }

    #[test]
    fn a_prefix_for_the_default_namespace_itself_goes() {
        let text = format!("{HEAD} xmlns:f=\"https://github.com/ferroui\"><f:Anything /></Grid>");
        assert_eq!(clean(&text).1, format!("{HEAD}><Anything /></Grid>"));
    }

    #[test]
    fn clr_namespace_with_an_assembly_and_generic_names_resolve() {
        let mut inventory = inventory();
        inventory.mapped.insert("FerroUI.Collections".into());
        let declaration = "xmlns:col=\"clr-namespace:FerroUI.Collections;assembly=FerroUI.Base\"";
        let text = format!("{HEAD} {declaration}><col:FerroList x:TypeArguments=\"x:String\" /></Grid>");
        let (_, cleaned) = clean_document(&text, &inventory, &BTreeSet::new());
        assert!(cleaned.contains("<FerroList x:TypeArguments"), "{cleaned}");
    }

    #[test]
    fn a_document_with_another_default_or_a_redeclared_prefix_is_left_alone() {
        let other = r#"<Grid xmlns="urn:other" xmlns:c="using:FerroUI.Controls"><c:Button /></Grid>"#;
        assert_eq!(clean(other).1, other);
        let inner = "<Grid xmlns:c=\"using:FerroUI.Controls.Converters\"><c:Button /></Grid>";
        let twice = format!("{HEAD} xmlns:c=\"using:FerroUI.Controls\">{inner}</Grid>");
        assert_eq!(clean(&twice).1, twice);
    }

    #[test]
    fn a_use_that_is_not_a_type_name_keeps_the_prefix() {
        let text = format!("{HEAD} xmlns:c=\"using:FerroUI.Controls\"><Button Tag=\"c:\" /></Grid>");
        assert_eq!(clean(&text).1, text);
    }

    #[test]
    fn a_prefix_listed_in_the_keep_file_stays() {
        let text = format!("{HEAD} xmlns:c=\"using:FerroUI.Controls\"><c:Button /></Grid>");
        let kept = BTreeSet::from(["c".to_owned()]);
        assert_eq!(clean_document(&text, &inventory(), &kept).1, text);
        let keep = parse_keep("docs/a.md c # explains prefixes\ndocs/b.md # the whole file\n").unwrap();
        assert_eq!(keep["docs/a.md"], Some(BTreeSet::from(["c".to_owned()])));
        assert_eq!(keep["docs/b.md"], None);
        assert!(parse_keep("docs/a.md c\n").is_err(), "a reason is required");
    }

    #[test]
    fn markdown_is_read_by_its_xml_blocks_only() {
        let block = "<Grid xmlns=\"https://github.com/ferroui\" xmlns:c=\"using:FerroUI.Controls\"><c:Button /></Grid>";
        let text = format!("Say `c:Button` in prose.\n\n```xml\n{block}\n```\n\n```rust\nlet c = 1;\n```\n");
        let text = text.as_str();
        let blocks = documents("docs/a.md", text);
        assert_eq!(blocks.len(), 1);
        assert!(text[blocks[0].clone()].starts_with("<Grid"));
        assert_eq!(documents("a.xaml", text), [0..text.len()]);
    }

    #[test]
    fn outside_the_product_is_left_alone() {
        assert!(is_outside_product("tests/FerroUI.Markup.Xaml.UnitTests/a.xaml"));
        assert!(is_outside_product("src/FerroUI.Controls/tests/a.xaml"));
        assert!(!is_outside_product("samples/ControlCatalog/Pages/a.xaml"));
    }
}
