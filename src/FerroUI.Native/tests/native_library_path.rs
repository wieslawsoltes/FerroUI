//! The native backend built as a library of its own, loaded through
//! `FerroNativePlatformOptions::ferro_native_library_path` (#1049).
//!
//! The objects of the backend are compiled with hidden visibility, so the factory has to stay
//! exported for this path to work. The test links the static libraries of this build into a dynamic
//! library, checks that it exports `CreateFerroNative`, initializes the platform through the
//! option, and checks that the factory the platform holds comes from that library. A second library
//! linked without the export shows that the check can fail.

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("native_library_path: macOS only, skipped");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_microcom::ComPtr;
    use ferroui_native::interop::IFerroNativeFactory;
    use ferroui_native::{FerroNativePlatformExtensions, FerroNativePlatformOptions, FerroNativeRenderingMode};
    use ferroui_skia::SkiaApplicationExtensions;
    use std::ffi::{c_char, c_int, c_void, CStr, CString};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::rc::Rc;

    /// Where the build script of the crate left the static libraries of the backend.
    const OUT_DIR: &str = env!("OUT_DIR");
    const ARCHIVES: [&str; 2] = ["libferro_native_osx.a", "libferro_native_osx_noarc.a"];
    /// The frameworks the build script links the backend with.
    const FRAMEWORKS: [&str; 12] = [
        "AppKit",
        "Foundation",
        "CoreFoundation",
        "CoreGraphics",
        "CoreVideo",
        "QuartzCore",
        "IOSurface",
        "IOKit",
        "OpenGL",
        "Carbon",
        "Metal",
        "UniformTypeIdentifiers",
    ];
    const FACTORY: &str = "_CreateFerroNative";
    const RTLD_NOW: c_int = 0x2;
    const RTLD_NOLOAD: c_int = 0x10;

    #[repr(C)]
    struct DlInfo {
        file_name: *const c_char,
        file_base: *mut c_void,
        symbol_name: *const c_char,
        symbol_address: *mut c_void,
    }

    extern "C" {
        fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dladdr(address: *const c_void, info: *mut DlInfo) -> c_int;
    }

    fn link(directory: &Path, name: &str, options: &[&str]) -> PathBuf {
        let output = directory.join(name);
        let mut command = Command::new("clang++");
        command.args(["-dynamiclib", "-fobjc-link-runtime", "-stdlib=libc++", "-o"]).arg(&output);
        for archive in ARCHIVES {
            command.arg(format!("-Wl,-force_load,{OUT_DIR}/{archive}"));
        }
        for framework in FRAMEWORKS {
            command.args(["-framework", framework]);
        }
        command.args(options);
        let result = command.output().expect("clang++ runs");
        assert!(result.status.success(), "the link failed: {}", String::from_utf8_lossy(&result.stderr));
        output
    }

    /// The names the library exports, from its own table.
    fn exports(library: &Path) -> Vec<String> {
        let result = Command::new("nm").arg("-gU").arg(library).output().expect("nm runs");
        assert!(result.status.success(), "nm failed: {}", String::from_utf8_lossy(&result.stderr));
        String::from_utf8_lossy(&result.stdout)
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2).map(str::to_owned))
            .collect()
    }

    fn c_path(path: &Path) -> CString {
        CString::new(path.to_str().expect("a UTF-8 path")).expect("no NUL in the path")
    }

    /// The file of the image that holds `address`.
    fn image_of(address: *const c_void) -> PathBuf {
        let mut info = DlInfo {
            file_name: std::ptr::null(),
            file_base: std::ptr::null_mut(),
            symbol_name: std::ptr::null(),
            symbol_address: std::ptr::null_mut(),
        };
        // SAFETY: `info` is a live structure of the layout of `Dl_info`.
        assert_ne!(unsafe { dladdr(address, &mut info) }, 0, "the address is in no image");
        // SAFETY: on success the name is a NUL-terminated string that lives with the image.
        let name = unsafe { CStr::from_ptr(info.file_name) };
        fs::canonicalize(name.to_str().expect("a UTF-8 path")).expect("the image exists")
    }

    pub fn run() {
        let directory = std::env::temp_dir().join(format!("ferroui-native-library-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("a directory");

        let library = link(&directory, "libFerroNativeWithFactory.dylib", &[]);
        let names = exports(&library);
        assert!(names.iter().any(|name| name == FACTORY), "{FACTORY} is not exported: {names:?}");
        println!("native_library_path: the library exports {} names, {FACTORY} among them", names.len());

        let _builder = AppBuilder::configure::<Application>()
            .use_harfbuzz()
            .use_ferro_native()
            .use_skia()
            .with(Rc::new(FerroNativePlatformOptions {
                rendering_mode: vec![FerroNativeRenderingMode::Software],
                ferro_native_library_path: Some(library.to_str().expect("a UTF-8 path").to_owned()),
                ..Default::default()
            }))
            .setup_without_starting();

        let path = c_path(&library);
        // SAFETY: only asks whether the library is loaded.
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW | RTLD_NOLOAD) };
        assert!(!handle.is_null(), "the platform did not load the library");
        let expected = fs::canonicalize(&library).expect("the library exists");
        // SAFETY: a live handle and a NUL-terminated name.
        let entry = unsafe { dlsym(handle, c"CreateFerroNative".as_ptr()) };
        assert!(!entry.is_null(), "the loaded library has no factory");
        assert_eq!(image_of(entry), expected, "the factory entry point is in another image");

        let factory = FerroLocator::current()
            .get_service::<ComPtr<IFerroNativeFactory>>()
            .expect("the platform holds a factory");
        // SAFETY: the first word of a live COM object is its table of methods.
        let table = unsafe { *(factory.as_ptr() as *const *const c_void) };
        assert_eq!(image_of(table), expected, "the factory object comes from another image");
        println!("native_library_path: the factory of the platform comes from the library");

        // The control: without the export the library loads and has no factory to find.
        let without = link(&directory, "libFerroNativeWithoutFactory.dylib", &["-Wl,-unexported_symbol,_CreateFerroNative"]);
        assert!(!exports(&without).iter().any(|name| name == FACTORY), "the control still exports the factory");
        let path = c_path(&without);
        // SAFETY: loads a library of the same code, as above.
        let handle = unsafe { dlopen(path.as_ptr(), RTLD_NOW) };
        assert!(!handle.is_null(), "the control does not load");
        // SAFETY: a live handle and a NUL-terminated name.
        assert!(unsafe { dlsym(handle, c"CreateFerroNative".as_ptr()) }.is_null(), "the control has a factory");
        println!("native_library_path: a library without the export has no factory");

        let _ = fs::remove_dir_all(&directory);
    }
}
