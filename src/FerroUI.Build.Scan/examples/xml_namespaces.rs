//! The guard of the default XML namespace (ground rule 7 of docs/porting/PORTING-GUIDE.md, #1279).
//!
//! Reads what every built-in crate declares with the source scanner and compares it with two lists:
//!
//! - `scripts/conventions/xml-namespaces.txt`, by hand: the namespaces that stay out and why, the
//!   work still open, the other XML namespaces and the names known to collide;
//! - `scripts/conventions/xml-namespace-prefixes.txt`, generated: the prefixes for mapped built-in
//!   namespaces that markup and docs still have.
//!
//! ```text
//! cargo run -p ferroui-build-scan --example xml_namespaces -- --check
//! cargo run -p ferroui-build-scan --example xml_namespaces -- --report
//! cargo run -p ferroui-build-scan --example xml_namespaces -- --write-prefixes
//! ```
//!
//! Something new fails the check. Something that went away is only said, so that a change that
//! shrinks the work does not have to edit the lists, and two such changes do not conflict.

use ferroui_build_scan::model::AssemblyModel;
use ferroui_build_scan::scanner::{scan_crate, ScanOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The XML namespace of the framework.
const DEFAULT_XML_NAMESPACE: &str = "https://github.com/ferroui";
/// The list written by hand.
const RULES_FILE: &str = "scripts/conventions/xml-namespaces.txt";
/// The generated list of prefixes.
const PREFIXES_FILE: &str = "scripts/conventions/xml-namespace-prefixes.txt";
/// The folders that hold the built-in crates.
const CRATE_FOLDERS: [&str; 2] = ["src/", "components/"];
/// What is not product markup: tests, copies of other projects and generated references.
const NOT_PRODUCT: [&str; 4] = ["tests/", "external/", "third_party/", "reference/"];

/// The value of `key = "value"` in one section of a manifest.
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

/// The tracked files of the repository.
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

/// The library crates that state markup (a `register_types.rs` beside their root file), as the
/// name of the crate and its root file.
fn markup_crates(repository: &Path, files: &[String]) -> Vec<(String, PathBuf)> {
    let mut crates = Vec::new();
    for file in files {
        let is_built_in = CRATE_FOLDERS.iter().any(|folder| file.starts_with(folder));
        if !is_built_in || !file.ends_with("/Cargo.toml") || is_outside_product(file) {
            continue;
        }
        let path = repository.join(file);
        let Ok(manifest) = fs::read_to_string(&path) else { continue };
        let Some(name) = manifest_value(&manifest, "[package]", "name") else { continue };
        let library = manifest_value(&manifest, "[lib]", "path").unwrap_or_else(|| "src/lib.rs".to_owned());
        let root = path.with_file_name(library);
        if root.is_file() && root.with_file_name("register_types.rs").is_file() {
            crates.push((name.replace('-', "_"), root));
        }
    }
    crates
}

fn is_outside_product(path: &str) -> bool {
    NOT_PRODUCT.iter().any(|part| path.starts_with(part) || path.contains(&format!("/{part}")))
}

/// Whether a namespace is one the rule leaves out by its name.
fn is_internal(namespace: &str) -> bool {
    namespace.split('.').any(|segment| segment == "Internal")
}

/// What the built-in crates declare.
#[derive(Default)]
struct Inventory {
    /// The namespaces mapped to the default XML namespace.
    mapped: BTreeSet<String>,
    /// The other XML namespaces, with the assemblies that declare them.
    other_uris: BTreeMap<String, BTreeSet<String>>,
    /// The public types: their namespace and the names in it.
    types: BTreeMap<String, BTreeSet<String>>,
}

impl Inventory {
    fn add(&mut self, model: &AssemblyModel) {
        for definition in &model.xmlns_definitions {
            if definition.xml_namespace == DEFAULT_XML_NAMESPACE {
                self.mapped.insert(definition.namespace.clone());
            } else {
                let assemblies = self.other_uris.entry(definition.xml_namespace.clone()).or_default();
                assemblies.insert(model.name.clone());
            }
        }
        for type_ in &model.types {
            let is_public = type_.public_path.is_some() || type_.stated_path.is_some();
            if is_public && !type_.namespace.is_empty() {
                self.types.entry(type_.namespace.clone()).or_default().insert(type_.name.clone());
            }
        }
    }

    /// The names that more than one namespace has, among the namespaces that are or will be in
    /// the default XML namespace.
    fn collisions(&self, rules: &Rules) -> BTreeMap<String, BTreeSet<String>> {
        let mut homes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (namespace, names) in &self.types {
            if is_internal(namespace) || rules.exceptions.contains(namespace) {
                continue;
            }
            for name in names {
                homes.entry(name.clone()).or_default().insert(namespace.clone());
            }
        }
        homes.retain(|_, namespaces| namespaces.len() > 1);
        homes
    }
}

/// The list written by hand.
#[derive(Default)]
struct Rules {
    exceptions: BTreeSet<String>,
    open: BTreeSet<String>,
    uris: BTreeSet<String>,
    collisions: BTreeSet<String>,
}

impl Rules {
    /// One entry a line: `exception|open|uri|collision <value>`, then `#` and why.
    fn parse(text: &str) -> Result<Self, String> {
        let mut rules = Rules::default();
        for (index, line) in text.lines().enumerate() {
            let entry = line.split('#').next().unwrap_or("").trim();
            if entry.is_empty() {
                continue;
            }
            let Some((kind, value)) = entry.split_once(char::is_whitespace) else {
                return Err(format!("{RULES_FILE}:{}: an entry is a kind and a value", index + 1));
            };
            let list = match kind {
                "exception" => &mut rules.exceptions,
                "open" => &mut rules.open,
                "uri" => &mut rules.uris,
                "collision" => &mut rules.collisions,
                _ => return Err(format!("{RULES_FILE}:{}: unknown kind `{kind}`", index + 1)),
            };
            list.insert(value.trim().to_owned());
            if kind == "exception" && !line.contains('#') {
                return Err(format!("{RULES_FILE}:{}: an exception says why after `#`", index + 1));
            }
        }
        Ok(rules)
    }
}

/// The namespace or URI of each `xmlns:prefix="..."` of `text` that a prefix is not needed for.
fn redundant_prefixes<'a>(text: &'a str, mapped: &BTreeSet<String>, uris: &BTreeSet<String>) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("xmlns:") {
        rest = &rest[at + "xmlns:".len()..];
        let Some(equals) = rest.find('=') else { break };
        let is_prefix = rest[..equals].chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');
        let value = &rest[equals + 1..];
        let Some(quote) = value.chars().next().filter(|c| *c == '"' || *c == '\'') else { continue };
        let Some(end) = value[1..].find(quote) else { continue };
        if !is_prefix {
            continue;
        }
        let target = &value[1..end + 1];
        let namespace = target
            .strip_prefix("using:")
            .or_else(|| target.strip_prefix("clr-namespace:"))
            .map(|namespace| namespace.split(';').next().unwrap_or(namespace));
        match namespace {
            Some(namespace) if mapped.contains(namespace) => found.push(namespace),
            None if uris.contains(target) => found.push(target),
            _ => {}
        }
    }
    found
}

