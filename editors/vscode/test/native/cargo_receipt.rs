use std::{
    env, fs,
    path::PathBuf,
    process::{self, Command},
};

fn main() {
    let arguments: Vec<_> = env::args_os().skip(1).collect();
    let result = Command::new("cargo")
        .args(&arguments)
        .status()
        .expect("run Cargo for the installed release fixture");
    let code = result.code().unwrap_or(1);
    if code != 0
        && arguments
            .first()
            .is_some_and(|argument| argument == "build")
    {
        let receipts = PathBuf::from(env::var_os("FERROUI_TEST_RUNTIME_RECEIPTS").unwrap());
        fs::write(receipts.join("failed-build"), code.to_string()).expect("record Cargo's exit");
    }
    process::exit(code);
}
