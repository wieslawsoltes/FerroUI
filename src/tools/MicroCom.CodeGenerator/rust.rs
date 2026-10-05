//! Rust bindings generator.
//!
//! For every IDL item the generated code contains:
//!
//! * enum `E` -> `#[repr(transparent)] struct E(pub i32)` with one associated
//!   constant per member (a newtype rather than a Rust `enum` because the
//!   native side may hand over flag combinations or values added later);
//! * struct `S` -> `#[repr(C)] struct S` with snake_case fields;
//! * interface `IFoo`:
//!   * `IFooVtbl` — `#[repr(C)]` table of `unsafe extern "C" fn` slots,
//!     starting with the base interface's table;
//!   * `IFoo` — `#[repr(C)]` struct holding the vtable pointer; a
//!     `*mut IFoo` is the native interface pointer, `&IFoo` derefs to the
//!     base interface and carries the snake_case proxy methods;
//!   * `IFooImpl` — the trait Rust code implements to provide the interface
//!     to native code, `IFoo::from_impl(value)` wraps an implementation into
//!     a COM object and returns the owning `ComPtr<IFoo>`.
//!
//! Signature mapping shared by proxies and `Impl` traits:
//!
//! | IDL                                   | Rust                               |
//! |---------------------------------------|------------------------------------|
//! | `HRESULT M(...)`                      | `-> Result<(), HResult>`           |
//! | `HRESULT M(..., IFoo** ppv)` (1)      | `-> Result<Option<ComPtr<IFoo>>, HResult>` |
//! | `HRESULT M(..., T* ret)` (1)          | `-> Result<T, HResult>`            |
//! | `IFoo* M()`                           | `-> Option<ComPtr<IFoo>>` (owned)  |
//! | `IFoo* p`                             | `p: Option<&IFoo>` (borrowed)      |
//! | `char* s` / `[const] char* s`         | `s: Option<&CStr>`                 |
//! | `[const] T& r`                        | `r: &T`                            |
//! | other pointers                        | raw pointers (proxy is `unsafe fn`)|
//! | `bool`                                | `bool` (C++ `bool` in the header)  |
//!
//! (1) only for a trailing parameter named `ret`, `retOut`, `ppv` or
//! `result`; every other pointer parameter is passed through unchanged.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use crate::ast::*;

#[derive(Debug, Clone)]
pub struct RustOptions {
    /// Path of the runtime crate in the generated code.
    pub runtime: String,
}

impl Default for RustOptions {
    fn default() -> Self {
        RustOptions { runtime: "::ferroui_microcom".to_string() }
    }
}

const OUT_PARAM_NAMES: &[&str] = &["ret", "retOut", "ppv", "result"];

const KEYWORDS: &[&str] = &[
    "as", "async", "await", "abstract", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in",
    "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "try", "type",
    "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// `GetClientSize` -> `get_client_size`, `texImageIOSurface2D` ->
/// `tex_image_io_surface2d`, `RootProvider_GetWindow` -> `root_provider_get_window`.
pub fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            let prev = if i > 0 { Some(chars[i - 1]) } else { None };
            let next = chars.get(i + 1).copied();
            let boundary = match prev {
                Some(p) if p.is_ascii_lowercase() => true,
                Some(p) if p.is_ascii_uppercase() => next.is_some_and(|n| n.is_ascii_lowercase()),
                _ => false,
            };
            if boundary && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    if KEYWORDS.contains(&out.as_str()) || out == "this" {
        out.push('_');
    }
    out
}

fn primitive(name: &str) -> Option<&'static str> {
    Some(match name {
        "bool" => "bool",
        "int" => "i32",
        "uint" | "unsigned int" => "u32",
        "byte" | "unsigned char" => "u8",
        "char" => "::core::ffi::c_char",
        "short" => "i16",
        "ushort" | "unsigned short" => "u16",
        "float" => "f32",
        "double" => "f64",
        "long" => "::core::ffi::c_long",
        "unsigned long" => "::core::ffi::c_ulong",
        "size_t" => "usize",
        "u_int64_t" | "uint64_t" => "u64",
        "int64_t" => "i64",
        "int32_t" => "i32",
        "uint32_t" => "u32",
        "void" => "::core::ffi::c_void",
        "HRESULT" => "RawHResult",
        _ => return None,
    })
}

