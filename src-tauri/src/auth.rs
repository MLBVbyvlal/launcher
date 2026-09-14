//! Microsoft account sign-in and session refresh. Tokens are written to the
//! vault (`vault.rs`) and never returned to the WebView.
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use crate::vault;

// ── Microsoft auth ────────────────────────────────────────────────────────────

/// What the frontend gets to see of a Microsoft account. Tokens stay in the
/// vault (`vault.rs`) and are looked up by UUID at launch time.
#[derive(serde::Serialize)]
pub(crate) struct MsAccount {
    username: String,
    uuid: String,
}

pub(crate) fn store_tokens(app: &tauri::AppHandle, uuid: &str, access: String, refresh: String) -> Result<(), String> {
    app.state::<vault::Vault>()
        .put(uuid, vault::StoredTokens { access_token: access, refresh_token: refresh, obtained_at: vault::now_secs() })
        .map_err(|e| format!("Saving account: {e:#}"))
}

/// Run the refresh-token grant and the Xbox chain; returns the new tokens.
pub(crate) async fn refresh_tokens(refresh_token: &str) -> Result<(String, String, String, String), String> {
    const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
    const CLIENT_ID:    &str = "00000000402b5328";

    if refresh_token.trim().is_empty() {
        return Err("No refresh token stored — sign in again.".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;

    let ms: serde_json::Value = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("redirect_uri", REDIRECT_URI),
        ])
        .send().await.map_err(|e| format!("MS refresh: {e}"))?
        .json().await.map_err(|e| format!("MS refresh parse: {e}"))?;

    let ms_token = ms["access_token"].as_str()
        .ok_or_else(|| "MS refresh failed (token response had no access_token)".to_string())?
        .to_string();
    let new_refresh = ms["refresh_token"].as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| refresh_token.to_string());

    let (mc_token, uuid, username) = ms_token_chain(&client, &ms_token).await?;
    Ok((username, uuid, mc_token, new_refresh))
}

/// Xbox Live → XSTS → Minecraft token chain, shared by the OAuth login flow
/// and the refresh-token flow. Returns (mc_token, uuid, username).
async fn ms_token_chain(client: &reqwest::Client, ms_token: &str) -> Result<(String, String, String), String> {
    // Xbox Live token
    let xbl: serde_json::Value = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": ms_token,
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))
        .send().await.map_err(|e| format!("Xbox Live: {e}"))?
        .json().await.map_err(|e| format!("XBL parse: {e}"))?;

    let xbl_token = xbl["Token"].as_str().ok_or("Xbox Live auth failed")?;
    let uhs = xbl["DisplayClaims"]["xui"][0]["uhs"].as_str().ok_or("No UHS in XBL")?;

    // XSTS token
    let xsts: serde_json::Value = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token],
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))
        .send().await.map_err(|e| format!("XSTS: {e}"))?
        .json().await.map_err(|e| format!("XSTS parse: {e}"))?;

    if let Some(xerr) = xsts["XErr"].as_u64() {
        return Err(match xerr {
            2148916238 => "Parental consent required for this Xbox account.".to_string(),
            2148916235 => "Xbox Live is not available in your region.".to_string(),
            2148916233 => "No Xbox account — create one at xbox.com first.".to_string(),
            _ => format!("Xbox error {xerr}"),
        });
    }
    let xsts_token = xsts["Token"].as_str().ok_or("XSTS auth failed")?;

    // Minecraft token
    let mc_auth: serde_json::Value = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{xsts_token}"),
        }))
        .send().await.map_err(|e| format!("Minecraft auth: {e}"))?
        .json().await.map_err(|e| format!("MC auth parse: {e}"))?;

    let mc_token = mc_auth["access_token"].as_str()
        .ok_or("Minecraft auth failed — account may not own Minecraft Java Edition")?
        .to_string();

    // Minecraft profile (UUID + username)
    let profile: serde_json::Value = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .header("Authorization", format!("Bearer {mc_token}"))
        .send().await.map_err(|e| format!("Profile fetch: {e}"))?
        .json().await.map_err(|e| format!("Profile parse: {e}"))?;

    if profile["error"].is_string() {
        return Err("This account does not own Minecraft Java Edition.".to_string());
    }

    let uuid     = profile["id"].as_str().ok_or("No UUID in profile")?.to_string();
    let username = profile["name"].as_str().ok_or("No name in profile")?.to_string();

    Ok((mc_token, uuid, username))
}

