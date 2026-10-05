//! C++ header generator (same output shape as MicroCom's C++ generator).

use std::fmt::Write;

use crate::ast::*;

/// IDL type name -> C++ spelling.
fn cpp_type_name(name: &str) -> &str {
    match name {
        "uint" => "unsigned int",
        "byte" => "unsigned char",
        other => other,
    }
}

fn cpp_type(ty: &TypeRef, is_const: bool) -> String {
    let mut s = String::new();
    if is_const {
        s.push_str("const ");
    }
    s.push_str(cpp_type_name(&ty.name));
    for _ in 0..ty.pointers {
        s.push('*');
    }
    if ty.reference {
        s.push('&');
    }
    s
}

/// Splits `809c652e-7396-11d2-9771-00a0c9b4d50c` into the eleven
/// `COMINTERFACE` macro arguments.
fn uuid_macro_args(uuid: &str) -> Result<String, String> {
    let hex: String = uuid.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("invalid uuid '{uuid}'"));
    }
    let mut parts = vec![hex[0..8].to_string(), hex[8..12].to_string(), hex[12..16].to_string()];
    for i in 0..8 {
        parts.push(hex[16 + i * 2..18 + i * 2].to_string());
    }
    Ok(parts.join(", "))
}

pub fn generate(idl: &Idl) -> Result<String, String> {
    let mut out = String::new();
    let w = &mut out;

    if let Some(preamble) = idl.directive("cpp-preamble") {
        writeln!(w, "{preamble}").unwrap();
    }

    for s in &idl.structs {
        writeln!(w, "struct {};", s.name).unwrap();
    }
    for i in &idl.interfaces {
        writeln!(w, "struct {};", i.name).unwrap();
    }

    for e in &idl.enums {
        let kw = if e.is_class_enum() { "enum class" } else { "enum" };
        writeln!(w, "{kw} {}\n{{", e.name).unwrap();
        for m in &e.members {
            match &m.value {
                Some(v) => writeln!(w, "    {} = {},", m.name, v).unwrap(),
                None => writeln!(w, "    {},", m.name).unwrap(),
            }
        }
        writeln!(w, "}};").unwrap();
    }

    for s in &idl.structs {
        writeln!(w, "struct {}\n{{", s.name).unwrap();
        for f in &s.fields {
            writeln!(w, "    {} {};", cpp_type(&f.ty, false), f.name).unwrap();
        }
        writeln!(w, "}};").unwrap();
    }

    for i in &idl.interfaces {
        let uuid = i.uuid().ok_or_else(|| format!("interface {} has no uuid", i.name))?;
        let virt = if i.cpp_virtual_inherits() { "virtual " } else { "" };
        writeln!(
            w,
            "COMINTERFACE({}, {}) : {}{}\n{{",
            i.name,
            uuid_macro_args(uuid)?,
            virt,
            i.base_name()
        )
        .unwrap();
        for m in &i.methods {
            write!(w, "    virtual {} {} (", cpp_type(&m.return_type, false), m.name).unwrap();
            if m.params.is_empty() {
                writeln!(w, ") = 0;").unwrap();
                continue;
            }
            writeln!(w).unwrap();
            for (idx, p) in m.params.iter().enumerate() {
                let sep = if idx + 1 < m.params.len() { ", " } else { "" };
                writeln!(w, "        {} {}{}", cpp_type(&p.ty, p.is_const()), p.name, sep).unwrap();
            }
            writeln!(w, "    ) = 0;").unwrap();
        }
        writeln!(w, "}};").unwrap();
    }

    Ok(out)
}