struct Ctx<'a> {
    interfaces: HashSet<&'a str>,
    known: HashSet<&'a str>,
}

impl Ctx<'_> {
    fn is_interface(&self, name: &str) -> bool {
        self.interfaces.contains(name)
    }

    fn base_type(&self, name: &str) -> Result<String, String> {
        if let Some(p) = primitive(name) {
            Ok(p.to_string())
        } else if self.known.contains(name) {
            Ok(name.to_string())
        } else {
            Err(format!("unknown type '{name}'"))
        }
    }

    /// ABI-level type with `levels` pointer indirections.
    fn raw_type_n(&self, name: &str, levels: u32, is_const: bool) -> Result<String, String> {
        let mut s = String::new();
        for i in 0..levels {
            // `const T*`: constness applies to the pointee of the innermost pointer.
            s.push_str(if is_const && i + 1 == levels { "*const " } else { "*mut " });
        }
        s.push_str(&self.base_type(name)?);
        Ok(s)
    }

    fn raw_type(&self, ty: &TypeRef, is_const: bool) -> Result<String, String> {
        self.raw_type_n(&ty.name, ty.pointers + ty.reference as u32, is_const)
    }
}

/// How a parameter appears in proxies / `Impl` traits.
enum ParamKind {
    /// Passed through unchanged (values and uninterpreted raw pointers).
    Plain { raw_pointer: bool },
    Interface,
    CStr { is_const: bool },
    ConstRef,
}

enum OutKind {
    Interface(String),
    Value(String),
}

struct ParamInfo {
    name: String,
    raw_ty: String,
    hi_ty: String,
    kind: ParamKind,
}

enum RetKind {
    Unit,
    HResult,
    Interface(String),
    Plain(String),
}

struct MethodInfo<'a> {
    method: &'a Method,
    snake: String,
    params: Vec<ParamInfo>,
    /// Trailing out-parameter folded into the return value: (name, raw type, kind).
    out: Option<(String, String, OutKind)>,
    ret: RetKind,
    raw_ret: String,
    hi_ret: String,
    needs_unsafe: bool,
}

