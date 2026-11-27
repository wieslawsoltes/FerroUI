//! The hosts a popup creates (#994), for the trimming of the themes: a `Popup` that opens creates
//! its host through `OverlayPopupHost::create_popup_host`, a `PopupRoot` (a platform popup) or an
//! `OverlayPopupHost` (on the overlay layer). The scan of the crate reads a construction only as
//! `X::new` and its kin in the code of a class, not this factory function of another class, so the
//! build script of the crate states the edges (`Export::creates`). With them the themes of the two
//! hosts are units of their own, kept where something constructs a popup (a tool tip, a flyout, a
//! context menu, a template with a `Popup`), not by every application.
//!
//! The test of this file keeps the table in step with the code that creates the hosts.

/// The class that creates the hosts, and the hosts, by their full markup names.
pub const CREATES: &[(&str, &[&str])] = &[(
    "FerroUI.Controls.Primitives.Popup",
    &["FerroUI.Controls.Primitives.OverlayPopupHost", "FerroUI.Controls.Primitives.PopupRoot"],
)];

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn sources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the src directory").to_path_buf()
    }

    fn text(file: &str) -> String {
        std::fs::read_to_string(sources().join(file)).unwrap_or_else(|error| panic!("{file}: {error}"))
    }

    /// `Popup` calls the factory, the factory constructs both hosts, and nothing else in the
    /// repository constructs them or calls the factory outside tests (the `PopupRoot` of the
    /// view models of DevTools is a type of its own).
    #[test]
    fn the_table_is_in_step_with_the_code_that_creates_the_hosts() {
        assert_eq!(CREATES.len(), 1);
        assert!(text("FerroUI.Controls/primitives/popup.rs").contains("OverlayPopupHost::create_popup_host("));
        let factory = text("FerroUI.Controls/primitives/overlay_popup_host.rs");
        let start = factory.find("pub fn create_popup_host(").expect("the factory");
        let body = &factory[start..start + factory[start..].find("\n    }\n").expect("the end of the factory")];
        assert!(body.contains("PopupRoot::new_with_resolver(") && body.contains("OverlayPopupHost::new("), "{body}");
        let allowed = [
            "FerroUI.Controls/primitives/overlay_popup_host.rs",
            "FerroUI.Controls/primitives/popup.rs",
            "FerroUI.Controls/primitives/popup_hosts.rs",
            "FerroUI.Diagnostics/diagnostics/view_models/visual_tree_node.rs",
        ];
        let mut pending = vec![sources()];
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
                if path.is_dir() {
                    if name != "target" && name != "tests" && !name.starts_with('.') {
                        pending.push(path);
                    }
                    continue;
                }
                if !name.ends_with(".rs") || name.ends_with("_tests.rs") || name.contains("_tests_") {
                    continue;
                }
                let Ok(source) = std::fs::read_to_string(&path) else { continue };
                let relative = path.strip_prefix(sources()).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                // A call of the name itself, not of a longer name that ends with it (`TrayPopupRoot`).
                let constructs = ["PopupRoot::new(", "PopupRoot::new_with_resolver(", "OverlayPopupHost::new(", "create_popup_host("]
                    .iter()
                    .any(|call| {
                        source.match_indices(call).any(|(at, _)| {
                            !source[..at].chars().next_back().is_some_and(|before| before.is_alphanumeric() || before == '_')
                        })
                    });
                assert!(
                    !constructs || allowed.contains(&relative.as_str()),
                    "{relative} creates a popup host: add the class that does to CREATES"
                );
            }
        }
    }
}
