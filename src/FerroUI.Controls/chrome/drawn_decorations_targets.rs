//! The targets on which no window backend constructs drawn window decorations (#1002), for the
//! trimming of the themes: `WindowDrawnDecorations` is constructed only by
//! `TopLevelHost::update_drawn_decorations`, when the platform window of a `Window` needs managed
//! decorations (`IWindowImpl::needs_managed_decorations`). The backends of these targets never do:
//!
//! - macOS: the native backend draws the decorations (`needs_managed_decorations` is `false`);
//! - iOS, Android and the browser (`wasm32`): they have no `IWindowImpl` with decorations.
//!
//! The build script of the crate states the table on the class (`Export::not_created_on`), and an
//! application built for one of these targets keeps the themes of the drawn decorations only when
//! it names them. Linux and the BSDs (X11, and Wayland when the compositor asks for client-side
//! decorations, decided at run time) and Windows (an extended client area, set at run time) can
//! need them in any application, and are not in the table. Any other target keeps them too.
//!
//! The test of this file keeps the table in step with the `needs_managed_decorations` of the
//! backends of the repository.

/// The `cfg` conditions of the targets on which no code constructs `WindowDrawnDecorations`.
pub const NOT_CREATED_ON: &[&str] =
    &["target_os = \"macos\"", "target_os = \"ios\"", "target_os = \"android\"", "target_arch = \"wasm32\""];

/// The full markup name of the class the table is stated on.
pub const CLASS: &str = "FerroUI.Controls.Chrome.WindowDrawnDecorations";

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// The source directory of the repository.
    fn sources() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the src directory").to_path_buf()
    }

    /// The body of `fn needs_managed_decorations` in `text`, trimmed.
    fn body(text: &str) -> Option<String> {
        let start = text.find("fn needs_managed_decorations(&self) -> bool {")?;
        let rest = &text[start..];
        let open = rest.find('{')?;
        let mut depth = 0usize;
        for (index, character) in rest[open..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(rest[open + 1..open + index].trim().to_string());
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The window backends of the repository, by the targets they serve: a backend whose
    /// `needs_managed_decorations` is always `false` serves only targets of the table, and a
    /// backend that can need managed decorations serves none of them. Every other implementation
    /// is a test or a designer stand-in, which builds with every unit of the themes.
    #[test]
    fn the_table_is_in_step_with_the_window_backends() {
        let holds = |os: &str| NOT_CREATED_ON.contains(&format!("target_os = \"{os}\"").as_str());
        let backends: &[(&str, &[&str])] = &[
            ("FerroUI.Native/window_impl.rs", &["macos"]),
            ("Windows/FerroUI.Win32/window_impl.rs", &["windows"]),
            ("FerroUI.X11/x11_window.rs", &["linux", "freebsd"]),
            ("FerroUI.Wayland/window_impl.rs", &["linux", "freebsd"]),
        ];
        for (file, targets) in backends {
            let text = std::fs::read_to_string(sources().join(file)).unwrap_or_else(|error| panic!("{file}: {error}"));
            let body = body(&text).unwrap_or_else(|| panic!("{file} has no needs_managed_decorations"));
            let never = body == "false";
            for target in *targets {
                assert_eq!(holds(target), never, "{file} ({target}): needs_managed_decorations is `{body}`");
            }
        }
        // No other crate of the repository implements a window backend with drawn decorations: the
        // targets of iOS, Android and the browser have none.
        let stand_ins = [
            // This file, which names the method it searches for.
            "FerroUI.Controls/chrome/drawn_decorations_targets.rs",
            "FerroUI.Controls/platform/i_window_impl.rs",
            "FerroUI.Controls/platform/platform_manager_tests.rs",
            "FerroUI.Controls/testing/mock_window_impl.rs",
            "FerroUI.DesignerSupport/remote/previewer_window_impl.rs",
            "FerroUI.DesignerSupport/remote/stubs.rs",
            "Headless/FerroUI.Headless/headless_window_impl.rs",
        ];
        let mut found = Vec::new();
        let mut pending = vec![sources()];
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if !path.file_name().is_some_and(|name| name == "target" || name.to_string_lossy().starts_with('.')) {
                        pending.push(path);
                    }
                } else if path.extension().is_some_and(|extension| extension == "rs")
                    && std::fs::read_to_string(&path).is_ok_and(|text| text.contains("fn needs_managed_decorations(&self) -> bool"))
                {
                    let relative = path.strip_prefix(sources()).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                    found.push(relative);
                }
            }
        }
        for file in &found {
            let known = backends.iter().any(|(backend, _)| backend == file) || stand_ins.contains(&file.as_str());
            assert!(known, "{file} implements needs_managed_decorations: add its targets to this test and to NOT_CREATED_ON");
        }
        for target in ["ios", "android"] {
            assert!(holds(target), "{target}");
        }
        assert!(NOT_CREATED_ON.contains(&"target_arch = \"wasm32\""));
    }
}
