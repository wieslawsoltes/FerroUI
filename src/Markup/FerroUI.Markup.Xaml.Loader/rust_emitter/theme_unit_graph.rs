//! The units of a compiled theme and their graph (#568, step 2 of
//! docs/porting/theme-trimming.md), read from the documents of the group.
//!
//! A class whose document merges other documents (`MergeResourceInclude`, inlined by
//! `XamlMergeResourceGroupTransformer`) is partitioned into units. Each merged document is a
//! unit: a control document with its control themes, text-keyed and type-keyed, and its local
//! resources; a resource document with its resources and those of its theme dictionaries. The
//! ambient unit (index 0, named [`AMBIENT`]) is what is always kept: the documents that are not
//! merged (the document of the class and the documents it includes otherwise, with their own
//! entries and global styles), the documents of the hosts the framework creates itself
//! ([`FRAMEWORK_HOSTS`]), and the documents with an entry no root can name (an entry without a
//! key, with a key that is neither text nor `{x:Type}`, or one that registers a name in the
//! scope of the document). The units are numbered in the order of the documents of the group.
//!
//! The graph is the key graph of the resource roots of #496 (`resource_roots.rs`), rewritten
//! for units and extended with types. A unit defines the keys and type keys of the outermost
//! keyed entries of the dictionaries of its documents (the entries of nested dictionaries of an
//! entry belong to the entry). It references every key of the group that its text names: every
//! identifier token of its attribute values and text that is a key of the group (conservative:
//! a `TargetType`, a selector or a text that happens to spell a key keeps it), and the argument
//! of every `{StaticResource}`, `{DynamicResource}` and `ResourceKey`, which may be any key: since
//! #848 also a key no document of the group defines, which another theme may provide. It
//! constructs the types of its elements, of its `TargetType`s and of its `{x:Type}`s, and the owner
//! of an attached property that makes the framework construct it (`ToolTip.Tip`). The types of
//! the type selectors of its selectors are not among them: a selector is a condition, and a style
//! that selects a type styles it where something else made it. Counted as constructed, the one
//! global style of a theme that selects a menu item kept the themes of the menu item, the scroll
//! viewer, the scroll bar and what their templates construct in every application
//! (docs/binary-size.md, section 4.19). The `Source` of an include and the content of
//! `Design.PreviewWith` are not read. A group with a document that is not plain XML (one with a
//! DTD, for example) has no units.

use std::collections::{BTreeSet, HashMap};

use ferroui_base::metadata::FERRO_XML_NAMESPACE;
use ferroui_build_scan::xaml_metadata::ThemeUnitModel;
use roxmltree::{Document, Node};
use xamlx::XamlNamespaces;

/// The name of the ambient unit.
pub(crate) const AMBIENT: &str = "ambient";

/// The local names of the types whose instances the framework creates itself (the hosts of
/// windows, popups, adorners and embedding, and the scope of a theme variant): a document that
/// defines the theme of one of them belongs to the ambient unit, since no application has to
/// name them to need them.
pub(crate) const FRAMEWORK_HOSTS: &[&str] =
    &["Window", "PopupRoot", "OverlayPopupHost", "AdornerLayer", "EmbeddableControlRoot", "ThemeVariantScope"];

/// The attached properties whose value makes the framework construct an instance of their owner:
/// `ToolTip.Tip` on any element shows a `ToolTip`, which no element of the document is. A document
/// that sets one constructs the owner, as an attribute or as a property element.
const ATTACHED_CONSTRUCTIONS: &[(&str, &str)] = &[("ToolTip", "Tip")];

/// The owner of the attached property `name` (`Owner.Property`) if setting it constructs the owner.
fn attached_construction(name: &str) -> Option<&'static str> {
    let (owner, property) = name.split_once('.')?;
    ATTACHED_CONSTRUCTIONS.iter().find(|(o, p)| *o == owner && *p == property).map(|(owner, _)| *owner)
}

/// The units of the compiled markup of a class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnitPlan {
    /// The units, in canonical order: the ambient unit first.
    pub(crate) units: Vec<ThemeUnitModel>,
    /// The unit of each merged document, by its URI.
    pub(crate) unit_of_uri: HashMap<String, u32>,
}

fn identifier(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The identifier tokens of `text`.
fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|token| !token.is_empty())
}

/// The local part of a type name (`controls:Button` is `Button`).
fn local_type_name(name: &str) -> &str {
    let name = name.trim();
    name.rsplit_once(':').map_or(name, |(_, local)| local).trim()
}

