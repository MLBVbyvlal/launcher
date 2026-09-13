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

- **Status: frozen.** Last *release*: `beta0.0.4`, dated 2026-06-14. No feature development;
  maintenance fixes (security, data integrity, broken updater) and the launch-pipeline
  consolidation were applied on 2026-09-12 — see §6 for what was fixed and what is still open.
- History is squashed and unhelpful (original sprint: 15 commits over two days).
- ~9 600 lines of first-party code: ~3 400 Rust, ~6 200 TypeScript/CSS
  (measured 2026-09-12 after the launch-pipeline consolidation).
- **No tests, no linter, no formatter config.** CI compiles; it does not verify behaviour.
- All published releases are marked as GitHub *pre-releases*. The updater now considers them
  (fixed 2026-09-12; see §6, landmine 9).

### File map

| Path | Lines | Role |
|---|---|---|
| `src-tauri/src/lib.rs` | ~1110 | Tauri commands: Microsoft auth + refresh, LiquidBounce API, update check, console window, instance scanning/metadata, mods, download controls, the single `launch_game` command and the `generate_handler!` list |
| `src-tauri/src/launcher.rs` | ~2290 | The launch pipeline (`launch` + the loader steps `prepare_loader` / `prepare_loader_stage`), per-instance process registry, Java provisioning, verified streaming downloads, legacy asset mapping, ZIP extraction, path helpers, `valid_instance_name` |
| `src-tauri/src/main.rs` | 6 | Windows entry point. Contains `windows_subsystem` — **do not touch** |
| `src-tauri/tauri.conf.json` | 40 | Window, bundle targets, CSP, identifier |
| `src-tauri/capabilities/*.json` | 34 | Tauri v2 permissions for the `main` and `console` windows |
| `src/App.tsx` | ~2920 | The entire main UI, including every modal |
| `src/SetupWizard.tsx` | ~545 | First-run wizard: language → prefs → account → Java |
| `src/ConsoleWindow.tsx` | ~190 | Separate window that streams the output of one instance |
| `src/LbConfigsPanel.tsx` | ~470 | LiquidBounce configs catalog (GitHub-backed, README sanitized with DOMPurify) |
| `src/i18n.ts` | ~705 | 258 keys per language, 516 total |
| `src/App.css` | ~1340 | All styling |
| `src/main.tsx` | 25 | Picks `App` or `ConsoleWindow` by window label |
| `.github/workflows/ci.yml` | 112 | The only way to compile Rust in a restricted environment (§4) |

## 2. Hard rules

**Always**

- Verify before claiming. Every statement about the code must be traceable to a file and line. If
  you did not run it, say "not verified" instead of implying you did.
- **Re-verify anything you are not certain about.** A little doubt is a signal to check, not to
  phrase more carefully. See §2.2 — a wrong claim costs far more than a slow answer.
- **Treat this repository as public and permanent.** No secrets, no personal data, ever — see §2.1.
- Keep changes minimal and scoped to the request. No drive-by reformatting, renaming or
  "while I was here" edits.
- Use `cargo check --locked` and keep `Cargo.lock` / `package-lock.json` committed and in sync.
- Write user-facing strings through `src/i18n.ts` in **both** `en` and `ru`.
- Keep TypeScript strict-clean: `npm run build` runs `tsc` with `strict`, `noUnusedLocals` and
  `noUnusedParameters`. `any` is not used anywhere in the codebase; keep it that way.
- Report honestly at the end, labelling each claim: what you changed (with files), what you verified
  and **how**, what you only reasoned about, and what you could not check at all.

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
- Never commit a secret or anything personal — see §2.1. Never invent, guess at or embellish
  evidence — see §2.2.

### 2.1 This repository is public — no secrets, no personal data

Every byte pushed here is public **permanently**. GitHub releases are mirrored, forked, cached and
archived by third parties within minutes; deleting the file or rewriting history does not un-leak
anything. There is no "I'll clean it up later". This applies to commits, workflow files, docs,
screenshots, issue and PR text, and CI logs alike.

Prohibited, without exception and without "it's only a test value":

- Tokens and credentials of any kind: Microsoft/Minecraft access and refresh tokens, session IDs,
  OAuth authorization codes, API keys, webhook URLs, SSH or GPG private keys, certificates,
  `.netrc`, `credentials.json`, cookie jars, password manager exports.
- `.env` files under any name — `.env`, `.env.local`, `.env.production`, secrets in `*.json` next
  to config, `launcher_accounts.json`-style account stores.
- Personal data: real Minecraft / Microsoft / Xbox nicknames, e-mail addresses, phone numbers,
  real account UUIDs, IP addresses and server addresses, machine paths that contain the owner's
  real name (`C:\Users\<real-name>\…`), chat logs, other people's files or screenshots.