fn analyze<'a>(ctx: &Ctx, m: &'a Method) -> Result<MethodInfo<'a>, String> {
    let rt = &m.return_type;
    if rt.reference {
        return Err("reference return types are not supported".into());
    }
    let ret = if rt.pointers == 0 && rt.name == "void" {
        RetKind::Unit
    } else if rt.pointers == 0 && rt.name == "HRESULT" {
        RetKind::HResult
    } else if rt.pointers == 1 && ctx.is_interface(&rt.name) {
        RetKind::Interface(rt.name.clone())
    } else {
        RetKind::Plain(ctx.raw_type(rt, false)?)
    };
    let raw_ret = match &ret {
        RetKind::Unit => String::new(),
        RetKind::HResult => " -> RawHResult".to_string(),
        RetKind::Interface(n) => format!(" -> *mut {n}"),
        RetKind::Plain(t) => format!(" -> {t}"),
    };

    let mut params = Vec::new();
    let mut out = None;
    for (idx, p) in m.params.iter().enumerate() {
        let name = snake_case(&p.name);
        let is_const = p.is_const();
        let raw_ty = ctx.raw_type(&p.ty, is_const)?;
        let ty = &p.ty;
        let is_last = idx + 1 == m.params.len();

        if is_last
            && matches!(ret, RetKind::HResult)
            && OUT_PARAM_NAMES.contains(&p.name.as_str())
            && ty.pointers >= 1
            && !ty.reference
            && !is_const
        {
            let kind = if ctx.is_interface(&ty.name) {
                (ty.pointers == 2).then(|| OutKind::Interface(ty.name.clone()))
            } else if ty.pointers == 1 && (ty.name == "void" || ty.name == "char") {
                None
            } else {
                Some(OutKind::Value(ctx.raw_type_n(&ty.name, ty.pointers - 1, false)?))
            };
            if let Some(kind) = kind {
                out = Some((name, raw_ty, kind));
                continue;
            }
        }

        let (kind, hi_ty) = if ty.reference {
            if !is_const || ty.pointers != 0 {
                return Err(format!("{}: only '[const] T&' references are supported", m.name));
            }
            (ParamKind::ConstRef, format!("&{}", ctx.base_type(&ty.name)?))
        } else if ty.pointers == 1 && ctx.is_interface(&ty.name) {
            (ParamKind::Interface, format!("Option<&{}>", ty.name))
        } else if ty.pointers == 1 && ty.name == "char" {
            (ParamKind::CStr { is_const }, "Option<&::core::ffi::CStr>".to_string())
        } else {
            (ParamKind::Plain { raw_pointer: ty.pointers > 0 }, raw_ty.clone())
        };
        params.push(ParamInfo { name, raw_ty, hi_ty, kind });
    }

    let hi_ret = match (&ret, &out) {
        (RetKind::Unit, _) => String::new(),
        (RetKind::HResult, None) => " -> Result<(), HResult>".to_string(),
        (RetKind::HResult, Some((_, _, OutKind::Interface(n)))) => {
            format!(" -> Result<Option<ComPtr<{n}>>, HResult>")
        }
        (RetKind::HResult, Some((_, _, OutKind::Value(t)))) => format!(" -> Result<{t}, HResult>"),
        (RetKind::Interface(n), _) => format!(" -> Option<ComPtr<{n}>>"),
        (RetKind::Plain(t), _) => format!(" -> {t}"),
    };
    let needs_unsafe =
        params.iter().any(|p| matches!(p.kind, ParamKind::Plain { raw_pointer: true }));

    Ok(MethodInfo { method: m, snake: snake_case(&m.name), params, out, ret, raw_ret, hi_ret, needs_unsafe })
}

impl MethodInfo<'_> {
    fn hi_params(&self) -> String {
        self.params.iter().map(|p| format!(", {}: {}", p.name, p.hi_ty)).collect()
    }

    fn raw_params(&self) -> String {
        let mut s: String = self.params.iter().map(|p| format!(", {}: {}", p.name, p.raw_ty)).collect();
        if let Some((name, raw, _)) = &self.out {
            write!(s, ", {name}: {raw}").unwrap();
        }
        s
    }

    fn forward_args(&self) -> String {
        self.params.iter().map(|p| format!(", {}", p.name)).collect()
    }
}

/// Evaluates the small expression language used for enum values
/// (integer literals, references to earlier members, `|`, `<<`, unary `-`).
fn eval_enum_expr(expr: &str, known: &HashMap<String, i64>) -> Option<i64> {
    let mut acc = 0i64;
    for term in expr.split('|') {
        let mut parts = term.split("<<");
        let mut v = eval_atom(parts.next()?.trim(), known)?;
        for shift in parts {
            v = v.checked_shl(u32::try_from(eval_atom(shift.trim(), known)?).ok()?)?;
        }
        acc |= v;
    }
    Some(acc)
}

fn eval_atom(s: &str, known: &HashMap<String, i64>) -> Option<i64> {
    let s = s.trim_start_matches('(').trim_end_matches(')').trim();
    if let Some(rest) = s.strip_prefix('-') {
        return eval_atom(rest, known).map(|v| -v);
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).ok();
    }
    s.parse::<i64>().ok().or_else(|| known.get(s).copied())
}

fn parse_uuid(uuid: &str) -> Result<u128, String> {
    let hex: String = uuid.chars().filter(|c| *c != '-').collect();
    if hex.len() != 32 {
        return Err(format!("invalid uuid '{uuid}'"));
    }
    u128::from_str_radix(&hex, 16).map_err(|_| format!("invalid uuid '{uuid}'"))
}

