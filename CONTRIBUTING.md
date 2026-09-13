# Contributing to MLBV

Thanks for wanting to help. Please read [`AGENTS.md`](AGENTS.md) first — it is required reading
for AI agents and just as useful for humans: it contains the file map, the build/verify matrix, the
conventions, and a list of known landmines with file references.

---

## Project status and expectations

MLBV is a **frozen hobby project**. The last release is `beta0.0.4` (14 June 2026) and there is no
active development. There are no maintainer SLAs: issues and pull requests may sit unanswered for a
long time, and changes are accepted on a best-effort basis. That is not a reason not to contribute —
just calibrate your expectations, and prefer small, self-contained changes over large refactors.

If you are an **AI agent**: read [`AGENTS.md`](AGENTS.md) in full before doing anything here — it
defines the workflow, the verification duties and the prohibited actions. In particular: do not open
a pull request, push to `main`, or post comments unless the person you are working for explicitly
asked for that specific action in that specific request; never fabricate evidence (invented command
output, unread logs, a status you did not check); re-verify anything you are not certain about
instead of guessing; and never commit a secret or personal data (see *Secrets and personal data*).

## Requirements

| Tool | Version | Notes |
|---|---|---|
| Node.js | 18+, 22 recommended | CI uses 22 |
| Rust | stable (edition 2021) | `rustup` is the usual way |
| Tauri prerequisites | per OS | Windows: WebView2 + VS C++ Build Tools · Linux: `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libxdo-dev`, `libssl-dev`, `build-essential`, `pkg-config` · macOS: Xcode CLT |

The launcher targets Windows and Linux: CI builds NSIS/MSI installers and `.deb`/`.rpm`/`.AppImage`
packages (plus a Flatpak bundle on release tags). macOS compiles, but no packages are published
for it. The in-app self-updater is Windows-only (NSIS/MSI).

## Setup

```bash
git clone https://github.com/MLBVbyvlal/launcher.git
cd launcher
npm ci
```

## Commands

| Command | Purpose |
|---|---|
| `npm run dev` | Vite dev server on `:1420`. Browser only — every Tauri command fails; use it for layout. |
| `npm run tauri dev` | Full desktop app with hot reload. First Rust build takes 5–15 min. |
| `npx tsc --noEmit` | Type-check. Also runs as part of `npm run build`. |
| `npm run build` | `tsc` + `vite build` → `dist/`. |
| `npm run tauri build -- --bundles nsis,msi` | Release build + installers in `src-tauri/target/release/bundle/`. |
| `cd src-tauri && cargo check --locked --all-targets` | Compile-check the backend. |
| `cd src-tauri && cargo test --locked` | Run the Rust test suite (unit + integration). |
| `cd src-tauri && cargo fmt` | Format Rust. No config beyond rustfmt defaults. |

On Windows, `run.bat` wraps `npm run tauri dev` and verifies Node and Cargo are on `PATH`.

There is a Rust test suite using the built-in harness (no extra dependencies): unit tests live
next to the code in `#[cfg(test)]` modules, black-box integration tests live in `src-tauri/tests/`.
If you add testable logic (parsing, version comparison, path building, argument resolution), cover
it the same way. There is **no linter**.

CI (`.github/workflows/ci.yml`) runs four jobs on every push: type-check + bundle, `cargo check`
plus `cargo test` on Linux, a full Windows installer build, and Linux packages. Run the same steps
locally before opening a PR.

## Project layout

```
src-tauri/src/lib.rs       Tauri commands + generate_handler! list
src-tauri/src/launcher.rs  Launch pipelines, Java provisioning, helpers
src-tauri/src/main.rs      Windows entry point (do not touch windows_subsystem)
src-tauri/tauri.conf.json  Window config, bundle, identifier
src-tauri/capabilities/    Tauri v2 permission sets (main, console)
src/App.tsx                Main UI and all modals
src/SetupWizard.tsx        First-run wizard
src/ConsoleWindow.tsx      Console window
src/LbConfigsPanel.tsx     LiquidBounce configs catalog
src/i18n.ts                EN/RU strings
```

## Code guidelines

**TypeScript**

- Strict mode is on, including `noUnusedLocals` and `noUnusedParameters`. CI fails on type errors.
- No `any` anywhere in the codebase — keep it that way.
- No new dependencies for something a few lines of code can do; if a dependency is genuinely
  needed, explain the trade-off in the PR description.
- All user-facing text goes through `t('key')` and must exist in **both** `en` and `ru` in
  `src/i18n.ts`. A key present in only one language renders as the key itself.
- Persisted `localStorage` keys are prefixed `mlbv_`. Do not rename existing keys without a
  migration path.
- Keep styling in `src/App.css` using the existing CSS custom properties; no CSS framework.

**Rust**

- `anyhow` with `.context()` inside the launcher; `Result<_, String>` at the Tauri command
  boundary. Match the existing pattern.
- Do not add a crate for something the standard library or an existing dependency already does.
- `Cargo.lock` is committed; use `--locked` when checking so CI and local builds agree.
- Never leave a fallible operation silently discarded: `let _ = ...` around downloads and file
  writes is a known source of "the game crashed and nothing explained why". If an error is
  genuinely ignorable, comment why.
