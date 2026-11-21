use super::{
    escape_xml, require, resource_scope, DeclarationIndex, DocumentEdits, LanguageError,
    LanguageService, Occurrence, OccurrenceSpelling, ResourceLookup, Result, SourceSpan,
    SymbolKind, TextEdit, WorkspaceEdit,
};
use std::collections::BTreeMap;

#[path = "resource_rename.rs"]
pub(super) mod resource_rename;

impl LanguageService {
    pub fn rename(&self, uri: &str, offset: usize, new_name: &str) -> Result<WorkspaceEdit> {
        let target = self
            .target(uri, offset)?
            .ok_or_else(|| LanguageError("No unambiguous symbol at the caret".into()))?;
        self.rename_declaration(&target, new_name)
    }

    fn rename_declaration(
        &self,
        target: &(String, Occurrence),
        new_name: &str,
    ) -> Result<WorkspaceEdit> {
        self.rename_declarations(std::slice::from_ref(target), new_name)
    }

    fn rename_declarations(
        &self,
        targets: &[(String, Occurrence)],
        new_name: &str,
    ) -> Result<WorkspaceEdit> {
        let (target_uri, declaration) = &targets[0];
        let resource = declaration.kind == SymbolKind::Resource;
        require(
            if resource {
                !new_name.is_empty()
                    && new_name.chars().all(|c| {
                        matches!(c, '\t' | '\n' | '\r')
                            || c >= ' ' && !matches!(c, '\u{fffe}' | '\u{ffff}')
                    })
            } else {
                crate::references::identifier(new_name)
            },
            "Names must be XAML identifiers. Resource keys must be nonempty XML text",
        )?;
        if new_name == declaration.name {
            return Ok(WorkspaceEdit::default());
        }
        // Keep syntax identities and the dictionary graph fixed while testing the
        // proposed spelling through the same resolver that powers navigation.
        let mut grouped = BTreeMap::<String, Vec<TextEdit>>::new();
        let mut new_entries = self.declarations.get(new_name).cloned().unwrap_or_default();
        let mut old_entries = self.declarations[&declaration.name].clone();
        let selected = |entry: &(String, Occurrence)| {
            targets.iter().any(|target| same_declaration(entry, target))
        };
        old_entries.retain(|entry| !selected(entry));
        for target in targets {
            let (uri, symbol) = target;
            let state = self.state(uri)?;
            require(
                !new_entries.iter().any(|(other_uri, other)| {
                    other_uri == uri && other.kind == symbol.kind
                        && (!resource || resource_scope(&state.document, other.node)
                            == resource_scope(&state.document, symbol.node))
                }),
                "A declaration with this name already exists in scope",
            )?;
            let mut renamed = target.clone();
            renamed.1.name = new_name.into();
            new_entries.push(renamed);
            grouped.entry(uri.clone()).or_default().push(TextEdit {
                range: state.index.range(symbol.span)?,
                new_text: replacement(symbol, new_name),
            });
        }
        let overrides = DeclarationIndex::from([
            (declaration.name.clone(), old_entries),
            (new_name.into(), new_entries),
        ]);
        for (uri, state) in &self.documents {
            if !resource && uri != target_uri {
                continue;
            }
            for reference in state.symbols.iter().filter(|symbol| {
                !symbol.declaration
                    && symbol.kind == declaration.kind
                    && (symbol.name == declaration.name || symbol.name == new_name)
            }) {
                let before = self.resolve_with_context(uri, reference);
                let rewrite = !before.matches.is_empty() && before.matches.iter().all(selected);
                let mut proposed = reference.clone();
                if rewrite {
                    proposed.name = new_name.into();
                }
                let after = self.resolve_with_declarations(uri, &proposed, &overrides);
                if !before.same_targets(&after) {
                    return Err(LanguageError(format!(
                        "Renaming {} to {new_name} would change the resolution of {} in {uri}",
                        declaration.name, reference.name,
                    )));
                }
                if rewrite {
                    require(
                        can_spell(reference, new_name),
                        format!("The new key cannot be written in the resource reference in {uri}"),
                    )?;
                    grouped.entry(uri.clone()).or_default().push(TextEdit {
                        range: state.index.range(reference.span)?,
                        new_text: replacement(reference, new_name),
                    });
                }
            }
        }
        let document_changes = grouped
            .into_iter()
            .map(|(uri, edits)| DocumentEdits {
                version: self.documents[&uri].version,
                uri,
                edits,
            })
            .collect();
        Ok(WorkspaceEdit { document_changes })
    }

    pub(crate) fn reference_case_fix(
        &self,
        uri: &str,
        span: SourceSpan,
    ) -> Result<Option<(String, String)>> {
        let state = self.state(uri)?;
        let Some(reference) = state
            .symbols
            .iter()
            .find(|o| !o.declaration && o.span == span)
        else {
            return Ok(None);
        };
        let mut candidate = reference.clone();
        let mut matches = self.declarations.keys().filter(|name| {
            if name.as_str() == reference.name
                || !name.eq_ignore_ascii_case(&reference.name)
                || !can_spell(reference, name)
            {
                return false;
            }
            candidate.name.clone_from(name);
            let resolution = self.resolve_with_context(uri, &candidate);
            resolution.matches.len() == 1 && !resolution.may_be_missing
        });
        Ok(match (matches.next(), matches.next()) {
            (Some(name), None) => Some((format!("Use {name}"), replacement(reference, name))),
            _ => None,
        })
    }
}

impl ResourceLookup {
    fn same_targets(&self, other: &Self) -> bool {
        self.may_be_missing == other.may_be_missing
            && self.themed == other.themed
            && self.matches.len() == other.matches.len()
            && self.matches.iter().all(|entry| {
                other
                    .matches
                    .iter()
                    .any(|other| same_declaration(entry, other))
            })
    }
}

fn same_declaration(a: &(String, Occurrence), b: &(String, Occurrence)) -> bool {
    a.0 == b.0 && a.1.span == b.1.span
}

fn replacement(occurrence: &Occurrence, name: &str) -> String {
    match occurrence.spelling {
        OccurrenceSpelling::Attribute if occurrence.kind == SymbolKind::Resource => {
            escape_xml(&literal_key(name))
        }
        OccurrenceSpelling::Attribute => escape_xml(name),
        OccurrenceSpelling::Text => escape_xml(name).replace('>', "&gt;"),
        OccurrenceSpelling::CData => name.into(),
        OccurrenceSpelling::MarkupArgument(authored_quote) => {
            let quote = authored_quote.or_else(|| {
                name.chars()
                    .any(|c| {
                        c.is_whitespace() || matches!(c, '{' | '}' | ',' | '=' | '\'' | '"' | '\\')
                    })
                    .then_some('\'')
            });
            let Some(quote) = quote else {
                return escape_xml(name);
            };
            let value = literal_key(name)
                .replace('\\', "\\\\")
                .replace(quote, &format!("\\{quote}"));
            if authored_quote.is_some() {
                escape_xml(&value)
            } else {
                escape_xml(&format!("{quote}{value}{quote}"))
            }
        }
    }
}

fn literal_key(name: &str) -> std::borrow::Cow<'_, str> {
    if name.starts_with('{') {
        format!("{{}}{name}").into()
    } else {
        name.into()
    }
}

fn can_spell(occurrence: &Occurrence, name: &str) -> bool {
    match occurrence.spelling {
        OccurrenceSpelling::Text => name.trim() == name,
        OccurrenceSpelling::CData => {
            name.trim() == name && !name.contains("]]>") && !name.contains('\r')
        }
        _ => true,
    }
}
