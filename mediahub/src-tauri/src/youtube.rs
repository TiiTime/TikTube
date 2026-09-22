use serde_json::Value;

use crate::db;
use crate::model::{clip, clean_thumb, is_youtube_id, Account, Channel, MediaItem, REDIRECT_URI};
use crate::net::{Http, NetFail};
use crate::oauth::Pkce;
use crate::secrets;
use crate::AppState;

const PREFIX: &str = "youtube";

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

pub async fn connect(state: &AppState, pkce: Pkce, code: String) -> Result<Account, String> {
    let client_id = {
        let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
        db::get_raw(&conn, "youtube_client_id")?
    };
    if client_id.is_empty() {
        return Err("YouTube-Client-ID fehlt. Trag sie in den Einstellungen ein.".into());
    }
    let token = state
        .http
        .form_post(
            "https://oauth2.googleapis.com/token",
            &[
                ("code", code.as_str()),
                ("client_id", client_id.as_str()),
                ("redirect_uri", REDIRECT_URI),
                ("grant_type", "authorization_code"),
                ("code_verifier", pkce.verifier.as_str()),
            ],
        )
        .await
        .map_err(|_| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())?;
    store_token_value(&token)?;
    let access = current_access()
        .map_err(|_| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())?;
    let (name, handle) = match fetch_profile(&state.http, &access).await {
        Ok(profile) => profile,
        Err(_) => ("YouTube".into(), String::new()),
    };
    {
        let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
        db::set_profile(&conn, PREFIX, &name, &handle)?;
    }
    account(state)
}

pub fn auth_url(client_id: &str, pkce: &Pkce) -> String {
    let mut url = url::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").expect("static url");
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("response_type", "code")
        .append_pair("scope", "https://www.googleapis.com/auth/youtube.readonly")
        .append_pair("state", &pkce.state)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    url.to_string()
}

pub async fn feed(state: &AppState) -> Result<Vec<MediaItem>, String> {
    if !session_alive()? {
        return Ok(Vec::new());
    }
    let value = authed_get(
        state,
        "https://www.googleapis.com/youtube/v3/search?part=snippet&type=video&videoDuration=short&maxResults=8&order=date&safeSearch=moderate&regionCode=DE",
    )
    .await?;
    let mut items = Vec::new();
    let Some(list) = value["items"].as_array() else {
        return Ok(items);
    };
    for entry in list.iter().take(8) {
        let Some(id) = entry["id"]["videoId"].as_str() else {
            continue;
        };
        if !is_youtube_id(id) {
            continue;
        }
        let title = entry["snippet"]["title"].as_str().unwrap_or("Video");
        let creator = entry["snippet"]["channelTitle"].as_str().unwrap_or("");
        let thumb = entry["snippet"]["thumbnails"]["medium"]["url"]
            .as_str()
            .unwrap_or("");
        items.push(MediaItem {
            platform: "youtube".into(),
            external_id: id.into(),
            title: clip(title, 180),
            creator: clip(creator, 80),
            url: format!("https://www.youtube.com/shorts/{id}"),
            thumb: clean_thumb(thumb),
        });
    }
    Ok(items)
}

pub async fn subscriptions(state: &AppState) -> Result<Vec<Channel>, String> {
    if !session_alive()? {
        return Ok(Vec::new());
    }
    let value = authed_get(
        state,
        "https://www.googleapis.com/youtube/v3/subscriptions?part=snippet&mine=true&maxResults=20",
    )
    .await?;
    let mut channels = Vec::new();
    let Some(list) = value["items"].as_array() else {
        return Ok(channels);
    };
    for entry in list.iter().take(20) {
        let name = entry["snippet"]["title"].as_str().unwrap_or("Kanal");
        let channel_id = entry["snippet"]["resourceId"]["channelId"].as_str().unwrap_or("");
        if channel_id.is_empty() || channel_id.len() > 64 {
            continue;
        }
        channels.push(Channel {
            name: clip(name, 80),
            handle: String::new(),
            url: format!("https://www.youtube.com/channel/{channel_id}"),
        });
    }
    Ok(channels)
}

pub fn client_id(state: &AppState) -> Result<String, String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::get_raw(&conn, "youtube_client_id")
}

async fn fetch_profile(http: &Http, token: &str) -> Result<(String, String), String> {
    let value = http
        .get_bearer(
            "https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true",
            token,
        )
        .await
        .map_err(|fail| fail.youtube())?;
    let snippet = &value["items"][0]["snippet"];
    let name = snippet["title"].as_str().unwrap_or("YouTube");
    let handle = snippet["customUrl"].as_str().unwrap_or("");
    Ok((clip(name, 80), clip(handle, 80)))
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
        .ok_or_else(|| "YouTube ist nicht verbunden.".to_string())?;
    let client_id = client_id(state)?;
    if client_id.is_empty() {
        return Err("YouTube-Client-ID fehlt. Trag sie in den Einstellungen ein.".into());
    }
    let token = match state
        .http
        .form_post(
            "https://oauth2.googleapis.com/token",
            &[
                ("client_id", client_id.as_str()),
                ("refresh_token", refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ],
        )
        .await
    {
        Ok(token) => token,
        Err(NetFail::Offline) => {
            return Err("Die Verbindung zu YouTube ist momentan nicht verfügbar.".into());
        }
        Err(_) => {
            clear_session()?;
            return Err("YouTube-Sitzung abgelaufen. Bitte erneut verbinden.".into());
        }
    };
    store_token_value(&token)?;
    secrets::get_secret(&format!("{PREFIX}_access"))?
        .ok_or_else(|| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())
}

fn store_token_value(token: &Value) -> Result<(), String> {
    let access = token["access_token"]
        .as_str()
        .ok_or_else(|| "Die Anmeldung konnte nicht abgeschlossen werden.".to_string())?;
    let expires_in = token["expires_in"].as_i64().unwrap_or(3600);
    let refresh = token["refresh_token"].as_str();
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

pub async fn authed_get(state: &AppState, url: &str) -> Result<Value, String> {
    let token = if let Ok(token) = current_access() {
        token
    } else {
        refresh(state).await?
    };
    match state.http.get_bearer(url, &token).await {
        Ok(value) => Ok(value),
        Err(NetFail::Denied) => {
            let token = refresh(state).await?;
            state.http.get_bearer(url, &token).await.map_err(|fail| fail.youtube())
        }
        Err(fail) => Err(fail.youtube()),
    }
}
