// Cargo content stamp for audited native packages; no runtime compatibility layer.
fn main() {
    println!("cargo::rerun-if-env-changed=FERRORIBBON_BUILD_FINGERPRINT");
    if let Ok(value) = std::env::var("FERRORIBBON_BUILD_FINGERPRINT") {
        assert!(
            value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid package source fingerprint"
        );
        println!("cargo::rustc-env=FERRORIBBON_BUILD_FINGERPRINT={value}");
    }
}