- Launching is one pipeline (`launcher::launch`) with loader steps around it. A change in `launch`,
  `download_vanilla_libraries`, `download_assets`, `build_launch_args` or `spawn_game` affects
  **every** loader; a change in `prepare_loader`, `prepare_loader_stage`,
  `download_profile_libraries`, `download_overlay_libraries` or `download_lb_mods` affects one
  loader only. Say which one you meant in the PR description.

**Comments and sections**

- Use the existing `// ─── Section ───────` banners.
- Comment the *decision*, not the syntax; e.g. explaining why a specific upstream endpoint is
  avoided is valuable, restating a `for` loop is not.

## Commits

- Imperative subject line, roughly ≤ 72 characters, optionally a body explaining the *why*.
- One logical change per commit; do not mix refactoring with behaviour changes.
- Do not add AI co-author trailers.

## Version bumps

The version is duplicated in five places and they are not synchronised today. A bump must update
all of them, otherwise the About panel, the debug panel and the updater disagree:

| File | Field |
|---|---|
| `package.json` | `version` |
| `package-lock.json` | `version` (two occurrences; `npm install` rewrites it) |
| `src-tauri/tauri.conf.json` | `version` |
| `src-tauri/Cargo.toml` | `version` — drives `env!("CARGO_PKG_VERSION")` and the update check |
| `vite.config.ts` | `define.__APP_VERSION__` — hardcoded, shown in the About panel |

Do not bump the version, create a tag, or publish a release unless the maintainer asked for it.

## Release process (maintainer)

1. Bump the version everywhere (table above).
2. Push a tag (`v0.0.x`, `beta0.0.x`, `release0.0.x` or `pre-release0.0.x`). The Release workflow
   builds every installer — NSIS `.exe` + WiX `.msi`, `.deb` + `.rpm` + `.AppImage`, Flatpak bundle —
   smoke-runs the Flatpak, and attaches everything to the tag's GitHub release. Tags starting with
   `beta`/`pre-release` publish as pre-releases, the rest as full releases.
3. To test without publishing: Actions → Release → Run workflow with `dry_run` on — the binaries
   land in the run's artifacts instead of a release.
4. The in-app updater reads that release and offers the `.exe` (silent `/S /D=` update,
   recommended) or the `.msi` (guided install); pre-release candidates are shown with a warning.

## Secrets and personal data

This repository is **public**, and everything pushed here is public permanently — forks, mirrors and
archives keep copies within minutes, so deleting a file or rewriting history does not un-leak it.

Never commit:

- tokens, session IDs, OAuth authorization codes, API keys, SSH/GPG private keys, certificates,
  `.netrc`, cookie jars, password stores;
- `.env` files under any name, or secret-bearing JSON sitting next to config;
- personal data: real Minecraft/Microsoft/Xbox nicknames, e-mail addresses, phone numbers, real
  account UUIDs, IP addresses and server addresses, paths containing your real name, chat logs,
  other people's files;
- `*.log`, crash reports and JVM dumps — they carry tokens, UUIDs and local paths.

Screenshots are committed data that no `grep` can check. Open every image before adding it and look
for nicknames, avatars, UUIDs, file paths, console output, server addresses and OS notifications. Use
a scratch profile with deliberately fake accounts — that is what the screenshots in
`docs/screenshots/` are — or pixelate the details. The same applies to screen recordings.

If something did leak: **rotate or revoke the credential first**, then remove it from the working tree
and tell the maintainer. Never rewrite `main` and never force-push shared history.

Not a secret: the bare `client_id = "00000000402b5328"` in `src-tauri/src/lib.rs` is Minecraft's
public launcher client ID. Its presence is not a precedent — do not add any other identifier, and
never add a real application secret next to it.

## Reporting bugs

A useful report includes:

- MLBV version (Settings → About, or the Debug info block) and Windows version.
- What you did, what you expected, what happened.
- The relevant log: instance `logs\latest.log`, the crash dialog's last 80 lines, or the console
  window output.
- The **Debug info** JSON from Settings → About — it contains Java installations and paths, with no
  tokens or personal data. Do not paste account tokens.

Known problems — sessions expiring, empty instance lists after clearing WebView data, legacy
(pre-1.7) assets, the dead update check — are listed in `README.md` → *Known limitations*. Please
read that section before filing.

## Scope

- Do not add features that circumvent Minecraft licensing or authentication, and do not extend the
  Launcher with anything that ships game content.
- Do not add telemetry, analytics, or any new outbound endpoint without documenting it in the
  *External services* table in `README.md`.
- Anything touching accounts, tokens or the Windows registry needs an explicit discussion first.
- Never add code that writes credentials to disk in plain text or sends them anywhere other than the
  auth chain documented in *External services* in `README.md`.

## License

MLBV is licensed under **GPL-3.0-or-later** (see [`LICENSE`](LICENSE)). By contributing you agree
that your contribution is licensed under the same terms. Do not paste code from projects whose
license is incompatible with the GPL.
