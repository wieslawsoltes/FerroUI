//! Re-locate the selected assembly's metadata in the current Rust buffer.
use super::super::source::{field_of, last_segment_of_type, pair_expressions};
use super::{module_items, rust_body};
use crate::authored_symbols::{AuthoredAssembly, AuthoredMetadata, MetadataSource};
use syn::{spanned::Spanned, Expr, File, Item, Lit, LitStr};

#[path = "metadata_constants.rs"]
mod constants;
use constants::{source_range, Literal, MetadataConstants};

impl AuthoredAssembly {
    /// Exact tuple operands in the assembly's metadata array. A text constant
    /// in this file can be edited at its use without changing its definition.
    pub fn metadata_in(&self, source: &str) -> Option<Vec<AuthoredMetadata>> {
        let source = MetadataSource {
            id: "",
            text: source,
        };
        let file = syn::parse_str::<File>(rust_body(source.text)).ok()?;
        let constants = MetadataConstants::local(source, &file)?;
        self.metadata(source, &file, &constants)
    }

    /// Resolve tuple operands from the inputs of one build-selected crate.
    /// The caller supplies the verified module edges and current file texts.
    pub fn metadata_in_project<'a>(
        &self,
        assembly: MetadataSource<'a>,
        root: MetadataSource<'a>,
        read_module: impl Fn(&str, &[String]) -> Option<MetadataSource<'a>> + 'a,
    ) -> Option<Vec<AuthoredMetadata>> {
        let file = syn::parse_str::<File>(rust_body(assembly.text)).ok()?;
        let constants = MetadataConstants::project(root, &read_module)?;
        self.metadata(assembly, &file, &constants)
    }

    fn metadata(
        &self,
        source: MetadataSource<'_>,
        file: &File,
        constants: &MetadataConstants<'_>,
    ) -> Option<Vec<AuthoredMetadata>> {
        let scope = constants.scope(source.id, &self.modules)?;
        let items = module_items(file, &self.modules)?;
        let mut candidates = items.iter().filter_map(|item| match item {
            Item::Static(item) if item.ident == self.item => Some(item),
            _ => None,
        });
        let assembly = candidates.next()?;
        if candidates.next().is_some()
            || last_segment_of_type(&assembly.ty).as_deref() != Some("MarkupAssembly")
        {
            return None;
        }
        let Expr::Struct(value) = assembly.expr.as_ref() else {
            return None;
        };
        let pairs = pair_expressions(field_of(value, "metadata")?)?;
        let authored_text = |expr: &Expr| {
            let operand = ungroup(expr);
            let range = source_range(source.text, operand.span().byte_range());
            match operand {
                Expr::Path(path) => constants.literal(scope, path)?.at_use(source, range),
                _ => Literal::capture(source, operand)?.at_use(source, range),
            }
        };
        Some(
            pairs
                .into_iter()
                .map(|(key, value)| AuthoredMetadata {
                    key: authored_text(key),
                    value: authored_text(value),
                    framework_key: constants.framework_key(scope, key),
                })
                .collect(),
        )
    }
}

fn ungroup(expr: &Expr) -> &Expr {
    match expr {
        Expr::Paren(inner) => ungroup(&inner.expr),
        Expr::Group(inner) => ungroup(&inner.expr),
        other => other,
    }
}

fn string_literal(expr: &Expr) -> Option<&LitStr> {
    match expr {
        Expr::Lit(literal) => match &literal.lit {
            Lit::Str(value) => Some(value),
            _ => None,
        },
        _ => None,
    }
}
