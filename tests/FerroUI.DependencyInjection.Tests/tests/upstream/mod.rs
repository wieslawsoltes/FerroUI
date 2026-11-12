//! Remaining upstream cases use typed constructors and closed generic registrations where
//! Rust has no CLR reflection or call-site compiler. Tests assert observable behavior plus
//! descriptor metadata; no dynamic-code engine or threading variant exists in this local port.
use super::*;
use ferroui_system::{task::Shared, IDisposable, ObservableError};
use ServiceLifetime::{Scoped, Singleton, Transient};
const LIFETIMES: [ServiceLifetime; 3] = [Singleton, Scoped, Transient];
fn build(s: &ServiceCollection) -> ServiceProvider {
    s.build_service_provider(Default::default()).unwrap()
}
fn checked(s: &ServiceCollection) -> ServiceProvider {
    s.build_service_provider(options()).unwrap()
}
fn missing<T: ?Sized + 'static>(error: ServiceError) {
    assert_eq!(
        error,
        ServiceError::MissingService(ServiceIdentifier::of::<T>())
    );
}
fn error<T>(result: ServiceResult<T>) -> ServiceError {
    match result {
        Err(e) => e,
        Ok(_) => panic!("expected resolution failure"),
    }
}
/// The directories that hold the libraries Cargo built for this test, to find them in and for the compiler to look
/// for what they depend on: the directory of the test executable where one directory holds every library
/// (`<profile>/deps`), and the output directory of every package where each package has its own
/// (`<profile>/build/<package>/<hash>/out`).
fn library_directories() -> Vec<std::path::PathBuf> {
    let executable = std::env::current_exe().unwrap();
    let own = executable.parent().unwrap().to_path_buf();
    let mut directories = vec![own.clone()];
    let packages = own.ancestors().nth(3).filter(|build| build.file_name().is_some_and(|name| name == "build"));
    let entries = |directory: &std::path::Path| -> Vec<std::path::PathBuf> {
        std::fs::read_dir(directory).into_iter().flatten().filter_map(Result::ok).map(|entry| entry.path()).collect()
    };
    for package in packages.map(entries).unwrap_or_default() {
        for unit in entries(&package) {
            let output = unit.join("out");
            if output != own && output.is_dir() {
                directories.push(output);
            }
        }
    }
    directories
}

/// The file of compiler arguments that names `directories` as the places of dependencies, written into
/// `directory`: there may be more of them than a command line holds.
fn library_search_arguments(directories: &[std::path::PathBuf], directory: &std::path::Path) -> String {
    let arguments: String =
        directories.iter().map(|directory| format!("-L\ndependency={}\n", directory.display())).collect();
    let file = directory.join("libraries.args");
    std::fs::write(&file, arguments).unwrap();
    format!("@{}", file.display())
}

/// Compile-time counterparts of CLR null-argument and abstract-type rejection tests.
fn assert_compile_error(source: &str, code: &str) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let directories = library_directories();
    let library = directories
        .iter()
        .flat_map(|directory| std::fs::read_dir(directory).unwrap())
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("libferroui_dependency_injection-")
                && e.path().extension().is_some_and(|x| x == "rlib")
        })
        .max_by_key(|e| e.metadata().unwrap().modified().unwrap())
        .expect("built DI library")
        .path();
    let dir = std::env::temp_dir().join(format!(
        "ferroui-di-ui-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let run = |text: &str| {
        std::fs::write(dir.join("case.rs"), text).unwrap();
        std::process::Command::new("rustc")
            .args(["--edition=2021", "--crate-type=lib", "--emit=metadata"])
            .arg(dir.join("case.rs"))
            .arg("--out-dir")
            .arg(&dir)
            .arg("--extern")
            .arg(format!(
                "ferroui_dependency_injection={}",
                library.display()
            ))
            .arg(library_search_arguments(&directories, &dir))
            .output()
            .unwrap()
    };
    let valid =
        run("use ferroui_dependency_injection::*; fn test(){let _=ServiceCollection::new();}");
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let result = run(source);
    std::fs::remove_dir_all(dir).unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success() && stderr.contains(code),
        "{stderr}"
    );
}

mod activator_utilities_tests;
mod async_service_scope_tests;
mod call_site_factory_test;
mod call_site_tests;
mod circular_dependency_tests;
mod service_collection_descriptor_extensions_tests;
mod service_collection_keyed_service_extensions_test;
mod service_collection_service_extensions_test;
mod service_provider_container_tests;
mod service_provider_engine_scope_tests;
mod service_provider_service_extensions_test;
mod service_provider_validation_tests;
