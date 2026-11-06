use std::{env, fs, path::PathBuf};

fn emscripten_icu() -> String {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    for variable in ["EMCC", "EMCC_CFLAGS", "EM_CACHE", "EMSDK"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    let compiler = env::var_os("EMCC").unwrap_or_else(|| "emcc".into());
    let threaded = env::var("CARGO_CFG_TARGET_FEATURE")
        .unwrap_or_default()
        .split(',')
        .any(|feature| feature == "atomics");
    let mut flags = vec!["--use-port=icu"];
    if threaded {
        flags.push("-pthread");
    }
    let mut child = Command::new(&compiler)
        .args(&flags)
        .args(["-E", "-P", "-x", "c", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Emscripten ICU requires emcc on PATH; activate the selected Emscripten SDK");
    child.stdin.take().expect("compiler input").write_all(
        b"#include <unicode/ucol.h>\nFERROBOARD_ICU_MAJOR U_ICU_VERSION_MAJOR_NUM\nFERROBOARD_ICU_SYMBOL ucol_open\n",
    ).expect("write ICU header query");
    let output = child.wait_with_output().expect("read ICU header query");
    assert!(
        output.status.success(),
        "Emscripten ICU header query failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let header = String::from_utf8(output.stdout).expect("ICU preprocessor UTF-8");
    let value = |marker: &str| {
        header
            .lines()
            .find_map(|line| line.trim().strip_prefix(marker))
            .expect("ICU header query marker")
            .trim()
    };
    assert_eq!(
        value("FERROBOARD_ICU_MAJOR "),
        "68",
        "Bundled browser locale data requires ICU 68; update the data and its provenance when changing the port"
    );
    let suffix = value("FERROBOARD_ICU_SYMBOL ")
        .strip_prefix("ucol_open")
        .expect("ICU C API symbol");
    assert!(
        suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "Invalid ICU symbol suffix"
    );
    // Query the compiler's actual archive paths. Do not assume the user's SDK/cache location.
    // Native archive metadata travels with the rlib to independent downstream consumers.
    for library in ["icu_common", "icu_stubdata", "icu_i18n", "icu_io"] {
        let name = if threaded {
            format!("{library}-mt")
        } else {
            library.into()
        };
        let output = Command::new(&compiler)
            .args(&flags)
            .arg(format!("--print-file-name=lib{name}.a"))
            .output()
            .expect("query ICU archive");
        assert!(
            output.status.success(),
            "ICU archive query failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let path = PathBuf::from(
            String::from_utf8(output.stdout)
                .expect("ICU archive path")
                .trim(),
        );
        assert!(
            path.is_absolute() && path.is_file(),
            "Emscripten ICU archive is missing: {}",
            path.display()
        );
        println!(
            "cargo:rustc-link-search=native={}",
            path.parent().expect("ICU archive directory").display()
        );
        println!("cargo:rustc-link-lib=static={name}");
    }
    // As in FerroUI's browser configuration, applications link with em++ so the
    // compiler chooses its C++ runtime for the selected WebAssembly exception ABI.
    suffix.into()
}

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
        "emscripten" => emscripten_icu(),
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
