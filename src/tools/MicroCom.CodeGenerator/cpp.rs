//! C++ header generator (same output shape as MicroCom's C++ generator).
//!
//! [`generate`] emits the plain header: one pure virtual method per IDL method.
//! [`generate_with`] with [`CppOptions::smart_pointer_wrappers`] emits the smart pointer
//! wrapper mode of MicroCom 0.11.7 (`CppSmartPointerWrappers`): every method gets a raw
//! `impl_*` vtable slot and a non-virtual wrapper that takes and returns `ComPtr`, and every
//! interface gets a `MicroComImpl<I>` adapter whose `impl_*` thunks run the
//! `@cpp-call-preamble`, keep `this` and every interface argument alive for the call, and
//! call the `ComPtr` shaped pure virtual methods an implementation overrides:
//!
//! | IDL                        | wrapper and override                  |
//! |----------------------------|---------------------------------------|
//! | `IFoo* GetFoo()`           | `ComPtr<IFoo> GetFoo()`               |
//! | `HRESULT GetFoo(IFoo** p)` | `HRESULT GetFoo(ComPtr<IFoo>& p)`     |
//! | `HRESULT SetFoo(IFoo* f)`  | `HRESULT SetFoo(const ComPtr<IFoo>& f)` |
//! | no interface types         | unchanged                             |
//!
//! `ComPtr` and its `_microcom_attach`, `_microcom_get`, `_microcom_out` and
//! `_microcom_detach` members come from the support header the `@cpp-preamble` includes
//! (`native/FerroUI.Native/inc/comimpl.h`).

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

/// Options of the C++ generator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CppOptions {
    /// The smart pointer wrapper mode (see the module documentation).
    pub smart_pointer_wrappers: bool,
}

/// The adapter base the smart pointer mode declares once: a non-virtual primary base, so an
/// adapter never takes over the primary base of an interface (Itanium ABI).
const IMPL_BASE: &str = "template<class T> struct MicroComImpl;
#ifndef MICROCOM_IMPL_BASE_DEFINED
#define MICROCOM_IMPL_BASE_DEFINED
// Non-virtual primary base for adapters, so they never take over the primary base of an interface (Itanium ABI)
template<class T> struct MicroComImplBase
{
    virtual ~MicroComImplBase() = default;
};
#endif
";

/// The plain header.
pub fn generate(idl: &Idl) -> Result<String, String> {
    generate_with(idl, &CppOptions::default())
}

/// The header with `options`.
pub fn generate_with(idl: &Idl, options: &CppOptions) -> Result<String, String> {
    let mut out = String::new();
    let w = &mut out;
    let smart = options.smart_pointer_wrappers;

    if let Some(preamble) = idl.directive("cpp-preamble") {
        writeln!(w, "{preamble}").unwrap();
    }
    if smart {
        w.push_str(IMPL_BASE);
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
        let prefix = if smart { "impl_" } else { "" };
        for m in &i.methods {
            write!(w, "    virtual {} {}{} (", cpp_type(&m.return_type, false), prefix, m.name).unwrap();
            if m.params.is_empty() {
                writeln!(w, ") = 0;").unwrap();
            } else {
                writeln!(w).unwrap();
                for (idx, p) in m.params.iter().enumerate() {
                    let sep = if idx + 1 < m.params.len() { ", " } else { "" };
                    writeln!(w, "        {} {}{}", cpp_type(&p.ty, p.is_const()), p.name, sep).unwrap();
                }
                writeln!(w, "    ) = 0;").unwrap();
            }
            if smart {
                writeln!(w, "    {};", smart_signature(idl, m, None)).unwrap();
            }
        }
        writeln!(w, "}};").unwrap();
    }

    if smart {
        for i in &idl.interfaces {
            for m in &i.methods {
                write_wrapper(w, idl, i, m);
            }
        }
        for i in &idl.interfaces {
            write_adapter(w, idl, i)?;
        }
    }

    Ok(out)
}

