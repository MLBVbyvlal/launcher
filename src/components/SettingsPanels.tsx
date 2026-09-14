import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import lbLogo from '../assets/lb-logo.svg'
import { getLang, useT } from '../i18n'
import { isTauri } from '../lib/types'

// ─── Debug Info Block ─────────────────────────────────────────────────────────

function DebugInfoBlock() {
  const [info, setInfo] = useState<Record<string, unknown> | null>(null)
  const [open, setOpen] = useState(false)
  const [copied, setCopied] = useState(false)

  const load = async () => {
    if (!isTauri) return
    try {
      const d = await invoke<Record<string, unknown>>('get_debug_info')
      setInfo(d)
    } catch (e) {
      setInfo({ error: String(e) })
    }
  }

  const toggle = () => {
    if (!open && !info) load()
    setOpen(o => !o)
  }

  const copy = () => {
    if (!info) return
    navigator.clipboard.writeText(JSON.stringify(info, null, 2)).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  return (
    <div className="debug-block">
      <button className="debug-toggle" onClick={toggle}>
        {open ? '▾' : '▸'} Debug info
      </button>
      {open && (
        <div className="debug-body">
          {info ? (
            <>
              <pre className="debug-pre">{JSON.stringify(info, null, 2)}</pre>
              <button className="debug-copy" onClick={copy}>{copied ? '✓ Copied' : 'Copy'}</button>
            </>
          ) : (
            <span className="debug-loading">Loading…</span>
          )}
        </div>
      )}
    </div>
  )
}

// ─── About panel ──────────────────────────────────────────────────────────────

type UpdateStatus = 'idle' | 'checking' | 'uptodate' | { version: string; htmlUrl: string } | { error: string }

export function AboutPanel({ updateCheckError }: { updateCheckError?: string | null }) {
  const t = useT(getLang())
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('idle')
  // The self-updater is Windows-only (NSIS/MSI); its UI is hidden elsewhere.
  const [osName, setOsName] = useState('')
  useEffect(() => {
    if (isTauri) invoke<Record<string, unknown>>('get_debug_info').then(d => setOsName(String(d.os ?? ''))).catch(() => {})
  }, [])

  const handleManualUpdateCheck = async () => {
    if (!isTauri) return
    setUpdateStatus('checking')
    try {
      type RawRelease = { version: string; tag_name: string; body: string; html_url: string; asset_url: string }
      const r = await invoke<RawRelease | null>('check_for_update')
      setUpdateStatus(r ? { version: r.version, htmlUrl: r.html_url } : 'uptodate')
    } catch (e) {
      setUpdateStatus({ error: String(e) })
    }
  }

  return (
<div className="about-panel">
  <div className="about-logos">
    <div className="about-logo-item">
      <div className="logo-mark" style={{ transform: 'scale(1.4)', margin: '4px 0' }}><span /><span /><span /><span /></div>
      <span className="about-logo-name">MLBV</span>
    </div>
    <span className="about-x">×</span>
    <div className="about-logo-item">
      <img src={lbLogo} className="about-lb-logo" alt="LiquidBounce" draggable={false} />
      <span className="about-logo-name">LiquidBounce</span>
    </div>
  </div>
  <p className="about-disclaimer">{t('settings.disclaimer')}</p>
  <div className="about-info">
    <span className="about-ver">MLBV v{__APP_VERSION__}</span>
    <span className="about-stack">{t('settings.stack')}</span>
  </div>
  <div className="about-update-row">
    {osName !== 'linux' && osName !== 'darwin' && (
    <button className="btn-secondary" onClick={handleManualUpdateCheck}
      disabled={updateStatus === 'checking'}>
      {updateStatus === 'checking' ? t('settings.checking') : t('settings.check_updates')}
    </button>
    )}
    {updateStatus === 'uptodate' && (
      <span className="about-update-ok">{t('settings.up_to_date')}</span>
    )}
    {typeof updateStatus === 'object' && 'version' in updateStatus && (
      <span className="about-update-avail">
        v{updateStatus.version} {t('settings.update_available')} —{' '}
        <button className="about-update-link"
          onClick={() => isTauri && invoke('open_url', { url: updateStatus.htmlUrl }).catch(() => {})}>
          {t('update.download')}
        </button>
      </span>
    )}
    {typeof updateStatus === 'object' && 'error' in updateStatus && (
      <span className="about-update-err">{t('settings.update_check_failed')} {updateStatus.error}</span>
    )}
    {updateCheckError && !(typeof updateStatus === 'object' && 'error' in updateStatus) && (
      <span className="about-update-err">{t('settings.update_check_failed')} {updateCheckError}</span>
    )}
  </div>
  <DebugInfoBlock />
  <div className="about-by">{t('settings.by')}</div>
</div>
  )
}

// ─── Danger zone panel ────────────────────────────────────────────────────────

export function DangerPanel({ dangerOpen, setDangerOpen, countdown, setCountdown }: {
  dangerOpen: boolean; setDangerOpen: (v: boolean) => void
  countdown: number; setCountdown: (v: number) => void
}) {
  const t = useT(getLang())
  const [deleting, setDeleting] = useState(false)
  const [blocked, setBlocked]   = useState('')
  // Refuse to wipe the data directory while a launch is writing to it: the
  // delete would race the download and the reload erases the only evidence that
  // it failed. `active_downloads` is the backend's own list of in-flight
  // launches, so the UI does not have to guess.
  const handleDelete = async () => {
    setDeleting(true); setBlocked('')
    if (isTauri) {
      let busy: string[] = []
      try { busy = await invoke<string[]>('active_downloads') }
      catch (e) { setBlocked(String(e)); setDeleting(false); return }
      if (busy.length > 0) {
        setBlocked(t('settings.danger_busy').replace('{0}', busy.join(', ')))
        setDeleting(false)
        return
      }
    }
    // localStorage is cleared only after the directory is really gone — the
    // other order leaves instances on disk with no UI knowing about them.
    try {
      if (isTauri) await invoke('reset_all_data')
      localStorage.clear()
      window.location.reload()
    } catch (e) { setBlocked(String(e)); setDeleting(false) }
  }
  return (
<div className="danger-panel">
  <div className="danger-title">{t('settings.danger_title')}</div>
  <div className="danger-hint">{t('settings.danger_hint')}</div>
  {!dangerOpen ? (
    <button className="btn-danger-trigger"
      onClick={() => { setDangerOpen(true); setCountdown(5) }}>
      {t('settings.danger_btn')}
    </button>
  ) : (
    <div className="danger-confirm-box">
      <div className="danger-warn-title">{t('settings.danger_warn')}</div>
      <ul className="danger-list">
        <li>{t('settings.danger_item1')}</li>
        <li>{t('settings.danger_item2')}</li>
        <li>{t('settings.danger_item3')}</li>
        <li>{t('settings.danger_item4')}</li>
        <li>{t('settings.danger_item5')}</li>
      </ul>
      {blocked && <div className="inst-error" style={{ marginTop: 8, whiteSpace: 'pre-line' }}>{blocked}</div>}
      <div className="danger-countdown">
        {countdown > 0
          ? t('settings.danger_wait').replace('{0}', String(countdown))
          : t('settings.danger_confirm_hint')}
      </div>
      <div className="danger-actions">
        <button className="btn-cancel"
          onClick={() => { setDangerOpen(false); setCountdown(5) }}>
          {t('settings.cancel')}
        </button>
        <button
          className={`btn-delete-confirm${countdown <= 0 ? ' enabled' : ''}`}
          disabled={countdown > 0 || deleting}
          onClick={handleDelete}
        >
          {deleting ? t('settings.deleting') : t('settings.confirm_delete')}
        </button>
      </div>
    </div>
  )}
</div>
  )
}
