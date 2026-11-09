# Private references

Nothing in the repository names a particular machine, account or local tool. The check
`scripts/conventions/private-references.py` (in CI, "Source conventions and packages") refuses:

- the home directory of an account: `/Users/<name>/`, `/home/<name>/`, `C:\Users\<name>\`;
- the folders and branch names of local tools (`.codex`, `codex/...`);
- other hosts of the repository;
- access tokens (`glpat-`, `ghp_`, `github_pat_`).

Write paths with placeholders instead:

| Instead of | Write |
|---|---|
| a checkout of this repository | `<checkout>` |
| a sibling checkout (a reference repository) | `../<repository>` |
| a directory of captured evidence | `<evidence>/` |
| a directory under the home directory | `~/...` (shell), `$env:USERPROFILE\...` (PowerShell) |

Generic examples such as `/home/user/`, `C:\Users\username\` and file paths in tests
(`/home/Projects.txt`) are allowed. Recorded results (JSON of measurements and smoke runs) store
paths relative to the checkout or with a placeholder. Third-party sources under `external/` and
`components/FerroBoard/reference/` are kept as published and are not checked.

Run it before pushing:

```sh
python3 scripts/conventions/private-references.py
```
