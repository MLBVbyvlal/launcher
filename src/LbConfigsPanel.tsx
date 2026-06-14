import { useState, useEffect, useRef } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { marked } from 'marked'
import { getLang, useT } from './i18n'

// ─── Types ────────────────────────────────────────────────────────────────────

interface ConfigMeta {
  author?: string
  date?: string
  time?: string
  clientVersion?: string
  clientCommit?: string
  type?: string
  status?: string
}

interface ConfigEntry {
  name: string
  jsonFile: string
  jsonUrl: string
  readmeHtml: string
  previewImg: string
  meta: ConfigMeta
}

interface InstallTarget {
  name: string
}

interface InstalledMap {
  [configName: string]: { date?: string; time?: string; file: string }
}

// ─── Constants ────────────────────────────────────────────────────────────────

const REPO   = 'MLBVbyvlal/lbconfig'
const BRANCH = 'main'
const RAW    = (path: string) => `https://raw.githubusercontent.com/${REPO}/${BRANCH}/${path}`
const API    = (path = '')    => `https://api.github.com/repos/${REPO}/contents/${path}`

// ─── Helpers ──────────────────────────────────────────────────────────────────

function parseMeta(text: string): ConfigMeta {
  const tail = text.split('\n').slice(-25).join('\n')
  const g = (key: string) => {
    const m = tail.match(new RegExp(`"${key}":\\s*"([^"]*)"`, 'i'))
    return m ? m[1] : undefined
  }
  return {
    author: g('author'), date: g('date'), time: g('time'),
    clientVersion: g('clientVersion'), clientCommit: g('clientCommit'),
    type: g('type'), status: g('status'),
  }
}

function resolveImgSrcs(md: string, folder: string): string {
  return md.replace(
    /!\[([^\]]*)\]\((?!https?:\/\/)([^)]+)\)/g,
    (_m, alt, src) => `![${alt}](${RAW(`${folder}/${src}`)})`
  )
}

function firstImage(md: string): string {
  const m = md.match(/!\[[^\]]*\]\(([^)]+)\)/)
  return m ? m[1] : ''
}

function renderMd(md: string): string {
  marked.setOptions({ breaks: true, gfm: true } as Parameters<typeof marked.setOptions>[0])
  return marked.parse(md) as string
}

function getInstalled(): InstalledMap {
  try { return JSON.parse(localStorage.getItem('mlbv_installed_configs') ?? '{}') }
  catch { return {} }
}
function saveInstalled(map: InstalledMap) {
  localStorage.setItem('mlbv_installed_configs', JSON.stringify(map))
}
function getFavs(): string[] {
  try { return JSON.parse(localStorage.getItem('mlbv_lb_fav_configs') ?? '[]') }
  catch { return [] }
}
function saveFavs(f: string[]) {
  localStorage.setItem('mlbv_lb_fav_configs', JSON.stringify(f))
}

const ease = [0.4, 0, 0.2, 1] as const

// ─── Component ────────────────────────────────────────────────────────────────

interface Props {
  onClose: () => void
  lbInstances: { name: string; version: string }[]
}

