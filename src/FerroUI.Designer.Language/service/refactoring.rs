use super::{
    escape_xml, require, resource_scope, DeclarationIndex, DocumentEdits, LanguageError,
    LanguageService, Occurrence, ResourceLookup, Result, SourceSpan, SymbolKind, TextEdit,
    WorkspaceEdit,
};
use std::collections::BTreeMap;

impl LanguageService {
    pub fn rename(&self, uri: &str, offset: usize, new_name: &str) -> Result<WorkspaceEdit> {
        let target = self
            .target(uri, offset)?
            .ok_or_else(|| LanguageError("No unambiguous symbol at the caret".into()))?;
        let (target_uri, declaration) = &target;
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
        let state = self.state(target_uri)?;
        let mut new_entries = self.declarations.get(new_name).cloned().unwrap_or_default();
        require(
            !new_entries.iter().any(|(uri, symbol)| {
                uri == target_uri
                    && symbol.kind == declaration.kind
                    && (!resource
                        || resource_scope(&state.document, symbol.node)
                            == resource_scope(&state.document, declaration.node))
            }),
            "A declaration with this name already exists in scope",
        )?;

        // Keep syntax identities and the dictionary graph fixed while testing the
        // proposed spelling through the same resolver that powers navigation.
        let mut old_entries = self.declarations[&declaration.name].clone();
        old_entries.retain(|entry| !same_declaration(entry, &target));
        let mut renamed = target.clone();
        renamed.1.name = new_name.into();
        new_entries.push(renamed);
        let overrides = DeclarationIndex::from([
            (declaration.name.clone(), old_entries),
            (new_name.into(), new_entries),
        ]);
        let mut grouped = BTreeMap::from([(
            target_uri.clone(),
            vec![TextEdit {
                range: state.index.range(declaration.span)?,
                new_text: replacement(declaration, new_name),
            }],
        )]);
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
                let rewrite = matches!(before.matches.as_slice(), [entry] if same_declaration(entry, &target));
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
            if name.as_str() == reference.name || !name.eq_ignore_ascii_case(&reference.name) {
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
    if occurrence.kind == SymbolKind::Resource && !occurrence.declaration {
        let quote = occurrence.quote.or_else(|| {
            name.chars()
                .any(|c| {
                    c.is_whitespace() || matches!(c, '{' | '}' | ',' | '=' | '\'' | '"' | '\\')
                })
                .then_some('\'')
        });
        if let Some(quote) = quote {
            let value = name
                .replace('\\', "\\\\")
                .replace(quote, &format!("\\{quote}"));
            if occurrence.quote.is_some() {
                return escape_xml(&value);
            }
            return escape_xml(&format!("{quote}{value}{quote}"));
        }
    }
    escape_xml(name)
}
