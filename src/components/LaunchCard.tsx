import { motion } from 'framer-motion'
import { getLang, useT } from '../i18n'
import { spring } from '../lib/types'
import type { LaunchJob } from '../lib/useLaunchQueue'

// ─── Launch progress card ─────────────────────────────────────────────────────
//
// One card per launch, always naming the instance it belongs to, so a
// download can never be mistaken for the instance currently selected in
// the sidebar. Rendered in the launch zone of the tab that owns the instance.

export const fmtSpeed = (bps: number) => {
  if (bps <= 0) return ''
  if (bps < 1024) return `${bps} B/s`
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(0)} KB/s`
  return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`
}

function stageLabel(stage: string, t: (k: string) => string): string {
  switch (stage) {
    case 'fetch':    return t('launch.stage.fetch')
    case 'download': return t('launch.stage.download')
    case 'install':  return t('launch.stage.install')
    case 'wait':     return t('launch.stage.wait')
    case 'launch':   return t('launch.stage.launch')
    default:         return t('launch.stage.prepare')
  }
}

export function LaunchCard({ job, lb, onPause, onCancel }: {
  job: LaunchJob; lb: boolean
  onPause: () => void; onCancel: () => void
}) {
  const t = useT(getLang())
  const error = job.phase === 'error'
  const waiting = job.stage === 'wait'
  const cls = ['launch-card', 'glass', lb ? 'lb' : '', error ? 'error' : '', job.paused ? 'paused' : ''].filter(Boolean).join(' ')
  return (
    <motion.div className={cls}
      initial={{ opacity: 0, scale: 0.94, y: 8 }} animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.94, y: 8 }} transition={spring}
    >
      <div className="lc-head">
        <div className="lc-title">
          <span className="lc-inst">{job.instance.name}</span>
          <span className="lc-ver">{job.instance.type === 'lb' ? 'LiquidBounce · ' : ''}MC {job.instance.mcVersion}</span>
        </div>
        <span className={`lc-stage${job.paused ? ' paused' : ''}${waiting ? ' waiting' : ''}`}>
          {error ? t('launch.stage.error') : job.paused ? t('dl.paused') : stageLabel(job.stage, t)}
        </span>
      </div>

      <div className="lc-status" title={job.status}>{job.status}</div>

      <div className="lc-track">
        <motion.div className="lc-fill" animate={{ width: `${job.progress}%` }} transition={{ duration: 0.3, ease: 'easeOut' }} />
        {!error && !job.paused && <div className="lc-shimmer" />}
      </div>

      <div className="lc-foot">
        <span className="lc-pct">{job.progress}%</span>
        <span className="lc-speed">{job.paused ? '' : fmtSpeed(job.bps)}</span>
        {!error && (
          <div className="lc-actions">
            <button className="lc-btn" onClick={onPause} title={job.paused ? t('dl.resume') : t('dl.pause')}>
              {job.paused
                ? <svg viewBox="0 0 16 16" fill="currentColor"><path d="M4 2.5v11l9-5.5z"/></svg>
                : <svg viewBox="0 0 16 16" fill="currentColor"><rect x="3" y="2.5" width="3.5" height="11" rx="1"/><rect x="9.5" y="2.5" width="3.5" height="11" rx="1"/></svg>}
              <span>{job.paused ? t('dl.resume') : t('dl.pause')}</span>
            </button>
            <button className="lc-btn danger" onClick={onCancel} title={t('dl.cancel')}>
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>
              <span>{t('dl.cancel')}</span>
            </button>
          </div>
        )}
      </div>
    </motion.div>
  )
}

// ─── Queue strip ──────────────────────────────────────────────────────────────

export function QueueStrip({ queued, lb, onRemove }: {
  queued: LaunchJob[]; lb: boolean; onRemove: (name: string) => void
}) {
  const t = useT(getLang())
  if (queued.length === 0) return null
  return (
    <motion.div className={`queue-strip${lb ? ' lb' : ''}`}
      initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 6 }}
    >
      <span className="qs-label">{t('launch.queue')}</span>
      {queued.map((j, i) => (
        <span key={j.instance.name} className="qs-chip">
          <span className="qs-n">{i + 1}</span>{j.instance.name}
          <button className="qs-x" onClick={() => onRemove(j.instance.name)} title={t('launch.queue.remove')}>×</button>
        </span>
      ))}
    </motion.div>
  )
}

// ─── Sidebar progress ring ────────────────────────────────────────────────────
//
// Replaces the instance icon while that instance is downloading. Stroke is
// the accent of the instance type; the number inside is the percentage.

export function ProgressRing({ pct, size = 24, queued = false, error = false }: {
  pct: number; size?: number; queued?: boolean; error?: boolean
}) {
  const stroke = 2.5
  const r = (size - stroke) / 2
  const c = 2 * Math.PI * r
  const off = c * (1 - Math.min(100, Math.max(0, pct)) / 100)
  return (
    <span className={`pring${queued ? ' queued' : ''}${error ? ' error' : ''}`} style={{ width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
        <circle className="pring-bg" cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} fill="none" />
        <circle className="pring-fg" cx={size / 2} cy={size / 2} r={r} strokeWidth={stroke} fill="none"
          strokeDasharray={c} strokeDashoffset={queued ? c : off} strokeLinecap="round"
          transform={`rotate(-90 ${size / 2} ${size / 2})`} />
      </svg>
      <span className="pring-txt" style={{ fontSize: Math.max(7, size * 0.34) }}>{queued ? '…' : pct}</span>
    </span>
  )
}