/// The prefixes of the product markup and docs, by file and target.
fn prefix_counts(
    repository: &Path,
    files: &[String],
    inventory: &Inventory,
    rules: &Rules,
) -> BTreeMap<(String, String), usize> {
    let mut counts = BTreeMap::new();
    for file in files {
        let is_markup = [".axaml", ".xaml", ".md"].iter().any(|extension| file.ends_with(extension));
        if !is_markup || is_outside_product(file) {
            continue;
        }
        let Ok(text) = fs::read_to_string(repository.join(file)) else { continue };
        for target in redundant_prefixes(&text, &inventory.mapped, &rules.uris) {
            *counts.entry((file.clone(), target.to_owned())).or_insert(0) += 1;
        }
    }
    counts
}

fn parse_prefixes(text: &str) -> BTreeMap<(String, String), usize> {
    let mut counts = BTreeMap::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let mut fields = line.split('\t');
        if let (Some(file), Some(target), Some(count)) = (fields.next(), fields.next(), fields.next()) {
            counts.insert((file.to_owned(), target.to_owned()), count.parse().unwrap_or(0));
        }
    }
    counts
}

fn format_prefixes(counts: &BTreeMap<(String, String), usize>) -> String {
    let mut text = String::from(
        "# Prefixes for mapped built-in namespaces still in markup and docs (#1282, #1283, #1284).\n\
         # Generated: cargo run -p ferroui-build-scan --example xml_namespaces -- --write-prefixes\n",
    );
    for ((file, target), count) in counts {
        text.push_str(&format!("{file}\t{target}\t{count}\n"));
    }
    text
}

