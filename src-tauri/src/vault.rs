//! Microsoft account tokens never reach the WebView: they are kept here,
//! keyed by profile UUID, in `<mlbv base>/accounts.bin`. On Windows the file
//! is sealed with DPAPI (`CryptProtectData`, current user scope), so only the
//! same Windows user can read it back. Elsewhere the file is written with
//! mode 0600 — no per-user crypto API is available without new dependencies.
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Access tokens live ~24 h; refresh a little earlier so a launch never
/// starts with a token that dies mid-session handshake.
pub const REFRESH_AFTER_SECS: u64 = 20 * 3600;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct StoredTokens {
    pub access_token:  String,
    pub refresh_token: String,
    /// Unix seconds when `access_token` was obtained.
    pub obtained_at:   u64,
}

impl StoredTokens {
    pub fn is_stale(&self) -> bool {
        now_secs().saturating_sub(self.obtained_at) > REFRESH_AFTER_SECS
    }
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// In-memory copy of the vault; every mutation is written through.
pub struct Vault {
    inner: Mutex<HashMap<String, StoredTokens>>,
}

fn vault_path() -> PathBuf {
    crate::launcher::mlbv_base().join("accounts.bin")
}

impl Vault {
    pub fn load() -> Self {
        let map = read_vault().unwrap_or_default();
        Vault { inner: Mutex::new(map) }
    }

    pub fn get(&self, uuid: &str) -> Option<StoredTokens> {
        self.inner.lock().ok().and_then(|m| m.get(uuid).cloned())
    }

    pub fn put(&self, uuid: &str, tokens: StoredTokens) -> Result<()> {
        let snapshot = {
            let mut m = self.inner.lock().map_err(|_| anyhow!("vault lock poisoned"))?;
            m.insert(uuid.to_string(), tokens);
            m.clone()
        };
        write_vault(&snapshot)
    }

    pub fn remove(&self, uuid: &str) -> Result<()> {
        let snapshot = {
            let mut m = self.inner.lock().map_err(|_| anyhow!("vault lock poisoned"))?;
            m.remove(uuid);
            m.clone()
        };
        write_vault(&snapshot)
    }

    pub fn has(&self, uuid: &str) -> bool {
        self.inner.lock().map(|m| m.contains_key(uuid)).unwrap_or(false)
    }
}

fn read_vault() -> Result<HashMap<String, StoredTokens>> {
    let path = vault_path();
    if !path.exists() { return Ok(HashMap::new()); }
    let raw = fs::read(&path).context("Reading account vault")?;
    let plain = unprotect(&raw).context("Decrypting account vault")?;
    Ok(serde_json::from_slice(&plain).context("Parsing account vault")?)
}

fn write_vault(map: &HashMap<String, StoredTokens>) -> Result<()> {
    let path = vault_path();
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    let plain = serde_json::to_vec(map)?;
    let sealed = protect(&plain).context("Encrypting account vault")?;
    // Write-then-rename so a crash mid-write cannot leave a truncated vault.
    let tmp = path.with_extension("bin.tmp");
    fs::write(&tmp, &sealed)?;
    restrict_permissions(&tmp);
    fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}
#[cfg(not(unix))]
fn restrict_permissions(_path: &PathBuf) {}

// ─── DPAPI (Windows) ──────────────────────────────────────────────────────────

#[cfg(windows)]
mod dpapi {
    use anyhow::{anyhow, Result};
    use windows_sys::Win32::Foundation::{LocalFree, HLOCAL};
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
    };

    fn call(data: &[u8], protect: bool) -> Result<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        // SAFETY: `input` points at a live slice for the duration of the call;
        // `output` is filled by the API with a LocalAlloc'd buffer that we copy
        // out and free exactly once below. No optional entropy, no prompt UI.
        let ok = unsafe {
            if protect {
                CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(),
                    std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            } else {
                CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(),
                    std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            }
        };
        if ok == 0 || output.pbData.is_null() {
            return Err(anyhow!("DPAPI call failed (protect={protect})"));
        }
        let out = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe { LocalFree(output.pbData as HLOCAL); }
        Ok(out)
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>> { call(data, true) }
    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>> { call(data, false) }
}

const MAGIC_DPAPI: &[u8] = b"MLBV1D";
const MAGIC_PLAIN: &[u8] = b"MLBV1P";

fn protect(plain: &[u8]) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        let sealed = dpapi::protect(plain)?;
        let mut out = MAGIC_DPAPI.to_vec();
        out.extend_from_slice(&sealed);
        return Ok(out);
    }
    #[allow(unreachable_code)]
    {
        let mut out = MAGIC_PLAIN.to_vec();
        out.extend_from_slice(plain);
        Ok(out)
    }
}

fn unprotect(raw: &[u8]) -> Result<Vec<u8>> {
    if let Some(body) = raw.strip_prefix(MAGIC_PLAIN) {
        return Ok(body.to_vec());
    }
    if let Some(body) = raw.strip_prefix(MAGIC_DPAPI) {
        #[cfg(windows)]
        { return dpapi::unprotect(body); }
        #[cfg(not(windows))]
        { let _ = body; return Err(anyhow!("Vault was sealed on Windows and cannot be opened here")); }
    }
    Err(anyhow!("Unknown vault format"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_roundtrip_has_magic() {
        let sealed = protect(b"hello").unwrap();
        assert!(sealed.starts_with(MAGIC_DPAPI) || sealed.starts_with(MAGIC_PLAIN));
        assert_eq!(unprotect(&sealed).unwrap(), b"hello");
    }

    #[test]
    fn unknown_format_is_rejected() {
        assert!(unprotect(b"garbage").is_err());
        assert!(unprotect(b"").is_err());
    }

    #[test]
    fn staleness_uses_refresh_window() {
        let fresh = StoredTokens { obtained_at: now_secs(), ..Default::default() };
        assert!(!fresh.is_stale());
        let old = StoredTokens { obtained_at: now_secs() - REFRESH_AFTER_SECS - 1, ..Default::default() };
        assert!(old.is_stale());
    }
}
