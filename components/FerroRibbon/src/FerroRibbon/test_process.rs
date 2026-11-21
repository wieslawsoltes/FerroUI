//! Fresh-process isolation for tests which mutate process-wide product services.
pub(crate) fn isolated_case(name: &str) -> bool {
    let name = name
        .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
        .unwrap_or(name);
    if std::env::var("FERRORIBBON_ISOLATED_TEST").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_ISOLATED_TEST", name)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("running 1 test\n") && output.status.success(),
        "{name}\n{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