/// The type of `{x:Type T}` (`T`, as written), if `text` is one, with any prefix for the XAML
/// namespace.
fn type_extension(text: &str) -> Option<&str> {
    let expression = text.trim().strip_prefix('{')?.strip_suffix('}')?.trim();
    let (extension, argument) = expression.split_once(char::is_whitespace)?;
    let extension = local_type_name(extension);
    if extension != "Type" && extension != "TypeExtension" {
        return None;
    }
    let argument = argument.trim();
    Some(argument.strip_prefix("TypeName=").unwrap_or(argument).trim())
}

/// What the key of an entry of a dictionary says.
enum Key<'a> {
    Text(&'a str),
    Type(&'a str),
    /// An entry without a key, or with a key no root can name.
    Opaque,
}

fn key_of<'a>(node: Node<'a, '_>) -> Key<'a> {
    let Some(key) = node.attribute((XamlNamespaces::XAML2006, "Key")) else {
        return Key::Opaque;
    };
    if let Some(text) = key.strip_prefix("{}") {
        return Key::Text(text);
    }
    if !key.starts_with('{') {
        return Key::Text(key);
    }
    match type_extension(key).map(local_type_name) {
        Some(local) if identifier(local) => Key::Type(local),
        _ => Key::Opaque,
    }
}

/// Whether `node` is an element of the framework namespace named `name`.
fn is_framework(node: Node<'_, '_>, name: &str) -> bool {
    node.has_tag_name((FERRO_XML_NAMESPACE, name))
}

/// Whether `node` is an entry of a dictionary: a direct child of a framework
/// `ResourceDictionary` element that is not a property element.
fn is_entry(node: Node<'_, '_>) -> bool {
    node.is_element()
        && !node.tag_name().name().contains('.')
        && node.parent_element().is_some_and(|parent| is_framework(parent, "ResourceDictionary"))
}

/// Whether the entry registers a name in the scope of its document, so that it is observed
/// without its key being looked up. The framework templates create a scope of their own for
/// their content.
fn registers_outer_name(entry: Node<'_, '_>) -> bool {
    let mut template_end = 0;
    for node in entry.descendants().filter(|n| n.is_element()) {
        if node.range().start < template_end {
            continue;
        }
        let tag = node.tag_name();
        if node.attribute((XamlNamespaces::XAML2006, "Name")).is_some() || node.attribute("Name").is_some() || tag.name().ends_with(".Name") {
            return true;
        }
        if tag.namespace() == Some(FERRO_XML_NAMESPACE)
            && matches!(
                tag.name(),
                "ControlTemplate"
                    | "DataTemplate"
                    | "TreeDataTemplate"
                    | "ItemsPanelTemplate"
                    | "FocusAdornerTemplate"
                    | "WindowDrawnDecorationsTemplate"
            )
        {
            template_end = node.range().end;
        }
    }
    false
}

/// The type names of a selector (`Button`, `local|Item`, the types of `:is(..)` and of the
/// selectors `:not(..)` takes): the identifiers that start with an upper-case letter and are
/// not a class (`.name`), a name (`#name`), a pseudo-class (`:name`), a namespace prefix
/// (`prefix|`) or inside the brackets of a property selector.
fn selector_types(selector: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let bytes = selector.as_bytes();
    let mut index = 0;
    let mut brackets = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        match byte {
            b'[' => brackets += 1,
            b']' => brackets -= 1,
            _ => {}
        }
        if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') {
            index += 1;
        }
        let before = if start == 0 { b' ' } else { bytes[start - 1] };
        let after = bytes.get(index).copied().unwrap_or(b' ');
        let word = &selector[start..index];
        if brackets == 0 && !matches!(before, b'.' | b'#' | b':' | b'-') && after != b'|' && word.as_bytes()[0].is_ascii_uppercase() {
            found.push(word);
        }
    }
    found
}

/// The arguments of the `{StaticResource}` and `{DynamicResource}` extensions of an attribute
/// value, and their `ResourceKey=` arguments: the keys it names whatever their spelling.
fn extension_keys(value: &str, found: &mut BTreeSet<String>) {
    let mut rest = value;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        let end = rest.find(['{', '}']).unwrap_or(rest.len());
        let expression = rest[..end].trim();
        let Some((extension, argument)) = expression.split_once(char::is_whitespace) else {
            continue;
        };
        if !matches!(local_type_name(extension), "StaticResource" | "DynamicResource" | "StaticResourceExtension" | "DynamicResourceExtension") {
            continue;
        }
        let argument = argument.trim();
        let argument = argument.strip_prefix("ResourceKey=").unwrap_or(argument).trim();
        let argument = argument.split(',').next().unwrap_or(argument).trim();
        if !argument.is_empty() {
            found.insert(argument.to_string());
        }
    }
}

