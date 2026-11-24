//! The smart pointer wrapper mode of the C++ generator (MicroCom 0.11.7
//! `CppSmartPointerWrappers`): raw `impl_*` slots, `ComPtr` wrappers, and the
//! `MicroComImpl<I>` adapters with the call preamble.

use microcom_codegen::cpp::{generate_with, CppOptions};

const IDL: &str = r#"
@cpp-preamble @@
#pragma once
#include "comimpl.h"
@@
@cpp-call-preamble START_ARP_CALL;

[uuid(00000000-0000-0000-0000-000000000001)]
interface IFoo : IUnknown
{
    int Count();
}

[uuid(00000000-0000-0000-0000-000000000002)]
interface IBar : IUnknown
{
    IFoo* GetFoo();
    HRESULT GetFooOut(IFoo** ppv);
    HRESULT SetFoo(IFoo* foo, int flags);
    void Notify(IFoo* foo);
    HRESULT Context(IUnknown** ppv);
}

[uuid(00000000-0000-0000-0000-000000000003)]
interface IBarEvents : IBar
{
    void Changed([const] char* name);
}

[uuid(00000000-0000-0000-0000-000000000004), cpp-virtual-inherits]
interface IBaz : IBar
{
    void Run();
}
"#;

fn header() -> String {
    let idl = microcom_codegen::parse(IDL).expect("parse");
    generate_with(&idl, &CppOptions { smart_pointer_wrappers: true }).expect("generate")
}

#[test]
fn every_method_has_a_raw_slot_and_a_wrapper() {
    let h = header();
    assert!(h.contains("    virtual int impl_Count () = 0;\n    int Count();"), "{h}");
    assert!(h.contains("    ComPtr<IFoo> GetFoo();"), "{h}");
    assert!(h.contains("    HRESULT GetFooOut(ComPtr<IFoo>& ppv);"), "{h}");
    assert!(h.contains("    HRESULT SetFoo(const ComPtr<IFoo>& foo, int flags);"), "{h}");
    assert!(h.contains("    HRESULT Context(ComPtr<IUnknown>& ppv);"), "{h}");
    assert!(h.contains("    void Changed(const char* name);"), "{h}");
}

#[test]
fn wrappers_pass_raw_pointers_and_take_returned_references() {
    let h = header();
    assert!(h.contains(
        "inline ComPtr<IFoo> IBar::GetFoo()\n{\n    ComPtr<IFoo> __microcom_rv;\n    \
         __microcom_rv._microcom_attach(impl_GetFoo(), true);\n    return __microcom_rv;\n}"
    ), "{h}");
    assert!(h.contains("    return impl_GetFooOut(ppv._microcom_out());"), "{h}");
    assert!(h.contains("    return impl_SetFoo(foo._microcom_get(), flags);"), "{h}");
    assert!(
        h.contains("inline void IBar::Notify(const ComPtr<IFoo>& foo)\n{\n    impl_Notify(foo._microcom_get());\n}"),
        "{h}"
    );
}

#[test]
fn adapters_keep_this_and_the_arguments_alive_and_run_the_call_preamble() {
    let h = header();
    assert!(
        h.contains("template<>\nstruct MicroComImpl<IBar> : public MicroComImplBase<IBar>, public virtual IBar\n{"),
        "{h}"
    );
    assert!(h.contains("    virtual HRESULT SetFoo(const ComPtr<IFoo>& foo, int flags) = 0;"), "{h}");
    assert!(h.contains(
        "    HRESULT impl_SetFoo(IFoo* foo, int flags) final\n    {\n        START_ARP_CALL;\n        \
         ComPtr<IBar> __microcom_self;\n        \
         __microcom_self._microcom_attach(static_cast<IBar*>(this), false);\n        \
         ComPtr<IFoo> __microcom_foo;\n        __microcom_foo._microcom_attach(foo, false);\n        \
         return SetFoo(__microcom_foo, flags);\n    }"
    ), "{h}");
    // An out parameter is handed back as an owned reference.
    assert!(h.contains(
        "        ComPtr<IFoo> __microcom_ppv;\n        auto __microcom_r = GetFooOut(__microcom_ppv);\n        \
         if (ppv) *ppv = __microcom_ppv._microcom_detach();\n        return __microcom_r;"
    ), "{h}");
    // A returned interface is handed back as an owned reference.
    assert!(h.contains("        return GetFoo()._microcom_detach();"), "{h}");
}

#[test]
fn a_plain_derived_interface_implements_its_bases_and_a_virtual_one_shares_them() {
    let h = header();
    let events = h.split("struct MicroComImpl<IBarEvents>").nth(1).expect("adapter of IBarEvents");
    let events = &events[..events.find("};").unwrap()];
    assert!(events.contains("impl_Changed(") && events.contains("impl_GetFoo()"), "{events}");
    assert!(h.contains(
        "struct MicroComImpl<IBaz> : public MicroComImplBase<IBaz>, public virtual IBaz, \
         public virtual MicroComImpl<IBar>"
    ), "{h}");
    let baz = h.split("struct MicroComImpl<IBaz>").nth(1).unwrap();
    let baz = &baz[..baz.find("};").unwrap()];
    assert!(baz.contains("impl_Run()") && !baz.contains("impl_GetFoo"), "{baz}");
}

#[test]
fn the_plain_mode_is_unchanged() {
    let idl = microcom_codegen::parse(IDL).expect("parse");
    let h = microcom_codegen::cpp::generate(&idl).expect("generate");
    assert!(h.contains("    virtual IFoo* GetFoo () = 0;"), "{h}");
    assert!(!h.contains("MicroComImpl") && !h.contains("impl_"), "{h}");
}
