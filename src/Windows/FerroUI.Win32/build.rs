//! Build script of `ferroui-win32`: runs the MicroCom generator over the
//! interface definition files whose bindings the crate compiles, and writes
//! them to `$OUT_DIR/<name>.rs` (included by the module of each file).
//!
//! The four files of the reference are in the crate (`direct_x/directx.idl`,
//! `d_composition/dcomp.idl`, `win32_com/win32.idl`, `win_rt/winrt.idl`); a
//! file is compiled from the stage that ports the structures it names and
//! the code that uses it (docs/porting/win32-platform.md, section 2.2).
//! Until then the tests of the generator hold that it still generates.

use std::path::{Path, PathBuf};
use std::{env, fs};

/// The files that are compiled: the path in the crate and the name of the
/// generated file.
const COMPILED: &[(&str, &str)] = &[("direct_x/directx.idl", "directx.rs")];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    for (idl, generated) in COMPILED {
        let idl_path = manifest_dir.join(idl);
        println!("cargo:rerun-if-changed={}", idl_path.display());
        let source = fs::read_to_string(&idl_path).unwrap_or_else(|e| panic!("{}: {e}", idl_path.display()));
        let parsed = microcom_codegen::parse(&source).unwrap_or_else(|e| panic!("{}:{e}", idl_path.display()));
        let bindings = microcom_codegen::rust::generate(&parsed, &Default::default())
            .unwrap_or_else(|e| panic!("{}: {e}", idl_path.display()));
        write_if_changed(&out_dir.join(generated), &bindings);
    }
}

fn write_if_changed(path: &Path, contents: &str) {
    if fs::read_to_string(path).is_ok_and(|existing| existing == contents) {
        return;
    }
    fs::write(path, contents).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}