- Logs and dumps: `*.log`, crash reports, `latest.log`, JVM dumps, browser profiles — they contain
  tokens, UUIDs and paths.
- Anything whose ownership or licence you are unsure of.

Practical rules:

- **Screenshots are committed data that no `grep` can check.** Before adding any image or video,
  open it and look for nicknames, avatars, UUIDs, file paths, console output, open tabs, server
  addresses, notification popups. When in doubt, pixelate it or re-take it with a scratch profile.
  The screenshots in `docs/screenshots/` were taken with deliberately fake accounts — keep that
  practice: never take a screenshot for this repository while signed into a real account, and never
  include a real nickname, avatar or UUID that you did not create for the purpose.
- **Check `.gitignore` before generating artifacts, not after.** `node_modules/`, `dist/`,
  `src-tauri/target/`, `src-tauri/gen/`, `*.exe`, `*.msi`, `*.log`, `.env*` are already covered; if
  you produce something new, add a rule in the same change.
- **Never paste secrets or personal data into issues, PR descriptions, commit messages or CI
  output.** Redact as `<redacted>` and reference the source instead.
- The bare `client_id = "00000000402b5328"` in `lib.rs` is Minecraft's public launcher client ID —
  it is not a secret. Its presence is not a precedent: do not add any other identifier, and never
  add a real application secret next to it.
- **If something did leak: rotate or revoke the credential first**, then remove it from the working
  tree, then tell the owner. Removing it from history is a separate, owner-approved step — never
  rewrite `main`, and never force-push anything but your own working branch (§2). Assume anything
  that was public for even a minute is compromised.
- If you are unsure whether something counts as personal — **ask before pushing**. Asking costs one
  message; leaking cannot be undone.

### 2.2 Verification discipline — no guessing, no invented evidence

Classify every factual claim you make by the strongest evidence you actually have:

| Level | Evidence | How to phrase it |
|---|---|---|
| 1 | You ran it and read the output / the CI status yourself | "verified: <command> → <result>" |
| 2 | You read it in the file | cite it, e.g. `src-tauri/src/lib.rs:438` |
| 3 | Official documentation | cite the URL |
| 4 | Inferred from 1–3 | "deduced from …, not observed directly" |
| 5 | Memory, habit, "it's probably" | **not a claim** — verify or label as unverified |

Non-negotiable:

- **Not sure means re-verify, not re-wording.** If you cannot verify it (blocked network, no
  toolchain, no access), say precisely what you could not check and why. "I could not verify this"
  is a complete, acceptable answer. A confident guess is not.
- **Never fabricate evidence.** No invented command output, no "CI is green" for a run you did not
  look at, no log lines you did not read, no invented test results, metrics or quotes, no citing a
  document you never opened. Fabricating or tampering with tool output is the worst possible
  failure here — worse than doing nothing.
- **Re-derive instead of trusting your own earlier statements**, including statements in this file.
  Files get edited and line numbers drift; a fact that was true last session may be false now.
  Re-check it before repeating it.
- **Identifiers come from a source, every time.** Versions, hashes, sizes, field names, CLI flags,
  permissions, defaults, paths: read them in the file or the official doc. "I believe the flag is
  `--foo`" is not acceptable — find it, or say you could not.
- **A compile is not behaviour.** `cargo check` and `npm run build` prove that things build. Only
  running the app proves anything runs. Never write "works", "fixed", "solved" on the strength of a
  green build; write "compiles" and say what remains unverified.
- **Verify your own documentation edits.** Line numbers, paths, commands and versions you just wrote
  are the most likely thing in your change to be wrong — re-open the target and confirm each one
  right before committing. This is cheap and it is skipped constantly.
- **Prefer primary sources, and two of them for protocol facts.** Minecraft JSON shape, loader APIs,
  OAuth endpoints, Tauri permissions: check the official documentation or the real endpoint.
  Forum and blog posts are a lead, not proof — mark them as such.
- **Never present a plan or an intention as a completed action.** If you are about to do something,
  say "next I will…", not "I did…".
- **When you turn out to be wrong, say so in plain words** and fix it in the same change — including
  the report, and including this file if this file is what was wrong. A silently corrected mistake
  is still a mistake the user will trust next time.

## 3. Before you change anything

1. Read the file you are about to edit **in full**, plus the `lib.rs` `generate_handler!` list to
   see which commands actually exist.
2. Check whether the frontend and backend agree. Commands are invoked by name from
   `src/**/*.tsx`; the frontend is not type-checked against Rust, so a rename breaks the app
   without any compile error. grep for the command name in both quote styles
   (`invoke('cmd'` and `invoke("cmd"`) before and after renaming anything.
