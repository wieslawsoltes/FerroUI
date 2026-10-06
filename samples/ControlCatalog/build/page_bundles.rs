//! Which page of the catalog uses which asset, and the asset bundles that
//! follow from it (part of the build script, see `build.rs`).
//!
//! Not a port: the split of the assets is a matter of the browser host of
//! the sample (`samples/ControlCatalog.Browser`), which downloads a bundle
//! when the catalog is about to create a page that needs it. The asset
//! loader stays synchronous, as upstream's: an asset a document names is
//! opened while the document is built, so the bundles of a page have to be
//! registered before the page is created.
//!
//! The sources of the crate (its documents and its Rust files, without
//! comments) are scanned for:
//!
//! * the assets they name: an asset path (`/Assets/x.png`,
//!   `ferres://ControlCatalog/Assets/x.png`, `Assets/x.png`); a file pattern
//!   (`/Assets/Fonts/SourceSansPro-*.ttf`); a directory as the source of a
//!   font collection (`/Assets/Fonts#Family`, which loads every font of the
//!   directory and its subdirectories into one collection); a directory
//!   joined with a file name at run time (`format!(".../Assets/Movies/{name}")`),
//!   which names the files of the directory whose names are string literals
//!   of the same source, or the whole directory when none is;
//! * the classes and other types they use (an identifier that a `struct`,
//!   `enum` or `trait` item, a class macro such as `xaml_class!(X, ..)` or
//!   the `x:Class` of a document declares), and the documents they name.
//!
//! A source uses what the sources it reaches use. The start-up of the
//! application reaches its documents (`App.xaml`, the main view and window
//! and the start page) and everything they reach, up to the pages of the
//! page list: a page of the list is created only by its entry, when the
//! catalog navigates to it. A page reaches its document and its class and
//! what they reach, up to what the start-up already reaches. The files that
//! only declare modules and tables of every type (`lib.rs`, `mod.rs`,
//! `register_types.rs`, `assets.rs`) are not followed.
//!
//! The assets the start-up reaches, and the assets no source names (the
//! scan cannot tell who uses them, so they are there from the start), make
//! the start-up bundle. An asset a page reaches goes to the bundle of the
//! set of pages that use it: one bundle per page for what one page uses, one
//! shared bundle per set of pages for what several use, so that no page
//! downloads an asset it does not use; an asset of [`LARGE_ASSET`] bytes or
//! more has a bundle of its own. An asset that only sources name that
//! neither the start-up nor a page reaches (documents whose class nothing
//! creates, such as the demos of a page whose class is not ported yet) goes
//! to [`UNREACHED_BUNDLE`], which no page waits for; the host fetches it in
//! the background after the others.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// The documents the application builds when it starts, before any
/// navigation: the application, the main view, the main window and the
/// start page.
const STARTUP_DOCUMENTS: &[&str] = &["App.xaml", "MainView.xaml", "MainWindow.xaml", "Pages/HomePage.xaml"];

/// The source of the page list of the catalog (`page_sections()`).
const PAGE_LIST: &str = "ViewModels/main_window_view_model_page_list.rs";

/// The name of the assembly of the sample in asset URIs.
const ASSET_URI_PREFIX: &str = "ferres://ControlCatalog/";

/// Directories of the crate without sources of the application.
const SKIPPED_DIRECTORIES: &[&str] = &["target", "tests", "examples", "build", "Assets", "PlaceholderAssets"];

/// An asset of this many bytes or more gets a bundle of its own.
pub const LARGE_ASSET: u64 = 1_000_000;

/// The name of the start-up bundle.
pub const STARTUP_BUNDLE: &str = "control-catalog.assets";

/// The name of the bundle of the assets that only sources name that
/// neither the start-up nor a page reaches.
pub const UNREACHED_BUNDLE: &str = "control-catalog.unreached.assets";

/// A document or a Rust file of the crate.
struct Unit {
    /// The path relative to the crate directory, with `/`.
    path: String,
    /// The content without comments.
    text: String,
}

