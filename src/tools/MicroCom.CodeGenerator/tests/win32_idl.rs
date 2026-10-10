//! The interface definition files of the Windows platform backend must parse and generate Rust
//! bindings: the backend compiles one of them today (`directx.idl`, in its build script) and the
//! others from the stages that use them, so this is what holds the other three until then.

use std::path::PathBuf;

fn generate(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Windows/FerroUI.Win32").join(file);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let idl = microcom_codegen::parse(&src).unwrap_or_else(|e| panic!("{}:{e}", path.display()));
    microcom_codegen::rust::generate(&idl, &Default::default()).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn directx_idl_generates() {
    let rs = generate("direct_x/directx.idl");
    // An out parameter marked as the return value is the result of the proxy.
    assert!(rs.contains("pub fn get_desc1(&self) -> Result<DXGI_ADAPTER_DESC1, HResult>"));
    assert!(rs.contains(
        "pub unsafe fn enum_adapters1(&self, adapter: u32, pp_adapter: *mut *mut ::core::ffi::c_void) -> i32"
    ));
    assert!(rs.contains("pub fn get_device_removed_reason(&self) -> i32"));
    assert!(rs.contains("const IID: Guid = Guid::from_u128(0xdb6f6ddbac774e888253819df9bbf140);"));
    assert!(rs.contains("unsafe extern \"system\" fn"));
    assert!(!rs.contains("extern \"C\""));
}

#[test]
fn dcomp_idl_generates() {
    let rs = generate("d_composition/dcomp.idl");
    assert!(rs.contains("pub struct IDCompositionDevice {"));
}

#[test]
fn win32_idl_generates() {
    let rs = generate("win32_com/win32.idl");
    assert!(rs.contains("pub struct IDataObject {"));
    assert!(rs.contains("pub trait IDropTargetImpl: IUnknownImpl"));
}

#[test]
fn winrt_idl_generates() {
    let rs = generate("win_rt/winrt.idl");
    assert!(rs.contains("pub struct ICompositor {"));
    // A property: the method that reads it keeps its name, the one that sets it is `Set...`.
    assert!(rs.contains("pub fn set_opacity(&self"));
}