/// How a parameter or a return value crosses the smart pointer wrappers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Not an interface: passed as it is.
    Plain,
    /// `IFoo*`: `const ComPtr<IFoo>&` (a return value: `ComPtr<IFoo>`).
    In,
    /// `IFoo**`: `ComPtr<IFoo>&`.
    Out,
}

/// Whether `name` is an interface: `IUnknown` or an interface of the IDL.
fn is_interface(idl: &Idl, name: &str) -> bool {
    name == "IUnknown" || idl.find_interface(name).is_some()
}

fn shape(idl: &Idl, ty: &TypeRef) -> Shape {
    if ty.reference || !is_interface(idl, &ty.name) {
        return Shape::Plain;
    }
    match ty.pointers {
        1 => Shape::In,
        2 => Shape::Out,
        _ => Shape::Plain,
    }
}

fn smart_param(idl: &Idl, p: &Param) -> String {
    match shape(idl, &p.ty) {
        Shape::Plain => format!("{} {}", cpp_type(&p.ty, p.is_const()), p.name),
        Shape::In => format!("const ComPtr<{}>& {}", p.ty.name, p.name),
        Shape::Out => format!("ComPtr<{}>& {}", p.ty.name, p.name),
    }
}

fn smart_return(idl: &Idl, m: &Method) -> String {
    match shape(idl, &m.return_type) {
        Shape::In => format!("ComPtr<{}>", m.return_type.name),
        _ => cpp_type(&m.return_type, false),
    }
}

/// `Ret Name(params)`, with `owner::` before the name when it is given.
fn smart_signature(idl: &Idl, m: &Method, owner: Option<&str>) -> String {
    let params: Vec<String> = m.params.iter().map(|p| smart_param(idl, p)).collect();
    let owner = owner.map(|o| format!("{o}::")).unwrap_or_default();
    format!("{} {}{}({})", smart_return(idl, m), owner, m.name, params.join(", "))
}

/// The non-virtual wrapper of `m` on `i`: the raw slot, called with the raw pointers.
fn write_wrapper(w: &mut String, idl: &Idl, i: &Interface, m: &Method) {
    let args: Vec<String> = m
        .params
        .iter()
        .map(|p| match shape(idl, &p.ty) {
            Shape::Plain => p.name.clone(),
            Shape::In => format!("{}._microcom_get()", p.name),
            Shape::Out => format!("{}._microcom_out()", p.name),
        })
        .collect();
    let call = format!("impl_{}({})", m.name, args.join(", "));
    writeln!(w, "inline {}\n{{", smart_signature(idl, m, Some(&i.name))).unwrap();
    if shape(idl, &m.return_type) == Shape::In {
        writeln!(w, "    ComPtr<{}> __microcom_rv;", m.return_type.name).unwrap();
        writeln!(w, "    __microcom_rv._microcom_attach({call}, true);").unwrap();
        writeln!(w, "    return __microcom_rv;").unwrap();
    } else if is_void(&m.return_type) {
        writeln!(w, "    {call};").unwrap();
    } else {
        writeln!(w, "    return {call};").unwrap();
    }
    writeln!(w, "}}").unwrap();
}

fn is_void(ty: &TypeRef) -> bool {
    ty.name == "void" && ty.pointers == 0 && !ty.reference
}

/// The methods an adapter of `i` implements: its own, and those of its bases up to the
/// first one the adapter inherits the adapter of (a `cpp-virtual-inherits` interface shares
/// its base, whose adapter implements the base methods).
fn adapter_methods<'a>(idl: &'a Idl, i: &'a Interface) -> Result<Vec<&'a Method>, String> {
    let mut methods: Vec<&Method> = i.methods.iter().collect();
    if i.cpp_virtual_inherits() {
        return Ok(methods);
    }
    let mut current = i;
    let mut depth = 0;
    while current.base_name() != "IUnknown" {
        let base = idl.find_interface(current.base_name()).ok_or_else(|| {
            format!(
                "Unable to generate MicroComImpl<{}>: base interface {} is not defined in this IDL file",
                i.name,
                current.base_name()
            )
        })?;
        methods.extend(base.methods.iter());
        current = base;
        depth += 1;
        if depth > 64 {
            return Err(format!("interface {} inherits itself", i.name));
        }
    }
    Ok(methods)
}

