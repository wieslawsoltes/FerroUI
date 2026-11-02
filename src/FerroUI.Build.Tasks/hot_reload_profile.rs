//! Whether a build writes the hot-reload manifest: only for a target with debug assertions.
//!
//! The gate reads the configuration of the target, not this build dependency's
//! configuration (a `build-override` can differ from the target). It is dependency-free:
//! `scripts/hot-reload/profile_probe.py` copies this file into a fixture crate.
//!
//! The rule, in order:
//! 1. An explicit `-C debug-assertions[=value]` in the target rustflags wins (the last one).
//!    Cargo does not reflect rustflags in `CARGO_CFG_*`.
//! 2. `CARGO_CFG_DEBUG_ASSERTIONS` set: on.
//! 3. Otherwise the profile root: `PROFILE` is `debug` for a profile that inherits `dev`
//!    and `release` for one that inherits `release`; without it, `OPT_LEVEL` 0 is on
//!    (the rustc default) and any other level off. `DEBUG` is the debuginfo setting, which
//!    says nothing about debug assertions, and is only reported. Neither known: an error.
//! 4. A root without debug assertions: off.
//! 5. A root with debug assertions: Cargo 1.93 and later set `CARGO_CFG_DEBUG_ASSERTIONS`
//!    from the profile (rust-lang/cargo#16160), so its absence means the profile turned
//!    them off; older Cargo never sets it, so the root decides (on). The version comes from
//!    `$CARGO -V`; when it cannot be read: an error.
//!
//! With Cargo older than 1.93 a custom profile that changes `debug-assertions` from the
//! one it inherits is decided by its root (step 3): Cargo gives build scripts no other
//! sign of that setting. A `-C debug-assertions` rustflag decides those builds.

/// The minor version of the first Cargo release that sets `CARGO_CFG_DEBUG_ASSERTIONS`
/// for build scripts.
const CFG_SINCE_MINOR: u32 = 93;

/// The day rust-lang/cargo#16160 was merged: a 1.93 prerelease of a later commit date has
/// it. A commit of that day is taken to predate it.
const CFG_MERGED_ON: &str = "2025-11-21";

/// Whether the build writes the hot-reload manifest: `requested` and the target has
/// debug assertions. An error when the target configuration cannot be determined.
pub(super) fn enabled(requested: bool) -> Result<bool, String> {
    if !requested {
        return Ok(false);
    }
    with_environment(|environment| decide(&environment, cargo_version))
}

/// Calls `f` with the values of the build script's environment.
fn with_environment<R>(f: impl FnOnce(ProfileInputs<'_>) -> R) -> R {
    let var = |name: &str| std::env::var(name).ok();
    let (encoded_rustflags, rustflags) = (var("CARGO_ENCODED_RUSTFLAGS"), var("RUSTFLAGS"));
    let (profile, opt_level, debug) = (var("PROFILE"), var("OPT_LEVEL"), var("DEBUG"));
    f(ProfileInputs {
        cfg_debug_assertions: std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some(),
        encoded_rustflags: encoded_rustflags.as_deref(),
        rustflags: rustflags.as_deref(),
        profile: profile.as_deref(),
        opt_level: opt_level.as_deref(),
        debug: debug.as_deref(),
    })
}

/// The values Cargo gives a build script that decide the gate.
#[derive(Clone, Copy, Default)]
struct ProfileInputs<'a> {
    /// `CARGO_CFG_DEBUG_ASSERTIONS` is set.
    cfg_debug_assertions: bool,
    /// `CARGO_ENCODED_RUSTFLAGS`: the target rustflags separated by `\x1f`.
    encoded_rustflags: Option<&'a str>,
    /// `RUSTFLAGS`, read only without `CARGO_ENCODED_RUSTFLAGS`.
    rustflags: Option<&'a str>,
    /// `PROFILE`: `debug` or `release`, the root of the profile.
    profile: Option<&'a str>,
    /// `OPT_LEVEL` of the profile.
    opt_level: Option<&'a str>,
    /// `DEBUG` (debuginfo) of the profile.
    debug: Option<&'a str>,
}

