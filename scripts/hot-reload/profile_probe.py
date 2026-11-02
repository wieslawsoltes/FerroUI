#!/usr/bin/env python3
"""Check the real Cargo target configuration used by the manifest gate.

Uses the production gate in a dependency-free fixture, including a build-script
profile that differs from its target. Requires Cargo 1.93 or newer, which sets
CARGO_CFG_DEBUG_ASSERTIONS for build scripts.

Each case also runs the gate as Cargo older than 1.93 sees it (no
CARGO_CFG_DEBUG_ASSERTIONS; the profile root, OPT_LEVEL and the rustflags decide,
#444) on the values this Cargo passes. That fallback matches the target for the
standard profiles and the rustflag overrides; a custom profile that changes the
debug-assertions setting it inherits follows its root, the recorded limit.
"""
import os
from pathlib import Path
import tempfile

from probe import ROOT, run


def main():
    with tempfile.TemporaryDirectory(prefix='ferroui-reload-profiles-') as directory:
        root = Path(directory)
        (root / 'src').mkdir()
        (root / 'Cargo.toml').write_text('''[package]
name = "reload-profile-probe"
version = "0.0.0"
edition = "2021"
[features]
development = []
[profile.noassert]
inherits = "dev"
debug-assertions = false
[profile.assertrelease]
inherits = "release"
debug-assertions = true
[profile.hostoverride]
inherits = "dev"
debug-assertions = true
[profile.hostoverride.build-override]
debug-assertions = false
''', encoding='utf-8')
        gate = (ROOT / 'src/FerroUI.Build.Tasks/hot_reload_profile.rs').read_text(encoding='utf-8')
        # The gate as Cargo before 1.93 runs it: the module's own rule without the variable.
        gate += '''
pub(super) fn before_cargo_1_93(requested: bool) -> Result<bool, String> {
    if !requested {
        return Ok(false);
    }
    with_environment(|environment| {
        let environment = ProfileInputs { cfg_debug_assertions: false, ..environment };
        decide(&environment, || Some("cargo 1.90.0 (840b83a10 2025-07-30)".to_string()))
    })
}
'''
        (root / 'hot_reload_profile.rs').write_text(gate, encoding='utf-8')
        (root / 'build.rs').write_text('''mod hot_reload_profile;
fn main() {
    let requested = std::env::var_os("CARGO_FEATURE_DEVELOPMENT").is_some();
    let enabled = hot_reload_profile::enabled(requested).unwrap_or_else(|error| panic!("{error}"));
    let fallback = hot_reload_profile::before_cargo_1_93(requested).unwrap_or_else(|error| panic!("{error}"));
    println!("cargo:rustc-env=MANIFEST_ENABLED={enabled}");
    println!("cargo:rustc-env=MANIFEST_FALLBACK={fallback}");
}
''', encoding='utf-8')
        (root / 'src/main.rs').write_text('''fn main() {
    let enabled: bool = env!("MANIFEST_ENABLED").parse().unwrap();
    assert_eq!(enabled, cfg!(all(debug_assertions, feature = "development")));
    println!("manifest={enabled} fallback={}", env!("MANIFEST_FALLBACK"));
}
''', encoding='utf-8')
        env = os.environ.copy()
        env['CARGO_TARGET_DIR'] = str(root / 'target')
        for variable in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS'):
            env.pop(variable, None)
        # The last column is the fallback before Cargo 1.93: the expected value except
        # for the two custom profiles that change the setting they inherit.
        for profile, requested, expected, flags, fallback in [
                ('dev', True, True, '', True), ('dev', False, False, '', False),
                ('release', True, False, '', False), ('noassert', True, False, '', True),
                ('assertrelease', True, True, '', False), ('hostoverride', True, True, '', True),
                ('dev', True, False, '-C\x1fdebug-assertions=no', False),
                ('release', True, True, '-Cdebug-assertions=yes', True),
                ('dev', True, False, '-Cdebug-assertions=yes\x1f-Cdebug-assertions=off', False),
                ('dev', False, False, '-Cdebug-assertions=yes', False)]:
            env['CARGO_ENCODED_RUSTFLAGS'] = flags
            command = ['cargo', 'run', '--offline', '--quiet', '--profile', profile]
            if requested:
                command += ['--features', 'development']
            output = run(command, root, env=env)
            line = f'manifest={str(expected).lower()} fallback={str(fallback).lower()}'
            assert line in output, output
            print(f'PASS {profile}: development={requested}, flags={flags!r}, manifest={expected}, '
                  f'before Cargo 1.93={fallback}')


if __name__ == '__main__':
    main()
