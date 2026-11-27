//! Whether the Skia this crate links carries the Graphite Metal ownership fix (#622).
//!
//! The Metal backend of Graphite in Skia m153 makes every render pipeline with two shader
//! functions, a vertex descriptor and a colour attachment that it retains and never releases: a
//! leak for each pipeline. The fix adopts the four references (`sk_cfp`) and lives in the patched
//! `skia-bindings` of the FerroUI repository (`external/rust-skia-bindings`), which builds Skia
//! from source with it on macOS. Cargo applies a `[patch.crates-io]` only in the workspace that
//! declares it, so an application built from the published crates without the same patch links
//! the published `skia-bindings` and its archive, without the fix (`docs/apps.md`, "The Graphite
//! Metal ownership fix").
//!
//! The patched binding says so to the crates that depend on it (`cargo:graphite_metal_ownership`,
//! read by the build script of this crate as `DEP_SKIA_GRAPHITE_METAL_OWNERSHIP`), and this crate
//! is built with `ferro_skia_graphite_ownership_fixed` then. A build without it warns once, when
//! the first Graphite context is made, and [`graphite_metal_ownership_fixed`] answers `false`.

/// Whether the Skia this crate is linked with carries the Graphite Metal ownership fix: `None`
/// where this build has no Graphite on Metal (every target but Apple's, and Apple targets built
/// with `ferro_skia_apple_ganesh`), `Some(false)` for a Skia without it, whose Metal pipelines
/// leak their shader functions and descriptors.
pub fn graphite_metal_ownership_fixed() -> Option<bool> {
    if cfg!(all(target_vendor = "apple", not(ferro_skia_apple_ganesh))) {
        Some(cfg!(ferro_skia_graphite_ownership_fixed))
    } else {
        None
    }
}

/// Tells the log once, when the first Graphite context is made, that the Skia of this build lacks
/// the fix.
#[cfg_attr(not(all(target_vendor = "apple", not(ferro_skia_apple_ganesh))), allow(dead_code))]
pub(crate) fn report_once() {
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use std::sync::Once;

    if graphite_metal_ownership_fixed() != Some(false) {
        return;
    }
    static REPORTED: Once = Once::new();
    REPORTED.call_once(|| {
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
            logger.log(
                None,
                "The Skia of this build lacks the Graphite Metal ownership fix: every Metal pipeline \
                 leaks its shader functions and descriptors. Build with the patched skia-bindings of \
                 FerroUI (docs/apps.md, \"The Graphite Metal ownership fix\").",
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The builds of this repository take the patched binding (`[patch.crates-io]` of the root
    /// manifest), which builds Skia from source with the fix on macOS: a test run there says so.
    /// A build given an archive of its own (`SKIA_LIBRARY_SEARCH_PATH`) is the caller's to vouch
    /// for, and is not asked.
    #[test]
    #[cfg(all(target_os = "macos", not(ferro_skia_apple_ganesh)))]
    fn a_build_of_this_repository_on_macos_has_the_fix() {
        if option_env!("SKIA_LIBRARY_SEARCH_PATH").is_some() {
            return;
        }
        assert_eq!(graphite_metal_ownership_fixed(), Some(true));
    }

    #[test]
    #[cfg(not(target_vendor = "apple"))]
    fn a_build_without_graphite_on_metal_has_nothing_to_fix() {
        assert_eq!(graphite_metal_ownership_fixed(), None);
    }
}