/// A page of the page list of the catalog.
struct Page {
    /// The header of its entry (what the host is asked to load assets for).
    header: String,
    /// The name of the document (`ButtonsPage`) or of the class.
    name: String,
    roots: Vec<usize>,
}

/// An asset bundle loaded on demand.
pub struct Bundle {
    pub name: String,
    /// Rooted asset paths.
    pub assets: Vec<String>,
    /// The headers of the pages that need it.
    pub pages: Vec<String>,
}

/// The split of the assets.
pub struct Plan {
    /// The assets of the start-up bundle (rooted paths), the unattributed
    /// ones included.
    pub startup: Vec<String>,
    /// The assets no source names.
    pub unattributed: Vec<String>,
    /// The assets only sources name that neither the start-up nor a page
    /// reaches (the bundle [`UNREACHED_BUNDLE`]).
    pub unreached: Vec<String>,
    /// Those sources.
    pub unreached_sources: Vec<String>,
    /// Asset paths that a source names but that are no asset of the crate.
    pub unresolved: Vec<String>,
    /// The bundles loaded on demand.
    pub bundles: Vec<Bundle>,
    /// The bundles of each page of the page list, by header.
    pub page_bundles: BTreeMap<String, Vec<String>>,
}

/// The split of `assets` (rooted paths of the non-document assets, with
/// their sizes) over the sources of the crate at `root`.
pub fn plan(root: &Path, assets: &BTreeMap<String, u64>) -> Plan {
    let mut units = Vec::new();
    collect_units(root, root, &mut units);
    units.sort_by(|a, b| a.path.cmp(&b.path));
    let index: HashMap<&str, usize> = units.iter().enumerate().map(|(i, unit)| (unit.path.as_str(), i)).collect();

    // The types each source declares, and the modules that only one source is.
    let mut declared: HashMap<String, Vec<usize>> = HashMap::new();
    let mut items: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, unit) in units.iter().enumerate() {
        if is_registry(&unit.path) {
            continue;
        }
        let (types, unit_items) = declarations(unit);
        for (names, table) in [(types, &mut declared), (unit_items, &mut items)] {
            for name in names {
                let entry = table.entry(name).or_default();
                if !entry.contains(&i) {
                    entry.push(i);
                }
            }
        }
    }
    for (name, declaring) in items {
        if declaring.len() == 1 && !declared.contains_key(&name) {
            declared.insert(name, declaring);
        }
    }

    // What each source reaches directly, and the assets it names.
    let mut unresolved = BTreeSet::new();
    let mut edges = Vec::with_capacity(units.len());
    let mut used = Vec::with_capacity(units.len());
    for (i, unit) in units.iter().enumerate() {
        let mut targets = BTreeSet::new();
        let mut seen = HashSet::new();
        for identifier in identifiers(&unit.text) {
            if seen.insert(identifier) {
                for &target in declared.get(identifier).map(Vec::as_slice).unwrap_or_default() {
                    if target != i {
                        targets.insert(target);
                    }
                }
            }
        }
        for document in documents_named(unit) {
            if let Some(&target) = index.get(document.as_str()) {
                if target != i && !is_registry(&units[target].path) {
                    targets.insert(target);
                }
            }
        }
        edges.push(targets);
        used.push(assets_named(unit, assets, &mut unresolved));
    }
    for (unit, names) in units.iter().zip(&used) {
        assert!(
            !is_registry(&unit.path) || names.is_empty(),
            "{} names assets ({names:?}) but is not followed by the scan of the asset bundles",
            unit.path
        );
    }

    // The roots of the start-up and of the pages.
    let class_units = |unit: usize| -> Vec<usize> {
        let mut found = vec![unit];
        if let Some(class) = document_class(&units[unit].text) {
            found.extend(declared.get(&class).map(Vec::as_slice).unwrap_or_default().iter().copied().filter(|&u| u != unit));
        }
        found
    };
    let pages = page_list(&units, &index, &declared, &class_units);
    let page_roots: HashSet<usize> = pages.iter().flat_map(|page| page.roots.iter().copied()).collect();
    let mut startup_roots = Vec::new();
    for document in STARTUP_DOCUMENTS {
        let unit = *index.get(document).unwrap_or_else(|| panic!("the start-up document {document} does not exist"));
        startup_roots.extend(class_units(unit));
    }
    for root in &startup_roots {
        assert!(!page_roots.contains(root), "{} is both a start-up source and a page of the page list", units[*root].path);
    }

    let reach = |roots: &[usize], stop: &dyn Fn(usize) -> bool| -> BTreeSet<usize> {
        let mut reached: BTreeSet<usize> = roots.iter().copied().collect();
        let mut queue: VecDeque<usize> = roots.iter().copied().collect();
        while let Some(unit) = queue.pop_front() {
            for &target in &edges[unit] {
                if !stop(target) && reached.insert(target) {
                    queue.push_back(target);
                }
            }
        }
        reached
    };
    let startup_units = reach(&startup_roots, &|unit| page_roots.contains(&unit));
    let mut startup: BTreeSet<String> = startup_units.iter().flat_map(|&unit| used[unit].iter().cloned()).collect();

    // The assets of each page that the start-up does not have.
    let mut users: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut reached = startup_units.clone();
    for (p, page) in pages.iter().enumerate() {
        for unit in reach(&page.roots, &|unit| startup_units.contains(&unit)) {
            reached.insert(unit);
            for asset in &used[unit] {
                if !startup.contains(asset) {
                    users.entry(asset.clone()).or_default().insert(p);
                }
            }
        }
    }
    let named: BTreeSet<&String> = used.iter().flatten().collect();
    let unattributed: Vec<String> = assets.keys().filter(|asset| !named.contains(asset)).cloned().collect();
    startup.extend(unattributed.iter().cloned());
    let unreached: Vec<String> =
        assets.keys().filter(|asset| named.contains(asset) && !startup.contains(*asset) && !users.contains_key(*asset)).cloned().collect();
    let unreached_sources: Vec<String> = units
        .iter()
        .enumerate()
        .filter(|(unit, _)| !reached.contains(unit) && used[*unit].iter().any(|asset| unreached.contains(asset)))
        .map(|(_, unit)| unit.path.clone())
        .collect();

    // One bundle per set of pages, and one per large asset.
    let mut groups: BTreeMap<(Option<String>, BTreeSet<usize>), Vec<String>> = BTreeMap::new();
    for (asset, pages_of) in &users {
        let large = (assets[asset] >= LARGE_ASSET).then(|| asset.clone());
        groups.entry((large, pages_of.clone())).or_default().push(asset.clone());
    }
    let mut bundles: Vec<Bundle> = groups
        .into_iter()
        .map(|((large, pages_of), mut members)| {
            members.sort();
            let name = if let Some(asset) = large {
                format!("control-catalog.{}.assets", file_stem(&asset))
            } else if pages_of.len() == 1 {
                format!("control-catalog.{}.assets", pages[*pages_of.iter().next().expect("a page")].name)
            } else {
                let largest = members.iter().max_by_key(|asset| (assets[*asset], std::cmp::Reverse(*asset))).expect("an asset");
                format!("control-catalog.shared.{}.assets", slug(largest))
            };
            Bundle { name, assets: members, pages: pages_of.iter().map(|&p| pages[p].header.clone()).collect() }
        })
        .collect();
    if !unreached.is_empty() {
        bundles.push(Bundle { name: UNREACHED_BUNDLE.to_string(), assets: unreached.clone(), pages: Vec::new() });
    }
    bundles.sort_by(|a, b| a.name.cmp(&b.name));
    let mut names = HashSet::new();
    for bundle in &bundles {
        assert!(bundle.name != STARTUP_BUNDLE && names.insert(bundle.name.clone()), "two asset bundles are named {}", bundle.name);
    }

    let mut page_bundles = BTreeMap::new();
    for page in &pages {
        let mut of_page: Vec<String> =
            bundles.iter().filter(|bundle| bundle.pages.contains(&page.header)).map(|bundle| bundle.name.clone()).collect();
        of_page.sort();
        page_bundles.insert(page.header.clone(), of_page);
    }

    Plan {
        startup: startup.into_iter().collect(),
        unattributed,
        unreached,
        unreached_sources,
        unresolved: unresolved.into_iter().collect(),
        bundles,
        page_bundles,
    }
}

