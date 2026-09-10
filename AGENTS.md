# AGENTS.md

**Read this file completely before touching anything in this repository. Do not open files, run
commands or create branches until you have read it to the end.** It describes how this project is
built, verified and documented, and it lists traps that will waste your time if you skip it.

This file applies to the whole repository. If a task contradicts it, follow the task — but say so
explicitly in your report, and state what you skipped and why.

---

## 1. What this repository is

MLBV is a Minecraft launcher: **Tauri 2** (Rust backend) + **React 19 / TypeScript** frontend,
packaged for Windows. It implements the Minecraft launcher protocol directly — version manifest,
version JSON, asset index, libraries, natives, classpath, JVM arguments — plus five mod loaders
(Fabric, Quilt, Forge, NeoForge, vanilla), LiquidBounce, Java provisioning, a configs catalog and a
self-update check.

Facts about the current state:

- **Status: frozen.** Last commit and release: `beta0.0.4`, dated 2026-06-14. There is no active
  development. Do not assume features are being worked on.
- 15 commits total, written in a single two-day sprint. History is squashed and unhelpful.
- ~9 300 lines of first-party code: ~4 400 Rust, ~4 900 TypeScript/CSS.
- **No tests, no linter, no formatter config.** CI compiles; it does not verify behaviour.
- All published releases are marked as GitHub *pre-releases*, and the in-app updater can never see
  them (see §6, landmine 9).

### File map

| Path | Lines | Role |
|---|---|---|
| `src-tauri/src/lib.rs` | ~1140 | Tauri commands: Microsoft auth, LiquidBounce API, update check, console window, instance scanning, mods, download controls, `run()` and the `generate_handler!` list |
| `src-tauri/src/launcher.rs` | ~3250 | The launch pipelines (`run`, `run_lb`, `run_fabric`, `run_quilt`, `run_forge`, `run_neoforge`), Java provisioning, ZIP extraction, path helpers |
| `src-tauri/src/main.rs` | 6 | Windows entry point. Contains `windows_subsystem` — **do not touch** |
| `src-tauri/tauri.conf.json` | 40 | Window, bundle targets, CSP, identifier |
| `src-tauri/capabilities/*.json` | 34 | Tauri v2 permissions for the `main` and `console` windows |
| `src/App.tsx` | ~2880 | The entire main UI, including every modal |
| `src/SetupWizard.tsx` | ~550 | First-run wizard: language → prefs → account → Java |
| `src/ConsoleWindow.tsx` | ~185 | Separate window that streams game output |
| `src/LbConfigsPanel.tsx` | ~465 | LiquidBounce configs catalog (GitHub-backed) |
| `src/i18n.ts` | ~700 | 514 EN/RU translation keys |
| `src/App.css` | ~1340 | All styling |
| `src/main.tsx` | 25 | Picks `App` or `ConsoleWindow` by window label |
| `.github/workflows/ci.yml` | 112 | The only way to compile Rust in a restricted environment (§4) |

## 2. Hard rules

**Always**

- Verify before claiming. Every statement about the code must be traceable to a file and line. If
  you did not run it, say "not verified" instead of implying you did.
- Keep changes minimal and scoped to the request. No drive-by reformatting, renaming or
  "while I was here" edits.
- Use `cargo check --locked` and keep `Cargo.lock` / `package-lock.json` committed and in sync.
- Write user-facing strings through `src/i18n.ts` in **both** `en` and `ru`.
- Keep TypeScript strict-clean: `npm run build` runs `tsc` with `strict`, `noUnusedLocals` and
  `noUnusedParameters`. `any` is not used anywhere in the codebase; keep it that way.
- Report honestly at the end: what you changed, what you verified and how, what you assumed, what
  you could not check.

**Never**

- **Never open a pull request, push to `main`, or comment on issues/PRs unless the user explicitly
  asked for that exact action in that exact message.** Agents running in a web session must not
  self-initiate PRs. Pushing to the branch the session was given is fine; everything else is not.
- Never rewrite published history (`push --force` only ever to your own working branch, and only
  when the user agreed to it).
