//! Resolve assembly metadata text through the scanner's Rust module bindings.
use super::{rust_body, string_literal, ungroup};
use crate::authored_symbols::{AuthoredText, MetadataSource};
use crate::scanner::source::local_imports;
use crate::scanner::{
    framework_text,
    modules::{Import, ItemKind, Modules, Target},
};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use syn::{Attribute, Expr, ExprPath, File, ImplItem, Item, ItemImpl, Type};

type LiteralSlot<'a> = Option<Literal<'a>>;

pub(super) struct MetadataConstants<'a> {
    tree: Modules,
    files: BTreeMap<&'a str, usize>,
    literals: BTreeMap<(usize, String), LiteralSlot<'a>>,
    associated: BTreeMap<(usize, String, String), LiteralSlot<'a>>,
}

pub(super) struct Literal<'a> {
    value: String,
    range: Range<usize>,
    source: MetadataSource<'a>,
}

impl<'a> Literal<'a> {
    pub(super) fn capture(source: MetadataSource<'a>, expr: &Expr) -> Option<Self> {
        let literal = string_literal(ungroup(expr))?;
        let range = source_range(source.text, literal.span().byte_range());
        Some(Self {
            value: literal.value(),
            range,
            source,
        })
    }

    pub(super) fn at_use(
        &self,
        source: MetadataSource<'_>,
        range: Range<usize>,
    ) -> Option<AuthoredText> {
        let imported_literal = if self.source.id != source.id {
            Some(self.source.text.get(self.range.clone())?.to_owned())
        } else {
            None
        };
        Some(AuthoredText {
            value: self.value.clone(),
            range,
            literal: self.range.clone(),
            imported_literal,
        })
    }
}

pub(super) fn source_range(source: &str, range: Range<usize>) -> Range<usize> {
    let offset = source.len() - rust_body(source).len();
    (range.start + offset)..(range.end + offset)
}

type ReadModule<'a> = dyn Fn(&str, &[String]) -> Option<MetadataSource<'a>> + 'a;

impl<'a> MetadataConstants<'a> {
    pub(super) fn local(source: MetadataSource<'a>, file: &File) -> Option<Self> {
        let mut constants = Self::new();
        // A file alone does not establish a crate root for absolute Rust paths.
        let scope = constants.tree.add_module(0, "", false);
        constants.files.insert(source.id, scope);
        constants.index(&file.items, source, scope, &mut Vec::new(), &|_, _| None)?;
        constants.tree.finish();
        Some(constants)
    }

    pub(super) fn project(root: MetadataSource<'a>, read: &ReadModule<'a>) -> Option<Self> {
        let mut constants = Self::new();
        constants.index_file(root, 0, read)?;
        constants.tree.finish();
        Some(constants)
    }

    fn new() -> Self {
        Self {
            tree: Modules::new("", &[]),
            files: BTreeMap::new(),
            literals: BTreeMap::new(),
            associated: BTreeMap::new(),
        }
    }

    pub(super) fn scope(&self, file: &str, inline: &[String]) -> Option<usize> {
        inline
            .iter()
            .try_fold(*self.files.get(file)?, |scope, name| {
                match self.tree.modules[scope].items.get(name)?.kind {
                    ItemKind::Module(child) => Some(child),
                    _ => None,
                }
            })
    }

