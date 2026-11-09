use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=text/locale_text/icu_ffi.rs.in");
    let target = env::var("CARGO_CFG_TARGET_OS").expect("Cargo target OS");
    let suffix = match target.as_str() {
        "macos" => {
            println!("cargo:rustc-link-lib=icucore");
            String::new()
        }
        "linux" => {
            // Native ICU's own pkg-config metadata supplies both its libraries and ABI suffix.
            // Missing ICU is a build error, never an ordinal-comparison fallback.
            let icu = pkg_config::Config::new()
                .atleast_version("60")
                .probe("icu-i18n")
                .expect(
                    "FerroBoard locale sorting/search requires ICU development files (libicu-dev)",
                );
            let major: u16 = icu
                .version
                .split('.')
                .next()
                .unwrap()
                .parse()
                .expect("ICU major version");
            format!("_{major}")
        }
        _ => return,
    };
    let declarations = fs::read_to_string("text/locale_text/icu_ffi.rs.in")
        .expect("native ICU declarations")
        .replace("@SUFFIX@", &suffix);
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"))
            .join("locale_icu.rs"),
        declarations,
    )
    .expect("write native ICU declarations");
}