impl Plan {
    /// The manifest the browser host reads (`control-catalog.assets.json`):
    /// the start-up bundle, every bundle with its size, assets and pages,
    /// the bundles of each page by header, the order in which the host
    /// fetches the bundles in the background (smallest first, so that the
    /// most pages are ready soonest; [`UNREACHED_BUNDLE`] last), the
    /// unattributed assets and the sources that name assets but that
    /// nothing reaches.
    pub fn manifest(&self, sizes: &BTreeMap<String, u64>) -> String {
        let bytes = |assets: &[String]| assets.iter().map(|asset| sizes[asset]).sum::<u64>();
        let list = |items: &[String]| items.iter().map(|item| json_string(item)).collect::<Vec<_>>().join(", ");
        let mut text = String::from("{\n");
        writeln!(text, "  \"startup\": {},", json_string(STARTUP_BUNDLE)).expect("write");
        text.push_str("  \"bundles\": {\n");
        let mut entries = vec![format!(
            "    {}: {{ \"bytes\": {}, \"pages\": [], \"assets\": [{}] }}",
            json_string(STARTUP_BUNDLE),
            bytes(&self.startup),
            list(&self.startup)
        )];
        for bundle in &self.bundles {
            entries.push(format!(
                "    {}: {{ \"bytes\": {}, \"pages\": [{}], \"assets\": [{}] }}",
                json_string(&bundle.name),
                bytes(&bundle.assets),
                list(&bundle.pages),
                list(&bundle.assets)
            ));
        }
        text.push_str(&entries.join(",\n"));
        text.push_str("\n  },\n  \"pages\": {\n");
        let pages: Vec<String> =
            self.page_bundles.iter().map(|(header, names)| format!("    {}: [{}]", json_string(header), list(names))).collect();
        text.push_str(&pages.join(",\n"));
        text.push_str("\n  },\n");
        let mut prefetch: Vec<&Bundle> = self.bundles.iter().collect();
        prefetch.sort_by_key(|bundle| (bundle.name == UNREACHED_BUNDLE, bytes(&bundle.assets), bundle.name.clone()));
        let prefetch: Vec<String> = prefetch.into_iter().map(|bundle| bundle.name.clone()).collect();
        writeln!(text, "  \"prefetch\": [{}],", list(&prefetch)).expect("write");
        writeln!(text, "  \"unattributed\": [{}],", list(&self.unattributed)).expect("write");
        writeln!(text, "  \"unreached\": [{}]", list(&self.unreached_sources)).expect("write");
        text.push_str("}\n");
        text
    }

