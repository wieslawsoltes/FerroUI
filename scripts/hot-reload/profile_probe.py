#!/usr/bin/env python3
"""Check the real Cargo target configuration used by the manifest gate.

Uses the production gate in a dependency-free fixture, including a build-script
profile that differs from its target. Requires Cargo 1.93 or newer.
"""
import os
from pathlib import Path
import shutil
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
        shutil.copyfile(ROOT / 'src/FerroUI.Build.Tasks/hot_reload_profile.rs',
                        root / 'hot_reload_profile.rs')
        (root / 'build.rs').write_text('''mod hot_reload_profile;
fn main() {
    let requested = std::env::var_os("CARGO_FEATURE_DEVELOPMENT").is_some();
    println!("cargo:rustc-env=MANIFEST_ENABLED={}", hot_reload_profile::enabled(requested));
}
''', encoding='utf-8')
        (root / 'src/main.rs').write_text('''fn main() {
    let enabled: bool = env!("MANIFEST_ENABLED").parse().unwrap();
    assert_eq!(enabled, cfg!(all(debug_assertions, feature = "development")));
    println!("manifest={enabled}");
}
''', encoding='utf-8')
        env = os.environ.copy()
        env['CARGO_TARGET_DIR'] = str(root / 'target')
        for variable in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS'):
            env.pop(variable, None)
        for profile, requested, expected, flags in [
                ('dev', True, True, ''), ('dev', False, False, ''),
                ('release', True, False, ''), ('noassert', True, False, ''),
                ('assertrelease', True, True, ''), ('hostoverride', True, True, ''),
                ('dev', True, False, '-C\x1fdebug-assertions=no'),
                ('release', True, True, '-Cdebug-assertions=yes'),
                ('dev', True, False, '-Cdebug-assertions=yes\x1f-Cdebug-assertions=off'),
                ('dev', False, False, '-Cdebug-assertions=yes')]:
            env['CARGO_ENCODED_RUSTFLAGS'] = flags
            command = ['cargo', 'run', '--offline', '--quiet', '--profile', profile]
            if requested:
                command += ['--features', 'development']
            output = run(command, root, env=env)
            assert f'manifest={str(expected).lower()}' in output, output
            print(f'PASS {profile}: development={requested}, flags={flags!r}, manifest={expected}')


if __name__ == '__main__':
    main()