pub fn generate(idl: &Idl, options: &RustOptions) -> Result<String, String> {
    let mut interfaces: HashSet<&str> = idl.interfaces.iter().map(|i| i.name.as_str()).collect();
    interfaces.insert("IUnknown");
    let mut known = interfaces.clone();
    known.extend(idl.enums.iter().map(|e| e.name.as_str()));
    known.extend(idl.structs.iter().map(|s| s.name.as_str()));
    let ctx = Ctx { interfaces, known };

    let mut out = String::new();
    let w = &mut out;
    let rt = &options.runtime;

    writeln!(w, "// @generated by microcom-codegen. Do not edit.\n").unwrap();
    writeln!(
        w,
        "#[allow(unused_imports)]\nuse {rt}::{{make_com, ComObject, ComPtr, Guid, HResult, IUnknown, IUnknownImpl, IUnknownVtbl, ImplementedBy, Interface, RawHResult, S_OK}};\n"
    )
    .unwrap();

    for e in &idl.enums {
        gen_enum(w, e)?;
    }
    for s in &idl.structs {
        writeln!(w, "#[repr(C)]\n#[derive(Clone, Copy, Debug, PartialEq)]\npub struct {} {{", s.name).unwrap();
        for f in &s.fields {
            if f.ty.reference {
                return Err(format!("{}.{}: reference fields are not supported", s.name, f.name));
            }
            writeln!(w, "    pub {}: {},", snake_case(&f.name), ctx.raw_type(&f.ty, false)?).unwrap();
        }
        writeln!(w, "}}\n").unwrap();
    }
    for i in &idl.interfaces {
        gen_interface(w, &ctx, i).map_err(|e| format!("{}: {e}", i.name))?;
    }
    Ok(out)
}

fn gen_enum(w: &mut String, e: &Enum) -> Result<(), String> {
    writeln!(
        w,
        "#[repr(transparent)]\n#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]\npub struct {}(pub i32);\n",
        e.name
    )
    .unwrap();
    writeln!(w, "#[allow(non_upper_case_globals)]\nimpl {} {{", e.name).unwrap();
    let mut known: HashMap<String, i64> = HashMap::new();
    let mut next = 0i64;
    let mut names: Vec<(i64, &str)> = Vec::new();
    let mut seen = HashSet::new();
    for m in &e.members {
        let value = match &m.value {
            Some(expr) => eval_enum_expr(expr, &known)
                .ok_or_else(|| format!("{}::{}: cannot evaluate '{expr}'", e.name, m.name))?,
            None => next,
        };
        if i32::try_from(value).is_err() {
            return Err(format!("{}::{}: value {value} does not fit in int", e.name, m.name));
        }
        next = value + 1;
        known.insert(m.name.clone(), value);
        if seen.insert(value) {
            names.push((value, &m.name));
        }
        writeln!(w, "    pub const {}: Self = Self({value});", m.name).unwrap();
    }
    writeln!(w, "\n    /// Name of the (first) member with this value, if any.").unwrap();
    writeln!(w, "    pub fn name(self) -> Option<&'static str> {{\n        Some(match self.0 {{").unwrap();
    for (v, n) in &names {
        writeln!(w, "            {v} => \"{n}\",").unwrap();
    }
    writeln!(w, "            _ => return None,\n        }})\n    }}").unwrap();
    writeln!(w, "\n    /// `true` if all bits of `other` are set in `self`.").unwrap();
    writeln!(w, "    pub const fn contains(self, other: Self) -> bool {{\n        self.0 & other.0 == other.0\n    }}\n}}\n").unwrap();
    let n = &e.name;
    writeln!(
        w,
        "impl ::core::fmt::Debug for {n} {{\n    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{\n        match self.name() {{\n            Some(n) => f.write_str(n),\n            None => write!(f, \"{n}({{}})\", self.0),\n        }}\n    }}\n}}\n"
    )
    .unwrap();
    writeln!(
        w,
        "impl ::core::ops::BitOr for {n} {{\n    type Output = Self;\n    fn bitor(self, rhs: Self) -> Self {{\n        Self(self.0 | rhs.0)\n    }}\n}}\n\nimpl ::core::ops::BitAnd for {n} {{\n    type Output = Self;\n    fn bitand(self, rhs: Self) -> Self {{\n        Self(self.0 & rhs.0)\n    }}\n}}\n"
    )
    .unwrap();
    Ok(())
}

