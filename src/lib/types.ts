// ─── Shared types and constants ──────────────────────────────────────────────

// ─── Types ────────────────────────────────────────────────────────────────────

// Microsoft tokens are not part of this type: they live in the Rust vault
// (src-tauri/src/vault.rs) and are looked up by uuid at launch.
export type Account   = { type: 'offline' | 'microsoft'; username: string; uuid: string }
export type MCVersion = { id: string; type: 'release' | 'snapshot' | 'old_alpha' | 'old_beta'; releaseTime: string }
export type LBVersion = { tag: string; mcVersion: string; date: string; buildId?: number }
export type Instance  = { id: string; name: string; type: 'mc' | 'lb'; version: string; mcVersion: string; buildId?: number; loader?: 'vanilla' | 'fabric' | 'quilt' | 'forge' | 'neoforge'; loaderVersion?: string }
export type VFilter    = 'release' | 'snapshot' | 'old' | 'all'
export type LoaderVersionInfo = { version: string; stable: boolean; latest: boolean }
export type ModRow = { filename: string; enabled: boolean; size: number; source: string; project_id: string; name: string; author: string; version_id: string; version_number: string; icon_url: string }
export type ModUpdateInfo = { filename: string; name: string; current: string; latest: string; version_id: string; file_name: string; download_url: string; size: number; source: string; project_id: string; author: string; icon_url: string }
export type AppState   = 'loading' | 'migrate' | 'ready' | 'error'
export type Tab        = 'mc' | 'lb'
export type UpdateInfo = { version: string; tagName: string; body: string; htmlUrl: string; assetUrl: string; msiUrl: string; assetSha256: string; msiSha256: string; unstableWarning?: boolean }

export const spring = { type: 'spring', stiffness: 400, damping: 30 } as const
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
export const tick = (ms: number) => new Promise<void>(r => setTimeout(r, ms))
