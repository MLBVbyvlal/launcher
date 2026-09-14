import { useState, useEffect, useCallback } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { getLang, useT } from './i18n'

// ─── Types ────────────────────────────────────────────────────────────────────

type Source = 'modrinth' | 'curseforge'

type ModHit = {
  id: string; slug: string; name: string; author: string; summary: string
  icon_url: string; downloads: number; updated: string
}

type ModFile = {
  version_id: string; name: string; version_number: string
  mc_versions: string[]; loaders: string[]; release_type: string
  file_name: string; download_url: string; size: number; published: string
}

type InstalledRef = { filename: string; source: string; project_id: string }

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

// ─── Shared bits ──────────────────────────────────────────────────────────────

export function ModIcon({ url, name, size = 30 }: { url: string; name: string; size?: number }) {
  const [failed, setFailed] = useState(false)
  if (!url || failed) {
    return (
      <div style={{
        width: size, height: size, borderRadius: 7, flexShrink: 0,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontWeight: 800, fontSize: size * 0.45,
        background: 'rgba(255,255,255,0.08)', color: 'var(--text-muted)',
      }}>{(name.trim()[0] ?? '?').toUpperCase()}</div>
    )
  }
  return (
    <img src={url} alt="" draggable={false} onError={() => setFailed(true)}
      style={{ width: size, height: size, borderRadius: 7, objectFit: 'cover', flexShrink: 0 }} />
  )
}

function fmtDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return String(n)
}

