use quote::ToTokens;
use std::collections::BTreeSet;
use syn::visit::Visit;
#[derive(Debug, PartialEq, Eq)]
pub enum Admission {
    Unchanged,
    Patch { functions: Vec<String> },
    RestartRequired(String),
}
/// Only top-level, registered, synchronous free-function bodies may change. Signatures,
/// attributes and every other token must remain identical. Unknown syntax fails closed.
pub fn admit(before: &str, after: &str, registered: &BTreeSet<String>) -> Admission {
    let (Ok(mut old), Ok(mut new)) = (syn::parse_file(before), syn::parse_file(after)) else {
        return Admission::RestartRequired("Rust source cannot be parsed".into());
    };
    if old.to_token_stream().to_string() == new.to_token_stream().to_string() {
        return Admission::Unchanged;
    }
    let mut changed = Vec::new();
    for item in &mut old.items {
        let syn::Item::Fn(previous) = item else {
            continue;
        };
        let name = previous.sig.ident.to_string();
        if !registered.contains(&name) {
            continue;
        }
        let Some(syn::Item::Fn(next)) = new.items.iter_mut().find(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == previous.sig.ident)) else {
            return Admission::RestartRequired(format!("Registered function removed: {name}"));
        };
        if previous.block.to_token_stream().to_string() == next.block.to_token_stream().to_string()
        {
            continue;
        }
        if unsupported_signature(&previous.sig) {
            return Admission::RestartRequired(format!("Unsupported patch boundary: {name}"));
        }
        for body in [&previous.block, &next.block] {
            if unsupported_body(body) {
                return Admission::RestartRequired(format!(
                    "Closure, macro, async block, unsafe block or declaration in {name}"
                ));
            }
        }
        previous.block = Box::new(syn::parse_quote!({}));
        next.block = Box::new(syn::parse_quote!({}));
        changed.push(name);
    }
    if old.to_token_stream().to_string() != new.to_token_stream().to_string() || changed.is_empty()
    {
        Admission::RestartRequired("Change outside registered function bodies".into())
    } else {
        Admission::Patch { functions: changed }
    }
}
fn unsupported_signature(signature: &syn::Signature) -> bool {
    signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.constness.is_some()
        || !signature.generics.params.is_empty()
}
fn unsupported_body(body: &syn::Block) -> bool {
    let mut unsupported = Unsupported(false);
    unsupported.visit_block(body);
    unsupported.0
}
pub(crate) fn validate_registration(source: &str, name: &str) -> Result<(), String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let function = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
        .ok_or_else(|| format!("Registered top-level function not found: {name}"))?;
    if unsupported_signature(&function.sig) || unsupported_body(&function.block) {
        return Err(format!("Unsupported patch boundary: {name}"));
    }
    Ok(())
}
struct Unsupported(bool);
impl<'ast> Visit<'ast> for Unsupported {
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {
        self.0 = true;
    }
    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {
        self.0 = true;
    }
    fn visit_expr_unsafe(&mut self, _: &'ast syn::ExprUnsafe) {
        self.0 = true;
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {
        self.0 = true;
    }
    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.0 = true;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn check(old: &str, new: &str) -> Admission {
        admit(old, new, &BTreeSet::from(["click".into()]))
    }
    #[test]
    fn admits_body_change() {
        assert!(matches!(
            check(
                "fn click(x: i32) -> i32 { x+1 }",
                "fn click(x: i32) -> i32 { x+2 }"
            ),
            Admission::Patch { .. }
        ));
    }
    #[test]
    fn rejects_signature() {
        assert!(matches!(
            check("fn click(x: i32) { }", "fn click(x: i64) { }"),
            Admission::RestartRequired(_)
        ));
    }
    #[test]
    fn rejects_layout_and_static() {
        for addition in [
            "struct State { x: i64 }",
            "static VALUE: i32 = 1;",
            "fn other() {}",
        ] {
            assert!(matches!(
                check("fn click() {}", &format!("fn click() {{}} {addition}")),
                Admission::RestartRequired(_)
            ));
        }
    }
    #[test]
    fn rejects_closure_and_macro() {
        for body in [
            "let f = || 1;",
            "println!(\"changed\");",
            "struct New;",
            "let f = async { 1 };",
            "unsafe {}",
        ] {
            assert!(matches!(
                check("fn click() {}", &format!("fn click() {{ {body} }}")),
                Admission::RestartRequired(_)
            ));
        }
    }
    #[test]
    fn ignores_whitespace() {
        assert_eq!(
            check("fn click() {}", "fn click( ) { }"),
            Admission::Unchanged
        );
    }
}
