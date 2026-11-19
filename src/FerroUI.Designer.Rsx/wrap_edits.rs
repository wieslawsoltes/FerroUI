//! Wrap a mapped child run while retaining the original Rust token spans.
use crate::{
    blocks::{comment_starts, keep_comments},
    xaml::XamlTree,
    RsxProjection,
};
use ferroui_designer_core::{Document, SourceSpan, TextPatch};
use proc_macro2::{TokenStream, TokenTree};
use std::collections::{BTreeMap, BTreeSet};

pub struct RsxWrap<'a> {
    pub parent: usize,
    pub children: &'a [usize],
    pub fragment: &'a str,
    /// Layout fields whose expressions cannot supply a measured position for the wrapper.
    pub guarded_properties: &'a [&'a str],
    pub attributes: &'a [(usize, &'a str, Option<&'a str>)],
}

pub struct RsxWrapResult {
    pub patches: Vec<TextPatch>,
    /// Name of the new wrapper in the authored source after applying the patches.
    pub wrapper: SourceSpan,
}

impl RsxProjection<'_> {
    pub fn wrap_children(&self, request: RsxWrap<'_>) -> Result<RsxWrapResult, String> {
        let parent = self
            .nodes
            .get(request.parent)
            .ok_or("Choose a mapped Rust container")?;
        if request.children.is_empty()
            || !parent
                .child_indices
                .windows(request.children.len())
                .any(|run| run == request.children)
        {
            return Err("Choose consecutive mapped Rust siblings".into());
        }
        let children: Vec<_> = request
            .children
            .iter()
            .map(|&index| &self.nodes[index])
            .collect();
        for node in &children {
            if let Some(name) = request
                .guarded_properties
                .iter()
                .find(|name| node.expressions.contains_key(**name))
            {
                return Err(format!(
                    "Edit the Rust {name} expression before changing this layout"
                ));
            }
        }
        if !children.windows(2).all(|pair| {
            self.source[pair[0].span.end..pair[1].span.start]
                .parse::<TokenStream>()
                .is_ok_and(|tokens| {
                    tokens.into_iter().all(
                        |token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ','),
                    )
                })
        }) {
            return Err(
                "A dynamic Rust child interrupts the selected run. Wrap it in source.".into(),
            );
        }
        let first = children[0].span.start;
        let last = children.last().unwrap().span.end;
        let wrapper = Document::new(request.fragment).map_err(|error| error.to_string())?;
        let wrapper = XamlTree::parse(&wrapper, self.catalog)?.tree.rsx_body();
        let open = wrapper
            .strip_suffix('}')
            .ok_or("The wrapper has no Rust constructor")?
            .trim_end();
        let mut patches = self.wrap_attributes(&request)?;
        if let Some(indent) = line_indent(self.source, first) {
            let unit = line_indent(self.source, parent.span.start)
                .and_then(|prefix| indent.strip_prefix(prefix))
                .filter(|unit| !unit.is_empty())
                .unwrap_or(if indent.ends_with('\t') { "\t" } else { "    " });
            let eol = if self.source[..first]
                .rsplit_once('\n')
                .is_some_and(|(prefix, _)| prefix.ends_with('\r'))
            {
                "\r\n"
            } else {
                "\n"
            };
            let inner = format!("{eol}{indent}{unit}");
            let opening = open
                .lines()
                .enumerate()
                .map(|(index, line)| {
                    if index == 0 {
                        line.to_owned()
                    } else {
                        format!("{inner}{}", line.trim_start())
                    }
                })
                .collect::<String>();
            let mut offsets = BTreeSet::new();
            let tokens = self.source[first..last]
                .parse::<TokenStream>()
                .map_err(|_| "Read the original Rust child tokens first")?;
            offsets.extend(comment_starts(&self.source[first..last], tokens.clone()));
            token_starts(tokens, &mut offsets);
            let indents: Vec<_> = offsets
                .into_iter()
                .map(|offset| first + offset)
                .filter(|&offset| {
                    offset > first
                        && line_indent(self.source, offset).is_some()
                        && !patches
                            .iter()
                            .any(|patch| patch.span.start <= offset && offset < patch.span.end)
                })
                .map(|offset| insert(offset, unit.to_owned()))
                .collect();
            patches.extend(indents);
            patches.push(insert(first, format!("{opening}{inner}")));
            patches.push(insert(last, format!("{eol}{indent}}}")));
        } else {
            patches.push(insert(
                first,
                format!(
                    "{} ",
                    open.lines().map(str::trim).collect::<Vec<_>>().join(" ")
                ),
            ));
            patches.push(insert(last, " }".into()));
        }
        let candidate = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let parsed = self.catalog.project(&candidate, self.invocation.start)?;
        let added = parsed
            .nodes
            .iter()
            .find(|node| node.span.start == first)
            .ok_or("The wrapper has no accepted Rust mapping")?;
        if parsed.nodes.len() != self.nodes.len() + 1
            || parsed.omitted_count != self.omitted_count
            || added.child_indices.len() != request.children.len()
        {
            return Err("Wrapping would change an unmapped Rust child or property".into());
        }
        Ok(RsxWrapResult {
            patches,
            wrapper: added.name,
        })
    }

    fn wrap_attributes(&self, request: &RsxWrap<'_>) -> Result<Vec<TextPatch>, String> {
        let mut changes = Vec::new();
        let mut insertions = BTreeMap::<usize, String>::new();
        for &(index, name, value) in request.attributes {
            if !request.children.contains(&index) {
                return Err("A wrapper layout edit targeted a different Rust control".into());
            }
            let mut patch = self.edit_property(index, name, value)?;
            if value.is_none() {
                patch.span.start = self.source[..patch.span.start]
                    .trim_end_matches([' ', '\t'])
                    .len();
            }
            if !keep_comments(&self.source[patch.span.range()])?.is_empty() {
                return Err(format!(
                    "Edit {name} in Rust to preserve the comments within it"
                ));
            }
            if patch.span.is_empty() {
                insertions
                    .entry(patch.span.start)
                    .or_default()
                    .push_str(&patch.replacement);
            } else {
                changes.push(patch);
            }
        }
        changes.extend(
            insertions
                .into_iter()
                .map(|(offset, text)| insert(offset, text)),
        );
        Ok(changes)
    }
}

fn token_starts(tokens: TokenStream, offsets: &mut BTreeSet<usize>) {
    for token in tokens {
        match token {
            TokenTree::Group(group) => {
                offsets.insert(group.span_open().byte_range().start);
                offsets.insert(group.span_close().byte_range().start);
                token_starts(group.stream(), offsets);
            }
            leaf => {
                offsets.insert(leaf.span().byte_range().start);
            }
        }
    }
}

fn line_indent(source: &str, offset: usize) -> Option<&str> {
    let line = source[..offset].rsplit('\n').next()?;
    line.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t'))
        .then_some(line)
}

fn insert(offset: usize, replacement: String) -> TextPatch {
    TextPatch {
        span: SourceSpan::new(offset, offset),
        replacement,
    }
}