- Never commit `node_modules/`, `dist/`, `src-tauri/target/`, `src-tauri/gen/`, installers or logs.
  `.gitignore` already covers them.
- Never bump the version, create a tag or publish a release unless explicitly asked (§5).
- Never add a dependency (npm or crate) without stating why an existing one cannot do the job.
- Never invent API endpoints, JSON field names or protocol details. Mojang/loader APIs are exact;
  a guessed field name fails silently at runtime. Look it up (§7).
- Never delete or modify user data directories in tests or scripts: `%APPDATA%\mlbv`,
  `~/.mlbv`, or the user's real `.minecraft`.
- Never log, print or transmit access tokens, refresh tokens or `client_id`-derived secrets.
- Never "fix" the LiquidBounce/cheat aspect of the project or add anything that circumvents
  Minecraft licensing or authentication. That is out of scope, permanently.
- Never state that something "works" based on compilation alone. Compiling ≠ working.

## 3. Before you change anything

1. Read the file you are about to edit **in full**, plus the `lib.rs` `generate_handler!` list to
   see which commands actually exist.
2. Check whether the frontend and backend agree. Commands are invoked by name from
   `src/**/*.tsx`; the frontend is not type-checked against Rust, so a rename breaks the app
   without any compile error. grep for the command name in both quote styles
   (`invoke('cmd'` and `invoke("cmd"`) before and after renaming anything.
3. Look for an existing helper. `launcher.rs` has the download/Java/ZIP helpers; do not introduce
   a second implementation (the six pipelines already suffer from exactly that).
4. If the change touches the launch flow, remember all six pipelines exist and ask the user whether
   the fix is meant for one loader or all of them.

## 4. How to build and verify

**The verification matrix — run everything you can, report what you skipped:**

```bash
npm ci                                  # install frontend deps
npx tsc --noEmit                        # type-check
npm run build                           # tsc + vite → dist/

cd src-tauri
cargo check --locked --all-targets       # compile the backend
```

Full installer build (Windows only, slow):

```bash
npm run tauri build -- --bundles nsis,msi   # → src-tauri/target/release/bundle/
```

UI-only preview (useful for layout, **not** for behaviour):

```bash
npm run dev        # vite on :1420; in a browser every invoke() fails, the app fakes progress steps
npm run tauri dev  # real desktop app with hot reload
```

### Restricted and offline environments

- **Rust is often unavailable.** In a container without `cargo`, do not conclude the code is
  broken: install the toolchain, or use CI as the compiler. `rustup.rs`,
  `static.rust-lang.org` and `crates.io` are commonly blocked even when `github.com` works; apt
  repositories and container registries may be blocked too. Check what is reachable
  (`curl -s -o /dev/null -w '%{http_code}' <url>`) before promising a build.
- **CI is the reliable compiler.** Push the workflow to a working branch and let GitHub run
  `cargo check` and the Windows installer build. Read statuses with
  `gh run view <id> --json jobs --jq '.jobs[] | "\(.name) \(.conclusion)"'` and per-step detail with
  `gh api repos/<owner>/<repo>/actions/jobs/<job_id> --jq '.steps[] | "\(.conclusion) \(.name)"'`.
  Log bytes and artifacts are served from `*.blob.core.windows.net`, which restricted sandboxes
  frequently block — statuses still work, raw logs may not. Say so instead of guessing at warnings.
- **Never treat a blocked host as a broken service.** `launchermeta.mojang.com`,
  `api.adoptium.net`, `api.modrinth.com`, `api.liquidbounce.net` and friends are routinely
  unreachable from sandboxes. Verify against documentation and report the limitation.
- The Windows build in CI produces real installers; downloading them may fail in a sandbox even
  though the build succeeded. Check artifact size via the API
  (`gh api repos/<owner>/<repo>/actions/runs/<id>/artifacts`) rather than assuming.

## 5. Version numbers live in many places

The version is duplicated and they are **not** in sync today. If a task requires a version bump,
update all of them and say so:

| File | Field | Current value |
|---|---|---|
| `package.json` | `version` | `beta0.0.4` |
| `package-lock.json` | `version` (2 places) | `0.0.4` — stale, `npm install` rewrites it |
| `src-tauri/tauri.conf.json` | `version` | `0.0.4` |
| `src-tauri/Cargo.toml` | `version` | `0.0.4` (this is what `env!("CARGO_PKG_VERSION")` reports, and it drives the update check) |
| `vite.config.ts` | `define.__APP_VERSION__` | `"0.0.4"` — hardcoded, shown in the About panel |

Note the asymmetry: the About screen shows `__APP_VERSION__` (hardcoded in Vite), the debug panel
shows `CARGO_PKG_VERSION`. A bump that misses either one produces two different versions in one UI.

Releases are tagged `v0.0.x` or `beta0.0.x`; every release so far is a GitHub pre-release. The
updater downloads `.exe` assets only.

## 6. Landmines — verified problems, do not rediscover them

Cite these instead of re-deriving, and fix one only if the task asks for it.

1. **Microsoft tokens are never refreshed.** `refresh_token` is read (`lib.rs:113`) and returned to
   the frontend (`lib.rs:184`), then dropped in `App.tsx:1783`; no refresh call exists anywhere.
   Sessions die after ~24 h and the user must sign in again.
2. **Instances are only in `localStorage`** (`App.tsx:1648`). `scan_instances` (`lib.rs:776`) and
   `save_instance_metadata` (`lib.rs:814`) are implemented and never called; `.mlbv-instance.json`
   is never written. Clearing WebView data loses the instance list while files stay on disk.
3. **`instance_name` is unvalidated and used as a path segment.** It comes from `localStorage` and
   reaches `join()` and `remove_dir_all` (`delete_instance_data` `lib.rs:910`, `reinstall_instance`
   `lib.rs:383`). File names are sanitised (`add_mod_file`), instance names are not. Any fix here is
   security-relevant; add a whitelist in Rust, not only in the UI.
4. **Downloads are unverified and buffered in RAM.** `download_file` (`launcher.rs:2773`) reads the
   whole body into memory, `is_valid_file` (`launcher.rs:2765`) compares size only — the SHA-1
   values in the manifests are ignored. Several call sites swallow errors with
   `let _ = download_file(...)` (e.g. `launcher.rs:396`), which turns network failures into
   confusing in-game crashes.
5. **Live download speed only counts assets.** `dl_bytes` is incremented solely inside
   `download_assets_parallel` (`launcher.rs:257`); libraries, Java, JARs and mods are invisible to
   the speed readout.
6. **51 of 75 progress strings in `launcher.rs` are hardcoded Russian** (e.g. `launcher.rs:869`,
   `894`, `896`, `2647`, `2677`). The app has 514 i18n keys; the backend bypasses them. Any
   user-visible string added in Rust must be discussed with the user, since there is no mechanism
   to translate it.
7. **Six near-identical pipelines** — `run` (`launcher.rs:272`), `run_lb` (`674`),
   `run_fabric` (`1079`), `run_quilt` (`1449`), `run_forge` (`1766`),
   `run_neoforge` (`2187`). ~2 000 of 3 250 lines are duplication, and fixes land in some copies
   only. Consolidate only when asked; if asked, do it as one mechanical change with a diff review.
8. **Legacy assets are not mapped.** Nothing copies objects into `assets/virtual/legacy/` for
   pre-1.7 versions (grep for `virtual` returns nothing), so old versions can start without
   textures or sounds. Also, `assetIndex` and `downloads` in `VersionJson` (`launcher.rs:63,64`)
   are non-optional — verify against real Mojang JSON before changing their types.
9. **The in-app update check can never fire.** `version_type("0.0.4")` classifies the running build
   as a *release* (`lib.rs:424`), so `check_for_update` filters out every release that is marked
   pre-release (`lib.rs:455`). All four published releases are pre-releases, so the command returns
   `Err("No releases found")`, and the frontend swallows it (`.catch(() => {})`, `App.tsx:1767`).
10. **One game process at a time.** `GameState` holds a single `child` slot (`launcher.rs:13`);
    starting a second instance orphans the first from the UI.
