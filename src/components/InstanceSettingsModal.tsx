import { useState, useEffect, useCallback } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { getLang, useT } from '../i18n'
import { isTauri, spring, type Instance, type ModRow, type ModUpdateInfo } from '../lib/types'
import ModBrowser, { ModIcon } from '../ModBrowser'

// ─── Instance Settings Modal ──────────────────────────────────────────────────

export default function InstanceSettingsModal({ inst, isLb, onClose }: { inst: Instance; isLb: boolean; onClose: () => void }) {
  const t = useT(getLang())
  const [tab, setTab] = useState<'overview' | 'mods' | 'logs'>('overview')

  // RAM
  const ramKey = `mlbv_inst_ram_${inst.id}`
  const globalRam = Number(localStorage.getItem('mlbv_ram') ?? '2048') || 2048
  const [useCustomRam, setUseCustomRam] = useState(() => !!localStorage.getItem(ramKey))
  const [ram, setRam] = useState(() => Number(localStorage.getItem(ramKey) ?? globalRam))
  // Min RAM (Xms)
  const minRamKey = `mlbv_inst_min_ram_${inst.id}`
  const globalMinRam = Number(localStorage.getItem('mlbv_min_ram') ?? '512') || 512
  const [useCustomMinRam, setUseCustomMinRam] = useState(() => !!localStorage.getItem(minRamKey))
  const [minRam, setMinRam] = useState(() => Number(localStorage.getItem(minRamKey) ?? globalMinRam))

  // Logs
  const [logText, setLogText] = useState('')
  const [logBusy, setLogBusy] = useState(false)

  // Mods
  const [mods, setMods] = useState<ModRow[]>([])
  const [selectedMods, setSelectedMods] = useState<Set<string>>(new Set())
  const [modsLoading, setModsLoading] = useState(false)
  const [updates, setUpdates] = useState<Record<string, ModUpdateInfo>>({})
  const [checkingUpdates, setCheckingUpdates] = useState(false)
  const [updating, setUpdating] = useState<string | null>(null)
  const [showModBrowser, setShowModBrowser] = useState(false)

  const refreshMods = useCallback(async () => {
    if (!isTauri) return
    setModsLoading(true)
    try {
      setMods(await invoke<ModRow[]>('list_mods', { instanceName: inst.name }))
      setSelectedMods(new Set())
    } catch { /* keep the old list */ }
    setModsLoading(false)
  }, [inst.name])

  const fmtSize = (b: number) =>
    b <= 0 ? '' : b < 1024 * 1024 ? `${(b / 1024).toFixed(0)} KB` : `${(b / (1024 * 1024)).toFixed(1)} MB`

  const toggleEnabled = async (m: ModRow) => {
    if (!isTauri) return
    try {
      await invoke<string>('set_mod_enabled', { instanceName: inst.name, filename: m.filename, enabled: !m.enabled })
      await refreshMods()
    } catch { /* */ }
  }

  const checkUpdates = async () => {
    if (!isTauri || checkingUpdates || updating) return
    setCheckingUpdates(true)
    try {
      const found = await invoke<ModUpdateInfo[]>('check_mod_updates', {
        instanceName: inst.name,
        mcVersion: inst.mcVersion,
        loader: inst.loader ?? 'vanilla',
        cfKey: localStorage.getItem('mlbv_cf_key') ?? '',
      })
      setUpdates(Object.fromEntries(found.map(u => [u.filename, u])))
    } catch { /* */ }
    setCheckingUpdates(false)
  }

  const installUpdate = async (u: ModUpdateInfo): Promise<boolean> => {
    try {
      await invoke<string>('install_mod_file', {
        instanceName: inst.name,
        downloadUrl: u.download_url,
        fileName: u.file_name,
        replaceExisting: u.filename !== u.file_name ? u.filename : null,
        meta: {
          source: u.source, project_id: u.project_id, name: u.name, author: u.author,
          version_id: u.version_id, version_number: u.latest, icon_url: u.icon_url,
        },
      })
      setUpdates(prev => { const n = { ...prev }; delete n[u.filename]; return n })
      return true
    } catch {
      return false
    }
  }

  const applyUpdate = async (u: ModUpdateInfo) => {
    if (!isTauri || updating) return
    setUpdating(u.filename)
    await installUpdate(u)
    await refreshMods()
    setUpdating(null)
  }

  const updateAll = async () => {
    const list = Object.values(updates)
    if (!isTauri || updating || list.length === 0) return
    setUpdating('all')
    for (const u of list) await installUpdate(u)
    await refreshMods()
    setUpdating(null)
  }

  const exportModList = () => {
    const lines = [
      `# Mods — ${inst.name}`,
      '',
      ...mods.map(m => {
        const label = m.name || m.filename
        const ver = m.version_number ? ` (${m.version_number})` : ''
        const off = m.enabled ? '' : ' [disabled]'
        return `- ${label}${ver}${off}`
      }),
    ]
    const blob = new Blob([lines.join('\n')], { type: 'text/markdown' })
    const a = document.createElement('a')
    a.href = URL.createObjectURL(blob)
    a.download = `mods-${inst.name}.md`
    a.click()
    setTimeout(() => URL.revokeObjectURL(a.href), 5000)
  }

  useEffect(() => {
    if (!isTauri) return
    invoke<string>('read_instance_log', { instanceName: inst.name }).then(s => setLogText(s)).catch(() => {})
  }, [inst.name])

  useEffect(() => {
    if (tab === 'mods') refreshMods()
  }, [tab, refreshMods])

  useEffect(() => {
    if (useCustomRam) localStorage.setItem(ramKey, String(ram))
    else localStorage.removeItem(ramKey)
  }, [useCustomRam, ram, ramKey])

  useEffect(() => {
    if (useCustomMinRam) localStorage.setItem(minRamKey, String(minRam))
    else localStorage.removeItem(minRamKey)
  }, [useCustomMinRam, minRam, minRamKey])

  const clampRam = (v: number) => Math.min(16384, Math.max(512, Math.round(v / 512) * 512))
  const clampMinRam = (v: number) => Math.min(8192, Math.max(256, Math.round(v / 256) * 256))
  const accentVar = isLb ? 'var(--lb-accent)' : 'var(--accent)'
  const updateCount = Object.keys(updates).length

  const loaderLabel = () => {
    if (inst.type === 'lb') return t('isettings.type_lb')
    switch (inst.loader) {
      case 'fabric': return `Fabric${inst.loaderVersion ? ` ${inst.loaderVersion}` : ''}`
      case 'quilt':  return `Quilt${inst.loaderVersion ? ` ${inst.loaderVersion}` : ''}`
      case 'forge':  return `Forge${inst.loaderVersion ? ` ${inst.loaderVersion}` : ''}`
      case 'neoforge': return `NeoForge${inst.loaderVersion ? ` ${inst.loaderVersion}` : ''}`
      default: return 'Vanilla'
    }
  }

  const handleAddMods = () => {
    const input = document.createElement('input')
    input.type = 'file'; input.accept = '.jar'; input.multiple = true
    input.onchange = async () => {
      const files = Array.from(input.files ?? [])
      for (const file of files) {
        if (!file.name.endsWith('.jar')) continue
        const buf = await file.arrayBuffer()
        await invoke('add_mod_file', { instanceName: inst.name, filename: file.name, data: Array.from(new Uint8Array(buf)) }).catch(() => {})
      }
      await refreshMods()
    }
    input.click()
  }

  const handleDeleteMods = async () => {
    if (selectedMods.size === 0) return
    await invoke('delete_mods', { instanceName: inst.name, filenames: Array.from(selectedMods) }).catch(() => {})
    await refreshMods()
  }

  const toggleMod = (filename: string) => {
    setSelectedMods(prev => { const n = new Set(prev); if (n.has(filename)) n.delete(filename); else n.add(filename); return n })
  }

  const navItems = [
    { id: 'overview', label: t('isettings.nav.overview'), icon: <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"><rect x="2" y="2" width="5" height="5" rx="1"/><rect x="9" y="2" width="5" height="5" rx="1"/><rect x="2" y="9" width="5" height="5" rx="1"/><rect x="9" y="9" width="5" height="5" rx="1"/></svg> },
    { id: 'mods',     label: t('isettings.nav.mods'),     icon: <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round"><path d="M13 6.5A3.5 3.5 0 0 0 6.5 3c-.18 0-.36.01-.53.04L3 6l1 1-1 1 1 1-1 1 2.5 2.5 1-1 1 1 1-1 1 1 3-3V9.5h1A1.5 1.5 0 0 0 13 8V6.5z"/><circle cx="10" cy="5.5" r="0.7" fill="currentColor" stroke="none"/></svg> },
    { id: 'logs',     label: t('isettings.nav.logs'),     icon: <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round"><path d="M3 4h10M3 8h10M3 12h6"/></svg> },
  ]

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={onClose}>
      <motion.div className="modal glass inst-settings-modal"
        style={{ '--inst-accent': accentVar } as React.CSSProperties}
        initial={{ opacity: 0, scale: 0.9, y: 20 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{inst.name}</span>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <div className="inst-settings-layout">
          {/* Left nav */}
          <div className="ist-nav">
            {navItems.map(n => (
              <button key={n.id}
                className={`ist-nav-btn${tab === n.id ? ` active${isLb ? ' lb-nav' : ''}` : ''}`}
                onClick={() => setTab(n.id as typeof tab)}
              >
                <div className="ist-nav-icon">{n.icon}</div>
                <span className="ist-nav-label">{n.label}</span>
              </button>
            ))}
          </div>
          {/* Right panel */}
          <div className="ist-panel">
            {tab === 'overview' && <>
              <div className="setting-group">
                <div className="setting-label">{t('isettings.info')}</div>
                <div className="setting-hint">
                  Version: {inst.version} · MC {inst.mcVersion}<br/>
                  Loader: {loaderLabel()}
                </div>
              </div>
              <div className="setting-group">
                <div className="setting-label">{t('isettings.ram')}</div>
                <label className="setting-toggle">
                  <input type="checkbox" checked={useCustomRam} onChange={e => setUseCustomRam(e.target.checked)} />
                  <span className="toggle-track ist-toggle-track"><span className="toggle-thumb" /></span>
                  <span className="toggle-label">{useCustomRam ? `${ram >= 1024 ? `${(ram/1024).toFixed(1)} GB` : `${ram} MB`}` : `Global (${globalRam >= 1024 ? `${(globalRam/1024).toFixed(1)} GB` : `${globalRam} MB`})`}</span>
                </label>
                {useCustomRam && (
                  <input type="range" className="glass-range" min={512} max={16384} step={512}
                    value={ram} onChange={e => setRam(clampRam(Number(e.target.value)))} />
                )}
              </div>
              <div className="setting-group">
                <div className="setting-label">{t('isettings.min_ram')}</div>
                <label className="setting-toggle">
                  <input type="checkbox" checked={useCustomMinRam} onChange={e => setUseCustomMinRam(e.target.checked)} />
                  <span className="toggle-track ist-toggle-track"><span className="toggle-thumb" /></span>
                  <span className="toggle-label">{useCustomMinRam ? `${minRam >= 1024 ? `${(minRam/1024).toFixed(1)} GB` : `${minRam} MB`}` : `Global (${globalMinRam >= 1024 ? `${(globalMinRam/1024).toFixed(1)} GB` : `${globalMinRam} MB`})`}</span>
                </label>
                {useCustomMinRam && (
                  <input type="range" className="glass-range" min={256} max={8192} step={256}
                    value={Math.min(minRam, 8192)} onChange={e => setMinRam(clampMinRam(Number(e.target.value)))} />
                )}
              </div>
            </>}

            {tab === 'mods' && <>
              <div className="ist-mods-toolbar" style={{ flexWrap: 'wrap' }}>
                <button className="btn-secondary" onClick={() => setShowModBrowser(true)}>{t('isettings.mods.download')}</button>
                <button className="btn-secondary" onClick={handleAddMods}>{t('isettings.mods.add_file')}</button>
                <button className="btn-secondary" onClick={() => isTauri && invoke('open_mods_folder', { instanceName: inst.name }).catch(() => {})}>{t('isettings.mods.open_folder')}</button>
                <button className="btn-secondary" onClick={exportModList} disabled={mods.length === 0}>{t('isettings.mods.export')}</button>
                <button className="btn-secondary" onClick={checkUpdates} disabled={checkingUpdates || updating !== null}>
                  {checkingUpdates ? t('isettings.mods.checking') : t('isettings.mods.check_updates')}
                </button>
                {updateCount > 0 && (
                  <button className="btn-ok" onClick={updateAll} disabled={updating !== null}>
                    {updating === 'all' ? t('isettings.mods.updating') : t('isettings.mods.update_all').replace('{0}', String(updateCount))}
                  </button>
                )}
                {selectedMods.size > 0 && (
                  <button className="btn-danger-sm" onClick={handleDeleteMods}>{t('isettings.mods.delete_selected')} ({selectedMods.size})</button>
                )}
              </div>
              {modsLoading ? (
                <div className="ist-mods-empty">{t('loading')}</div>
              ) : mods.length === 0 ? (
                <div className="ist-mods-empty">{t('isettings.mods.empty')}</div>
              ) : (
                <div className="ist-mods-list">
                  {mods.map(m => {
                    const u = updates[m.filename]
                    const display = m.name || m.filename
                    return (
                      <div key={m.filename}
                        className={`ist-mod-row${selectedMods.has(m.filename) ? (isLb ? ' lb-mod-selected' : ' mod-selected') : ''}`}
                        style={{ opacity: m.enabled ? 1 : 0.55 }}
                        onClick={() => toggleMod(m.filename)}
                      >
                        <div className="mod-radio" />
                        <ModIcon url={m.icon_url} name={display} size={30} />
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div className="mod-name" style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{display}</div>
                          <div style={{ fontSize: 10, color: 'var(--text-muted)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                            {m.version_number || m.filename}
                            {m.source && ` · ${m.source}`}
                            {m.size > 0 && ` · ${fmtSize(m.size)}`}
                            {!m.enabled && ` · ${t('isettings.mods.disabled')}`}
                            {!m.source && ` · ${t('isettings.mods.unknown')}`}
                          </div>
                        </div>
                        {u && (
                          <button className="btn-ok" style={{ padding: '4px 10px', fontSize: 11, flexShrink: 0 }}
                            disabled={updating !== null}
                            onClick={e => { e.stopPropagation(); applyUpdate(u) }}>
                            {updating === u.filename ? '…' : `⬆ ${u.latest}`}
                          </button>
                        )}
                        <button className="btn-secondary" style={{ padding: '4px 10px', fontSize: 11, flexShrink: 0 }}
                          title={m.enabled ? t('isettings.mods.disable') : t('isettings.mods.enable')}
                          onClick={e => { e.stopPropagation(); toggleEnabled(m) }}>
                          {m.enabled ? '✓' : '○'}
                        </button>
                      </div>
                    )
                  })}
                </div>
              )}
            </>}

            {tab === 'logs' && <>
              <div className="inst-log-actions">
                <button className="btn-secondary" onClick={() => { navigator.clipboard.writeText(logText); setLogBusy(true); setTimeout(() => setLogBusy(false), 1200) }}>
                  {logBusy ? t('isettings.copied') : t('isettings.copy_log')}
                </button>
                {isTauri && (
                  <button className="btn-secondary" onClick={() => invoke('open_instance_logs_folder', { instanceName: inst.name }).catch(() => {})}>
                    {t('isettings.open_logs')}
                  </button>
                )}
              </div>
              {logText
                ? <pre className="inst-log-preview">{logText}</pre>
                : <div className="setting-hint">{t('isettings.no_log')}</div>
              }
            </>}
          </div>
        </div>
        {showModBrowser && (
          <ModBrowser
            instanceName={inst.name}
            mcVersion={inst.mcVersion}
            loader={inst.loader}
            cfKey={localStorage.getItem('mlbv_cf_key') ?? ''}
            installed={mods.filter(m => m.project_id).map(m => ({ filename: m.filename, source: m.source, project_id: m.project_id }))}
            onInstalled={refreshMods}
            onClose={() => setShowModBrowser(false)}
          />
        )}
      </motion.div>
    </motion.div>
  )
}
