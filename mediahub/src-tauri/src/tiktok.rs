use serde_json::{json, Value};

use crate::db;
use crate::model::{clip, clean_thumb, ensure_platform_url, is_tiktok_id, Account, MediaItem, REDIRECT_URI};
use crate::net::NetFail;
use crate::oauth::Pkce;
use crate::secrets;
use crate::AppState;

const PREFIX: &str = "tiktok";

pub fn account(state: &AppState) -> Result<Account, String> {
    let alive = session_alive()?;
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    let mut account = db::account_label(&conn, PREFIX)?;
    account.connected = alive;
    if !alive {
        account.display_name.clear();
        account.handle.clear();
    }
    Ok(account)
}

pub fn disconnect(state: &AppState) -> Result<(), String> {
    clear_session()?;
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::set_profile(&conn, PREFIX, "", "")
}

pub fn auth_url(client_key: &str, pkce: &Pkce) -> String {
    let mut url = url::Url::parse("https://www.tiktok.com/v2/auth/authorize/").expect("static url");
    url.query_pairs_mut()
        .append_pair("client_key", client_key)
        .append_pair("response_type", "code")
        .append_pair("scope", "user.info.basic,video.list")
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("state", &pkce.state)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256");
    url.to_string()
}

pub async fn connect(state: &AppState, pkce: Pkce, code: String) -> Result<Account, String> {
    let client_key = client_key(state)?;
    if client_key.is_empty() {
        return Err("TikTok-Client-Key fehlt. Trag ihn in den Einstellungen ein.".into());
    }
    let secret = secrets::get_secret("tiktok_client_secret")?
        .ok_or_else(|| "TikTok-Client-Secret fehlt. Trag es in den Einstellungen ein.".to_string())?;
    let token = state
        .http
        .form_post(
            "https://open.tiktokapis.com/v2/oauth/token/",
            &[
                ("client_key", client_key.as_str()),
                ("client_secret", secret.as_str()),
                ("code", code.as_str()),
                ("grant_type", "authorization_code"),
                ("redirect_uri", REDIRECT_URI),
                ("code_verifier", pkce.verifier.as_str()),
            ],
        )
        .await
        .map_err(|_| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())?;
    store_token_value(&token)?;
    let access = current_access()
        .map_err(|_| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())?;
    let (name, handle) = match fetch_profile(state, &access).await {
        Ok(profile) => profile,
        Err(_) => ("TikTok".into(), String::new()),
    };
    {
        let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
        db::set_profile(&conn, PREFIX, &name, &handle)?;
    }
    account(state)
}

pub async fn feed(state: &AppState) -> Result<Vec<MediaItem>, String> {
    if !session_alive()? {
        return Ok(Vec::new());
    }
    let value = authed_post(
        state,
        "https://open.tiktokapis.com/v2/video/list/?fields=id,title,share_url,cover_image_url",
        &json!({ "max_count": 8 }),
    )
    .await?;
    if let Some(code) = value["error"]["code"].as_str() {
        if code != "ok" {
            return Err("Die Verbindung zu TikTok ist momentan nicht verfügbar.".into());
        }
    }
    let mut items = Vec::new();
    let Some(list) = value["data"]["videos"].as_array() else {
        return Ok(items);
    };
    let creator = {
        let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
        db::get_raw(&conn, "tiktok_display_name")?
    };
    for entry in list.iter().take(8) {
        let Some(id) = entry["id"].as_str() else {
            continue;
        };
        if !is_tiktok_id(id) {
            continue;
        }
        let title = entry["title"].as_str().unwrap_or("Video");
        let share = entry["share_url"].as_str().unwrap_or("");
        let url = ensure_platform_url(share)
            .unwrap_or_else(|_| format!("https://www.tiktok.com/video/{id}"));
        let thumb = entry["cover_image_url"].as_str().unwrap_or("");
        items.push(MediaItem {
            platform: "tiktok".into(),
            external_id: id.into(),
            title: clip(title, 180),
            creator: clip(&creator, 80),
            url,
            thumb: clean_thumb(thumb),
        });
    }
    Ok(items)
}

pub fn client_key(state: &AppState) -> Result<String, String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::get_raw(&conn, "tiktok_client_key")
}