/// What the check finds: what fails it, and what could leave the lists.
#[derive(Default, Debug, PartialEq)]
struct Findings {
    errors: Vec<String>,
    notes: Vec<String>,
}

fn compare(
    inventory: &Inventory,
    rules: &Rules,
    prefixes: &BTreeMap<(String, String), usize>,
    allowed_prefixes: &BTreeMap<(String, String), usize>,
) -> Findings {
    let mut findings = Findings::default();
    for namespace in inventory.types.keys() {
        let is_out = !inventory.mapped.contains(namespace) && !is_internal(namespace);
        if is_out && !rules.exceptions.contains(namespace) && !rules.open.contains(namespace) {
            findings.errors.push(format!(
                "the public namespace {namespace} is not in the default XML namespace: add an \
                 XmlnsDefinition for it to the MarkupAssembly of its crate"
            ));
        }
    }
    for (uri, assemblies) in &inventory.other_uris {
        if !rules.uris.contains(uri) {
            let names: Vec<&str> = assemblies.iter().map(String::as_str).collect();
            findings.errors.push(format!("{} adds its own XML namespace {uri}", names.join(", ")));
        }
    }
    let collisions = inventory.collisions(rules);
    for (name, namespaces) in &collisions {
        if !rules.collisions.contains(name) {
            let homes: Vec<&str> = namespaces.iter().map(String::as_str).collect();
            findings.errors.push(format!("the name {name} is in more than one namespace: {}", homes.join(", ")));
        }
    }
    for ((file, target), count) in prefixes {
        let allowed = allowed_prefixes.get(&(file.clone(), target.clone())).copied().unwrap_or(0);
        if *count > allowed {
            findings.errors.push(format!(
                "{file}: a prefix for {target}, which needs none ({count} here, {allowed} allowed)"
            ));
        }
    }
    for namespace in rules.open.iter().filter(|namespace| inventory.mapped.contains(*namespace)) {
        findings.notes.push(format!("{namespace} is mapped now: its `open` line can go"));
    }
    for name in rules.collisions.iter().filter(|name| !collisions.contains_key(*name)) {
        findings.notes.push(format!("{name} no longer collides: its `collision` line can go"));
    }
    for uri in rules.uris.iter().filter(|uri| !inventory.other_uris.contains_key(*uri)) {
        findings.notes.push(format!("no crate declares {uri} now: its `uri` line can go"));
    }
    findings
}

fn report(inventory: &Inventory, rules: &Rules) {
    println!("| Name | Namespaces | Already live |\n|---|---|---|");
    for (name, namespaces) in inventory.collisions(rules) {
        let live = namespaces.iter().filter(|namespace| inventory.mapped.contains(*namespace)).count() > 1;
        let homes: Vec<&str> = namespaces.iter().map(String::as_str).collect();
        println!("| `{name}` | {} | {} |", homes.join(", "), if live { "yes" } else { "no" });
    }
}

