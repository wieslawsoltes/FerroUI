//! Lexical scopes from Rust syntax. Unverified and conditional bindings remain opaque.
use super::{Binding, LocalRust, Scope};
use crate::animation::syntax::span;
use ferroui_designer_core::SourceSpan;
use ferroui_designer_project::{RustModuleSources, visit_rust_imports};
use std::collections::BTreeMap;
use syn::ext::IdentExt;
use syn::visit::{self, Visit};
use syn::{Attribute, GenericParam, Ident, Item};

pub(super) fn parse(source: &str, offset: usize, modules: Option<&RustModuleSources>) -> LocalRust {
    let target = modules.map(RustModuleSources::target);
    let root = target.map_or(source, |file| file.source.as_ref());
    let mut builder = Collector {
        scopes: vec![Scope {
            span: SourceSpan::new(0, root.len()),
            parent: None,
            module: 0,
            super_module: None,
            file: 0,
            indexed: modules.is_some(),
            members: BTreeMap::new(),
        }],
        files: vec![target.map_or_else(String::new, |file| file.uri.clone())],
        active: modules
            .is_none_or(|modules| modules.active().uri == modules.target().uri)
            .then_some(0),
        modules,
        names: Vec::new(),
        current: 0,
        offset,
        has_invocation: false,
    };
    builder.read_file(root);
    if !builder.has_invocation {
        if modules.is_some() {
            return parse(source, offset, None);
        }
        builder.scopes.truncate(1);
        builder.scopes[0].members.clear();
    }
    LocalRust {
        scopes: builder.scopes,
        files: builder.files,
        active: builder.active.unwrap_or(0),
        target_known: modules.is_some(),
    }
}

struct Collector<'a> {
    scopes: Vec<Scope>,
    files: Vec<String>,
    active: Option<usize>,
    modules: Option<&'a RustModuleSources>,
    names: Vec<String>,
    current: usize,
    offset: usize,
    has_invocation: bool,
}

impl Collector<'_> {
    fn read_file(&mut self, source: &str) {
        if let Ok(file) = syn::parse_str::<syn::File>(source) {
            self.visit_file(&file);
        }
    }

    fn add(&mut self, name: &Ident, binding: Binding) {
        let name = name.unraw().to_string();
        let members = &mut self.scopes[self.current].members;
        members
            .entry(name)
            .and_modify(|entry| *entry = Binding::Opaque)
            .or_insert(binding);
    }

    fn lexical(&mut self, span: SourceSpan, visit: impl FnOnce(&mut Self)) {
        if self.active != Some(self.scopes[self.current].file) || !span.contains(self.offset) {
            return;
        }
        let parent = self.current;
        self.current = self.scopes.len();
        self.scopes.push(Scope {
            span,
            parent: Some(parent),
            module: self.scopes[parent].module,
            super_module: None,
            file: self.scopes[parent].file,
            indexed: false,
            members: BTreeMap::new(),
        });
        visit(self);
        self.current = parent;
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        self.has_invocation |= self.active == Some(self.scopes[self.current].file)
            && item
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "rsx")
            && span(item).contains(self.offset);
    }

    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Enum(item) => self.add(&item.ident, Binding::Opaque),
            Item::Fn(item) => self.add(&item.sig.ident, Binding::Opaque),
            Item::Static(item) => self.add(&item.ident, Binding::Opaque),
            Item::Struct(item) => self.add(&item.ident, Binding::Opaque),
            Item::Trait(item) => self.add(&item.ident, Binding::Opaque),
            Item::TraitAlias(item) => self.add(&item.ident, Binding::Opaque),
            Item::Type(item) => self.add(&item.ident, Binding::Opaque),
            Item::Union(item) => self.add(&item.ident, Binding::Opaque),
            _ => {}
        }
        visit::visit_item(self, item);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        visit_rust_imports(&item.tree, |name, path| {
            let binding = if conditional(&item.attrs) {
                Binding::Opaque
            } else {
                Binding::Import {
                    absolute: item.leading_colon.is_some(),
                    path: path.iter().map(|name| name.unraw().to_string()).collect(),
                }
            };
            self.add(name, binding);
        });
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        let binding = if conditional(&item.attrs) || !item.generics.params.is_empty() {
            Binding::Opaque
        } else {
            Binding::Constant(item.expr.clone())
        };
        self.add(&item.ident, binding);
        visit::visit_item_const(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let parent = self.current;
        let indexed = self.scopes[parent].indexed && !conditional(&item.attrs);
        let parent_file = self.scopes[parent].file;
        self.names.push(item.ident.unraw().to_string());
        let external = if item.content.is_none() && indexed {
            self.modules
                .and_then(|modules| modules.module_file(&self.files[parent_file], &self.names))
        } else {
            None
        };
        let (file, module_span) = if let Some((braces, _)) = &item.content {
            (
                parent_file,
                SourceSpan::new(
                    braces.span.open().byte_range().start,
                    braces.span.close().byte_range().end,
                ),
            )
        } else if let Some(external) = external {
            let file = self.files.len();
            self.files.push(external.uri.clone());
            if self
                .modules
                .is_some_and(|modules| modules.active().uri == external.uri)
            {
                self.active = Some(file);
            }
            (file, SourceSpan::new(0, external.source.len()))
        } else {
            self.names.pop();
            self.add(&item.ident, Binding::Opaque);
            return;
        };
        let module = self.scopes.len();
        let attached = if conditional(&item.attrs) {
            Binding::Opaque
        } else {
            Binding::Module(module)
        };
        self.add(&item.ident, attached);
        self.scopes.push(Scope {
            span: module_span,
            parent: None,
            module,
            super_module: Some(self.scopes[parent].module),
            file,
            indexed,
            members: BTreeMap::new(),
        });
        self.current = module;
        if let Some((_, items)) = &item.content {
            for item in items {
                self.visit_item(item);
            }
        } else if let Some(external) = external {
            let names = std::mem::take(&mut self.names);
            self.read_file(&external.source);
            self.names = names;
        }
        self.current = parent;
        self.names.pop();
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.lexical(span(block), |this| visit::visit_block(this, block));
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.lexical(span(item), |this| visit::visit_item_fn(this, item));
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.lexical(span(item), |this| visit::visit_item_impl(this, item));
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.lexical(span(item), |this| visit::visit_item_trait(this, item));
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        self.lexical(span(item), |this| visit::visit_trait_item_fn(this, item));
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.lexical(span(item), |this| visit::visit_impl_item_fn(this, item));
    }

    fn visit_expr_closure(&mut self, item: &'ast syn::ExprClosure) {
        self.lexical(span(item), |this| visit::visit_expr_closure(this, item));
    }

    fn visit_generic_param(&mut self, param: &'ast GenericParam) {
        match param {
            GenericParam::Type(param) => self.add(&param.ident, Binding::Opaque),
            GenericParam::Const(param) => self.add(&param.ident, Binding::Opaque),
            GenericParam::Lifetime(_) => {}
        }
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.add(&pattern.ident, Binding::Opaque);
        visit::visit_pat_ident(self, pattern);
    }
}

fn conditional(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr"))
}