    /// The split as Rust tables, for the tests of the crate: the assets of
    /// the start-up bundle and, per page header, the assets of the bundles
    /// of the page.
    pub fn rust_tables(&self) -> String {
        let mut text = String::from("pub(crate) static STARTUP_ASSETS: &[&str] = &[\n");
        for asset in &self.startup {
            writeln!(text, "    {asset:?},").expect("write");
        }
        text.push_str("];\npub(crate) static PAGE_ASSETS: &[(&str, &[&str])] = &[\n");
        for (header, names) in &self.page_bundles {
            let assets: Vec<&String> =
                self.bundles.iter().filter(|bundle| names.contains(&bundle.name)).flat_map(|bundle| &bundle.assets).collect();
            writeln!(text, "    ({header:?}, &{assets:?}),").expect("write");
        }
        text.push_str("];\n");
        text
    }
}

fn collect_units(root: &Path, directory: &Path, found: &mut Vec<Unit>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", directory.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if !(directory == root && SKIPPED_DIRECTORIES.contains(&name.as_str())) {
                collect_units(root, &path, found);
            }
            continue;
        }
        let relative = path.strip_prefix(root).expect("a path under the crate directory");
        let relative: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        let relative = relative.join("/");
        let text = if name.ends_with(".xaml") {
            strip_xml_comments(&read(&path))
        } else if name.ends_with(".rs") && relative != "build.rs" {
            println!("cargo::rerun-if-changed={}", path.display());
            strip_rust_comments(&read(&path))
        } else {
            continue;
        };
        found.push(Unit { path: relative, text });
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The files that only declare modules and tables of every type.
fn is_registry(path: &str) -> bool {
    matches!(path, "lib.rs" | "register_types.rs" | "assets.rs") || path == "mod.rs" || path.ends_with("/mod.rs")
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The identifiers of a text, with their byte ranges.
fn identifier_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = None;
    let mut previous = '\0';
    for (at, c) in text.char_indices() {
        match start {
            Some(_) if is_identifier_char(c) => {}
            Some(begin) => {
                spans.push((begin, at));
                start = None;
            }
            None => {}
        }
        // `$name` in a macro is a metavariable, `1e5` is a number.
        if start.is_none() && is_identifier_start(c) && !is_identifier_char(previous) && previous != '$' {
            start = Some(at);
        }
        previous = c;
    }
    if let Some(begin) = start {
        spans.push((begin, text.len()));
    }
    spans
}