export default function LbConfigsPanel({ onClose, lbInstances }: Props) {
  const t = useT(getLang())
  const [configs, setConfigs]         = useState<ConfigEntry[]>([])
  const [loading, setLoading]         = useState(true)
  const [error, setError]             = useState('')
  const [selected, setSelected]       = useState<ConfigEntry | null>(null)
  const [favs, setFavsState]          = useState<string[]>(getFavs)
  const [installed, setInstalled]     = useState<InstalledMap>(getInstalled)

  const [installing, setInstalling]   = useState(false)
  const [installErr, setInstallErr]   = useState('')
  const [targets, setTargets]         = useState<InstallTarget[]>([])
  const [pickerOpen, setPickerOpen]   = useState(false)
  const [loadingTargets, setLoadingT] = useState(false)
  const pickerRef                     = useRef<HTMLDivElement>(null)

  const [postCmd, setPostCmd]         = useState('')
  const [copied, setCopied]           = useState(false)
  const [warnInst, setWarnInst]       = useState<{ cfg: ConfigEntry; t: InstallTarget } | null>(null)

  useEffect(() => { load() }, [])

  useEffect(() => {
    if (!pickerOpen) return
    const handler = (e: MouseEvent) => {
      if (pickerRef.current && !pickerRef.current.contains(e.target as Node))
        setPickerOpen(false)
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [pickerOpen])

  async function load() {
    setLoading(true); setError('')
    try {
      const roots: { name: string; type: string }[] = await fetch(API(), {
        headers: { Accept: 'application/vnd.github+json' },
      }).then(r => { if (!r.ok) throw new Error(`GitHub ${r.status}`); return r.json() })

      const folders = roots.filter(e => e.type === 'dir')

      const entries = await Promise.allSettled(folders.map(async folder => {
        const contents: { name: string; type: string }[] = await fetch(API(folder.name), {
          headers: { Accept: 'application/vnd.github+json' },
        }).then(r => r.json())

        const jsonEntry   = contents.find(f => f.type === 'file' && f.name.endsWith('.json'))
        if (!jsonEntry) return null
        const readmeEntry = contents.find(f => f.type === 'file' && /^readme\.md$/i.test(f.name))
        const jsonUrl     = RAW(`${folder.name}/${jsonEntry.name}`)

        const [jsonText, readmeText] = await Promise.all([
          fetch(jsonUrl).then(r => r.text()).catch(() => ''),
          readmeEntry
            ? fetch(RAW(`${folder.name}/${readmeEntry.name}`)).then(r => r.text()).catch(() => '')
            : Promise.resolve(''),
        ])

        const resolvedMd = resolveImgSrcs(readmeText, folder.name)
        return {
          name:       folder.name,
          jsonFile:   jsonEntry.name,
          jsonUrl,
          readmeHtml: readmeText ? renderMd(resolvedMd) : '',
          previewImg: firstImage(resolvedMd),
          meta:       parseMeta(jsonText),
        } satisfies ConfigEntry
      }))

      const valid = entries
        .map(r => r.status === 'fulfilled' ? r.value : null)
        .filter((e): e is ConfigEntry => e !== null)

      const favsNow = getFavs()
      valid.sort((a, b) => {
        const af = favsNow.includes(a.name) ? 0 : 1
        const bf = favsNow.includes(b.name) ? 0 : 1
        if (af !== bf) return af - bf
        return a.name.localeCompare(b.name)
      })
      setConfigs(valid)
    } catch (e) {
      setError(`Ошибка загрузки: ${e}`)
    } finally {
      setLoading(false)
    }
  }

  function toggleFav(name: string) {
    const next = favs.includes(name) ? favs.filter(f => f !== name) : [...favs, name]
    setFavsState(next); saveFavs(next)
  }

  function hasUpdate(cfg: ConfigEntry) {
    const inst = installed[cfg.name]
    return !!(inst && cfg.meta.date && (inst.date !== cfg.meta.date || inst.time !== cfg.meta.time))
  }

  async function openPicker() {
    setLoadingT(true)
    try {
      const res = await invoke<InstallTarget[]>('get_lb_installable_instances')
      setTargets(res)
      setPickerOpen(true)
    } catch {
      setInstallErr('Не удалось получить список инстансов')
    } finally {
      setLoadingT(false)
    }
  }

  async function doInstall(cfg: ConfigEntry, t: InstallTarget) {
    setInstalling(true); setInstallErr(''); setPickerOpen(false)
    try {
      await invoke('install_lb_config', {
        jsonUrl:      cfg.jsonUrl,
        instanceName: t.name,
        fileName:     cfg.jsonFile,
      })
      const map = getInstalled()
      map[cfg.name] = { date: cfg.meta.date, time: cfg.meta.time, file: cfg.jsonFile }
      saveInstalled(map); setInstalled({ ...map })
      setPostCmd(`.localconfig load ${cfg.jsonFile.replace(/\.json$/i, '')}`)
    } catch (e) {
      setInstallErr(`Ошибка: ${e}`)
    } finally {
      setInstalling(false); setWarnInst(null)
    }
  }

  function requestInstall(cfg: ConfigEntry, t: InstallTarget) {
    const instVersion = lbInstances.find(i => i.name === t.name)?.version
    // Only warn if we know the exact installed version AND it differs from config's version
    const mismatch = cfg.meta.clientVersion && instVersion &&
      instVersion !== 'latest' && instVersion !== cfg.meta.clientVersion
    if (mismatch) setWarnInst({ cfg, t })
    else doInstall(cfg, t)
  }

  function openDetail(cfg: ConfigEntry) {
    setSelected(cfg); setPostCmd(''); setInstallErr(''); setPickerOpen(false)
  }

  function closeDetail() {
    setSelected(null); setPostCmd(''); setInstallErr(''); setPickerOpen(false)
  }

  const panelVariants = {
    hidden:  { opacity: 0, y: 28, scale: 0.96 },
    visible: { opacity: 1, y: 0,  scale: 1    },
  }

  return (
    <motion.div className="lb-configs-overlay"
      initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
      transition={{ duration: 0.2 }}
      onClick={e => { if (e.target === e.currentTarget) onClose() }}
    >
      <motion.div
        className={`lb-configs-panel${selected ? ' lb-configs-panel--detail' : ''}`}
        variants={panelVariants} initial="hidden" animate="visible" exit="hidden"
        transition={{ duration: 0.28, ease }}
        layout
      >
        {/* ── Header ── */}
        <div className="lb-configs-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <AnimatePresence mode="wait" initial={false}>
              {selected ? (
                <motion.button key="back"
                  className="btn-secondary" style={{ padding: '3px 10px', fontSize: 12, flexShrink: 0 }}
                  onClick={closeDetail}
                  initial={{ opacity: 0, x: -8 }} animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: -8 }} transition={{ duration: 0.15 }}
                >{t('lb.back')}</motion.button>
              ) : null}
            </AnimatePresence>
            <span className="lb-configs-title" style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: 280 }}>
              {selected ? selected.name : 'LB Configs'}
            </span>
            {selected?.meta.author && (
              <span style={{ fontSize: 12, color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>by {selected.meta.author}</span>
            )}
            {selected?.meta.clientVersion && (
              <span style={{ fontSize: 11, color: 'var(--text-muted)', opacity: 0.7, whiteSpace: 'nowrap' }}>· LB {selected.meta.clientVersion}</span>
            )}
          </div>
          <div className="lb-configs-header-actions">
            {selected ? (
              <button
                className={`btn-secondary${favs.includes(selected.name) ? ' lb-fav-active' : ''}`}
                style={{ padding: '4px 10px', fontSize: 13 }}
                onClick={() => toggleFav(selected.name)}
              >★</button>
            ) : (
              <button className="btn-secondary" style={{ padding: '4px 10px', fontSize: 11 }}
                onClick={load} disabled={loading}>{loading ? '…' : '↺'}</button>
            )}
            <button className="wc cls" onClick={onClose}>✕</button>
          </div>
        </div>

        {/* ── Body ── */}
        <AnimatePresence mode="wait" initial={false}>
          {!selected ? (
            /* GRID VIEW */
            <motion.div key="grid" className="lb-configs-body"
              initial={{ opacity: 0, x: -16 }} animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -16 }} transition={{ duration: 0.2, ease }}
            >
              {loading && <div className="lb-configs-empty">{t('lb.loading')}</div>}
              {!loading && error && <div className="lb-configs-empty" style={{ color: '#f87171' }}>{error}</div>}
              {!loading && !error && configs.length === 0 && <div className="lb-configs-empty">{t('lb.empty')}</div>}
              {!loading && !error && (
                <div className="lb-configs-grid">
                  {configs.map((cfg, i) => (
                    <motion.div key={cfg.name} className="lb-config-card"
                      initial={{ opacity: 0, y: 14 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: i * 0.06, duration: 0.28, ease }}
                      onClick={() => openDetail(cfg)}
                    >
                      {cfg.previewImg
                        ? <img src={cfg.previewImg} alt="" className="lb-config-card-img" />
                        : <div className="lb-config-card-img lb-config-card-noimg" />}
                      <div className="lb-config-card-body">
                        <div className="lb-config-card-name">
                          {cfg.name}
                          {favs.includes(cfg.name) && <span className="lb-config-fav-badge">★</span>}
                          {hasUpdate(cfg) && <span className="lb-config-update-badge">Update</span>}
                        </div>
                        <div className="lb-config-card-meta">
                          {cfg.meta.type && <span>{cfg.meta.type}</span>}
                          {cfg.meta.clientVersion && <span>LB {cfg.meta.clientVersion}</span>}
                        </div>
                        {cfg.meta.author && <div className="lb-config-card-author">by {cfg.meta.author}</div>}
                      </div>
                    </motion.div>
                  ))}
                </div>
              )}
            </motion.div>
          ) : (
            /* DETAIL VIEW: two-column */
            <motion.div key="detail" className="lb-detail-split"
              initial={{ opacity: 0, x: 24 }} animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 24 }} transition={{ duration: 0.22, ease }}
            >
              {/* LEFT: scrollable README */}
              <div className="lb-detail-left">
                <div className="lb-detail-readme"
                  dangerouslySetInnerHTML={{ __html: selected.readmeHtml || `<span style="color:var(--text-muted);font-size:13px">${t('lb.no_desc')}</span>` }}
                />
              </div>

              {/* RIGHT: metadata + install sidebar */}
              <div className="lb-detail-right">
                {/* Meta chips */}
                <div className="lb-detail-meta" style={{ flexWrap: 'wrap', gap: 6, marginBottom: 18 }}>
                  {selected.meta.clientVersion && (
                    <span className="lb-meta-chip" style={{ color: 'var(--accent)', borderColor: 'rgba(var(--accent-rgb)/0.3)' }}>
                      LB {selected.meta.clientVersion}
                    </span>
                  )}
                  {selected.meta.type   && <span className="lb-meta-chip">{selected.meta.type}</span>}
                  {selected.meta.status && <span className="lb-meta-chip">{selected.meta.status}</span>}
                  {selected.meta.date   && <span className="lb-meta-chip">📅 {selected.meta.date}</span>}
                  {selected.meta.time   && <span className="lb-meta-chip">🕐 {selected.meta.time}</span>}
                  {selected.meta.clientCommit && (
                    <span className="lb-meta-chip" style={{ fontFamily: 'monospace', fontSize: 10 }}>
                      {selected.meta.clientCommit.slice(0, 10)}
                    </span>
                  )}
                  {installed[selected.name] && !hasUpdate(selected) && (
                    <span className="lb-meta-chip" style={{ color: '#4ade80', borderColor: 'rgba(74,222,128,0.3)' }}>{t('lb.installed_badge')}</span>
                  )}
                  {hasUpdate(selected) && (
                    <span className="lb-meta-chip" style={{ color: '#fbbf24', borderColor: 'rgba(251,191,36,0.3)' }}>{t('lb.update_badge')}</span>
                  )}
                </div>

                {/* Install button + picker */}
                <div className="lb-install-wrap" ref={pickerRef} style={{ position: 'relative', marginBottom: 8 }}>
                  <button
                    className="lb-install-btn"
                    style={{ width: '100%' }}
                    disabled={installing || loadingTargets}
                    onClick={openPicker}
                  >
                    {installing ? t('lb.installing') : loadingTargets ? '…' : t('lb.install')}
                  </button>
                  <AnimatePresence>
                    {pickerOpen && (
                      <motion.div className="lb-install-picker"
                        initial={{ opacity: 0, y: -8, scale: 0.96 }}
                        animate={{ opacity: 1, y: 0,  scale: 1    }}
                        exit={{ opacity: 0, y: -8, scale: 0.96 }}
                        transition={{ duration: 0.18, ease }}
                      >
                        {targets.length === 0
                          ? <div className="lb-install-picker-empty">{t('lb.no_lb_instances')}</div>
                          : targets.map(t => (
                            <button key={t.name} className="lb-install-picker-item"
                              onClick={() => requestInstall(selected, t)}
                            >{t.name}</button>
                          ))
                        }
                      </motion.div>
                    )}
                  </AnimatePresence>
                </div>
                {installErr && <div style={{ fontSize: 12, color: '#f87171', marginBottom: 8 }}>{installErr}</div>}

                {/* Post-install command */}
                <AnimatePresence>
                  {postCmd && (
                    <motion.div className="lb-postinstall"
                      initial={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, height: 'auto' }}
                      exit={{ opacity: 0, height: 0 }}
                      transition={{ duration: 0.22 }}
                    >
                      <span className="lb-postinstall-title">✓ Установлен!</span>
                      <span style={{ fontSize: 12 }}>Загрузи в игре:</span>
                      <div className="lb-postinstall-cmd">
                        <code>{postCmd}</code>
                        <button className="btn-secondary" style={{ padding: '3px 8px', fontSize: 11, marginLeft: 8 }}
                          onClick={() => { navigator.clipboard.writeText(postCmd); setCopied(true); setTimeout(() => setCopied(false), 1600) }}
                        >{copied ? '✓' : 'Копировать'}</button>
                      </div>
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>

      {/* ── Version mismatch warning modal ── */}
      <AnimatePresence>
        {warnInst && (
          <motion.div className="lb-configs-overlay" style={{ zIndex: 3002 }}
            initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
          >
            <motion.div className="lb-warn-modal"
              initial={{ scale: 0.94, opacity: 0, y: 16 }}
              animate={{ scale: 1,    opacity: 1, y: 0  }}
              exit={{ scale: 0.94, opacity: 0, y: 16 }}
              transition={{ duration: 0.18, ease }}
            >
              <div className="lb-warn-title">⚠ Версия не совпадает</div>
              <div className="lb-warn-body">
                Конфиг сохранён на <strong>LB {warnInst.cfg.meta.clientVersion}</strong>.
                Возможны баги и несовместимые настройки. Установить всё равно?
              </div>
              <div className="lb-warn-actions">
                <button className="btn-secondary" onClick={() => setWarnInst(null)}>Отмена</button>
                <button className="lb-install-btn" style={{ borderRadius: 8 }}
                  onClick={() => doInstall(warnInst.cfg, warnInst.t)}>Установить</button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  )
}