3. Look for an existing helper. `launcher.rs` has the download/Java/ZIP helpers; do not introduce
   a second implementation (the six pipelines already suffer from exactly that).
4. If the change touches the launch flow, decide explicitly whether it belongs in the shared
   pipeline (`launch`, used by every loader) or in a loader step (`prepare_loader`,
   `prepare_loader_stage`, `download_profile_libraries`, `download_overlay_libraries`,
   `download_lb_mods`). Anything put in a loader step reaches **one** loader only — say which.

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
  `cargo check` and the Windows installer build. When `cargo check` fails, the rust-check job
  re-emits every compiler diagnostic as a check-run annotation (`Surface compiler errors as
  annotations` step), so the errors are readable without the log:
  `gh api repos/<owner>/<repo>/check-runs/<job_id>/annotations --paginate
  --jq '.[] | "\(.path):\(.start_line) \(.message)"'`.
  Read statuses with
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
| `package.json` | `version` | `0.0.4` (was `beta0.0.4`, aligned 2026-09-12) |
| `package-lock.json` | `version` (2 places) | `0.0.4` |
| `src-tauri/tauri.conf.json` | `version` | `0.0.4` |
| `src-tauri/Cargo.toml` | `version` | `0.0.4` (this is what `env!("CARGO_PKG_VERSION")` reports, and it drives the update check) |
| `vite.config.ts` | `define.__APP_VERSION__` | `"0.0.4"` — hardcoded, shown in the About panel |

The values are in sync as of 2026-09-12. Note the asymmetry: the About screen shows
`__APP_VERSION__` (hardcoded in Vite), the debug panel shows `CARGO_PKG_VERSION`. A bump that misses
either one produces two different versions in one UI — update all five places.

Releases are tagged `v0.0.x` or `beta0.0.x`; every release so far is a GitHub pre-release. The
updater downloads `.exe` assets only.

## 6. Landmines — verified problems, do not rediscover them

Cite these instead of re-deriving, and fix one only if the task asks for it.
Items marked **FIXED 2026-09-12** are resolved; the note explains the current mechanism so it is
not accidentally "re-fixed" into a regression.

1. **FIXED 2026-09-12 — Microsoft token refresh.** The refresh token is now stored in the account
   (`App.tsx` `Account.refreshToken`/`tokenAt`, same in `SetupWizard.tsx`), and `handlePlay`
   re-runs the chain via the `refresh_ms_token` command (`lib.rs`) when the session is >20 h old.
   `microsoft_login` and `refresh_ms_token` share `ms_token_chain`. Microsoft rotates refresh
   tokens — the frontend persists the new one from the response.
2. **FIXED 2026-09-12 — instance persistence.** `save_instance_metadata` is called on
   add/rename (`persistInstance` in `App.tsx`), `scan_instances` runs on startup and recovers
   directories missing from localStorage. Renaming also moves the on-disk directory
   (`rename_instance_data`, `lib.rs`). Residual: recovery without a metadata file is a filesystem
   guess (LB instances lose `buildId`).
3. **FIXED 2026-09-12 — instance name validation.** `launcher::valid_instance_name` (whitelist:
   ASCII alnum + space `_- . +`, length 1–64, no leading dot, no Windows device names even
   with an extension) is called at
   the entry of every command that takes `instance_name` (`lib.rs`). Do not move it to the UI.
4. **FIXED 2026-09-12 — verified streaming downloads.** `download_file` (`launcher.rs`) streams
   to a `.part` file, checks size + SHA-1 (when the manifest has one) and renames on success.
   Call sites propagate errors; the remaining `let _ =` sites are deliberate best-effort (single
   asset objects, optional extra LB mods) and carry a comment.
5. **FIXED 2026-09-12 — speed readout coverage.** Every streaming download (assets, libraries,
   natives, JARs, loader libs, mods, Java) increments the shared `dl_bytes` counter.
6. **FIXED 2026-09-12 — backend strings are English.** All hardcoded Russian user-visible strings
   in `launcher.rs` were translated to English (the backend has no i18n mechanism). The rule
   remains: any user-visible string added in Rust must be discussed with the user.
