//! Compile native async suspensions to generators driven by Promise reactions.
//! WebScene ABI 3 has a resource callback, but no V8 isolate Promise hook for
//! the implicit reactions installed by the native await instruction.
use crate::{
    javascript::{error, parser},
    Diagnostic,
};
use swc_common::{Globals, Mark, GLOBALS};
use swc_ecma_ast::{ArrowExpr, Function, Pass, Program};
use swc_ecma_transforms_base::{
    fixer::fixer,
    helpers::{inject_helpers, Helpers, HELPERS},
    hygiene::hygiene,
    resolver,
};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Default)]
struct ContainsAsync(bool);
impl Visit for ContainsAsync {
    fn visit_function(&mut self, node: &Function) {
        self.0 |= node.is_async;
        if !self.0 {
            node.visit_children_with(self);
        }
    }
    fn visit_arrow_expr(&mut self, node: &ArrowExpr) {
        self.0 |= node.is_async;
        if !self.0 {
            node.visit_children_with(self);
        }
    }
}

/// Transform executable JavaScript after QML-specific syntax has been prepared.
/// Expressions keep the caller's lexical `this` and `arguments`.
pub fn lower(source: &str, expression: bool) -> Result<Option<String>, Diagnostic> {
    if !source.contains("async") {
        return Ok(None);
    }
    let wrapped;
    let code = if expression {
        wrapped = format!("return (\n{source}\n);");
        &wrapped
    } else {
        source
    };
    let mut parser = parser(code, 0, true)?;
    let script = parser.parse_script().map_err(|e| error(code, e))?;
    let mut found = ContainsAsync::default();
    script.visit_with(&mut found);
    if !found.0 {
        return Ok(None);
    }
    GLOBALS.set(&Globals::new(), || {
        HELPERS.set(&Helpers::new(false), || {
            let unresolved = Mark::new();
            let mut program = Program::Script(script);
            resolver(unresolved, Mark::new(), false).process(&mut program);
            swc_ecma_compat_es2017::async_to_generator(Default::default(), unresolved)
                .process(&mut program);
            inject_helpers(unresolved).process(&mut program);
            hygiene().process(&mut program);
            fixer(None).process(&mut program);
            let code = swc_ecma_codegen::to_code(&program);
            Ok(Some(if expression {
                format!("(()=>{{\n{code}\n}})()")
            } else {
                code
            }))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_ordinary_code_untouched_and_lowers_nested_awaits() {
        assert!(lower("return 'async function';", false).unwrap().is_none());
        let compiled = lower("(async () => this.factor * await pending)()", true)
            .unwrap()
            .unwrap();
        assert!(compiled.starts_with("(()=>{"));
        assert!(!compiled.contains("await pending"));
        assert!(compiled.contains("yield pending"));
    }
}