function fmtSize(b: number): string {
  if (!b) return ''
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(0)} KB`
  return `${(b / (1024 * 1024)).toFixed(1)} MB`
}

const BADGE: Record<string, { fg: string; bg: string }> = {
  release: { fg: '#4ade80', bg: 'rgba(74,222,128,0.12)' },
  beta:    { fg: '#fbbf24', bg: 'rgba(251,191,36,0.12)' },
  alpha:   { fg: '#f87171', bg: 'rgba(248,113,113,0.12)' },
}

// ─── Browser ──────────────────────────────────────────────────────────────────
// Rendered as an absolute overlay inside the instance-settings modal (same
// pattern as the unstable-loader warning), so no z-index wrestling.

export default function ModBrowser({ instanceName, mcVersion, loader, cfKey, installed, onInstalled, onClose }: {
  instanceName: string
  mcVersion: string
  loader?: string
  cfKey: string
  installed: InstalledRef[]
  onInstalled: () => void
  onClose: () => void
}) {
  const t = useT(getLang())
  const [source, setSource] = useState<Source>('modrinth')
  const [query, setQuery] = useState('')
  const [items, setItems] = useState<ModHit[]>([])
  const [total, setTotal] = useState(0)
  const [offset, setOffset] = useState(0)
  const [loading, setLoading] = useState(false)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState('')
  const [keyIssue, setKeyIssue] = useState<'missing' | 'invalid' | null>(null)

  const [selected, setSelected] = useState<ModHit | null>(null)
  const [versions, setVersions] = useState<ModFile[]>([])
  const [versionsLoading, setVersionsLoading] = useState(false)
  const [versionsError, setVersionsError] = useState('')
  const [installing, setInstalling] = useState<string | null>(null)
  const [installedIds, setInstalledIds] = useState<Set<string>>(new Set())

  const scopeMc = mcVersion ? `MC ${mcVersion}` : t('mods.all_versions')
  const scopeLoader = loader && loader !== 'vanilla' ? ` · ${loader}` : ''

  const runSearch = useCallback(async (src: Source, q: string, off: number, append: boolean) => {
    if (!isTauri) return
    if (append) setLoadingMore(true); else setLoading(true)
    setError('')
    setKeyIssue(null)
    try {
      const r = await invoke<{ items: ModHit[]; total: number }>('search_mods', {
        source: src, query: q, mcVersion, loader: loader ?? 'vanilla', cfKey, limit: 25, offset: off,
      })
      setItems(prev => append ? [...prev, ...r.items] : r.items)
      setTotal(r.total)
      setOffset(off + r.items.length)
    } catch (e) {
      const msg = String(e)
      if (msg.includes('CF_KEY_MISSING')) setKeyIssue('missing')
      else if (msg.includes('CF_KEY_INVALID')) setKeyIssue('invalid')
      else setError(msg)
      if (!append) { setItems([]); setTotal(0) }
    } finally {
      setLoading(false)
      setLoadingMore(false)
    }
  }, [mcVersion, loader, cfKey])

  // Debounced search; empty query searches immediately (popular first).
  useEffect(() => {
    const id = setTimeout(() => runSearch(source, query, 0, false), query ? 400 : 0)
    return () => clearTimeout(id)
  }, [source, query, runSearch])

  useEffect(() => {
    if (!selected) return
    setVersions([])
    setVersionsLoading(true)
    setVersionsError('')
    invoke<ModFile[]>('get_mod_versions', {
      source, projectId: selected.id, mcVersion, loader: loader ?? 'vanilla', cfKey,
    })
      .then(setVersions)
      .catch(e => setVersionsError(String(e)))
      .finally(() => setVersionsLoading(false))
  }, [selected, source, mcVersion, loader, cfKey])

  const install = async (f: ModFile) => {
    if (!selected || installing) return
    setInstalling(f.version_id)
    try {
      // Re-installing the same project replaces the old jar instead of
      // stacking two copies that Minecraft would choke on.
      const existing = installed.find(i => i.source === source && i.project_id === selected.id)
      await invoke<string>('install_mod_file', {
        instanceName,
        downloadUrl: f.download_url,
        fileName: f.file_name,
        replaceExisting: existing && existing.filename !== f.file_name ? existing.filename : null,
        meta: {
          source,
          project_id: selected.id,
          name: selected.name,
          author: selected.author,
          version_id: f.version_id,
          version_number: f.version_number || f.name,
          icon_url: selected.icon_url,
        },
      })
      setInstalledIds(prev => new Set(prev).add(f.version_id))
      onInstalled()
    } catch (e) {
      setVersionsError(String(e))
    }
    setInstalling(null)
  }

  const back = () => { setSelected(null); setVersions([]); setVersionsError(''); setInstalledIds(new Set()) }

  return (
    <motion.div
      initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
      style={{
        position: 'absolute', inset: 0, zIndex: 60, display: 'flex',
        alignItems: 'center', justifyContent: 'center',
        background: 'rgba(0,0,0,0.72)', backdropFilter: 'blur(6px)', borderRadius: 'inherit',
      }}
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.94, opacity: 0, y: 12 }} animate={{ scale: 1, opacity: 1, y: 0 }}
        transition={{ type: 'spring', stiffness: 380, damping: 30 }}
        style={{
          width: 'min(580px, 94%)', height: '88%', display: 'flex', flexDirection: 'column',
          background: 'rgba(16,16,22,0.97)', border: '1px solid rgba(255,255,255,0.09)',
          borderRadius: 14, overflow: 'hidden',
        }}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head" style={{ padding: '12px 16px' }}>
          <span className="modal-title" style={{ fontSize: 14 }}>
            {selected ? selected.name : t('mods.title')}
            {!selected && <span style={{ color: 'var(--text-muted)', fontWeight: 400, fontSize: 12 }}> · {scopeMc}{scopeLoader}</span>}
          </span>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>

        {!selected ? (
          <>
            <div style={{ display: 'flex', gap: 8, padding: '0 16px 10px', alignItems: 'center' }}>
              <div className="vtabs" style={{ display: 'flex', gap: 4 }}>
                {(['modrinth', 'curseforge'] as Source[]).map(s => (
                  <button key={s} className={`vtab${source === s ? ' on' : ''}`}
                    onClick={() => { setSource(s); setSelected(null) }}>
                    {s === 'modrinth' ? 'Modrinth' : 'CurseForge'}
                  </button>
                ))}
              </div>
              <input className="glass-input" style={{ flex: 1, padding: '7px 12px', fontSize: 13 }}
                placeholder={t('mods.search_ph')} value={query}
                onChange={e => setQuery(e.target.value)} autoFocus />
            </div>

            <div className="vlist" style={{ flex: 1, padding: '0 12px 12px' }}>
              {loading ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('loading')}</div>
              ) : keyIssue ? (
                <div style={{ padding: 22, fontSize: 13, color: 'var(--text-muted)', lineHeight: 1.6 }}>
                  {keyIssue === 'missing' ? t('mods.needs_key') : t('mods.key_invalid')}
                </div>
              ) : error ? (
                <div style={{ padding: 16, color: '#f87171', fontSize: 12 }}>{t('error.prefix')} {error}</div>
              ) : items.length === 0 ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('mods.no_results')}</div>
              ) : items.map(m => (
                <motion.button key={m.id}
                  onClick={() => setSelected(m)}
                  whileHover={{ x: 3 }} transition={{ type: 'spring', stiffness: 400, damping: 30 }}
                  style={{
                    display: 'flex', gap: 10, alignItems: 'center', width: '100%', textAlign: 'left',
                    background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
                    borderRadius: 10, padding: '8px 10px', marginBottom: 6, cursor: 'pointer', color: 'inherit',
                  }}
                >
                  <ModIcon url={m.icon_url} name={m.name} size={34} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontSize: 13, fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                      {m.name}
                      {m.author && <span style={{ fontWeight: 400, color: 'var(--text-muted)', fontSize: 11 }}> · {m.author}</span>}
                    </div>
                    <div style={{ fontSize: 11, color: 'var(--text-muted)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                      {m.summary || m.slug}
                    </div>
                  </div>
                  <div style={{ textAlign: 'right', flexShrink: 0, fontSize: 11, color: 'var(--text-muted)' }}>
                    <div>⬇ {fmtDownloads(m.downloads)}</div>
                    {m.updated && <div>{m.updated}</div>}
                  </div>
                </motion.button>
              ))}
              {!loading && !keyIssue && !error && items.length > 0 && items.length < total && (
                <button className="btn-secondary" style={{ width: '100%', marginTop: 4 }}
                  disabled={loadingMore} onClick={() => runSearch(source, query, offset, true)}>
                  {loadingMore ? t('loading') : `${t('mods.load_more')} (${items.length}/${total})`}
                </button>
              )}
            </div>
          </>
        ) : (
          <>
            <div style={{ display: 'flex', gap: 10, padding: '0 16px 10px', alignItems: 'center' }}>
              <button className="btn-secondary" style={{ padding: '5px 12px', fontSize: 12 }} onClick={back}>
                {t('mods.back')}
              </button>
              <ModIcon url={selected.icon_url} name={selected.name} size={26} />
              <div style={{ flex: 1, minWidth: 0, fontSize: 12, color: 'var(--text-muted)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {selected.summary}
              </div>
            </div>

            <div className="vlist" style={{ flex: 1, padding: '0 12px 12px' }}>
              {versionsLoading ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('loading')}</div>
              ) : versionsError ? (
                <div style={{ padding: 16, color: '#f87171', fontSize: 12 }}>{t('error.prefix')} {versionsError}</div>
              ) : versions.length === 0 ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('mods.no_results')}</div>
              ) : versions.map(f => {
                const badge = BADGE[f.release_type] ?? BADGE.release
                const done = installedIds.has(f.version_id)
                return (
                  <div key={f.version_id}
                    style={{
                      display: 'flex', gap: 10, alignItems: 'center',
                      background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(255,255,255,0.06)',
                      borderRadius: 10, padding: '8px 10px', marginBottom: 6,
                    }}
                  >
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ fontSize: 13, fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                        {f.version_number || f.name}
                      </div>
                      <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>
                        {f.mc_versions.slice(0, 3).join(', ')}{f.mc_versions.length > 3 ? '…' : ''}
                        {f.size > 0 && <> · {fmtSize(f.size)}</>}
                        {f.published && <> · {f.published}</>}
                      </div>
                      <div style={{ fontSize: 10, color: 'var(--text-muted)', opacity: 0.7, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                        {f.file_name}
                      </div>
                    </div>
                    <span style={{
                      fontSize: 10, fontWeight: 700, padding: '2px 8px', borderRadius: 20,
                      color: badge.fg, background: badge.bg, flexShrink: 0,
                    }}>{f.release_type}</span>
                    <button className={done ? 'btn-secondary' : 'btn-ok'}
                      style={{ padding: '6px 14px', fontSize: 12, flexShrink: 0 }}
                      disabled={installing !== null || done}
                      onClick={() => install(f)}>
                      {installing === f.version_id ? '…' : done ? `✓ ${t('mods.installed')}` : t('mods.install')}
                    </button>
                  </div>
                )
              })}
            </div>
          </>
        )}
      </motion.div>
    </motion.div>
  )
}
