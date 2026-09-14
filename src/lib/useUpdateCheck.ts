import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { isTauri, type AppState, type UpdateInfo } from './types'

// ─── Update check on startup ──────────────────────────────────────────────────

export function useUpdateCheck(appState: AppState) {
  const [updateInfo, setUpdateInfo]           = useState<UpdateInfo | null>(null)
  const [updateCheckError, setUpdateCheckError] = useState<string | null>(null)
  const [justUpdated, setJustUpdated]         = useState<string | null>(null)

  // Check for updates once the app is ready; also check if we just updated
  useEffect(() => {
    if (appState !== 'ready' || !isTauri) return
    // Was the app just updated?
    invoke<string>('get_just_updated')
      .then(ver => { if (ver) { setJustUpdated(ver); setTimeout(() => setJustUpdated(null), 5000) } })
      .catch(() => {})
    type RawRelease = { version: string; tag_name: string; body: string; html_url: string; asset_url: string; msi_url: string; asset_sha256: string; msi_sha256: string; unstable_warning: boolean }
    const checkForUpdate = () => {
      invoke<RawRelease | null>('check_for_update')
        .then(r => {
          setUpdateCheckError(null)
          if (r) setUpdateInfo({ version: r.version, tagName: r.tag_name, body: r.body, htmlUrl: r.html_url, assetUrl: r.asset_url, msiUrl: r.msi_url, assetSha256: r.asset_sha256, msiSha256: r.msi_sha256, unstableWarning: r.unstable_warning })
        })
        .catch(e => setUpdateCheckError(String(e)))
    }
    // The self-updater is Windows-only (NSIS/MSI); on other systems there is
    // nothing to check for, so skip quietly instead of showing an error.
    invoke<Record<string, unknown>>('get_debug_info')
      .then(d => { if (d.os === 'windows') checkForUpdate() })
      .catch(() => checkForUpdate())
  }, [appState])

  return { updateInfo, setUpdateInfo, updateCheckError, justUpdated }
}
