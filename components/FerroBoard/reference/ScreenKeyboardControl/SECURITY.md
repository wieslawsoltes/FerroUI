# Security Policy

## Supported versions

Security fixes are provided for the latest released minor version.

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Use GitHub's
[private vulnerability reporting](https://github.com/wieslawsoltes/ScreenKeyboardControl/security/advisories/new)
instead. Include a description, affected versions and steps to reproduce. You will receive a response within a few
days.

## Privacy notes

ScreenKeyboard processes everything locally. Learned words, emoji history and clipboard history stay in memory
unless the host application persists them. Password fields, fields marked `ScreenKeyboardInput.IsPrivate` and
incognito mode disable learning and history. The optional `EmojiFontFallback.UseNotoColorEmoji()` downloads a font
from a public CDN; host the font yourself (`EmojiFontFallback.Use(...)`) if network access is not desired.

## Release integrity

Packages are built and published exclusively by the `release.yml` GitHub Actions workflow using NuGet Trusted
Publishing (short-lived OIDC credentials, no stored API keys). Packages are deterministic, include Source Link and
symbol packages, and every release is attached to a GitHub release.