/// The rule of the module documentation. `cargo_version` is the output of `cargo -V`, read
/// only for step 5.
fn decide(environment: &ProfileInputs<'_>, cargo_version: impl FnOnce() -> Option<String>) -> Result<bool, String> {
    if let Some(value) = rustflag_override(environment) {
        return Ok(value);
    }
    if environment.cfg_debug_assertions {
        return Ok(true);
    }
    let Some(inherited) = profile_default(environment) else {
        return Err(format!(
            "the hot-reload manifest is requested (Build::hot_reload_manifest), but whether the target has debug assertions cannot be determined: CARGO_CFG_DEBUG_ASSERTIONS is not set and {} name no Cargo profile; add `-C debug-assertions=on` or `-C debug-assertions=off` to the rustflags",
            describe(environment)
        ));
    };
    if !inherited {
        return Ok(false);
    }
    let version = cargo_version();
    match version.as_deref().and_then(cargo_sets_cfg) {
        // The variable is reliable: the profile turned debug assertions off.
        Some(true) => Ok(false),
        Some(false) => Ok(true),
        None => Err(format!(
            "the hot-reload manifest is requested (Build::hot_reload_manifest), but whether the target has debug assertions cannot be determined: CARGO_CFG_DEBUG_ASSERTIONS is not set for a profile that inherits them ({}), and the version of Cargo, which says whether Cargo sets that variable (1.93 and later), cannot be read from `$CARGO -V` ({}); add `-C debug-assertions=on` or `-C debug-assertions=off` to the rustflags",
            describe(environment),
            version.as_deref().map_or("no output".to_string(), |text| format!("`{}`", text.trim()))
        )),
    }
}

/// The last `-C debug-assertions[=value]` of the target rustflags.
fn rustflag_override(environment: &ProfileInputs<'_>) -> Option<bool> {
    match environment.encoded_rustflags {
        Some(flags) => last_debug_assertions(flags.split('\u{1f}')),
        None => last_debug_assertions(environment.rustflags.unwrap_or_default().split_whitespace()),
    }
}

fn last_debug_assertions<'a>(mut flags: impl Iterator<Item = &'a str>) -> Option<bool> {
    let mut last = None;
    while let Some(flag) = flags.next() {
        let option = match flag {
            "-C" | "--codegen" => flags.next(),
            _ => flag.strip_prefix("--codegen=").or_else(|| flag.strip_prefix("-C")),
        };
        match option {
            Some("debug-assertions") => last = Some(true),
            Some(option) => {
                if let Some(value) = option.strip_prefix("debug-assertions=") {
                    last = Some(matches!(value, "y" | "yes" | "on" | "true"));
                }
            }
            None => (),
        }
    }
    last
}

/// Whether the root of the profile has debug assertions: `PROFILE`, else `OPT_LEVEL`.
fn profile_default(environment: &ProfileInputs<'_>) -> Option<bool> {
    match environment.profile {
        Some("debug") => Some(true),
        Some("release") => Some(false),
        _ => match environment.opt_level {
            Some("0") => Some(true),
            Some("1" | "2" | "3" | "s" | "z") => Some(false),
            _ => None,
        },
    }
}

/// Whether the Cargo of `cargo -V` output sets `CARGO_CFG_DEBUG_ASSERTIONS` for build
/// scripts; `None` for output that is not a Cargo version.
fn cargo_sets_cfg(text: &str) -> Option<bool> {
    let mut words = text.split_whitespace();
    if words.next()? != "cargo" {
        return None;
    }
    let version = words.next()?;
    let (number, prerelease) = match version.split_once('-') {
        Some((number, prerelease)) => (number, Some(prerelease)),
        None => (version, None),
    };
    let mut parts = number.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    if major != 1 {
        return Some(major > 1);
    }
    if minor != CFG_SINCE_MINOR || prerelease.is_none() {
        return Some(minor >= CFG_SINCE_MINOR);
    }
    // A 1.93 nightly, beta or development build: the date of its commit, as in
    // `cargo 1.93.0-nightly (6c1b61003 2025-11-28)`.
    let date = words.map(|word| word.trim_matches(|c: char| c == '(' || c == ')')).find(|word| is_date(word))?;
    Some(date > CFG_MERGED_ON)
}

