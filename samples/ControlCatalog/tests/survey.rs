//! A survey of the documents: loads each one without its class and writes
//! what happened to `$CATALOG_SURVEY` (not a test of the sample; run with
//! `-- --ignored survey`).

use super::support::*;
use crate::markup::{describe, try_load_text};

#[test]
#[ignore = "survey"]
fn survey_documents_without_classes() {
    let Ok(out) = std::env::var("CATALOG_SURVEY") else { return };
    let filter = std::env::var("CATALOG_SURVEY_FILTER").ok();
    let mut report = String::new();
    for (path, class) in crate::assets::documents() {
        if filter.as_deref().is_some_and(|f| !path.contains(f)) {
            continue;
        }
        let text = std::str::from_utf8(crate::assets::asset(path).unwrap()).unwrap().to_string();
        let text = match class {
            Some(class) => text.replacen(&format!("x:Class=\"{class}\""), "", 1),
            None => text,
        };
        let path_owned = path.to_string();
        let result = std::thread::spawn(move || {
            let _app = start_application();
            match try_load_text(&text, Some(&path_owned), None) {
                Ok(_) => "OK".to_string(),
                Err(error) => format!("ERR {}", describe(&error).replace('\n', " ")),
            }
        })
        .join();
        let line = match result {
            Ok(line) => line,
            Err(panic) => format!(
                "PANIC {}",
                panic.downcast_ref::<String>().cloned().or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default().replace('\n', " ")
            ),
        };
        report.push_str(&format!("{path} | {line}\n"));
    }
    std::fs::write(out, report).unwrap();
}

/// Loads the markup of the file `$CATALOG_SNIPPET` and prints what happened.
#[test]
#[ignore = "survey"]
fn survey_snippet() {
    let Ok(file) = std::env::var("CATALOG_SNIPPET") else { return };
    let _app = start_application();
    for (index, text) in std::fs::read_to_string(file).unwrap().split("\n---\n").enumerate() {
        match try_load_text(text, None, None) {
            Ok(_) => println!("SNIPPET {index}: OK"),
            Err(error) => println!("SNIPPET {index}: ERR {}", describe(&error)),
        }
    }
}
