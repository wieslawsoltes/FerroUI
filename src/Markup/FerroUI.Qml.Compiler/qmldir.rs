//! Identified module manifests. Parsing performs no filesystem access or plugin loading.
use crate::{Diagnostic, SourceSpan};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RequestedVersion {
    #[default]
    Latest,
    Major(u16),
    Exact(ModuleVersion),
}

impl FromStr for RequestedVersion {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        fn part(s: &str) -> Option<u16> {
            if s.is_empty() || !s.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            s.parse().ok()
        }
        let invalid = || format!("invalid module version '{text}'");
        if let Some((major, minor)) = text.split_once('.') {
            Ok(Self::Exact(ModuleVersion {
                major: part(major).ok_or_else(invalid)?,
                minor: part(minor).ok_or_else(invalid)?,
            }))
        } else {
            Ok(Self::Major(part(text).ok_or_else(invalid)?))
        }
    }
}

impl fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportKind {
    Object,
    Singleton,
    Script,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Export {
    pub file: String,
    pub kind: ExportKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportVersion {
    Request(RequestedVersion),
    Inherit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleImport {
    pub uri: String,
    pub version: ImportVersion,
    pub optional: bool,
    pub default: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plugin {
    pub name: String,
    pub path: Option<String>,
    pub optional: bool,
}

/// Identity and available version of a plugin implemented by a statically linked host.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StaticPlugin {
    pub module: String,
    pub name: String,
    pub version: ModuleVersion,
}

impl StaticPlugin {
    pub fn new(module: &str, name: &str, major: u16, minor: u16) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
            version: ModuleVersion { major, minor },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModuleDefinition {
    pub uri: String,
    /// Actual versions exported by this manifest.
    versions: BTreeSet<ModuleVersion>,
    native_versions: BTreeMap<u16, u16>,
    exports: BTreeMap<String, BTreeMap<Option<ModuleVersion>, Export>>,
    pub internal: BTreeMap<String, String>,
    pub imports: Vec<ModuleImport>,
    pub dependencies: Vec<(String, RequestedVersion)>,
    pub plugins: Vec<Plugin>,
    pub class_name: Option<String>,
    pub type_info: Vec<String>,
    pub prefer: Option<String>,
    pub designer_supported: bool,
    pub is_static: bool,
    pub is_system: bool,
}

impl ModuleDefinition {
    /// Resolve manifest plugin names against host registrations. An absent optional
    /// plugin contributes no exports. This never loads a file named by the manifest.
    pub fn bind_static_plugins(&mut self, plugins: &BTreeSet<StaticPlugin>) -> Result<(), String> {
        let mut versions = BTreeMap::<u16, u16>::new();
        for entry in &self.plugins {
            let mut matches = plugins
                .iter()
                .filter(|p| p.module == self.uri && p.name == entry.name)
                .peekable();
            if matches.peek().is_none() && !entry.optional {
                return Err(format!("QML module '{}' requires unsupported native Qt plugin '{}'; register a matching static implementation", self.uri, entry.name));
            }
            for plugin in matches {
                let maximum = versions.entry(plugin.version.major).or_default();
                *maximum = (*maximum).max(plugin.version.minor);
            }
        }
        self.native_versions = versions;
        Ok(())
    }

    pub fn has_native_exports(&self) -> bool {
        !self.native_versions.is_empty()
    }

    /// Select an import once, before resolving any individual type.
    pub fn select(&self, request: RequestedVersion) -> Result<ModuleView<'_>, String> {
        let version = match request {
            RequestedVersion::Latest => self.versions.last().copied().max(
                self.native_versions
                    .last_key_value()
                    .map(|(&major, &minor)| ModuleVersion { major, minor }),
            ),
            RequestedVersion::Major(major) => {
                let first = ModuleVersion { major, minor: 0 };
                let last = ModuleVersion {
                    major,
                    minor: u16::MAX,
                };
                Some(
                    self.versions
                        .range(first..=last)
                        .next_back()
                        .copied()
                        .max(
                            self.native_versions
                                .get(&major)
                                .map(|&minor| ModuleVersion { major, minor }),
                        )
                        .ok_or_else(|| {
                            format!(
                                "module '{}' does not provide major version {major}",
                                self.uri
                            )
                        })?,
                )
            }
            RequestedVersion::Exact(version) => {
                if !self.versions.contains(&version)
                    && !self
                        .native_versions
                        .get(&version.major)
                        .is_some_and(|max| version.minor <= *max)
                {
                    return Err(format!(
                        "module '{}' does not export version {version}",
                        self.uri
                    ));
                }
                Some(version)
            }
        };
        Ok(ModuleView {
            module: self,
            version,
        })
    }
}

/// A version-selected set of public module types. Internal entries are separate.
pub struct ModuleView<'m> {
    module: &'m ModuleDefinition,
    pub version: Option<ModuleVersion>,
}

impl<'m> ModuleView<'m> {
    pub fn get(&self, name: &str) -> Option<&'m Export> {
        let editions = self.module.exports.get(name)?;
        if let Some(version) = self.version {
            let first = ModuleVersion {
                major: version.major,
                minor: 0,
            };
            if let Some((_, export)) = editions.range(Some(first)..=Some(version)).next_back() {
                return Some(export);
            }
        }
        editions.get(&None)
    }
}

/// Read a module definition using the documented, line-oriented `qmldir` grammar.
/// Unsupported directives produce source-located diagnostics.
pub fn parse_qmldir(source: &str) -> Result<ModuleDefinition, Diagnostic> {
    parse_manifest(source, true)
}

/// Read a local directory listing or an identified module manifest.
/// Listings expose only their declared unversioned public names.
pub fn parse_directory_manifest(source: &str) -> Result<ModuleDefinition, Diagnostic> {
    parse_manifest(source, false)
}

fn parse_manifest(source: &str, require_uri: bool) -> Result<ModuleDefinition, Diagnostic> {
    let mut module = ModuleDefinition::default();
    let mut first = true;
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start_matches('\u{feff}').trim_start();
        let start = offset + line.len() - trimmed.len();
        let stripped = trimmed.trim_end();
        offset += line.len();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        let span = SourceSpan {
            start,
            end: start + stripped.len(),
        };
        let fields: Vec<_> = stripped
            .split_whitespace()
            .take_while(|field| !field.starts_with('#'))
            .collect();
        let fail = |message: String| Diagnostic::new(source, span, message);
        if first {
            first = false;
            if let ["module", uri] = fields.as_slice() {
                if !uri.split('.').all(ident) {
                    return Err(fail(format!("invalid module URI '{uri}'")));
                }
                module.uri = (*uri).to_owned();
                continue;
            }
            if require_uri {
                return Err(fail(
                    "expected 'module <URI>' as the first directive".into(),
                ));
            }
        }
        if module.uri.is_empty()
            && !matches!(fields.as_slice(), [name,_] | ["internal",name,_] if export_name(name))
        {
            return Err(fail(
                "expected an unversioned public or internal directory entry".into(),
            ));
        }
        apply(&mut module, &fields).map_err(fail)?;
    }
    if require_uri && module.uri.is_empty() {
        return Err(Diagnostic::new(
            source,
            SourceSpan { start: 0, end: 0 },
            "missing module identifier",
        ));
    }
    Ok(module)
}