fn is_date(word: &str) -> bool {
    let bytes = word.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

fn describe(environment: &ProfileInputs<'_>) -> String {
    let show = |value: Option<&str>| value.map_or("unset".to_string(), |value| format!("`{value}`"));
    format!(
        "PROFILE {}, OPT_LEVEL {}, DEBUG {}",
        show(environment.profile),
        show(environment.opt_level),
        show(environment.debug)
    )
}

fn cargo_version() -> Option<String> {
    let cargo = std::env::var_os("CARGO")?;
    let output = std::process::Command::new(cargo).arg("-V").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEW_CARGO: &str = "cargo 1.93.0 (083ac5135 2025-12-15)\n";
    const OLD_CARGO: &str = "cargo 1.90.0 (840b83a10 2025-07-30)\n";

    fn profile(profile: &'static str, opt_level: &'static str) -> ProfileInputs<'static> {
        let debug = if profile == "debug" { "true" } else { "false" };
        ProfileInputs { profile: Some(profile), opt_level: Some(opt_level), debug: Some(debug), encoded_rustflags: Some(""), ..ProfileInputs::default() }
    }

    fn with_cfg(environment: ProfileInputs<'static>) -> ProfileInputs<'static> {
        ProfileInputs { cfg_debug_assertions: true, ..environment }
    }

    fn with_flags(environment: ProfileInputs<'static>, flags: &'static str) -> ProfileInputs<'static> {
        ProfileInputs { encoded_rustflags: Some(flags), ..environment }
    }

    /// The decision with a Cargo of the given `cargo -V` output.
    fn on(environment: ProfileInputs<'_>, cargo: &str) -> Result<bool, String> {
        decide(&environment, || Some(cargo.to_string()))
    }

    /// The decision where reading the Cargo version is a test failure.
    fn without_version(environment: ProfileInputs<'_>) -> Result<bool, String> {
        decide(&environment, || panic!("the Cargo version is not needed"))
    }

    #[test]
    fn not_requested_is_off_without_reading_the_environment() {
        assert_eq!(enabled(false), Ok(false));
    }

    #[test]
    fn the_cfg_variable_turns_debug_assertions_on() {
        for (root, opt_level) in [("debug", "0"), ("release", "3"), ("release", "0"), ("debug", "3")] {
            assert_eq!(without_version(with_cfg(profile(root, opt_level))), Ok(true), "{root} {opt_level}");
        }
        assert_eq!(without_version(with_cfg(ProfileInputs::default())), Ok(true));
    }

    #[test]
    fn a_release_root_without_the_cfg_variable_is_off_on_every_cargo() {
        for opt_level in ["0", "1", "2", "3", "s", "z"] {
            assert_eq!(without_version(profile("release", opt_level)), Ok(false), "release {opt_level}");
        }
    }

    #[test]
    fn a_debug_root_without_the_cfg_variable_follows_the_cargo_version() {
        for opt_level in ["0", "1", "3", "s"] {
            // Cargo 1.93 sets the variable from the profile: its absence means a profile
            // that turns debug assertions off (`inherits = "dev"`, `debug-assertions = false`).
            assert_eq!(on(profile("debug", opt_level), NEW_CARGO), Ok(false), "new debug {opt_level}");
            // Older Cargo never sets it: the dev root decides, as for `cargo build`.
            assert_eq!(on(profile("debug", opt_level), OLD_CARGO), Ok(true), "old debug {opt_level}");
        }
    }

    #[test]
    fn without_a_known_profile_root_the_opt_level_decides() {
        let level = |profile: Option<&'static str>, opt_level| ProfileInputs { profile, opt_level: Some(opt_level), ..ProfileInputs::default() };
        assert_eq!(on(level(None, "0"), OLD_CARGO), Ok(true));
        assert_eq!(on(level(Some("custom"), "0"), OLD_CARGO), Ok(true));
        assert_eq!(on(level(None, "0"), NEW_CARGO), Ok(false));
        for opt_level in ["1", "2", "3", "s", "z"] {
            assert_eq!(without_version(level(None, opt_level)), Ok(false), "{opt_level}");
        }
    }

    #[test]
    fn an_undetermined_profile_is_an_error() {
        let error = without_version(ProfileInputs::default()).unwrap_err();
        assert!(error.contains("CARGO_CFG_DEBUG_ASSERTIONS is not set"), "{error}");
        assert!(error.contains("PROFILE unset, OPT_LEVEL unset, DEBUG unset"), "{error}");
        let unknown = ProfileInputs { profile: Some("other"), opt_level: Some("4"), ..ProfileInputs::default() };
        assert!(without_version(unknown).unwrap_err().contains("PROFILE `other`, OPT_LEVEL `4`"));
    }

    #[test]
    fn an_unreadable_cargo_version_is_an_error_where_it_decides() {
        let error = decide(&profile("debug", "0"), || None).unwrap_err();
        assert!(error.contains("PROFILE `debug`, OPT_LEVEL `0`, DEBUG `true`"), "{error}");
        assert!(error.contains("no output"), "{error}");
        let error = on(profile("debug", "0"), "rustc 1.93.0").unwrap_err();
        assert!(error.contains("`rustc 1.93.0`"), "{error}");
        // A release root and the cfg variable do not need it.
        assert_eq!(decide(&profile("release", "3"), || None), Ok(false));
        assert_eq!(decide(&with_cfg(profile("debug", "0")), || None), Ok(true));
    }

    #[test]
    fn an_explicit_rustflag_wins() {
        let cases = [
            ("-C\u{1f}debug-assertions=no", false),
            ("-Cdebug-assertions=off", false),
            ("-Cdebug-assertions=false", false),
            ("-Cdebug-assertions=n", false),
            ("-Cdebug-assertions=yes", true),
            ("-Cdebug-assertions=on", true),
            ("-C\u{1f}debug-assertions", true),
            ("-Cdebug-assertions", true),
            ("--codegen\u{1f}debug-assertions=off", false),
            ("--codegen=debug-assertions=on", true),
            ("-Cdebug-assertions=yes\u{1f}-Cdebug-assertions=off", false),
            ("-Cdebug-assertions=off\u{1f}-Copt-level=0\u{1f}-Cdebug-assertions=on", true),
        ];
        for (flags, expected) in cases {
            for environment in [profile("debug", "0"), profile("release", "3"), with_cfg(profile("debug", "0")), with_cfg(profile("release", "3")), ProfileInputs::default()] {
                assert_eq!(without_version(with_flags(environment, flags)), Ok(expected), "{flags:?}");
            }
        }
    }

    #[test]
    fn other_rustflags_do_not_decide() {
        let flags = "-Copt-level=3\u{1f}--cfg\u{1f}debug_assertions\u{1f}-Ctarget-cpu=native";
        assert_eq!(without_version(with_flags(profile("release", "3"), flags)), Ok(false));
        assert_eq!(on(with_flags(profile("debug", "0"), flags), OLD_CARGO), Ok(true));
    }

    #[test]
    fn rustflags_are_read_only_without_the_encoded_variable() {
        let plain = |rustflags| ProfileInputs { rustflags: Some(rustflags), encoded_rustflags: None, ..profile("release", "3") };
        assert_eq!(without_version(plain("-C debug-assertions=on")), Ok(true));
        assert_eq!(without_version(plain("-Cdebug-assertions  -C  debug-assertions=off")), Ok(false));
        let both = ProfileInputs { rustflags: Some("-Cdebug-assertions=on"), ..profile("release", "3") };
        assert_eq!(without_version(both), Ok(false));
    }

    #[test]
    fn the_cargo_version_says_whether_the_cfg_variable_is_set() {
        let cases = [
            ("cargo 1.89.0 (c24e10642 2025-06-23)", Some(false)),
            ("cargo 1.90.0 (840b83a10 2025-07-30)", Some(false)),
            ("cargo 1.92.0 (344c4567c 2025-10-21)", Some(false)),
            ("cargo 1.92.0-nightly (6c1b61003 2025-10-28)", Some(false)),
            ("cargo 1.93.0-nightly (6c1b61003 2025-11-14)", Some(false)),
            ("cargo 1.93.0-nightly (2a7c49606 2025-11-21)", Some(false)),
            ("cargo 1.93.0-nightly (2a7c49606 2025-11-28)", Some(true)),
            ("cargo 1.93.0-beta.3 (083ac5135 2025-12-15)", Some(true)),
            ("cargo 1.93.0 (083ac5135 2025-12-15)", Some(true)),
            ("cargo 1.93.1 (083ac5135 2025-12-15)\n", Some(true)),
            ("cargo 1.99.0 (00000000 2026-08-01)", Some(true)),
            ("cargo 1.99.0-nightly (00000000 2026-08-01)", Some(true)),
            ("cargo 2.0.0", Some(true)),
            ("cargo 1.93.0-dev", None),
            ("cargo", None),
            ("cargo x.y.z", None),
            ("rustc 1.93.0 (254b59607 2026-01-19)", None),
            ("", None),
        ];
        for (text, expected) in cases {
            assert_eq!(cargo_sets_cfg(text), expected, "{text:?}");
        }
    }
}