11. **No CSP, devtools in release builds, third-party Markdown rendered as HTML.**
    `csp: null` (`tauri.conf.json:26`), `features = ["devtools"]` (`Cargo.toml:21`),
    `dangerouslySetInnerHTML` on rendered README content (`LbConfigsPanel.tsx:350`) produced with
    unsanitised `marked` (`LbConfigsPanel.tsx:71`). Do not widen this surface.
12. **`poll_jvm_output` is unbounded.** `GameState.jvm_lines` grows forever; long sessions
    accumulate memory.

### Dead code — do not assume it is wired up

Commands present in `generate_handler!` but never invoked from the frontend:
`check_version_installed` (`lib.rs:276`), `poll_console` (`lib.rs:660`, superseded by
`poll_jvm_output`), `scan_instances` (`lib.rs:776`), `save_instance_metadata` (`lib.rs:814`).
Frontend leftovers: `public/vite.svg`, `public/tauri.svg`, `src/assets/react.svg` are unused
template files.

## 7. Sources to consult when unsure

Search the web instead of guessing. Prefer primary sources and cite them in your report.

- Minecraft launcher protocol / version JSON: <https://minecraft.wiki/w/Client.json>
  (shape of `libraries[].rules`, `natives`, `arguments`, `assetIndex`, `downloads`)
- Version manifest: `https://launchermeta.mojang.com/mc/game/version_manifest_v2.json`
- Fabric: `https://meta.fabricmc.net/v2/` · Quilt: `https://meta.quiltmc.org/v3/` ·
  Forge/NeoForge: their `maven-metadata.xml` files
- Adoptium: <https://api.adoptium.net/q/swagger-ui/> · Modrinth: <https://docs.modrinth.com/>
- LiquidBounce API: <https://api.liquidbounce.net/api/v1/version/branches>
- Tauri v2: <https://v2.tauri.app/> (capabilities, `on_navigation`, `on_download`)
- Microsoft/Xbox auth chain: documented community write-ups for `oauth20_desktop.srf`,
  `user.auth.xboxlive.com`, `xsts.auth.xboxlive.com`, `login_with_xbox`

When a third-party API matters to a change, verify it with a request if the network allows, or
quote the documentation. "I believe the field is called …" is not acceptable.

## 8. Conventions in this codebase

- **Section headers.** Both Rust and TS files use `// ─── Section ───────` banners. Keep them.
- **Comments explain decisions, not syntax** — e.g. why `/assets/latest/` is avoided in
  `ensure_java`. Follow that style; do not add noise comments.
- **Error handling.** Rust uses `anyhow` with `.context()` in the launcher and `Result<_, String>`
  at the Tauri command boundary (`map_err(|e| format!("{e:#}"))`). Match it.
- **Windows-first checks** use `cfg!(windows)` at runtime; platform paths use `cfg!(target_os)`.
- **React state** is plain `useState`/`useEffect` with `framer-motion` for animation. No state
  library, no router — `main.tsx` routes by window label.
- **Styling** is one `App.css` with CSS custom properties for theming (`--accent`, `--lb-accent`,
  `--accent-rgb`). Do not add a CSS framework.
- **Persisted keys** are prefixed `mlbv_` (`mlbv_accounts`, `mlbv_instances`, `mlbv_accent`, …).
  Keep the prefix and do not rename existing keys without a migration.
- Commit messages: imperative subject, optionally a body explaining why. Do not add AI
  co-author trailers.

## 9. Definition of done

- [ ] `npx tsc --noEmit` and `npm run build` pass.
- [ ] `cargo check --locked` passes (locally or via CI — say which).
- [ ] New user-facing text exists in `en` **and** `ru` in `src/i18n.ts`.
- [ ] No new dead code, no new `let _ =` around fallible work without justification.
- [ ] If a command name, invocation argument or data path changed, frontend and backend were both
      updated and both were grepped for the old name.
- [ ] Docs updated when behaviour, paths, or limitations changed (`README.md`, this file).
- [ ] Report states: what changed (with files), what was verified and how, what was not verified,
      and any assumption made.

---

If something in this file turns out to be wrong, fix the file in the same change and say so.
