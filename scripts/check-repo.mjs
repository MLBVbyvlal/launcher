// ─── Repository invariants ────────────────────────────────────────────────────
//
// Zero-dependency checks for the things that break this app silently. Each one
// exists because that specific breakage already happened once:
//
//   1. i18n parity + no missing keys. en.ts/ru.ts drift by an edit to one file
//      (333 keys each today); a `t('key')` with no entry renders the raw key.
//   2. Frontend/backend command contract. Commands are matched by *name string*,
//      so a rename on one side compiles clean and fails at runtime.
//   3. No dead commands. An uninvoked command is how `vault_has_account` ended
//      up documented as wiring account pruning that the UI never did.
//   4. The 500-line file rule from AGENTS.md §2.
//   5. The version lives in five files and is bumped by hand (AGENTS.md §5);
//      a miss leaves two versions in one UI.
//
// Run: `npm run check:repo` (CI runs it in the frontend job).
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'

const ROOT = new URL('..', import.meta.url).pathname
const LINE_LIMIT = 500

const walk = (dir, out = []) => {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) walk(p, out)
    else out.push(p)
  }
  return out
}
const rel = p => relative(ROOT, p)
const read = p => readFileSync(p, 'utf8')

const problems = []
const fail = msg => problems.push(msg)

// ── 1. i18n ───────────────────────────────────────────────────────────────────
const i18nDir = join(ROOT, 'src/i18n')
const keySet = src => {
  const keys = new Set()
  for (const line of src.split('\n')) {
    const m = /^\s*'([^']+)':/.exec(line)
    if (m) keys.add(m[1])
  }
  return keys
}
const en = keySet(read(join(i18nDir, 'en.ts')))
const ru = keySet(read(join(i18nDir, 'ru.ts')))
for (const k of [...en].filter(k => !ru.has(k))) fail(`i18n: '${k}' is in en.ts but missing from ru.ts`)
for (const k of [...ru].filter(k => !en.has(k))) fail(`i18n: '${k}' is in ru.ts but missing from en.ts`)

// ── 2–3. the command contract ─────────────────────────────────────────────────
const rustFiles = walk(join(ROOT, 'src-tauri/src')).filter(p => p.endsWith('.rs'))
const defined = new Map()
for (const p of rustFiles) {
  const src = read(p)
  for (const m of src.matchAll(/#\[tauri::command\][\s\S]{0,400}?\bfn\s+(\w+)/g)) {
    defined.set(m[1], rel(p))
  }
}
const libSrc = read(join(ROOT, 'src-tauri/src/lib.rs'))
const handlerBlock = /generate_handler!\[([\s\S]*?)\]/.exec(libSrc)
if (!handlerBlock) fail('lib.rs: no generate_handler! block found — the checker is out of date')
const registered = new Set(
  [...(handlerBlock ? handlerBlock[1].matchAll(/(\w+)(?=\s*,)/g) : [])].map(m => m[1]),
)
for (const name of registered) {
  if (!defined.has(name)) fail(`command '${name}' is registered in lib.rs but no #[tauri::command] fn of that name exists`)
}
for (const [name, where] of defined) {
  if (!registered.has(name)) fail(`#[tauri::command] in ${where} is not registered in generate_handler! — the app cannot call '${name}'`)
}

const frontend = walk(join(ROOT, 'src')).filter(p => /\.tsx?$/.test(p))

/// Name of the command in each `invoke('name', …)` / `invoke<...>('name', …)`.
/// Written as a scanner because the generic argument is nested
/// (`invoke<Record<string, unknown>>(…)`) and no single regex stays correct.
const invokesIn = src => {
  const names = []
  for (const m of src.matchAll(/\binvoke\b/g)) {
    let i = m.index + m[0].length
    if (src[i] === '<') {
      let depth = 0
      for (; i < src.length; i++) {
        if (src[i] === '<') depth++
        else if (src[i] === '>') { depth--; if (depth === 0) { i++; break } }
      }
    }
    while (i < src.length && /\s/.test(src[i])) i++
    if (src[i] !== '(') continue
    const rest = src.slice(i + 1, i + 40)
    const lit = /^\s*['"]([\w]+)['"]/.exec(rest)
    if (lit) names.push(lit[1])
  }
  return names
}

const invoked = new Map()
for (const p of frontend) {
  for (const name of invokesIn(read(p))) if (!invoked.has(name)) invoked.set(name, rel(p))
}
for (const [name, where] of invoked) {
  if (!registered.has(name)) fail(`${where} invokes '${name}', which is not registered in lib.rs`)
}
for (const name of registered) {
  if (!invoked.has(name)) fail(`command '${name}' is registered but never invoked from src/ — invoke it or delete it (AGENTS.md §6, "Dead code")`)
}

// Every t('…') literal must resolve.
const tMissing = new Set()
for (const p of frontend) {
  for (const m of read(p).matchAll(/\bt\(\s*'([^']+)'\s*\)/g)) {
    if (!en.has(m[1])) tMissing.add(`${rel(p)} → ${m[1]}`)
  }
}
for (const item of tMissing) fail(`i18n: no key for t('${item.split(' → ')[1]}') used in ${item.split(' → ')[0]}`)

// ── 4. file size ──────────────────────────────────────────────────────────────
const sourceFiles = [
  ...walk(join(ROOT, 'src')),
  ...walk(join(ROOT, 'src-tauri/src')),
].filter(p => /\.(rs|ts|tsx|css)$/.test(p))
for (const p of sourceFiles) {
  const lines = read(p).split('\n').length
  if (lines > LINE_LIMIT) fail(`${rel(p)} is ${lines} lines — the limit is ${LINE_LIMIT} (AGENTS.md §2); split it in the same change`)
}

// ── 5. version sync ───────────────────────────────────────────────────────────
const versions = {
  'package.json': /"version":\s*"([^"]+)"/.exec(read(join(ROOT, 'package.json')))?.[1],
  'package-lock.json': /"version":\s*"([^"]+)"/.exec(read(join(ROOT, 'package-lock.json')))?.[1],
  'src-tauri/tauri.conf.json': /"version":\s*"([^"]+)"/.exec(read(join(ROOT, 'src-tauri/tauri.conf.json')))?.[1],
  'src-tauri/Cargo.toml': /^version\s*=\s*"([^"]+)"/m.exec(read(join(ROOT, 'src-tauri/Cargo.toml')))?.[1],
  'vite.config.ts': /__APP_VERSION__:\s*JSON\.stringify\("([^"]+)"\)/.exec(read(join(ROOT, 'vite.config.ts')))?.[1],
}
const values = new Set(Object.values(versions))
if (values.size !== 1 || values.has(undefined)) {
  fail(`version drift (AGENTS.md §5): ${Object.entries(versions).map(([k, v]) => `${k}=${v ?? '?'}`).join(', ')}`)
}

// ── report ────────────────────────────────────────────────────────────────────
if (problems.length > 0) {
  console.error(`✗ repo invariants: ${problems.length} problem(s)`)
  for (const p of problems) console.error(`  - ${p}`)
  process.exit(1)
}
console.log(
  `✓ repo invariants — ${en.size} i18n keys in both locales, ` +
  `${registered.size} commands registered and all of them invoked, ` +
  `${sourceFiles.length} files within ${LINE_LIMIT} lines, version ${versions['package.json']} everywhere`,
)
