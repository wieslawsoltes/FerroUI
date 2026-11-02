//! Reference analysis belongs to the authored fragment and document revision.
//! Outlines, completions and navigation elsewhere never need to resolve this JS.
use ferroui_qml_compiler::{
    editor::{ScriptFragment, ScriptReference},
    JavaScript, SourceSpan,
};
use std::cell::OnceCell;

#[derive(Default)]
pub(super) struct ReferenceIndex {
    sites: Vec<Site>,
    ordered: bool,
}

enum Site {
    Assignment(ScriptReference),
    Script {
        fragment: ScriptFragment,
        parameters: Box<[String]>,
        resolved: OnceCell<Reads>,
    },
}

struct Reads {
    entries: Vec<ScriptReference>,
    ordered: bool,
}

impl Site {
    fn span(&self) -> SourceSpan {
        match self {
            Self::Assignment(r) => r.span,
            Self::Script { fragment, .. } => fragment.span(),
        }
    }
}

impl ReferenceIndex {
    pub(super) fn assignment(&mut self, name: &str, span: SourceSpan) {
        self.sites.push(Site::Assignment(ScriptReference {
            span,
            name: name.into(),
            receiver: Some("this".into()),
        }));
    }
    pub(super) fn script(&mut self, script: &JavaScript, parameters: &[&str]) {
        self.sites.push(Site::Script {
            fragment: ScriptFragment::new(script),
            parameters: parameters.iter().map(|s| (*s).into()).collect(),
            resolved: OnceCell::new(),
        });
    }
    pub(super) fn finish(&mut self) {
        self.ordered = index_disjoint(&mut self.sites, Site::span);
    }

    pub(super) fn at(&self, source: &str, at: usize, name: &str) -> Option<&ScriptReference> {
        let hit = |r: &ScriptReference| r.name == name && contains(r.span, at);
        for site in candidates(&self.sites, self.ordered, at, Site::span) {
            if !contains(site.span(), at) {
                continue;
            }
            match site {
                Site::Assignment(r) if hit(r) => return Some(r),
                Site::Assignment(_) => {}
                Site::Script {
                    fragment,
                    parameters,
                    resolved,
                } => {
                    let reads = resolved.get_or_init(|| {
                        let params: Vec<_> = parameters.iter().map(String::as_str).collect();
                        let mut entries = fragment.references(source, &params);
                        let ordered = index_disjoint(&mut entries, |r| r.span);
                        Reads { entries, ordered }
                    });
                    if let Some(r) = candidates(&reads.entries, reads.ordered, at, |r| r.span)
                        .iter()
                        .find(|r| hit(r))
                    {
                        return Some(r);
                    }
                }
            }
        }
        None
    }
}

fn contains(span: SourceSpan, at: usize) -> bool {
    span.start <= at && at < span.end
}

// Small collections stay in AST order. For large collections, index only
// disjoint ranges: overlapping fallback spans retain their original precedence.
fn index_disjoint<T>(entries: &mut [T], span: impl Fn(&T) -> SourceSpan) -> bool {
    if entries.len() <= 16 {
        return false;
    }
    if entries
        .windows(2)
        .all(|w| span(&w[0]).end <= span(&w[1]).start)
    {
        return true;
    }
    let mut ranges: Vec<_> = entries.iter().map(&span).collect();
    ranges.sort_unstable_by_key(|s| s.start);
    if ranges.windows(2).any(|w| w[0].end > w[1].start) {
        return false;
    }
    entries.sort_unstable_by_key(|entry| span(entry).start);
    true
}
fn candidates<T>(entries: &[T], ordered: bool, at: usize, span: impl Fn(&T) -> SourceSpan) -> &[T] {
    if !ordered {
        return entries;
    }
    let after = entries.partition_point(|entry| span(entry).start <= at);
    &entries[after.saturating_sub(1)..after]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_qml_compiler::{parse_external, MemberKind, Stmt};

    #[test]
    fn resolving_a_reference_only_indexes_its_own_function() {
        let source = "import QtQuick\nItem { function first(arg) { return arg + root.x } function other() { return sibling.y } }";
        let doc = parse_external(source).unwrap();
        let mut index = ReferenceIndex::default();
        for member in &doc.root.members {
            let MemberKind::Function {
                parameters, body, ..
            } = &member.kind
            else {
                continue;
            };
            let [Stmt::JavaScript(script)] = body.as_slice() else {
                panic!("JavaScript body")
            };
            let params = parameters
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>();
            index.script(script, &params);
        }
        index.finish();
        assert_eq!(index.sites.len(), 2);
        assert!(index
            .at(source, source.find("arg +").unwrap(), "arg")
            .is_none());
        let read = index
            .at(source, source.find(".x").unwrap() + 1, "x")
            .unwrap();
        assert_eq!(read.receiver.as_deref(), Some("root"));
        assert!(
            matches!(&index.sites[1], Site::Script { resolved, .. } if resolved.get().is_none())
        );
    }
}