/// Which stored accounts still have a token on this machine. The frontend
/// badges the Microsoft accounts that do not (src/lib/useAccounts.ts →
/// `needsRelogin`) and offers a sign-in, instead of letting Play fail after the
/// whole download. `refresh_tokens` is deliberately not exposed as a command:
/// `launch_game` refreshes by itself, and a session with no vault entry can
/// only be fixed by `microsoft_login`.
#[tauri::command]
pub(crate) fn vault_has_account(app: tauri::AppHandle, uuid: String) -> bool {
    app.state::<vault::Vault>().has(&uuid)
}

/// Delete an account's tokens. Called when the user removes the account, so a
/// refresh token never outlives the list entry that points at it.
#[tauri::command]
pub(crate) fn vault_forget_account(app: tauri::AppHandle, uuid: String) -> Result<(), String> {
    app.state::<vault::Vault>().remove(&uuid).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub(crate) async fn microsoft_login(app: tauri::AppHandle) -> Result<MsAccount, String> {
    const REDIRECT_URI: &str = "https://login.live.com/oauth20_desktop.srf";
    const CLIENT_ID:    &str = "00000000402b5328";

    let auth_url = url::Url::parse(&format!(
        "https://login.live.com/oauth20_authorize.srf\
         ?client_id={CLIENT_ID}\
         &response_type=code\
         &scope=service%3A%3Auser.auth.xboxlive.com%3A%3AMBI_SSL\
         &redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf\
         &prompt=login"
    )).map_err(|e| format!("URL: {e}"))?;

    // Shared state between on_navigation callback and async polling loop
    let code_slot: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let closed     = Arc::new(AtomicBool::new(false));

    let code_nav   = code_slot.clone();
    let closed_ev  = closed.clone();

    // Embedded WebView window — Microsoft login renders here
    let win = tauri::WebviewWindowBuilder::new(
        &app, "ms-auth",
        tauri::WebviewUrl::External(auth_url),
    )
    .title("Sign in with Microsoft — MLBV")
    .inner_size(480.0, 660.0)
    .center()
    .on_navigation(move |url| {
        // Intercept the final redirect to oauth20_desktop.srf
        if url.host_str() == Some("login.live.com")
            && url.path() == "/oauth20_desktop.srf"
        {
            // query_pairs() properly URL-decodes the code value
            let code = url.query_pairs()
                .find(|(k, _)| k == "code")
                .map(|(_, v)| v.into_owned());
            if let Some(c) = code {
                *code_nav.lock().unwrap() = Some(c);
            }
            return false; // block the blank redirect page from loading
        }
        true
    })
    .build()
    .map_err(|e| format!("Auth window: {e}"))?;

    // Detect window close (user cancelled)
    win.on_window_event({
        let c = closed_ev.clone();
        move |ev| {
            if matches!(ev, tauri::WindowEvent::Destroyed
                          | tauri::WindowEvent::CloseRequested { .. }) {
                c.store(true, Ordering::Relaxed);
            }
        }
    });

    // Poll until we have the code or the window is closed
    let start = std::time::Instant::now();
    let auth_code = loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        {
            let g = code_slot.lock().unwrap();
            if let Some(c) = g.as_ref() { break c.clone(); }
        }
        if closed.load(Ordering::Relaxed) {
            return Err("Sign-in cancelled.".to_string());
        }
        if start.elapsed().as_secs() > 300 {
            let _ = win.close();
            return Err("Login timed out after 5 minutes.".to_string());
        }
    };
    let _ = win.close();

    let client = reqwest::Client::builder()
        .user_agent("MLBV/1.0")
        .build().map_err(|e| e.to_string())?;

    // Exchange auth code → MS access token
    let ms: serde_json::Value = client
        .post("https://login.live.com/oauth20_token.srf")
        .form(&[
            ("client_id", CLIENT_ID),
            ("code", auth_code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", REDIRECT_URI),
        ])
        .send().await.map_err(|e| format!("MS token: {e}"))?
        .json().await.map_err(|e| format!("MS token parse: {e}"))?;

    let ms_token = ms["access_token"].as_str()
        .ok_or_else(|| format!("MS auth failed: {ms}"))?
        .to_string();
    let refresh_token = ms["refresh_token"].as_str().unwrap_or("").to_string();

    // Xbox → XSTS → Minecraft chain (shared with the launch-time refresh in lib.rs)
    let (mc_token, uuid, username) = ms_token_chain(&client, &ms_token).await?;

    store_tokens(&app, &uuid, mc_token, refresh_token)?;
    Ok(MsAccount { username, uuid })
}