/// What one document of the group defines and names.
#[derive(Default)]
struct DocumentFacts {
    keys: BTreeSet<String>,
    type_keys: BTreeSet<String>,
    tokens: BTreeSet<String>,
    extension_keys: BTreeSet<String>,
    types: BTreeSet<String>,
    /// The types the type selectors of the selectors name: conditions, not constructions.
    selected_types: BTreeSet<String>,
    /// An entry no root can name (see the module documentation).
    opaque: bool,
    /// Every text key the document defines, at any depth: the outermost entries and the entries of
    /// nested dictionaries, which a reference of the document resolves locally (#848).
    local_keys: BTreeSet<String>,
}

/// Whether the attribute is not read for references: the `Source` of an include, and the
/// directives that name the entry or the document rather than refer to anything.
fn skipped_attribute(element: Node<'_, '_>, attribute: &roxmltree::Attribute<'_, '_>) -> bool {
    if attribute.namespace() == Some(XamlNamespaces::XAML2006) {
        return matches!(attribute.name(), "Key" | "Name" | "Class" | "ClassModifier" | "FieldModifier" | "Uid");
    }
    let include = element.tag_name().namespace() == Some(FERRO_XML_NAMESPACE)
        && matches!(element.tag_name().name(), "ResourceInclude" | "MergeResourceInclude" | "StyleInclude");
    include && attribute.namespace().is_none() && attribute.name() == "Source" && !attribute.value().starts_with('{')
}

fn facts_of(xml: &Document<'_>) -> DocumentFacts {
    let mut facts = DocumentFacts::default();
    let mut entry_end = 0;
    let mut design_end = 0;
    for node in xml.descendants() {
        let start = node.range().start;
        if start < design_end {
            continue;
        }
        if node.is_text() {
            facts.tokens.extend(tokens(node.text().unwrap_or_default()).map(str::to_string));
            continue;
        }
        if !node.is_element() {
            continue;
        }
        if is_framework(node, "Design.PreviewWith") {
            design_end = node.range().end;
            continue;
        }
        if let Key::Text(key) = key_of(node) {
            facts.local_keys.insert(key.to_string());
        }
        if start >= entry_end && is_entry(node) {
            entry_end = node.range().end;
            match key_of(node) {
                Key::Text(key) => {
                    facts.keys.insert(key.to_string());
                }
                Key::Type(type_key) => {
                    facts.type_keys.insert(type_key.to_string());
                }
                Key::Opaque => facts.opaque = true,
            }
            if registers_outer_name(node) {
                facts.opaque = true;
            }
        }
        let tag = node.tag_name();
        if !tag.name().contains('.') && tag.namespace() != Some(XamlNamespaces::XAML2006) {
            facts.types.insert(tag.name().to_string());
        }
        if let Some(owner) = attached_construction(tag.name()) {
            facts.types.insert(owner.to_string());
        }
        for attribute in node.attributes() {
            let value = attribute.value();
            if attribute.namespace().is_none() {
                if let Some(owner) = attached_construction(attribute.name()) {
                    facts.types.insert(owner.to_string());
                }
            }
            if attribute.namespace().is_none() && attribute.name() == "TargetType" {
                let target = type_extension(value).unwrap_or(value);
                let target = local_type_name(target);
                if identifier(target) {
                    facts.types.insert(target.to_string());
                }
            }
            if attribute.namespace().is_none() && attribute.name() == "Selector" {
                facts.selected_types.extend(selector_types(value).into_iter().map(str::to_string));
            }
            if skipped_attribute(node, &attribute) {
                continue;
            }
            // `{x:Type T}` anywhere in the value.
            let mut rest = value;
            while let Some(open) = rest.find('{') {
                let candidate = &rest[open..];
                let close = candidate.find('}').map_or(candidate.len(), |close| close + 1);
                if let Some(type_name) = type_extension(&candidate[..close]) {
                    let local = local_type_name(type_name);
                    if identifier(local) {
                        facts.types.insert(local.to_string());
                    }
                }
                rest = &candidate[1..];
            }
            if attribute.namespace().is_none() && attribute.name() == "ResourceKey" && !value.starts_with('{') {
                facts.extension_keys.insert(value.trim().to_string());
            }
            extension_keys(value, &mut facts.extension_keys);
            facts.tokens.extend(tokens(value).map(str::to_string));
        }
    }
    facts
}

