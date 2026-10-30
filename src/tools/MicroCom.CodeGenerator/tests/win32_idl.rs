//! The interface definition files of the Windows platform backend must parse and generate Rust
//! bindings: the backend compiles three of them (`directx.idl`, `winrt.idl` and `win32.idl`, in
//! its build script) and `dcomp.idl` from the stage that uses it, so this is what holds that one
//! until then, and what says of the others how the generator reads them.

use std::path::PathBuf;

/// The bindings of a file, or `None` where the file is not there: the tests run in the repository,
/// and the package of this crate is built without the Windows backend beside it.
fn generate(file: &str) -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Windows/FerroUI.Win32").join(file);
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!("skipping: {} not found", path.display());
        return None;
    };
    let idl = microcom_codegen::parse(&src).unwrap_or_else(|e| panic!("{}:{e}", path.display()));
    Some(microcom_codegen::rust::generate(&idl, &Default::default()).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

#[test]
fn directx_idl_generates() {
    let Some(rs) = generate("direct_x/directx.idl") else { return };
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
    let Some(rs) = generate("d_composition/dcomp.idl") else { return };
    assert!(rs.contains("pub struct IDCompositionDevice {"));
}

#[test]
fn win32_idl_generates() {
    let Some(rs) = generate("win32_com/win32.idl") else { return };
    assert!(rs.contains("pub struct IDataObject {"));
    assert!(rs.contains("pub trait IDropTargetImpl: IUnknownImpl"));
}

#[test]
fn winrt_idl_generates() {
    let Some(rs) = generate("win_rt/winrt.idl") else { return };
    assert!(rs.contains("pub struct ICompositor {"));
    // A property: the method that reads it keeps its name, the one that sets it is `Set...`.
    assert!(rs.contains("pub fn set_opacity(&self"));
    // `UISettings.AnimationsEnabled` is a member of the first interface, `IUISettings`, and a
    // WinRT boolean: one byte, not a Win32 `BOOL`.
    assert!(rs.contains("pub fn animations_enabled(&self) -> Result<u8, HResult>"));
    assert!(rs.contains("const IID: Guid = Guid::from_u128(0x853616001c634627bcb13a89e0bc9c55);"));
}

/// The provider interfaces of UI Automation, written from the C# declarations of the reference.
#[test]
fn uia_idl_generates() {
    let Some(rs) = generate("automation/interop/uia.idl") else { return };
    // An interface that derives from another: its implementation trait asks for the one of the base.
    assert!(rs.contains("pub trait IRawElementProviderSimple2Impl: IRawElementProviderSimpleImpl"));
    // A structure returned through the last parameter is the result.
    assert!(rs.contains(
        "fn get_property_value(&self, property_id: i32) -> Result<crate::automation::marshalling::ComVariant, HResult>;"
    ));
    assert!(rs.contains("const IID: Guid = Guid::from_u128(0xd6dd68d186fd433286669abedea2d24c);"));
}