fn identifiers(text: &str) -> impl Iterator<Item = &str> {
    identifier_spans(text).into_iter().map(move |(start, end)| &text[start..end])
}

/// The types a source declares (the class of a document, the `struct`,
/// `enum` and `trait` items and the classes of the class macros of a Rust
/// file), and the module a Rust file is. Functions, statics and constants
/// are left out: their names are those of methods of the framework too
/// often (`bounds`, `text_block`), which would link unrelated sources; a
/// function of another module is reached through the name of its module
/// (`navigation_demo_helper::parse_color`, `use super::sample_info::..`).
fn declarations(unit: &Unit) -> (Vec<String>, Vec<String>) {
    if unit.path.ends_with(".xaml") {
        return (document_class(&unit.text).into_iter().collect(), Vec::new());
    }
    let text = &unit.text;
    let spans = identifier_spans(text);
    let mut types = Vec::new();
    let mut items = Vec::new();
    if let Some(module) = unit.path.rsplit('/').next().and_then(|name| name.strip_suffix(".rs")) {
        items.push(module.to_string());
    }
    for (i, &(start, end)) in spans.iter().enumerate() {
        let word = &text[start..end];
        let Some(&(next_start, next_end)) = spans.get(i + 1) else { continue };
        let between = &text[end..next_start];
        let next = text[next_start..next_end].to_string();
        if matches!(word, "struct" | "enum" | "trait") && between.trim().is_empty() {
            types.push(next);
        } else if (word.ends_with("_class") || word == "ferro_markup_enum")
            && between.trim_start().starts_with('!')
            && between.trim_start()[1..].trim() == "("
        {
            types.push(next);
        }
    }
    (types, items)
}

/// The last segment of the `x:Class` of a document.
fn document_class(text: &str) -> Option<String> {
    let start = text.find("x:Class=")? + "x:Class=".len();
    let quote = text[start..].chars().next()?;
    let rest = &text[start + 1..];
    let class = &rest[..rest.find(quote)?];
    Some(class.rsplit('.').next().unwrap_or(class).to_string())
}

fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '*')
}

/// The documents of the crate a source names (`/Pages/X.xaml`,
/// `ferres://ControlCatalog/X.xaml`, relative to the source), as paths
/// relative to the crate directory.
fn documents_named(unit: &Unit) -> Vec<String> {
    let text = &unit.text;
    let mut found = Vec::new();
    for (at, _) in text.match_indices(".xaml") {
        let end = at + ".xaml".len();
        if text[end..].chars().next().is_some_and(is_identifier_char) {
            continue;
        }
        let start = text[..at]
            .char_indices()
            .rev()
            .find(|&(_, c)| !(is_path_char(c) || c == ':'))
            .map_or(0, |(i, c)| i + c.len_utf8());
        let token = &text[start..end];
        let path = if let Some(rest) = token.strip_prefix(ASSET_URI_PREFIX) {
            rest.to_string()
        } else if token.contains("://") {
            continue;
        } else if let Some(rest) = token.strip_prefix('/') {
            rest.to_string()
        } else {
            let directory = unit.path.rsplit_once('/').map_or("", |(directory, _)| directory);
            normalize(&format!("{directory}/{token}"))
        };
        found.push(path);
    }
    found
}

fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

/// The assets a source names (see the module documentation). Asset paths
/// that name no asset are added to `unresolved`.
fn assets_named(unit: &Unit, assets: &BTreeMap<String, u64>, unresolved: &mut BTreeSet<String>) -> BTreeSet<String> {
    let text = &unit.text;
    let mut found = BTreeSet::new();
    // Assets outside `Assets/` (`Pages/teapot.bin`), by their path.
    for asset in assets.keys().filter(|asset| !asset.starts_with("/Assets/")) {
        if text.contains(&asset[1..]) {
            found.insert(asset.clone());
        }
    }
    for (at, _) in text.match_indices("Assets/") {
        if text[..at].chars().next_back().is_some_and(is_identifier_char) {
            continue;
        }
        let end = text[at..].find(|c: char| !is_path_char(c)).map_or(text.len(), |i| at + i);
        let reference = format!("/{}", &text[at..end]);
        let next = text[end..].chars().next();
        if assets.contains_key(&reference) {
            found.insert(reference);
            continue;
        }
        if reference.contains('*') {
            let matched: Vec<&String> = assets.keys().filter(|asset| matches_pattern(&reference, asset)).collect();
            if matched.is_empty() {
                unresolved.insert(format!("{reference} ({})", unit.path));
            }
            found.extend(matched.into_iter().cloned());
            continue;
        }
        let directory = format!("{}/", reference.trim_end_matches('/'));
        let members: Vec<&String> = assets.keys().filter(|asset| asset.starts_with(&directory)).collect();
        if members.is_empty() {
            unresolved.insert(format!("{reference} ({})", unit.path));
            continue;
        }
        if next == Some('#') {
            // A font collection: every font of the directory and its subdirectories.
            found.extend(members.into_iter().cloned());
            continue;
        }
        // A file name joined at run time: the files of the directory named by string literals.
        let named: Vec<&String> = members
            .iter()
            .copied()
            .filter(|asset| text.contains(&format!("\"{}\"", asset.rsplit('/').next().unwrap_or(asset))))
            .collect();
        found.extend(if named.is_empty() { members } else { named }.into_iter().cloned());
    }
    found
}

/// Whether `asset` is in the directory of `pattern` and its file name
/// matches the file name of `pattern`, where `*` stands for any text.
fn matches_pattern(pattern: &str, asset: &str) -> bool {
    let (Some((directory, name_pattern)), Some((asset_directory, name))) = (pattern.rsplit_once('/'), asset.rsplit_once('/'))
    else {
        return false;
    };
    if directory != asset_directory {
        return false;
    }
    let mut parts = name_pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else { return false };
    let parts: Vec<&str> = parts.collect();
    for (i, part) in parts.iter().enumerate() {
        if i + 1 == parts.len() {
            return rest.len() >= part.len() && rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    rest.is_empty()
}

/// The pages of the page list: `s.add("/Pages/X.xaml", "Header", ..)`,
/// `s.add_with_samples("/Pages/X.xaml", "Header", ..)` and
/// `s.add_page(|| X::new().upcast(), "Header", ..)`.
fn page_list(
    units: &[Unit],
    index: &HashMap<&str, usize>,
    declared: &HashMap<String, Vec<usize>>,
    class_units: &dyn Fn(usize) -> Vec<usize>,
) -> Vec<Page> {
    let list = &units[*index.get(PAGE_LIST).unwrap_or_else(|| panic!("the page list {PAGE_LIST} does not exist"))].text;
    let mut pages: Vec<Page> = Vec::new();
    for (at, call) in list.match_indices("s.add") {
        if list[..at].chars().next_back().is_some_and(is_identifier_char) {
            continue;
        }
        let rest = &list[at + call.len()..];
        let Some(open) = rest.find('(') else { continue };
        let kind = &rest[..open];
        let arguments = &rest[open + 1..];
        let page = match kind {
            "" | "_with_samples" => {
                let literals = string_literals(arguments, 2);
                let [document, header] = literals.as_slice() else { panic!("{PAGE_LIST}: an entry without a document and a header") };
                let path = document.trim_start_matches('/');
                let unit = *index.get(path).unwrap_or_else(|| panic!("{PAGE_LIST}: the document {document} does not exist"));
                let name = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".xaml").to_string();
                Page { header: header.clone(), name, roots: class_units(unit) }
            }
            "_page" => {
                let span = &arguments[..arguments.find('"').unwrap_or_else(|| panic!("{PAGE_LIST}: add_page without a header"))];
                let class = identifiers(span)
                    .filter(|name| declared.contains_key(*name))
                    .last()
                    .unwrap_or_else(|| panic!("{PAGE_LIST}: add_page names no class of the crate"));
                let header = string_literals(arguments, 1).remove(0);
                Page { header, name: class.to_string(), roots: declared[class].clone() }
            }
            _ => continue,
        };
        assert!(!pages.iter().any(|other| other.header == page.header), "{PAGE_LIST}: two pages have the header {}", page.header);
        pages.push(page);
    }
    assert!(!pages.is_empty(), "{PAGE_LIST}: no page found");
    pages
}

/// The first `count` string literals of a text (without escapes).
fn string_literals(text: &str, count: usize) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while found.len() < count {
        let Some(start) = rest.find('"') else { break };
        let Some(length) = rest[start + 1..].find('"') else { break };
        found.push(rest[start + 1..start + 1 + length].to_string());
        rest = &rest[start + 1 + length + 1..];
    }
    found
}