/// What a document names that can root the units of a theme (#568, step 3): the build of an
/// application reads its documents with the rules the unit graph reads the documents of a theme
/// with ([`document_references`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentReferences {
    /// The local names of the types the document constructs or selects: its elements (template
    /// content included), its `TargetType`s, the type selectors of its selectors and its
    /// `{x:Type}`s.
    pub types: BTreeSet<String>,
    /// What may name a key of a theme: every identifier token of its attribute values and text,
    /// and the argument of every `{StaticResource}`, `{DynamicResource}` and `ResourceKey`.
    pub keys: BTreeSet<String>,
}

/// What the document `xaml` names that can root the units of a theme (#568, step 3), read as the
/// unit graph reads a document of a theme: conservative, so that a token that happens to spell a
/// key keeps the unit that defines it. `None` for a document that is not plain XML.
pub fn document_references(xaml: &str) -> Option<DocumentReferences> {
    let xml = Document::parse(xaml).ok()?;
    let facts = facts_of(&xml);
    let mut keys = facts.tokens;
    keys.extend(facts.extension_keys);
    // The documents of an application root what their selectors name as well: an application
    // that styles a type is taken to use it, by whatever means it makes it.
    let mut types = facts.types;
    types.extend(facts.selected_types);
    Some(DocumentReferences { types, keys })
}

/// The units of the group `documents` (`(name, xaml, URI)`, the document of the class first),
/// given the URIs of the documents whose entries were merged into another document (`merged`).
/// `None` when there is nothing to partition: no document is merged, every merged document
/// belongs to the ambient unit, or a document of the group is not plain XML.
pub(crate) fn plan(documents: &[(String, String, Option<String>)], merged: &[String]) -> Option<UnitPlan> {
    if merged.is_empty() {
        return None;
    }
    let mut facts = Vec::with_capacity(documents.len());
    for (_, xaml, _) in documents {
        let xml = Document::parse(xaml).ok()?;
        facts.push(facts_of(&xml));
    }

    // The units: the ambient one, then each merged document that is not ambient, in order.
    let mut members: Vec<Vec<usize>> = vec![Vec::new()];
    let mut names = vec![AMBIENT.to_string()];
    let mut unit_of_uri = HashMap::new();
    for (index, (name, _, uri)) in documents.iter().enumerate() {
        let is_merged = uri.as_ref().is_some_and(|uri| merged.contains(uri));
        let document = &facts[index];
        let ambient = !is_merged
            || document.opaque
            || document.type_keys.iter().any(|type_key| FRAMEWORK_HOSTS.contains(&type_key.as_str()));
        let unit = match ambient {
            true => 0,
            false => {
                members.push(Vec::new());
                names.push(name.clone());
                members.len() - 1
            }
        };
        members[unit].push(index);
        if let (true, Some(uri)) = (is_merged, uri) {
            unit_of_uri.insert(uri.clone(), unit as u32);
        }
    }
    if members.len() == 1 {
        return None;
    }

    // The keys of the group: what a reference can name.
    let keys: BTreeSet<&str> = facts.iter().flat_map(|document| document.keys.iter().map(String::as_str)).collect();
    let units = members
        .iter()
        .zip(names)
        .enumerate()
        .map(|(index, (members, name))| {
            let mut unit = ThemeUnitModel { name, index: index as u32, ..ThemeUnitModel::default() };
            let (mut defined, mut type_keys, mut references, mut types) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
            for &member in members {
                let document = &facts[member];
                unit.documents.push(documents[member].0.clone());
                defined.extend(document.keys.iter().cloned());
                type_keys.extend(document.type_keys.iter().cloned());
                types.extend(document.types.iter().cloned());
                references.extend(document.tokens.iter().filter(|token| keys.contains(token.as_str())).cloned());
                // The argument of a resource extension names a key, whichever document defines it:
                // another theme the application uses, or the application (#848), unless the
                // document defines it itself in a nested dictionary and resolves it there. A token
                // is only a guess, so it counts when it spells a key of the group.
                references.extend(
                    document
                        .extension_keys
                        .iter()
                        .filter(|key| keys.contains(key.as_str()) || !document.local_keys.contains(*key))
                        .cloned(),
                );
            }
            unit.keys = defined.into_iter().collect();
            unit.type_keys = type_keys.into_iter().collect();
            unit.references = references.into_iter().collect();
            unit.types = types.into_iter().collect();
            unit
        })
        .collect();
    Some(UnitPlan { units, unit_of_uri })
}

#[cfg(test)]
#[path = "theme_unit_graph_tests.rs"]
mod tests;