7. **FIXED 2026-09-12 — six near-identical pipelines.** There is one pipeline now:
   `launcher::launch` (`launcher.rs`) does the shared work (manifest, version JSON, client JAR,
   vanilla libraries, natives, assets, Java, argument building, spawn, exit watch) and the loaders
   only contribute an overlay: `prepare_loader` resolves what must happen *before* the vanilla
   download (Fabric/Quilt loader version, Forge/NeoForge installer, LB manifest) and
   `prepare_loader_stage` what happens after it (profile/overlay libraries, mods) and returns a
   `LaunchPlan` (`version_name`, `main_class`, extra classpath, overlay JVM/game args, extra
   substitution variables). `build_launch_args` assembles the command line for every loader, so an
   argument-order fix cannot land in one loader only. The six `launch_*_game` commands collapsed
   into one `launch_game(loader, …)`; the frontend has one `invoke` call. **Behaviour changes that
   came with it** (deliberate, and not runtime-verified — see the caveat below): the client JAR and
   version JSON are shared-only (no per-instance copy), natives are extracted per instance
   (`instances/<name>/natives/`, as in PrismLauncher), progress percentages are one schedule for
   all loaders, and conditional (rule-bearing) loader JVM args are now honoured for Fabric/Quilt
   too (previously only plain strings were pushed). **Still not parsed:** `arguments.game` in a
   Fabric/Quilt profile — `FabricArguments` only reads `jvm`, exactly as before the consolidation.
   Adding it is a two-line change, but neither loader API was reachable from the sandbox to
   confirm the shape, so it was left alone rather than guessed at.
8. **FIXED 2026-09-12 — legacy assets.** `AssetIndex` now parses `map_to_resources`, and
   `map_legacy_assets` (called from `download_assets_parallel`) hard-links each object into
   `assets/virtual/legacy/<index key>` for pre-1.7.3 indexes. `assetIndex`/`downloads` in
   `VersionJson` are non-optional — verify against real Mojang JSON before changing their types.
9. **FIXED 2026-09-12 — updater.** `check_for_update` (`lib.rs`) considers all non-draft
   releases, sorts by semver and returns the newest one newer than `CARGO_PKG_VERSION`;
   pre-release candidates set `unstable_warning`. Being up to date returns `None` (shown as
   "up to date"), not an error; real check failures are shown in Settings → About
   (no more silent `.catch(() => {})`).
10. **FIXED 2026-09-12 — one game process at a time.** `GameState` now holds
    `children: Mutex<HashMap<String, Child>>` and
    `jvm_buffers: Mutex<HashMap<String, Arc<Mutex<Vec<String>>>>>`, keyed by instance name, and
    `watch_exit` polls only its own instance. `stop_game(instance_name)` kills one game;
    `game-running` and `game-crashed` payloads carry `instance`; `poll_jvm_output` takes
    `instance_name`; the frontend keeps a `running: string[]` list instead of a single slot. After
    exit the buffer is trimmed to `JVM_TAIL_AFTER_EXIT` (2 000) lines.
    **Not runtime-verified** (no Windows machine in the sandbox): launching two instances at once,
    and the crash dialog for the second one. Still single by design: the download queue and its
    progress/speed events are global, so only one *launch* runs at a time, and there is still one
    console window (opening it for another instance closes the previous one).
11. **FIXED 2026-09-12 — CSP / devtools / Markdown.** CSP is set in `tauri.conf.json`
    (`default-src 'self'`, no inline scripts, fonts from Google Fonts allowed — the page loads
    Inter from fonts.googleapis.com); the `devtools` cargo feature is removed (release builds have
    no DevTools; `tauri dev` keeps it); config READMEs are sanitized with DOMPurify before
    `dangerouslySetInnerHTML` (`LbConfigsPanel.tsx` `renderMd`). **Not runtime-verified:** whether
    the configured CSP is also applied to the dev server pages (may degrade Vite fast-refresh in
    `tauri dev`) — smoke-test on Windows after this change.
12. **FIXED 2026-09-12 — bounded JVM output.** Each per-instance buffer in `GameState.jvm_buffers`
    is capped at `JVM_LINES_CAP` (20 000) via `jvm_push`; older lines are dropped from the front
    (full history stays in `latest.log`). The poll-offset protocol degrades gracefully (a stale
    offset simply reads as "caught up"), and polling an instance that was never launched returns an
    empty page instead of failing.

### Dead code — do not assume it is wired up

As of 2026-09-12: `check_version_installed`, `poll_console` and the five extra
`launch_*_game` commands were removed;
`scan_instances` / `save_instance_metadata` are now called from `App.tsx`. `public/vite.svg` is
the favicon (in use — do not delete); `public/tauri.svg` and `src/assets/react.svg` were removed
as unused template leftovers. If you add a command, invoke it or do not add it.

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
- [ ] Nothing secret or personal added: no tokens, keys, `.env`, real nicknames, e-mails, real
      paths, unreviewed screenshots or log files (§2.1). Re-check with a grep, and open any image.
- [ ] Every `file:line`, path, command, version and flag touched by the change was re-verified
      against the real file **after** editing it (§2.2).
- [ ] The report labels each statement: verified (with the command or source), deduced, or
      not verified — and names anything you could not check.

---

If something in this file turns out to be wrong, fix the file in the same change and say so.
The same goes for anything an agent recorded here from a previous session: this file is a claim like
any other, and claims get re-verified.