/// `/Assets/Fonts/WenQuanYiMicroHei-01.ttf` as `WenQuanYiMicroHei-01`.
fn file_stem(asset: &str) -> &str {
    let name = asset.rsplit('/').next().unwrap_or(asset);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// `/Assets/ModernApp/gallery_alpine.jpg` as `ModernApp-gallery_alpine`.
fn slug(asset: &str) -> String {
    let path = asset.trim_start_matches("/Assets/").trim_start_matches('/');
    let path = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
    path.replace('/', "-")
}

fn json_string(text: &str) -> String {
    let mut literal = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            c if (c as u32) < 0x20 => write!(literal, "\\u{:04x}", c as u32).expect("write"),
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

/// XML without its comments.
fn strip_xml_comments(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        stripped.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + "-->".len()..],
            None => rest = "",
        }
    }
    stripped.push_str(rest);
    stripped
}

/// Rust source without its comments (string and character literals are
/// kept as they are).
fn strip_rust_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut stripped = String::with_capacity(text.len());
    let mut i = 0;
    let mut copied = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                stripped.push_str(&text[copied..i]);
                i = text[i..].find('\n').map_or(bytes.len(), |end| i + end);
                copied = i;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                stripped.push_str(&text[copied..i]);
                let mut depth = 0;
                while i < bytes.len() {
                    if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                stripped.push(' ');
                copied = i;
            }
            b'r' if (bytes.get(i + 1) == Some(&b'"') || bytes.get(i + 1) == Some(&b'#'))
                && (i == 0 || !is_identifier_char(bytes[i - 1] as char)) =>
            {
                // A raw string: r"..", r#".."#.
                let hashes = bytes[i + 1..].iter().take_while(|&&b| b == b'#').count();
                if bytes.get(i + 1 + hashes) != Some(&b'"') {
                    i += 1;
                    continue;
                }
                let closing = format!("\"{}", "#".repeat(hashes));
                let body = i + 2 + hashes;
                i = text[body..].find(&closing).map_or(bytes.len(), |end| body + end + closing.len());
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'\'' => {
                // A character literal ('x', '\n', '\u{..}', '"'); otherwise a lifetime.
                if bytes.get(i + 1) == Some(&b'\\') {
                    i = text[i + 2..].find('\'').map_or(bytes.len(), |end| i + 2 + end + 1);
                } else if let Some(c) = text[i + 1..].chars().next() {
                    let after = i + 1 + c.len_utf8();
                    i = if bytes.get(after) == Some(&b'\'') { after + 1 } else { i + 1 };
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    stripped.push_str(&text[copied.min(text.len())..]);
    stripped
}