fn run(mode: &str, repository: &Path) -> Result<bool, String> {
    let files = tracked_files(repository)?;
    let mut inventory = Inventory::default();
    for (name, root) in markup_crates(repository, &files) {
        let mut model = scan_crate(&ScanOptions::new(&name, root)).model;
        if model.name.is_empty() {
            model.name = name;
        }
        inventory.add(&model);
    }
    let text = fs::read_to_string(repository.join(RULES_FILE)).map_err(|error| format!("{RULES_FILE}: {error}"))?;
    let rules = Rules::parse(&text)?;
    let prefixes = prefix_counts(repository, &files, &inventory, &rules);
    match mode {
        "--report" => {
            report(&inventory, &rules);
            Ok(true)
        }
        "--write-prefixes" => {
            fs::write(repository.join(PREFIXES_FILE), format_prefixes(&prefixes))
                .map_err(|error| format!("{PREFIXES_FILE}: {error}"))?;
            Ok(true)
        }
        "--check" => {
            let allowed = fs::read_to_string(repository.join(PREFIXES_FILE))
                .map_err(|error| format!("{PREFIXES_FILE}: {error}"))?;
            let findings = compare(&inventory, &rules, &prefixes, &parse_prefixes(&allowed));
            findings.notes.iter().for_each(|note| println!("note: {note}"));
            findings.errors.iter().for_each(|error| eprintln!("error: {error}"));
            println!(
                "{} public namespaces, {} mapped, {} prefixes left, {} errors",
                inventory.types.len(),
                inventory.mapped.len(),
                prefixes.values().sum::<usize>(),
                findings.errors.len()
            );
            Ok(findings.errors.is_empty())
        }
        other => Err(format!("unknown mode `{other}`: --check, --report or --write-prefixes")),
    }
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let mode = arguments.next().unwrap_or_else(|| "--check".to_owned());
    let repository = PathBuf::from(arguments.next().unwrap_or_else(|| ".".to_owned()));
    match run(&mode, &repository) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn inventory(mapped: &[&str], types: &[(&str, &[&str])]) -> Inventory {
        Inventory {
            mapped: set(mapped),
            other_uris: BTreeMap::new(),
            types: types.iter().map(|(namespace, names)| ((*namespace).to_owned(), set(names))).collect(),
        }
    }

    #[test]
    fn a_prefix_is_redundant_for_a_mapped_namespace_or_a_listed_uri() {
        let text = r#"<A xmlns="https://github.com/ferroui" xmlns:x="http://x" xmlns:a="using:My.Mapped"
            xmlns:b='clr-namespace:My.Mapped;assembly=Mine' xmlns:c="using:Their.Own" xmlns:d="urn:old"/>"#;
        let found = redundant_prefixes(text, &set(&["My.Mapped"]), &set(&["urn:old"]));
        assert_eq!(found, ["My.Mapped", "My.Mapped", "urn:old"]);
    }

    #[test]
    fn a_new_public_namespace_fails_and_an_internal_one_does_not() {
        let found = inventory(&["A"], &[("A", &["X"]), ("B", &["Y"]), ("B.Internal", &["Z"])]);
        let findings = compare(&found, &Rules::default(), &BTreeMap::new(), &BTreeMap::new());
        assert_eq!(findings.errors.len(), 1);
        assert!(findings.errors[0].contains("namespace B "));
    }

    #[test]
    fn a_new_collision_fails_and_a_listed_one_does_not() {
        let found = inventory(&["A", "B"], &[("A", &["Same"]), ("B", &["Same"])]);
        let none = BTreeMap::new();
        assert_eq!(compare(&found, &Rules::default(), &none, &none).errors.len(), 1);
        let rules = Rules { collisions: set(&["Same"]), ..Rules::default() };
        assert!(compare(&found, &rules, &none, &none).errors.is_empty());
    }

    #[test]
    fn an_excepted_namespace_does_not_collide() {
        let found = inventory(&["A"], &[("A", &["Same"]), ("System", &["Same"])]);
        let rules = Rules { exceptions: set(&["System"]), ..Rules::default() };
        assert!(found.collisions(&rules).is_empty());
    }

    #[test]
    fn more_prefixes_fail_and_fewer_do_not() {
        let found = inventory(&["A"], &[("A", &["X"])]);
        let key = ("a.axaml".to_owned(), "A".to_owned());
        let allowed = BTreeMap::from([(key.clone(), 2)]);
        let fewer = BTreeMap::from([(key.clone(), 1)]);
        let more = BTreeMap::from([(key, 3)]);
        assert!(compare(&found, &Rules::default(), &fewer, &allowed).errors.is_empty());
        assert_eq!(compare(&found, &Rules::default(), &more, &allowed).errors.len(), 1);
    }

    #[test]
    fn what_went_away_is_only_a_note() {
        let found = inventory(&["A"], &[("A", &["X"])]);
        let rules =
            Rules { open: set(&["A"]), collisions: set(&["Gone"]), uris: set(&["urn:gone"]), ..Rules::default() };
        let findings = compare(&found, &rules, &BTreeMap::new(), &BTreeMap::new());
        assert!(findings.errors.is_empty());
        assert_eq!(findings.notes.len(), 3);
    }

    #[test]
    fn an_exception_says_why() {
        assert!(Rules::parse("exception System\n").is_err());
        assert!(Rules::parse("exception System # the runtime library\nopen A\n").is_ok());
        assert!(Rules::parse("nonsense A\n").is_err());
    }

    #[test]
    fn the_prefix_list_reads_back() {
        let counts = BTreeMap::from([(("a.md".to_owned(), "A.B".to_owned()), 4)]);
        assert_eq!(parse_prefixes(&format_prefixes(&counts)), counts);
    }
}