async fn fetch_profile(state: &AppState, token: &str) -> Result<(String, String), String> {
    let value = state
        .http
        .get_bearer(
            "https://open.tiktokapis.com/v2/user/info/?fields=display_name,open_id",
            token,
        )
        .await
        .map_err(|fail| fail.tiktok())?;
    let name = value["data"]["user"]["display_name"]
        .as_str()
        .unwrap_or("TikTok");
    Ok((clip(name, 80), String::new()))
}

fn current_access() -> Result<String, ()> {
    let expiry = secrets::get_secret(&format!("{PREFIX}_expiry")).map_err(|_| ())?;
    let access = secrets::get_secret(&format!("{PREFIX}_access")).map_err(|_| ())?;
    match (access, expiry) {
        (Some(access), Some(expiry)) if expiry.parse::<i64>().unwrap_or(0) > db::now_secs() + 20 => {
            Ok(access)
        }
        _ => Err(()),
    }
}

async fn refresh(state: &AppState) -> Result<String, String> {
    let refresh_token = secrets::get_secret(&format!("{PREFIX}_refresh"))?
        .ok_or_else(|| "TikTok ist nicht verbunden.".to_string())?;
    let client_key = client_key(state)?;
    let secret = secrets::get_secret("tiktok_client_secret")?
        .ok_or_else(|| "TikTok-Client-Secret fehlt. Trag es in den Einstellungen ein.".to_string())?;
    let token = match state
        .http
        .form_post(
            "https://open.tiktokapis.com/v2/oauth/token/",
            &[
                ("client_key", client_key.as_str()),
                ("client_secret", secret.as_str()),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token.as_str()),
            ],
        )
        .await
    {
        Ok(token) => token,
        Err(NetFail::Offline) => {
            return Err("Die Verbindung zu TikTok ist momentan nicht verfügbar.".into());
        }
        Err(_) => {
            clear_session()?;
            return Err("TikTok-Sitzung abgelaufen. Bitte erneut verbinden.".into());
        }
    };
    store_token_value(&token)?;
    secrets::get_secret(&format!("{PREFIX}_access"))?
        .ok_or_else(|| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())
}

async fn authed_post(state: &AppState, url: &str, body: &Value) -> Result<Value, String> {
    let token = if let Ok(token) = current_access() {
        token
    } else {
        refresh(state).await?
    };
    match state.http.post_bearer_json(url, &token, body).await {
        Ok(value) => Ok(value),
        Err(NetFail::Denied) => {
            let token = refresh(state).await?;
            state
                .http
                .post_bearer_json(url, &token, body)
                .await
                .map_err(|fail| fail.tiktok())
        }
        Err(fail) => Err(fail.tiktok()),
    }
}

fn store_token_value(token: &Value) -> Result<(), String> {
    let source = token.get("data").unwrap_or(token);
    let access = source["access_token"]
        .as_str()
        .ok_or_else(|| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())?;
    let expires_in = source["expires_in"].as_i64().unwrap_or(3600);
    let refresh = source["refresh_token"].as_str();
    secrets::set_secret(&format!("{PREFIX}_access"), access)?;
    if let Some(refresh) = refresh {
        if !refresh.is_empty() {
            secrets::set_secret(&format!("{PREFIX}_refresh"), refresh)?;
        }
    }
    let expiry = db::now_secs() + expires_in.max(60) - 30;
    secrets::set_secret(&format!("{PREFIX}_expiry"), &expiry.to_string())?;
    Ok(())
}

fn session_alive() -> Result<bool, String> {
    if secrets::has_secret(&format!("{PREFIX}_refresh")) {
        return Ok(true);
    }
    let Some(expiry) = secrets::get_secret(&format!("{PREFIX}_expiry"))? else {
        return Ok(false);
    };
    Ok(expiry.parse::<i64>().unwrap_or(0) > db::now_secs() + 15)
}

fn clear_session() -> Result<(), String> {
    secrets::delete_secret(&format!("{PREFIX}_access"))?;
    secrets::delete_secret(&format!("{PREFIX}_refresh"))?;
    secrets::delete_secret(&format!("{PREFIX}_expiry"))?;
    Ok(())
}
