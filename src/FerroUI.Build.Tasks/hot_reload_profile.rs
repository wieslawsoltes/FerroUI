/// Use the target configuration, not this build dependency's configuration or
/// Cargo's PROFILE alias (custom profiles inherit either "debug" or "release").
/// Cargo 1.93 exposes the profile setting. Explicit target rustflags can override
/// that setting, but Cargo does not reflect those overrides in CARGO_CFG_*.
pub(super) fn enabled(requested: bool) -> bool {
    if !requested {
        return false;
    }
    let mut enabled = std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some();
    let flags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let mut flags = flags.split('\u{1f}');
    while let Some(flag) = flags.next() {
        let option = if flag == "-C" {
            flags.next()
        } else {
            flag.strip_prefix("-C")
        };
        match option {
            Some("debug-assertions") => enabled = true,
            Some(option) => {
                if let Some(value) = option.strip_prefix("debug-assertions=") {
                    enabled = matches!(value, "y" | "yes" | "on" | "true");
                }
            }
            None => (),
        }
    }
    enabled
}
