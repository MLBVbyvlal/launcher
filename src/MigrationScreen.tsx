import { useState } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { getLang, useT } from './i18n'
import type { Instance, MCVersion } from './App'

export interface GarbageItem {
  path: string
  bytes: number
  kind: string
}

export interface MigrationScanT {
  garbage: GarbageItem[]
  total_bytes: number
  has_instances: boolean
  has_versions: boolean
}

export interface PinPreview {
  id: string
  name: string
  /** Concrete version the instance will be pinned to ('' = LB head, resolved at run). */
  to: string
}

type RawBuild = { build_id: number; lb_version: string; mc_version: string }

const spring = { type: 'spring', stiffness: 400, damping: 30 } as const

function fmtBytes(b: number): string {
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(0)} KB`
  if (b < 1024 * 1024 * 1024) return `${(b / (1024 * 1024)).toFixed(1)} MB`
  return `${(b / (1024 * 1024 * 1024)).toFixed(2)} GB`
}

// Pin one rolling instance to a concrete version. MC pins to its installed
// hint (validated against the manifest); LB resolves the branch head, falling
// back to the last launched build when offline.
async function resolvePin(inst: Instance, versions: MCVersion[]): Promise<Instance> {
  if (inst.type === 'mc') {
    const ids = new Set(versions.map(v => v.id))
    const hint = inst.mcVersion
    if (hint && hint !== 'latest' && ids.has(hint)) return { ...inst, version: hint }
    const lastKnown = localStorage.getItem('mlbv_last_mc_latest') ?? ''
    if (lastKnown && ids.has(lastKnown)) return { ...inst, version: lastKnown, mcVersion: lastKnown }
    const rel = versions.find(v => v.type === 'release')
    if (rel) return { ...inst, version: rel.id, mcVersion: rel.id }
    throw new Error(hint && hint !== 'latest' ? hint : 'unknown version')
  }
  // LB instances never stored their branch; latest always resolved nextgen.
  try {
    const builds = await invoke<RawBuild[]>('get_lb_versions', { branch: 'nextgen' })
    const head = builds[0]
    if (head) return { ...inst, version: head.lb_version, mcVersion: head.mc_version, buildId: head.build_id }
  } catch { /* offline → fall through to last-known */ }
  const tag = localStorage.getItem('mlbv_last_lb_latest') ?? ''
  const bid = Number(localStorage.getItem('mlbv_last_lb_latest_buildid') ?? '0')
  const mcv = localStorage.getItem('mlbv_last_lb_latest_mcver') ?? ''
  if (tag && bid && mcv) return { ...inst, version: tag, mcVersion: mcv, buildId: bid }
  throw new Error('lb-offline')
}

const KIND_ORDER = ['natives', 'inst_versions', 'part', 'temp']

export default function MigrationScreen({ pins, scan, versions, instances, onApplyPins, onDone }: {
  pins: PinPreview[]
  scan: MigrationScanT
  versions: MCVersion[]
  instances: Instance[]
  onApplyPins: (updated: Instance[]) => void
  onDone: () => void
}) {
  const t = useT(getLang())
  const [phase, setPhase] = useState<'plan' | 'running' | 'done' | 'error'>('plan')
  const [step, setStep] = useState(0)
  const [error, setError] = useState('')
  const [freed, setFreed] = useState(0)

  const emptyPlan = pins.length === 0 && scan.garbage.length === 0
  const steps = [t('migrate.step_pin'), t('migrate.step_clean'), t('migrate.step_verify')]

  const run = async () => {
    setPhase('running')
    setError('')
    try {
      // 1. Pin every rolling instance (idempotent: concrete ones pass through).
      setStep(0)
      const updated = [...instances]
      for (const inst of instances) {
        if (inst.version !== 'latest') continue
        try {
          const pinned = await resolvePin(inst, versions)
          const i = updated.findIndex(u => u.id === inst.id)
          updated[i] = pinned
        } catch (e) {
          const reason = e instanceof Error && e.message === 'lb-offline'
            ? t('migrate.err_lb_offline')
            : String(e)
          throw new Error(t('migrate.error_pin').replace('{0}', inst.name).replace('{1}', reason))
        }
      }
      onApplyPins(updated)

      // 2. Delete the stale files found by the scan.
      setStep(1)
      const report = await invoke<{ deleted: string[]; freed_bytes: number; errors: string[] }>('migration_clean')
      setFreed(report.freed_bytes)

      // 3. Verify: no rolling versions left, no stale version dirs left.
      //    temp/.part leftovers are hygiene, not the fix — they don't block.
      setStep(2)
      const rescan = await invoke<MigrationScanT>('migration_scan')
      const critical = rescan.garbage.filter(g => g.kind === 'natives' || g.kind === 'inst_versions')
      if (updated.some(u => u.version === 'latest')) {
        throw new Error(t('migrate.err_verify').replace('{0}', 'latest'))
      }
      if (critical.length > 0) {
        const what = critical[0].path + (critical.length > 1 ? ` (+${critical.length - 1})` : '')
        throw new Error(t('migrate.err_verify').replace('{0}', what))
      }
      await invoke('set_data_version')
      ;['mlbv_last_mc_latest', 'mlbv_last_lb_latest', 'mlbv_last_lb_latest_buildid', 'mlbv_last_lb_latest_mcver']
        .forEach(k => localStorage.removeItem(k))
      setPhase('done')
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      setPhase('error')
    }
  }

  const grouped = KIND_ORDER
    .map(kind => ({ kind, items: scan.garbage.filter(g => g.kind === kind) }))
    .filter(g => g.items.length > 0)

  return (
    <div className="migrate-root">
      <div className="bg-canvas" style={{ pointerEvents: 'none', position: 'fixed' }}>
        <div className="bg-grid" />
        <div className="orb orb-1" />
        <div className="orb orb-2" />
      </div>
      <motion.div className="modal glass migrate-card"
        initial={{ opacity: 0, scale: 0.94, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        transition={spring}
      >
        <div className="migrate-head">
          <span className="migrate-icon">📦</span>
          <div>
            <div className="migrate-title">{t('migrate.title')}</div>
            <div className="migrate-subtitle">{t('migrate.subtitle')}</div>
          </div>
        </div>

        {phase === 'plan' && <>
          <div className="migrate-section">
            <div className="setting-label">{t('migrate.what')}</div>
            <ul className="migrate-list">
              <li>{t('migrate.p1')}</li>
              <li>{t('migrate.p2')}</li>
              <li>{t('migrate.p3')}</li>
            </ul>
          </div>
          {emptyPlan ? (
            <div className="migrate-empty">{t('migrate.plan_empty')}</div>
          ) : <>
            {pins.length > 0 && (
              <div className="migrate-section">
                <div className="setting-label">{t('migrate.plan_pin')}</div>
                <div className="migrate-rows">
                  {pins.map(p => (
                    <div key={p.id} className="migrate-row">
                      <span className="migrate-row-name">{p.name}</span>
                      <span className="migrate-row-arrow">Latest → {p.to || t('migrate.lb_head')}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
            {grouped.length > 0 && (
              <div className="migrate-section">
                <div className="setting-label">
                  {t('migrate.plan_clean')} · {fmtBytes(scan.total_bytes)}
                </div>
                <div className="migrate-rows migrate-scroll">
                  {grouped.map(g => (
                    <div key={g.kind}>
                      <div className="migrate-kind">
                        {t(`migrate.kind.${g.kind}`)} · {fmtBytes(g.items.reduce((a, i) => a + i.bytes, 0))}
                      </div>
                      {g.items.slice(0, 6).map(i => (
                        <div key={i.path} className="migrate-row migrate-row-dim">
                          <span className="migrate-row-name">{i.path}</span>
                          <span>{fmtBytes(i.bytes)}</span>
                        </div>
                      ))}
                      {g.items.length > 6 && (
                        <div className="migrate-row migrate-row-dim">+{g.items.length - 6}</div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>}
          <button className="btn-ok migrate-btn" onClick={run}>
            {emptyPlan ? t('migrate.continue') : t('migrate.convert')}
          </button>
        </>}

        {phase === 'running' && (
          <div className="migrate-section">
            {steps.map((label, i) => (
              <div key={label} className={`migrate-step${i < step ? ' done' : i === step ? ' active' : ''}`}>
                <span className="migrate-step-dot">{i < step ? '✓' : i === step ? '…' : '○'}</span>
                {label}
              </div>
            ))}
          </div>
        )}

        {phase === 'done' && <>
          <div className="migrate-done">✓ {t('migrate.done').replace('{0}', fmtBytes(freed))}</div>
          <div className="migrate-kept">{t('migrate.kept')}</div>
          <button className="btn-ok migrate-btn" onClick={onDone}>{t('migrate.enter')}</button>
        </>}

        {phase === 'error' && <>
          <div className="migrate-error">{t('error.prefix')} {error}</div>
          <button className="btn-retry migrate-btn" onClick={run}>{t('error.retry')}</button>
        </>}
      </motion.div>
    </div>
  )
}