/// `MicroComImpl<I>`: the `ComPtr` shaped methods to override, and the final thunks of the
/// raw slots.
fn write_adapter(w: &mut String, idl: &Idl, i: &Interface) -> Result<(), String> {
    let methods = adapter_methods(idl, i)?;
    let virtual_chain = i.cpp_virtual_inherits();
    write!(w, "template<>\nstruct MicroComImpl<{}> : public MicroComImplBase<{}>, ", i.name, i.name).unwrap();
    write!(w, "public virtual {}", i.name).unwrap();
    if virtual_chain && i.base_name() != "IUnknown" {
        if idl.find_interface(i.base_name()).is_none() {
            return Err(format!(
                "Unable to generate MicroComImpl<{}>: base interface {} is not defined in this IDL file",
                i.name,
                i.base_name()
            ));
        }
        write!(w, ", public virtual MicroComImpl<{}>", i.base_name()).unwrap();
    }
    writeln!(w, "\n{{").unwrap();
    for m in &methods {
        writeln!(w, "    virtual {} = 0;", smart_signature(idl, m, None)).unwrap();
    }
    let preamble = idl.directive("cpp-call-preamble");
    for m in &methods {
        write_thunk(w, idl, i, m, preamble);
    }
    writeln!(w, "}};").unwrap();
    Ok(())
}

fn write_thunk(w: &mut String, idl: &Idl, i: &Interface, m: &Method, preamble: Option<&str>) {
    let raw: Vec<String> = m.params.iter().map(|p| format!("{} {}", cpp_type(&p.ty, p.is_const()), p.name)).collect();
    writeln!(w, "    {} impl_{}({}) final", cpp_type(&m.return_type, false), m.name, raw.join(", ")).unwrap();
    writeln!(w, "    {{").unwrap();
    if let Some(preamble) = preamble {
        writeln!(w, "        {}", preamble.trim()).unwrap();
    }
    writeln!(w, "        ComPtr<{}> __microcom_self;", i.name).unwrap();
    writeln!(w, "        __microcom_self._microcom_attach(static_cast<{}*>(this), false);", i.name).unwrap();
    let mut args = Vec::new();
    let mut outs = Vec::new();
    for p in &m.params {
        match shape(idl, &p.ty) {
            Shape::Plain => args.push(p.name.clone()),
            Shape::In => {
                writeln!(w, "        ComPtr<{}> __microcom_{};", p.ty.name, p.name).unwrap();
                writeln!(w, "        __microcom_{}._microcom_attach({}, false);", p.name, p.name).unwrap();
                args.push(format!("__microcom_{}", p.name));
            }
            Shape::Out => {
                writeln!(w, "        ComPtr<{}> __microcom_{};", p.ty.name, p.name).unwrap();
                args.push(format!("__microcom_{}", p.name));
                outs.push(p.name.clone());
            }
        }
    }
    let call = format!("{}({})", m.name, args.join(", "));
    let returns_interface = shape(idl, &m.return_type) == Shape::In;
    if outs.is_empty() {
        if returns_interface {
            writeln!(w, "        return {call}._microcom_detach();").unwrap();
        } else if is_void(&m.return_type) {
            writeln!(w, "        {call};").unwrap();
        } else {
            writeln!(w, "        return {call};").unwrap();
        }
    } else {
        let void = is_void(&m.return_type);
        if void {
            writeln!(w, "        {call};").unwrap();
        } else if returns_interface {
            writeln!(w, "        auto __microcom_r = {call}._microcom_detach();").unwrap();
        } else {
            writeln!(w, "        auto __microcom_r = {call};").unwrap();
        }
        for out in &outs {
            writeln!(w, "        if ({out}) *{out} = __microcom_{out}._microcom_detach();").unwrap();
        }
        if !void {
            writeln!(w, "        return __microcom_r;").unwrap();
        }
    }
    writeln!(w, "    }}").unwrap();
}
