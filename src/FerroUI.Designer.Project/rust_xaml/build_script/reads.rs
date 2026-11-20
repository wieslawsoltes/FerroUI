//! Follow transparent, immutable path and file-read bindings in a Cargo build script.
use super::{string_literal, transparent, BuildScript, MacroSyntax};
use std::path::Path;
use syn::{Expr, ExprCall, ExprMethodCall, LitStr};

#[derive(Clone, Copy)]
pub(super) enum ReadPath<'source> {
    Manifest,
    Literal(&'source LitStr),
    ManifestRelative(&'source LitStr),
}

impl<'source> BuildScript<'source> {
    pub(super) fn read_path(
        &self,
        expression: &'source Expr,
        macros: &MacroSyntax,
    ) -> Option<ReadPath<'source>> {
        let expression = transparent(expression);
        match expression {
            Expr::Path(path) => self.binding(path)?.path,
            Expr::MethodCall(call) if call.method == "join" && call.args.len() == 1 => {
                if matches!(self.read_path(&call.receiver, macros)?, ReadPath::Manifest) {
                    string_literal(self.value(&call.args[0])).map(ReadPath::ManifestRelative)
                } else {
                    None
                }
            }
            Expr::MethodCall(call)
                if matches!(call.method.to_string().as_str(), "as_path" | "as_ref")
                    && call.args.is_empty() =>
            {
                self.read_path(&call.receiver, macros)
            }
            Expr::Call(call)
                if std_call(call, macros, &["path", "PathBuf", "from"])
                    || std_call(call, macros, &["path", "Path", "new"]) =>
            {
                self.read_path(&call.args[0], macros)
            }
            value if manifest_directory(value, macros) => Some(ReadPath::Manifest),
            value => string_literal(value).map(ReadPath::Literal),
        }
    }

    pub(super) fn read_source(
        &self,
        expression: &'source Expr,
        macros: &MacroSyntax,
    ) -> Option<&'source LitStr> {
        let mut expression = transparent(expression);
        loop {
            expression = match expression {
                Expr::Path(path) => return self.binding(path)?.read,
                Expr::Try(value) => transparent(&value.expr),
                Expr::MethodCall(call)
                    if unwrapped(call, macros)
                        || (matches!(call.method.to_string().as_str(), "as_str" | "as_ref")
                            && call.args.is_empty()) =>
                {
                    transparent(&call.receiver)
                }
                Expr::Call(call) if std_call(call, macros, &["fs", "read_to_string"]) => {
                    return match self.read_path(&call.args[0], macros)? {
                        ReadPath::ManifestRelative(path) => Some(path),
                        ReadPath::Literal(path)
                            if !macros.changes_directory
                                || Path::new(&path.value()).is_absolute() =>
                        {
                            Some(path)
                        }
                        _ => None,
                    };
                }
                _ => return None,
            };
        }
    }
}

fn manifest_directory(expression: &Expr, macros: &MacroSyntax) -> bool {
    let expression = match expression {
        Expr::MethodCall(call) if unwrapped(call, macros) => transparent(&call.receiver),
        Expr::Try(value) => transparent(&value.expr),
        other => other,
    };
    match expression {
        Expr::Macro(value) => macros.arguments(&value.mac, "env").is_some_and(|args| {
            args.first()
                .and_then(string_literal)
                .is_some_and(|value| value.value() == "CARGO_MANIFEST_DIR")
        }),
        Expr::Call(call)
            if std_call(call, macros, &["env", "var_os"])
                || std_call(call, macros, &["env", "var"]) =>
        {
            string_literal(&call.args[0]).is_some_and(|value| value.value() == "CARGO_MANIFEST_DIR")
        }
        _ => false,
    }
}

fn std_call(call: &ExprCall, macros: &MacroSyntax, function: &[&str]) -> bool {
    let Expr::Path(path) = &*call.func else {
        return false;
    };
    call.args.len() == 1 && path.qself.is_none() && macros.standard_path(&path.path, function)
}

fn unwrapped(call: &ExprMethodCall, macros: &MacroSyntax) -> bool {
    match call.method.to_string().as_str() {
        "unwrap" => call.args.is_empty(),
        "expect" => call.args.len() == 1,
        "unwrap_or_else" if call.args.len() == 1 => {
            let Expr::Closure(fallback) = &call.args[0] else {
                return false;
            };
            let Expr::Macro(panic) = transparent(&fallback.body) else {
                return false;
            };
            macros.arguments(&panic.mac, "panic").is_some()
        }
        _ => false,
    }
}