fn ident(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(crate::lexer::identifier_start)
        && name
            .chars()
            .all(|ch| crate::lexer::identifier_start(ch) || ch.is_numeric())
}

fn export_name(name: &str) -> bool {
    ident(name) && name.chars().next().is_some_and(char::is_uppercase)
}

fn file_kind(file: &str) -> Result<ExportKind, String> {
    if file.starts_with(['/', char::from(92)]) || file.contains(':') {
        return Err(format!("expected a relative module file, got '{file}'"));
    }
    if file.ends_with(".qml") {
        Ok(ExportKind::Object)
    } else if file.ends_with(".js") || file.ends_with(".mjs") {
        Ok(ExportKind::Script)
    } else {
        Err(format!("unsupported module source file '{file}'"))
    }
}

fn apply(module: &mut ModuleDefinition, fields: &[&str]) -> Result<(), String> {
    match fields {
        ["module", ..] => return Err("duplicate module identifier".into()),
        ["designersupported"] => module.designer_supported = true,
        ["static"] => module.is_static = true,
        ["system"] => module.is_system = true,
        ["classname", name] if module.class_name.is_none() => {
            module.class_name = Some((*name).into())
        }
        ["prefer", path] if module.prefer.is_none() => module.prefer = Some((*path).into()),
        ["typeinfo", path] => module.type_info.push((*path).into()),
        ["depends", uri] if uri.split('.').all(ident) => module
            .dependencies
            .push(((*uri).into(), RequestedVersion::Latest)),
        ["depends", uri, version] if uri.split('.').all(ident) => {
            module.dependencies.push(((*uri).into(), version.parse()?))
        }
        ["internal", name, file] if export_name(name) && file_kind(file)? == ExportKind::Object => {
            if module.exports.contains_key(*name)
                || module
                    .internal
                    .insert((*name).into(), (*file).into())
                    .is_some()
            {
                return Err(format!("duplicate internal type '{name}'"));
            }
        }
        ["optional", "plugin", rest @ ..] | ["plugin", rest @ ..] => {
            let (name, path) = match rest {
                [name] => (*name, None),
                [name, path] => (*name, Some((*path).into())),
                _ => return Err("invalid plugin declaration".into()),
            };
            module.plugins.push(Plugin {
                name: name.into(),
                path,
                optional: fields[0] == "optional",
            });
        }
        ["import", rest @ ..] | ["optional" | "default", "import", rest @ ..] => {
            let (uri, version) = match rest {
                [uri] => (*uri, ImportVersion::Request(RequestedVersion::Latest)),
                [uri, "auto"] => (*uri, ImportVersion::Inherit),
                [uri, v] => (*uri, ImportVersion::Request(v.parse()?)),
                _ => return Err("invalid module import".into()),
            };
            if !uri.split('.').all(ident) {
                return Err(format!("invalid module URI '{uri}'"));
            }
            module.imports.push(ModuleImport {
                uri: uri.into(),
                version,
                optional: fields[0] == "optional",
                default: fields[0] == "default",
            });
        }
        _ => {
            let (singleton, decl) = if let ["singleton", rest @ ..] = fields {
                (true, rest)
            } else {
                (false, fields)
            };
            let (name, version, file) = match decl {
                [name, ver, file] if export_name(name) => {
                    let RequestedVersion::Exact(version) = ver.parse()? else {
                        return Err("an export requires major.minor version syntax".into());
                    };
                    (*name, Some(version), *file)
                }
                [name, file] if export_name(name) => (*name, None, *file),
                _ => {
                    return Err(format!(
                        "unsupported or malformed qmldir directive '{}'",
                        fields.join(" ")
                    ))
                }
            };
            let mut kind = file_kind(file)?;
            if singleton {
                if kind != ExportKind::Object {
                    return Err("singleton requires a QML file".into());
                }
                kind = ExportKind::Singleton;
            }
            if module.internal.contains_key(name) {
                return Err(format!("export conflicts with internal type '{name}'"));
            }
            let editions = module.exports.entry(name.to_owned()).or_default();
            if editions
                .insert(
                    version,
                    Export {
                        file: file.into(),
                        kind,
                    },
                )
                .is_some()
            {
                return Err(format!("duplicate export '{name}' at the same version"));
            }
            if let Some(v) = version {
                module.versions.insert(v);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_revision_stays_within_major_and_public_namespace() {
        let manifest = parse_qmldir("# shared widgets\r\nmodule Studio.Controls\r\nTile 1.0 v1.qml\nTile 1.10 v1new.qml\nTile 2.0 v2.qml\nMinor 1.12 later.qml\nLegacy 1.0 l.qml\ninternal Cache cache.qml\nShared Shared.qml").unwrap();
        assert_eq!(
            manifest
                .select("1".parse().unwrap())
                .unwrap()
                .get("Tile")
                .unwrap()
                .file,
            "v1new.qml"
        );
        let view = manifest.select("1.0".parse().unwrap()).unwrap();
        assert_eq!(view.get("Tile").unwrap().file, "v1.qml");
        assert!(view.get("Minor").is_none());
        assert!(manifest.select("1.9".parse().unwrap()).is_err());
        let latest = manifest.select(RequestedVersion::Latest).unwrap();
        assert_eq!(latest.get("Tile").unwrap().file, "v2.qml");
        assert!(latest.get("Cache").is_none());
        assert!(latest.get("Legacy").is_none());
        assert!(latest.get("Shared").is_some());
    }
    #[test]
    fn retains_dependencies_and_tool_metadata_without_executing_them() {
        let manifest=parse_qmldir("module Demo\nimport Core auto\noptional import Optional 2\ndepends Images 3.1\noptional plugin Loader native\nclassname LoaderPlugin\ntypeinfo demo.qmltypes\nprefer :/demo/\ndesignersupported\nsingleton Theme 1.0 Theme.qml\nScript 1.0 script.mjs").unwrap();
        assert_eq!(manifest.imports[0].version, ImportVersion::Inherit);
        assert!(manifest.imports[1].optional && manifest.plugins[0].optional);
        assert_eq!(manifest.plugins[0].path.as_deref(), Some("native"));
        let latest = manifest.select(RequestedVersion::Latest).unwrap();
        assert_eq!(latest.get("Theme").unwrap().kind, ExportKind::Singleton);
        assert_eq!(latest.get("Script").unwrap().kind, ExportKind::Script);
    }
    #[test]
    fn rejects_invalid_modules_and_ambiguous_exports() {
        for text in [
            "",
            "Tile 1.0 A.qml",
            "module A\nmodule B",
            "module A.9",
            "module A\nTile 1.0 B.qml\nTile 1.0 A.qml",
            "module A\nsingleton Tile 1.0 js.js",
            "module A\nTile 1.0 /etc/q.qml",
            "module A\nimport A 2.3.4",
            "module A\ninternal Tile X.qml\nTile X.qml",
        ] {
            assert!(parse_qmldir(text).is_err(), "accepted {text}");
        }
        let e = parse_qmldir("# first\r\n module A\r\n   Unknown @\r\n").unwrap_err();
        assert_eq!((e.line, e.column), (3, 4));
    }
}

#[cfg(test)]
mod static_plugin_tests {
    use super::*;
    #[test]
    fn static_registration_is_scoped_to_the_declared_module_and_minor_range() {
        let mut manifest = parse_qmldir(
            "module Lab.Device\nplugin devices\noptional plugin style\nCard 1.1 Card.qml",
        )
        .unwrap();
        let unrelated = StaticPlugin::new("Other", "devices", 1, 8);
        assert!(manifest.bind_static_plugins(&[unrelated].into()).is_err());
        manifest
            .bind_static_plugins(&[StaticPlugin::new("Lab.Device", "devices", 1, 4)].into())
            .unwrap();
        let from_native = manifest.select("1.3".parse().unwrap()).unwrap();
        assert_eq!(from_native.get("Card").unwrap().file, "Card.qml");
        assert!(manifest.select("1.5".parse().unwrap()).is_err());
        assert!(manifest.select("2".parse().unwrap()).is_err());
    }
}