fn gen_interface(w: &mut String, ctx: &Ctx, i: &Interface) -> Result<(), String> {
    let name = &i.name;
    let base = i.base_name();
    if !ctx.is_interface(base) {
        return Err(format!("unknown base interface '{base}'"));
    }
    let uuid = parse_uuid(i.uuid().ok_or("missing uuid")?)?;
    let mut seen = HashSet::new();
    let mut methods = Vec::new();
    for m in &i.methods {
        if !seen.insert(m.name.as_str()) {
            return Err(format!("overloaded method '{}' is not supported", m.name));
        }
        methods.push(analyze(ctx, m).map_err(|e| format!("{}: {e}", m.name))?);
    }
    let this = "self as *const Self as *mut ::core::ffi::c_void";

    // --- vtable -----------------------------------------------------------
    writeln!(w, "#[repr(C)]\n#[allow(non_snake_case)]\npub struct {name}Vtbl {{\n    pub base: {base}Vtbl,").unwrap();
    for m in &methods {
        writeln!(
            w,
            "    pub {}: unsafe extern \"C\" fn(this: *mut ::core::ffi::c_void{}){},",
            m.method.name,
            m.raw_params(),
            m.raw_ret
        )
        .unwrap();
    }
    writeln!(w, "}}\n").unwrap();

    // --- interface type ---------------------------------------------------
    writeln!(w, "#[repr(C)]\npub struct {name} {{\n    vtbl: *const {name}Vtbl,\n}}\n").unwrap();
    writeln!(
        w,
        "unsafe impl Interface for {name} {{\n    const IID: Guid = Guid::from_u128(0x{uuid:032x});\n    const NAME: &'static str = \"{name}\";\n    type Vtbl = {name}Vtbl;\n\n    fn matches_iid(iid: &Guid) -> bool {{\n        *iid == Self::IID\n            || *iid == Self::IID.as_com_h_materialized()\n            || <{base} as Interface>::matches_iid(iid)\n    }}\n}}\n"
    )
    .unwrap();
    writeln!(
        w,
        "impl ::core::ops::Deref for {name} {{\n    type Target = {base};\n    #[inline]\n    fn deref(&self) -> &{base} {{\n        unsafe {{ &*(self as *const Self as *const {base}) }}\n    }}\n}}\n"
    )
    .unwrap();

    // --- proxy methods ----------------------------------------------------
    writeln!(w, "impl {name} {{").unwrap();
    writeln!(
        w,
        "    /// Wraps a Rust implementation into a COM object native code can call.\n    pub fn from_impl<T: {name}Impl + 'static>(value: T) -> ComPtr<{name}> {{\n        make_com(value)\n    }}"
    )
    .unwrap();
    for m in &methods {
        let unsafe_kw = if m.needs_unsafe { "unsafe " } else { "" };
        writeln!(w, "\n    /// `{}::{}`", name, m.method.name).unwrap();
        if m.needs_unsafe {
            writeln!(w, "    ///\n    /// # Safety\n    /// Raw pointer arguments must satisfy the native method's contract.").unwrap();
        }
        writeln!(w, "    pub {unsafe_kw}fn {}(&self{}){} {{", m.snake, m.hi_params(), m.hi_ret).unwrap();
        let mut args = String::new();
        for p in &m.params {
            let n = &p.name;
            match &p.kind {
                ParamKind::Plain { .. } => write!(args, ", {n}").unwrap(),
                ParamKind::Interface => {
                    write!(args, ", {n}.map_or(::core::ptr::null_mut(), |p| p.as_raw())").unwrap()
                }
                ParamKind::CStr { is_const: true } => {
                    write!(args, ", {n}.map_or(::core::ptr::null(), |s| s.as_ptr())").unwrap()
                }
                ParamKind::CStr { is_const: false } => {
                    write!(args, ", {n}.map_or(::core::ptr::null_mut(), |s| s.as_ptr() as *mut _)").unwrap()
                }
                ParamKind::ConstRef => write!(args, ", {n} as *const _").unwrap(),
            }
        }
        let call = |extra: &str| format!("((*self.vtbl).{})({this}{args}{extra})", m.method.name);
        writeln!(w, "        unsafe {{").unwrap();
        match (&m.ret, &m.out) {
            (RetKind::Unit, _) => writeln!(w, "            {}", call("")).unwrap(),
            (RetKind::HResult, None) => writeln!(w, "            HResult::check({})", call("")).unwrap(),
            (RetKind::HResult, Some((_, _, OutKind::Interface(n)))) => {
                writeln!(w, "            let mut __out: *mut {n} = ::core::ptr::null_mut();").unwrap();
                writeln!(w, "            HResult::check({})?;", call(", &mut __out")).unwrap();
                writeln!(w, "            Ok(ComPtr::from_raw(__out))").unwrap();
            }
            (RetKind::HResult, Some((_, _, OutKind::Value(t)))) => {
                writeln!(w, "            let mut __out = ::core::mem::MaybeUninit::<{t}>::zeroed();").unwrap();
                writeln!(w, "            HResult::check({})?;", call(", __out.as_mut_ptr()")).unwrap();
                writeln!(w, "            Ok(__out.assume_init())").unwrap();
            }
            (RetKind::Interface(_), _) => writeln!(w, "            ComPtr::from_raw({})", call("")).unwrap(),
            (RetKind::Plain(_), _) => writeln!(w, "            {}", call("")).unwrap(),
        }
        writeln!(w, "        }}\n    }}").unwrap();
    }
    writeln!(w, "}}\n").unwrap();

    // --- implementation trait ----------------------------------------------
    writeln!(w, "/// Implement this to provide `{name}` to native code (see [`{name}::from_impl`]).").unwrap();
    writeln!(w, "pub trait {name}Impl: {base}Impl {{").unwrap();
    for m in &methods {
        writeln!(w, "    fn {}(&self{}){};", m.snake, m.hi_params(), m.hi_ret).unwrap();
    }
    writeln!(w, "}}\n").unwrap();
    for wrapper in ["::std::rc::Rc", "::std::boxed::Box"] {
        writeln!(w, "impl<T: {name}Impl + ?Sized> {name}Impl for {wrapper}<T> {{").unwrap();
        for m in &methods {
            writeln!(
                w,
                "    #[inline]\n    fn {}(&self{}){} {{\n        <T as {name}Impl>::{}(&**self{})\n    }}",
                m.snake,
                m.hi_params(),
                m.hi_ret,
                m.snake,
                m.forward_args()
            )
            .unwrap();
        }
        writeln!(w, "}}\n").unwrap();
    }

    // --- thunks + vtable constructor ----------------------------------------
    writeln!(w, "#[allow(non_snake_case)]\nmod {name}_thunks {{\n    #[allow(unused_imports)]\n    use super::*;").unwrap();
    for m in &methods {
        writeln!(
            w,
            "\n    pub unsafe extern \"C\" fn {}<T: {name}Impl>(__this: *mut ::core::ffi::c_void{}){} {{",
            m.method.name,
            m.raw_params(),
            m.raw_ret
        )
        .unwrap();
        let mut args = String::new();
        for p in &m.params {
            let n = &p.name;
            match &p.kind {
                ParamKind::Plain { .. } => write!(args, ", {n}").unwrap(),
                ParamKind::Interface => write!(args, ", {n}.as_ref()").unwrap(),
                ParamKind::CStr { .. } => write!(
                    args,
                    ", if {n}.is_null() {{ None }} else {{ Some(::core::ffi::CStr::from_ptr({n})) }}"
                )
                .unwrap(),
                ParamKind::ConstRef => write!(args, ", &*{n}").unwrap(),
            }
        }
        let call = format!("<T as {name}Impl>::{}(ComObject::<T>::value(__this){args})", m.snake);
        match (&m.ret, &m.out) {
            (RetKind::Unit, _) | (RetKind::Plain(_), _) => writeln!(w, "        {call}").unwrap(),
            (RetKind::HResult, None) => writeln!(w, "        HResult::from_result({call})").unwrap(),
            (RetKind::HResult, Some((n, _, kind))) => {
                let conv = match kind {
                    OutKind::Interface(_) => "__v.map_or(::core::ptr::null_mut(), ComPtr::into_raw)",
                    OutKind::Value(_) => "__v",
                };
                writeln!(
                    w,
                    "        match {call} {{\n            Ok(__v) => {{\n                if !{n}.is_null() {{\n                    *{n} = {conv};\n                }}\n                S_OK\n            }}\n            Err(__e) => __e.0,\n        }}"
                )
                .unwrap();
            }
            (RetKind::Interface(_), _) => {
                writeln!(w, "        {call}.map_or(::core::ptr::null_mut(), ComPtr::into_raw)").unwrap()
            }
        }
        writeln!(w, "    }}").unwrap();
    }
    writeln!(w, "}}\n").unwrap();

    writeln!(
        w,
        "impl {name}Vtbl {{\n    /// Vtable for a `ComObject<T>` whose most-derived interface is `I`.\n    pub const fn new<I: Interface, T: {name}Impl>() -> Self {{\n        Self {{\n            base: {base}Vtbl::new::<I, T>(),"
    )
    .unwrap();
    for m in &methods {
        writeln!(w, "            {0}: {name}_thunks::{0}::<T>,", m.method.name).unwrap();
    }
    writeln!(w, "        }}\n    }}\n}}\n").unwrap();
    writeln!(
        w,
        "unsafe impl<T: {name}Impl> ImplementedBy<T> for {name} {{\n    const VTBL: &'static {name}Vtbl = &{name}Vtbl::new::<{name}, T>();\n}}\n"
    )
    .unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake() {
        assert_eq!(snake_case("GetClientSize"), "get_client_size");
        assert_eq!(snake_case("texImageIOSurface2D"), "tex_image_io_surface2d");
        assert_eq!(snake_case("RootProvider_GetWindow"), "root_provider_get_window");
        assert_eq!(snake_case("GetIOKitRegistryId"), "get_io_kit_registry_id");
        assert_eq!(snake_case("ObtainNSViewHandle"), "obtain_ns_view_handle");
        assert_eq!(snake_case("type"), "type_");
        assert_eq!(snake_case("retOut"), "ret_out");
    }

    #[test]
    fn enum_values() {
        let mut known = HashMap::new();
        known.insert("A".to_string(), 1);
        known.insert("B".to_string(), 4);
        assert_eq!(eval_enum_expr("-1", &known), Some(-1));
        assert_eq!(eval_enum_expr("0x10", &known), Some(16));
        assert_eq!(eval_enum_expr("A | B", &known), Some(5));
        assert_eq!(eval_enum_expr("1 << 3", &known), Some(8));
        assert_eq!(eval_enum_expr("nope", &known), None);
    }

    #[test]
    fn signatures() {
        let idl = crate::parse(
            r#"
            struct S { int A; }
            [uuid(00000000-0000-0000-0000-000000000001)]
            interface IA : IUnknown {
                HRESULT Get(int index, IA** ppv);
                HRESULT Size(S* ret);
                HRESULT SetFrame(S* fb);
                IA* Parent();
                bool Key(int type, [const] char* text, [const] S& s, IA* other);
            }"#,
        )
        .unwrap();
        let rs = generate(&idl, &RustOptions::default()).unwrap();
        assert!(rs.contains("pub fn get(&self, index: i32) -> Result<Option<ComPtr<IA>>, HResult>"));
        assert!(rs.contains("pub fn size(&self) -> Result<S, HResult>"));
        assert!(rs.contains("pub unsafe fn set_frame(&self, fb: *mut S) -> Result<(), HResult>"));
        assert!(rs.contains("pub fn parent(&self) -> Option<ComPtr<IA>>"));
        assert!(rs.contains(
            "pub fn key(&self, type_: i32, text: Option<&::core::ffi::CStr>, s: &S, other: Option<&IA>) -> bool"
        ));
        assert!(rs.contains(
            "pub Key: unsafe extern \"C\" fn(this: *mut ::core::ffi::c_void, type_: i32, text: *const ::core::ffi::c_char, s: *const S, other: *mut IA) -> bool,"
        ));
    }
}
