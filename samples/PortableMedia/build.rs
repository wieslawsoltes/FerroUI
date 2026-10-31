fn main() {
    println!("cargo:rerun-if-changed=Info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=../PortableMedia.Mobile/plane.m");
        cc::Build::new()
            .file("../PortableMedia.Mobile/plane.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("portable_media_plane");
        println!("cargo:rustc-link-lib=framework=Metal");
        let path = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("Info.plist");
        println!(
            "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            path.display()
        );
    }
}
