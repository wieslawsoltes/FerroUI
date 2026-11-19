//! The C header is generated from the sources of the crate, never written by hand: this test
//! writes it with cbindgen and compares it with `include/ferroui.h`. With
//! `FERROUI_CAPI_WRITE_HEADER=1` it writes the file instead.

use std::path::Path;

fn generated() -> String {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")).expect("cbindgen.toml");
    let bindings = cbindgen::Builder::new()
        .with_config(config)
        .with_src(crate_dir.join("lib.rs"))
        .generate()
        .expect("cbindgen reads the sources");
    let mut header = Vec::new();
    bindings.write(&mut header);
    String::from_utf8(header).expect("UTF-8")
}

#[test]
fn the_header_is_up_to_date() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("include").join("ferroui.h");
    let header = generated();
    if std::env::var_os("FERROUI_CAPI_WRITE_HEADER").is_some() {
        std::fs::write(&path, &header).expect("the header is written");
        return;
    }
    let current = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
    assert!(
        current == header,
        "include/ferroui.h is not the header of the sources: write it again with \
         FERROUI_CAPI_WRITE_HEADER=1 cargo test -p ferroui-capi --lib header"
    );
}

#[test]
fn the_header_declares_every_export() {
    let header = generated();
    for name in [
        "ferro_abi_version",
        "ferro_init",
        "ferro_last_error",
        "ferro_type_id",
        "ferro_property_id",
        "ferro_event_id",
        "ferro_create",
        "ferro_release",
        "ferro_set_str",
        "ferro_set_f64",
        "ferro_set_i64",
        "ferro_set_bool",
        "ferro_set_handle",
        "ferro_get_str",
        "ferro_add_child",
        "ferro_subscribe",
        "ferro_post",
        "ferro_app_run",
        "ferro_show",
        "ferro_api_fingerprint",
        "ferro_frame_open",
        "ferro_frame_commit",
        "ferro_frame_commit_bytes",
        "ferro_frame_dispatcher",
    ] {
        assert!(header.contains(&format!("{name}(")), "the header declares {name}");
    }
    for name in ["typedef uint64_t FerroHandle;", "typedef int32_t FerroStatus;", "#define FERRO_WRONG_THREAD -7"] {
        assert!(header.contains(name), "the header has {name}");
    }
}
