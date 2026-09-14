import { useState, useEffect, useRef, useCallback } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { getLang, useT } from '../i18n'
import { isTauri, spring, type Instance, type MCVersion, type LBVersion, type VFilter, type LoaderVersionInfo, type Tab } from '../lib/types'
import { LbBadge, verTag } from './ui'

// ─── Create Instance Modal ────────────────────────────────────────────────────

export default function CreateInstanceModal({ defaultTab, mcVersions, existingNames, onAdd, onClose }: {
  defaultTab: Tab
  mcVersions: MCVersion[]
  existingNames: string[]
  onAdd: (inst: Instance) => void
  onClose: () => void
}) {
  const [instType, setInstType]       = useState<Tab>(defaultTab)
  const [step, setStep]               = useState<1 | 2 | 3>(1)
  const [selectedLoader, setSelectedLoader] = useState<'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge'>('vanilla')
  const [loaderVersions, setLoaderVersions] = useState<LoaderVersionInfo[]>([])
  const [loaderVerLoading, setLoaderVerLoading] = useState(false)
  const [loaderVerError, setLoaderVerError] = useState('')
  const [selectedLoaderVer, setSelectedLoaderVer] = useState<string>('')
  const [loaderShowAll, setLoaderShowAll] = useState(false)
  const [unstableWarn, setUnstableWarn]       = useState(false)
  const [unstableCd, setUnstableCd]           = useState(10)
  const [pendingUnstableVer, setPendingUnstableVer] = useState<string>('')
  const unstableCdRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const [vFilter, setVFilter]         = useState<VFilter>('release')
  const [selVer, setSelVer]           = useState<string>('')
  const [name, setName]               = useState('')
  const [nameEdited, setNameEdited]   = useState(false)
  const [shake, setShake]             = useState(false)
  const [error, setError]             = useState('')

  // LB branch state
  const t = useT(getLang())

  const [lbBranches, setLbBranches]       = useState<string[]>(['nextgen', 'legacy'])
  const [lbBranch, setLbBranch]           = useState('nextgen')
  const [lbVersionsMap, setLbVersionsMap] = useState<Record<string, LBVersion[]>>({})
  const [lbLoading, setLbLoading]         = useState(false)
  const [lbLoadError, setLbLoadError]     = useState<string | null>(null)
  const loadedBranches = useRef(new Set<string>())

  const currentLbVersions = lbVersionsMap[lbBranch] ?? []

  const loadBranchVersions = useCallback(async (branch: string) => {
    if (loadedBranches.current.has(branch)) return
    loadedBranches.current.add(branch)
    setLbLoading(true)
    setLbLoadError(null)
    try {
      type RawBuild = { build_id: number; lb_version: string; mc_version: string; date: string }
      const builds: RawBuild[] = isTauri
        ? await invoke<RawBuild[]>('get_lb_versions', { branch })
        : await fetch(`https://api.liquidbounce.net/api/v1/version/builds/${branch}/release`).then(r => r.json())
      const mapped: LBVersion[] = builds.map(r => ({
        buildId: r.build_id, tag: r.lb_version, mcVersion: r.mc_version, date: r.date?.slice(0, 10) ?? ''
      }))
      setLbVersionsMap(prev => ({ ...prev, [branch]: mapped }))
      setSelVer(v => (!v ? (mapped[0]?.tag ?? '') : v))
    } catch (e) {
      loadedBranches.current.delete(branch) // allow retry
      setLbLoadError(String(e))
    }
    setLbLoading(false)
  }, [])

  // Fetch branches list once — always keep nextgen + legacy, add any extras from API
  useEffect(() => {
    if (!isTauri) return
    const fixed = ['nextgen', 'legacy']
    invoke<string[]>('get_lb_branches')
      .then(b => {
        const extra = b.filter(x => !fixed.includes(x))
        setLbBranches([...fixed, ...extra])
      })
      .catch(() => {}) // on error keep hardcoded default
  }, [])

  // Load versions when switching to LB tab or changing branch
  useEffect(() => {
    if (instType === 'lb') loadBranchVersions(lbBranch)
  }, [instType, lbBranch, loadBranchVersions])

  // Reset step when switching to LB type
  useEffect(() => { if (instType === 'lb') setStep(1) }, [instType])

  // Auto-pick first version when type/branch changes
  useEffect(() => {
    if (instType === 'mc') {
      const filtered = mcVersions.filter(v => v.type === 'release')
      setSelVer(filtered[0]?.id ?? '')
    } else {
      setSelVer(lbVersionsMap[lbBranch]?.[0]?.tag ?? '')
    }
    setNameEdited(false)
    setError('')
  }, [instType, lbBranch]) // eslint-disable-line react-hooks/exhaustive-deps

  // When LB versions finish loading, auto-select first if nothing selected
  useEffect(() => {
    if (instType === 'lb' && !selVer && currentLbVersions.length > 0) {
      setSelVer(currentLbVersions[0].tag)
    }
  }, [currentLbVersions.length, instType]) // eslint-disable-line react-hooks/exhaustive-deps

  const loaderSuffix = selectedLoader !== 'vanilla'
    ? ` (${selectedLoader.charAt(0).toUpperCase() + selectedLoader.slice(1)})` : ''
  const autoName = instType === 'mc'
    ? `Minecraft ${selVer}${loaderSuffix}`
    : `LiquidBounce ${selVer}`

  const displayName = nameEdited ? name : autoName

  const filteredMcBase = mcVersions.filter(v => {
    if (vFilter === 'all')      return true
    if (vFilter === 'release')  return v.type === 'release'
    if (vFilter === 'snapshot') return v.type === 'snapshot'
    return v.type === 'old_beta' || v.type === 'old_alpha'
  })
  // Rolling "Latest" versions were removed in 0.0.5: every instance pins the
  // exact version picked here (legacy latest-instances convert on migration).
  const filteredMc = filteredMcBase

  const fetchLoaderVers = (mcId: string) => {
    if (!isTauri) return
    setLoaderVerLoading(true)
    setLoaderVerError('')
    invoke<LoaderVersionInfo[]>('get_loader_versions', { mcVer: mcId, loader: selectedLoader })
      .then(vs => {
        setLoaderVersions(vs)
        const first = vs.find(v => v.stable) ?? vs[0]
        if (first) setSelectedLoaderVer(first.version)
      })
      .catch((e) => setLoaderVerError(String(e)))
      .finally(() => setLoaderVerLoading(false))
  }

  const handleCreate = () => {
    const finalName = displayName.trim()
    if (!finalName) return
    if (existingNames.includes(finalName)) {
      setError(t('inst.name_taken'))
      setShake(true)
      setTimeout(() => setShake(false), 500)
      return
    }
    const lbBuild = currentLbVersions.find(v => v.tag === selVer)
    const mcVerHint = instType === 'mc'
      ? selVer
      : (lbBuild?.mcVersion ?? currentLbVersions[0]?.mcVersion ?? selVer)
    onAdd({
      id: crypto.randomUUID(),
      name: finalName,
      type: instType,
      version: selVer,
      mcVersion: mcVerHint,
      buildId: lbBuild?.buildId,
      loader: instType === 'mc' ? selectedLoader : undefined,
      loaderVersion: (instType === 'mc' && selectedLoader !== 'vanilla') ? (selectedLoaderVer || undefined) : undefined,
    })
    onClose()
  }

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
      onClick={onClose}
    >
      <motion.div className={`modal glass create-inst-modal${shake ? ' shake' : ''}`}
        initial={{ opacity: 0, scale: 0.9, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{instType === 'lb' ? t('inst.modal.title_lb') : t('inst.modal.title_mc')}</span>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div className="vtabs">
              <button className={`vtab${instType === 'mc' ? ' on' : ''}`} onClick={() => setInstType('mc')}>{t('tab.mc')}</button>
              <button className={`vtab${instType === 'lb' ? ' on' : ''}`} onClick={() => setInstType('lb')}>{t('tab.lb')}</button>
            </div>
            <button className="modal-close" onClick={onClose}>×</button>
          </div>
        </div>

        {instType === 'mc' ? (
          step === 1 ? (
          <>
            <div className="vtabs" style={{ padding: '8px 16px 0', gap: 4, display: 'flex' }}>
              {(['release','snapshot','old','all'] as VFilter[]).map(f => (
                <button key={f} className={`vtab${vFilter === f ? ' on' : ''}`} onClick={() => setVFilter(f)}>
                  {f === 'release' ? t('inst.modal.filter.release') : f === 'snapshot' ? t('inst.modal.filter.snapshot') : f === 'old' ? t('inst.modal.filter.old') : t('inst.modal.filter.all')}
                </button>
              ))}
            </div>
            <div className="vlist">
              {filteredMc.map(v => (
                <motion.button key={v.id}
                  className={`vitem${v.id === selVer ? ' picked' : ''}`}
                  onClick={() => { setSelVer(v.id); setNameEdited(false) }}
                  whileHover={{ x: 3 }} transition={spring}
                >
                  <span className={`vbadge ${v.type}`}>
                    {verTag(v.type)}
                  </span>
                  <span className="vid">{v.id}</span>
                  <span className="vyr">{new Date(v.releaseTime).getFullYear()}</span>
                  {v.id === selVer && <span className="vcheck">✓</span>}
                </motion.button>
              ))}
            </div>
          </>
          ) : step === 2 ? (
          <>
            <div className="loader-grid">
              {([
                { id: 'vanilla',  icon: '🌿', label: t('inst.loader.vanilla'),  desc: t('inst.loader.vanilla_desc') },
                { id: 'fabric',   icon: '🧵', label: t('inst.loader.fabric'),   desc: t('inst.loader.fabric_desc') },
                { id: 'quilt',    icon: '🪡', label: 'Quilt',    desc: t('inst.loader.quilt_desc') },
                { id: 'forge',    icon: '⚒️', label: 'Forge',    desc: t('inst.loader.forge_desc') },
                { id: 'neoforge', icon: '🔥', label: 'NeoForge', desc: t('inst.loader.neoforge_desc') },
              ]).map(opt => (
                <div key={opt.id}
                  className={['loader-opt', selectedLoader === opt.id ? 'loader-selected' : ''].filter(Boolean).join(' ')}
                  onClick={() => setSelectedLoader(opt.id as typeof selectedLoader)}
                >
                  <div className="loader-icon">{opt.icon}</div>
                  <div className="loader-info">
                    <div className="loader-name">{opt.label}</div>
                    <div className="loader-desc">{opt.desc}</div>
                  </div>
                  <div className="loader-radio" />
                </div>
              ))}
            </div>
            <div className="loader-step-hint">
              {selectedLoader === 'vanilla' ? t('inst.loader.vanilla_desc')
                : selectedLoader === 'fabric' ? t('inst.loader.fabric_desc')
                : selectedLoader === 'quilt' ? t('inst.loader.quilt_desc')
                : selectedLoader === 'forge' ? t('inst.loader.forge_desc')
                : t('inst.loader.neoforge_desc')}
            </div>
          </>
          ) : (
          <>
            <div style={{ padding: '10px 16px 4px', display: 'flex', alignItems: 'center', gap: 8 }}>
              <div className="field-label" style={{ flex: 1 }}>{t('inst.loader.ver.title')} — {selectedLoader.charAt(0).toUpperCase() + selectedLoader.slice(1)}</div>
              <div style={{ display: 'flex', gap: 3 }}>
                {(['releases', 'all'] as const).map(f => (
                  <button key={f}
                    style={{
                      padding: '2px 9px', borderRadius: 6, fontSize: 11, border: 'none', cursor: 'pointer',
                      background: (f === 'releases') === !loaderShowAll ? 'var(--accent)' : 'rgba(255,255,255,0.07)',
                      color: (f === 'releases') === !loaderShowAll ? '#fff' : 'var(--text-muted)',
                      fontWeight: (f === 'releases') === !loaderShowAll ? 700 : 400,
                      transition: 'background 0.18s, color 0.18s',
                    }}
                    onClick={() => setLoaderShowAll(f === 'all')}
                  >{f === 'releases' ? t('loader.filter.releases') : t('loader.filter.all')}</button>
                ))}
              </div>
            </div>
            <div className="vlist">
              {loaderVerLoading ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('inst.loader.ver.loading')}</div>
              ) : loaderVerError ? (
                <div style={{ padding: 20, display: 'flex', flexDirection: 'column', gap: 10, alignItems: 'flex-start' }}>
                  <div style={{ color: '#f87171', fontSize: 12 }}>{t('error.prefix')} {loaderVerError}</div>
                  <button className="btn-retry" onClick={() => fetchLoaderVers(selVer)}>{t('error.retry')}</button>
                </div>
              ) : loaderVersions.length === 0 ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('inst.loader.ver.none')}</div>
              ) : (() => {
                const displayed = loaderShowAll ? loaderVersions : loaderVersions.filter(v => v.stable)
                if (displayed.length === 0) return (
                  <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('loader.no_releases')}</div>
                )
                return displayed.map(v => (
                  <motion.button key={v.version}
                    className={`vitem${v.version === selectedLoaderVer ? ' picked' : ''}`}
                    onClick={() => {
                      if (!v.stable) {
                        setPendingUnstableVer(v.version)
                        setUnstableWarn(true)
                        setUnstableCd(10)
                        if (unstableCdRef.current) clearInterval(unstableCdRef.current)
                        unstableCdRef.current = setInterval(() => {
                          setUnstableCd(prev => {
                            if (prev <= 1) { clearInterval(unstableCdRef.current!); unstableCdRef.current = null; return 0 }
                            return prev - 1
                          })
                        }, 1000)
                      } else {
                        setSelectedLoaderVer(v.version)
                      }
                    }}
                    whileHover={{ x: 3 }} transition={spring}
                  >
                    {v.latest
                      ? <span className="vbadge latest" style={{ fontSize: 10 }}>★</span>
                      : v.stable
                        ? <span className="vbadge release" style={{ fontSize: 8 }}>V</span>
                        : <span className="vbadge unstable" style={{ fontSize: 11 }}>⚠</span>
                    }
                    <span className="vid">{v.version}</span>
                    {!v.stable && <span style={{ fontSize: 10, color: '#f87171', marginLeft: 'auto', opacity: 0.8 }}>{t('loader.beta_label')}</span>}
                    {v.version === selectedLoaderVer && <span className="vcheck">✓</span>}
                  </motion.button>
                ))
              })()}
            </div>

            {/* Unstable loader version warning modal */}
            <AnimatePresence>
              {unstableWarn && (
                <motion.div
                  initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                  style={{
                    position: 'absolute', inset: 0, zIndex: 60, display: 'flex', alignItems: 'center', justifyContent: 'center',
                    background: 'rgba(0,0,0,0.72)', backdropFilter: 'blur(6px)', borderRadius: 'inherit',
                  }}
                >
                  <motion.div
                    initial={{ scale: 0.88, opacity: 0 }} animate={{ scale: 1, opacity: 1 }} exit={{ scale: 0.88, opacity: 0 }}
                    transition={{ type: 'spring', stiffness: 380, damping: 28 }}
                    style={{
                      background: 'rgba(18,10,10,0.96)', border: '1.5px solid rgba(248,113,113,0.55)',
                      borderRadius: 16, padding: '24px 28px', maxWidth: 320, width: '90%',
                      boxShadow: '0 0 40px rgba(248,113,113,0.22)',
                    }}
                  >
                    <div style={{ fontSize: 28, textAlign: 'center', marginBottom: 8 }}>⚠️</div>
                    <div style={{ fontSize: 15, fontWeight: 700, color: '#f87171', textAlign: 'center', marginBottom: 6 }}>
                      {t('loader.unstable.title')}
                    </div>
                    <div style={{ fontSize: 12, color: 'rgba(248,113,113,0.82)', textAlign: 'center', marginBottom: 18, lineHeight: 1.5 }}>
                      <b style={{ color: '#fca5a5' }}>{pendingUnstableVer}</b> {t('loader.unstable.body').split('\n').map((l, i) => <span key={i}>{i > 0 && <br/>}{l}</span>)}
                    </div>
                    <div style={{ display: 'flex', gap: 10 }}>
                      <button
                        style={{
                          flex: 1, padding: '9px 0', borderRadius: 9, border: 'none', cursor: 'pointer',
                          background: 'var(--accent)', color: '#fff', fontWeight: 700, fontSize: 13,
                          transition: 'opacity 0.2s',
                        }}
                        onClick={() => {
                          if (unstableCd > 0) return
                          if (unstableCdRef.current) { clearInterval(unstableCdRef.current); unstableCdRef.current = null }
                          setSelectedLoaderVer(pendingUnstableVer)
                          setUnstableWarn(false)
                        }}
                      >
                        {unstableCd > 0 ? t('loader.unstable.confirm_cd').replace('{0}', String(unstableCd)) : t('loader.unstable.confirm')}
                      </button>
                      <button
                        style={{
                          flex: 1, padding: '9px 0', borderRadius: 9, border: '1.5px solid var(--accent)', cursor: 'pointer',
                          background: 'transparent', color: 'var(--accent)', fontWeight: 800, fontSize: 12,
                          letterSpacing: 0.3,
                        }}
                        onClick={() => {
                          if (unstableCdRef.current) { clearInterval(unstableCdRef.current); unstableCdRef.current = null }
                          setPendingUnstableVer('')
                          setUnstableWarn(false)
                        }}
                      >
                        {t('loader.unstable.cancel')}
                      </button>
                    </div>
                  </motion.div>
                </motion.div>
              )}
            </AnimatePresence>
          </>
          )
        ) : (
          <>
            <div className="vtabs" style={{ padding: '8px 16px 0', gap: 4, display: 'flex' }}>
              {lbBranches.map(b => (
                <button key={b}
                  className={`vtab${lbBranch === b ? ' on' : ''}`}
                  onClick={() => setLbBranch(b)}
                >
                  {b === 'nextgen' ? 'Nextgen' : b === 'legacy' ? 'Legacy' : b.charAt(0).toUpperCase() + b.slice(1)}
                </button>
              ))}
            </div>
            <div className="vlist">
              {lbLoading ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('loading')}</div>
              ) : lbLoadError ? (
                <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 8 }}>
                  <div style={{ color: '#f87171', fontSize: 12 }}>{t('error.prefix')} {lbLoadError}</div>
                  <button className="btn-retry" style={{ alignSelf: 'flex-start' }}
                    onClick={() => { loadedBranches.current.delete(lbBranch); loadBranchVersions(lbBranch) }}>
                    {t('error.retry')}
                  </button>
                </div>
              ) : currentLbVersions.length === 0 ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('inst.no_versions')}</div>
              ) : (() => {
                return currentLbVersions.map(v => (
                  <motion.button key={v.tag}
                    className={`vitem${v.tag === selVer ? ' picked lb-picked' : ''}`}
                    onClick={() => { setSelVer(v.tag); setNameEdited(false) }}
                    whileHover={{ x: 3 }} transition={spring}
                  >
                    <LbBadge />
                    <span className="vid">{v.tag}</span>
                    <span className="vyr">MC {v.mcVersion}</span>
                    {v.tag === selVer && <span className="vcheck" style={{ color: 'var(--lb-accent)' }}>✓</span>}
                  </motion.button>
                ))
              })()}
            </div>
          </>
        )}

        <div className="inst-name-row">
          <input
            className={`glass-input${error ? ' input-error' : ''}`}
            value={displayName}
            onChange={e => { setName(e.target.value); setNameEdited(true); setError('') }}
            onKeyDown={e => e.key === 'Enter' && handleCreate()}
            placeholder={t('inst.modal.name_ph')}
            maxLength={64}
          />
          {error && <div className="inst-error">{error}</div>}
        </div>

        <div className="inst-modal-footer">
          {instType === 'mc' && step > 1 ? (
            <button className="btn-cancel" onClick={() => setStep(step === 3 ? 2 : 1)}>{t('btn.back')}</button>
          ) : (
            <button className="btn-cancel" onClick={onClose}>{t('inst.modal.cancel')}</button>
          )}
          {instType === 'mc' && step === 1 ? (
            <button className="btn-ok" onClick={() => setStep(2)} disabled={!selVer}>
              {t('inst.loader.next')}
            </button>
          ) : instType === 'mc' && step === 2 ? (
            <button className="btn-ok" onClick={() => {
              if (selectedLoader === 'vanilla') { handleCreate(); return }
              setStep(3)
              setSelectedLoaderVer('')
              setLoaderVersions([])
              fetchLoaderVers(selVer)
            }}>
              {selectedLoader === 'vanilla' ? t('inst.modal.create') : t('inst.loader.next')}
            </button>
          ) : (
            <button
              className={`btn-ok${instType === 'lb' ? ' lb-btn' : ''}`}
              onClick={handleCreate}
              disabled={!selVer || (instType === 'mc' && step === 3 && !selectedLoaderVer)}
            >
              {t('inst.modal.create')}
            </button>
          )}
        </div>
      </motion.div>
    </motion.div>
  )
}
