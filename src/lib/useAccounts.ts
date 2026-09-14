import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { isTauri, type Account } from './types'

// ─── Accounts ─────────────────────────────────────────────────────────────────
//
// The frontend copy of an account is `{type, username, uuid}` — nothing else.
// Tokens live in the Rust vault (src-tauri/src/vault.rs) and are looked up by
// uuid when a launch starts, which gives this hook two jobs:
//
//   * A Microsoft entry without a vault record cannot launch (the token does
//     not exist on this machine — data reset, another install, a pre-0.0.6
//     profile). Those accounts are flagged and offered for re-login, not
//     silently kept in the list and not silently deleted either.
//   * Removing an account has to remove its vault entry in the same breath, or
//     the refresh token outlives the account it was issued for.

/** Parse the stored list, keeping only the three fields that belong here. */
function readStored(): Account[] {
  try {
    const raw = JSON.parse(localStorage.getItem('mlbv_accounts') ?? '[]') as
      Array<Partial<Account> & Record<string, unknown>>
    if (!Array.isArray(raw)) return []
    // Rebuilding the object is what drops the `accessToken`/`refreshToken`
    // fields that pre-0.0.6 builds persisted, so the plaintext copy disappears
    // on the next write instead of lingering until the next sign-in.
    return raw
      .filter(a => (a.type === 'offline' || a.type === 'microsoft')
        && typeof a.username === 'string' && typeof a.uuid === 'string')
      .map(a => ({ type: a.type as Account['type'], username: a.username as string, uuid: a.uuid as string }))
  } catch { return [] }
}

export function useAccounts() {
  const [accounts, setAccounts] = useState<Account[]>(readStored)
  const [selected, setSelected] = useState<Account | null>(() => {
    const uuid = localStorage.getItem('mlbv_selected_uuid')
    return uuid ? readStored().find(a => a.uuid === uuid) ?? null : null
  })
  const [msLoading, setMsLoading] = useState(false)
  const [msError, setMsError]     = useState('')
  // uuids of Microsoft accounts that are listed but have no token on disk.
  const [needsRelogin, setNeedsRelogin] = useState<string[]>([])

  useEffect(() => { localStorage.setItem('mlbv_accounts', JSON.stringify(accounts)) }, [accounts])
  useEffect(() => {
    if (selected) localStorage.setItem('mlbv_selected_uuid', selected.uuid)
    else localStorage.removeItem('mlbv_selected_uuid')
  }, [selected])

  // Ask the vault which Microsoft accounts can actually launch. Re-run on
  // every list change so a fresh sign-in clears its own badge; the check is a
  // lookup in a state object, not a scan. A failed check keeps the account
  // listed — hiding someone's account because a probe errored is worse than a
  // stale badge, and `launch_game` still refuses with an explicit error.
  useEffect(() => {
    if (!isTauri) { setNeedsRelogin([]); return }
    const ms = accounts.filter(a => a.type === 'microsoft')
    if (ms.length === 0) { setNeedsRelogin([]); return }
    let live = true
    void Promise.all(ms.map(a => invoke<boolean>('vault_has_account', { uuid: a.uuid }).then(ok => ok, () => true)))
      .then(present => {
        if (!live) return
        setNeedsRelogin(ms.filter((_, i) => !present[i]).map(a => a.uuid))
      })
    return () => { live = false }
  }, [accounts])

  /** Put an account in the list (and select it), whether it is new or signing
   *  back in. Used by the modal, the sidebar and the first-run wizard. */
  const adopt = useCallback((acct: Account) => {
    setAccounts(prev => [...prev.filter(a => a.uuid !== acct.uuid), acct])
    setNeedsRelogin(prev => prev.filter(u => u !== acct.uuid))
    setSelected(acct)
  }, [])

  const addOffline = useCallback((name: string) => {
    const trimmed = name.trim()
    if (!trimmed) return
    adopt({ type: 'offline', username: trimmed, uuid: crypto.randomUUID() })
  }, [adopt])

  /** Real Microsoft sign-in. Also the "sign in again" path for a badged
   *  account: the flow writes a fresh vault entry for the returned uuid.
   *  Resolves to the failure text (or null) so a caller can decide whether to
   *  close its modal — `msError` state is not readable synchronously here. */
  const loginMicrosoft = useCallback(async (): Promise<string | null> => {
    if (!isTauri) return null
    setMsLoading(true); setMsError('')
    try {
      const raw = await invoke<{ username: string; uuid: string }>('microsoft_login')
      adopt({ type: 'microsoft', username: raw.username, uuid: raw.uuid })
      setMsLoading(false)
      return null
    } catch (err) {
      const msg = String(err)
      setMsError(msg); setMsLoading(false)
      return msg
    }
  }, [adopt])

  /** Forget an account. For a Microsoft one this deletes the vault entry too;
   *  if that delete fails the account is kept listed and the error comes back
   *  to the caller — dropping it would leave a live refresh token behind with
   *  nothing in the UI pointing at it. */
  const removeAccount = useCallback(async (acct: Account): Promise<string | null> => {
    if (acct.type === 'microsoft' && isTauri) {
      try { await invoke('vault_forget_account', { uuid: acct.uuid }) }
      catch (err) { return String(err) }
    }
    setAccounts(prev => prev.filter(a => a.uuid !== acct.uuid))
    setNeedsRelogin(prev => prev.filter(u => u !== acct.uuid))
    setSelected(prev => prev?.uuid === acct.uuid ? null : prev)
    return null
  }, [])

  return {
    accounts, selected, setSelected, adopt,
    msLoading, msError, setMsError,
    needsRelogin,
    addOffline, loginMicrosoft, removeAccount,
  }
}