    pub(super) fn literal(&self, scope: usize, path: &ExprPath) -> Option<&Literal<'a>> {
        let Target::Item { module, name, rest } = self.target(scope, path)? else {
            return None;
        };
        match rest.as_slice() {
            [] => self.literals.get(&(module, name))?.as_ref(),
            [member]
                if matches!(
                    self.tree.modules[module].items.get(&name)?.kind,
                    ItemKind::Type
                ) =>
            {
                self.associated
                    .get(&(module, name, member.clone()))?
                    .as_ref()
            }
            _ => None,
        }
    }

    pub(super) fn framework_key(&self, scope: usize, expr: &Expr) -> bool {
        let Expr::Path(path) = ungroup(expr) else {
            return false;
        };
        matches!(self.target(scope, path), Some(Target::External(segments)) if framework_text(&segments).is_some())
    }

    fn target(&self, scope: usize, path: &ExprPath) -> Option<Target> {
        if path.qself.is_some()
            || !path
                .path
                .segments
                .iter()
                .all(|segment| segment.arguments.is_empty())
        {
            return None;
        }
        let segments = path
            .path
            .leading_colon
            .is_some()
            .then(String::new)
            .into_iter()
            .chain(
                path.path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string()),
            )
            .collect::<Vec<_>>();
        self.tree.resolve(scope, &segments)
    }

    fn index_file(
        &mut self,
        source: MetadataSource<'a>,
        scope: usize,
        read: &ReadModule<'a>,
    ) -> Option<()> {
        let syntax = syn::parse_str::<File>(rust_body(source.text)).ok()?;
        if self.files.contains_key(source.id) || conditional(&syntax.attrs) {
            return None;
        }
        self.files.insert(source.id, scope);
        self.index(&syntax.items, source, scope, &mut Vec::new(), read)
    }

    fn index(
        &mut self,
        items: &[Item],
        source: MetadataSource<'a>,
        scope: usize,
        inline: &mut Vec<String>,
        read: &ReadModule<'a>,
    ) -> Option<()> {
        let mut children = BTreeMap::new();
        let mut imports = Vec::new();
        let mut uncertain = BTreeSet::new();
        for entry in items {
            match entry {
                Item::Const(item) => {
                    let name = item.ident.to_string();
                    let literal = (!conditional(&item.attrs))
                        .then(|| Literal::capture(source, &item.expr))
                        .flatten();
                    self.tree.add_item(scope, &name, ItemKind::Value, false);
                    self.literals
                        .entry((scope, name))
                        .and_modify(|previous| *previous = None)
                        .or_insert(literal);
                }
                Item::Mod(item) => {
                    let body = (!conditional(&item.attrs)).then_some(item);
                    children
                        .entry(item.ident.to_string())
                        .and_modify(|previous| *previous = None)
                        .or_insert(body);
                }
                Item::Use(item) => collect_imports(item, &mut imports, &mut uncertain),
                Item::Struct(item) if !conditional(&item.attrs) => {
                    self.tree
                        .add_item(scope, &item.ident.to_string(), ItemKind::Type, false)
                }
                Item::Enum(item) if !conditional(&item.attrs) => {
                    self.tree
                        .add_item(scope, &item.ident.to_string(), ItemKind::Type, false)
                }
                Item::Impl(item) => self.index_inherent(source, scope, item),
                _ => {}
            }
        }
        for import in &imports {
            if let Some(literal) = self.literals.get_mut(&(scope, import.name.clone())) {
                *literal = None;
            }
        }
        for name in &uncertain {
            // Keep the name bound so a conditional import cannot fall through
            // to an external crate with the same spelling.
            self.tree.add_item(scope, name, ItemKind::Value, false);
        }
        self.tree.modules[scope].imports = imports
            .into_iter()
            .filter(|import| !uncertain.contains(&import.name))
            .collect();
        for (name, body) in children {
            let child = self.tree.add_module(scope, &name, false);
            let Some(body) = body else {
                continue;
            };
            inline.push(name);
            if let Some((_, items)) = &body.content {
                self.index(items, source, child, inline, read)?;
            } else if let Some(child_source) = read(source.id, inline) {
                self.index_file(child_source, child, read)?;
            }
            inline.pop();
        }
        Some(())
    }

    fn index_inherent(&mut self, source: MetadataSource<'a>, scope: usize, item: &ItemImpl) {
        let Type::Path(owner) = item.self_ty.as_ref() else {
            return;
        };
        if item.trait_.is_some() || owner.qself.is_some() || !item.generics.params.is_empty() {
            return;
        }
        let Some(owner) = owner.path.get_ident() else {
            return;
        };
        for member in &item.items {
            let ImplItem::Const(member) = member else {
                continue;
            };
            let literal = (!conditional(&item.attrs) && !conditional(&member.attrs))
                .then(|| Literal::capture(source, &member.expr))
                .flatten();
            self.associated
                .entry((scope, owner.to_string(), member.ident.to_string()))
                .and_modify(|previous| *previous = None)
                .or_insert(literal);
        }
    }
}

fn collect_imports(
    item: &syn::ItemUse,
    imports: &mut Vec<Import>,
    uncertain: &mut BTreeSet<String>,
) {
    let mut bindings = Vec::new();
    let mut prefix = item
        .leading_colon
        .is_some()
        .then(String::new)
        .into_iter()
        .collect();
    local_imports(&item.tree, &mut prefix, &mut bindings);
    for (name, path) in bindings {
        if conditional(&item.attrs) || imports.iter().any(|import| import.name == name) {
            uncertain.insert(name.clone());
        }
        imports.push(Import {
            name,
            path,
            public: false,
            explicit_alias: false,
        });
    }
}

fn conditional(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}
