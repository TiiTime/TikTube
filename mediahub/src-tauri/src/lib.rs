mod db;
mod model;
mod net;
mod oauth;
mod secrets;
mod tiktok;
mod viewer;
mod youtube;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use model::{canonical_item, validate_secret, Account, Accounts, LocalMedia, MediaItem, Settings};
use tauri::Manager;
use viewer::{nudge_feed, open_login, set_playback_quality, show_feed};

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub http: net::Http,
    pub login_busy: Arc<AtomicBool>,
    pub cancel_login: Arc<AtomicBool>,
}

struct BusyFlag<'a>(&'a AtomicBool);

impl Drop for BusyFlag<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn lock_login(state: &AppState) -> Result<BusyFlag<'_>, String> {
    if state.login_busy.swap(true, Ordering::SeqCst) {
        return Err("Eine Anmeldung läuft bereits.".into());
    }
    state.cancel_login.store(false, Ordering::SeqCst);
    Ok(BusyFlag(&state.login_busy))
}

async fn finish_login(
    state: &AppState,
    pkce_state: String,
    open_url: String,
) -> Result<String, String> {
    let cancel = state.cancel_login.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let join = tauri::async_runtime::spawn_blocking(move || {
        oauth::listen_for_code(&pkce_state, &cancel, ready_tx)
    });
    match ready_rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            let _ = join.await;
            return Err(error);
        }
        Err(_) => {
            state.cancel_login.store(true, Ordering::SeqCst);
            let _ = join.await;
            return Err("Die Anmeldung konnte nicht gestartet werden.".into());
        }
    }
    if let Err(error) = oauth::open_browser(&open_url) {
        state.cancel_login.store(true, Ordering::SeqCst);
        let _ = join.await;
        return Err(error);
    }
    match join.await {
        Ok(Ok(code)) => Ok(code),
        Ok(Err(error)) => Err(error),
        Err(_) => Err("Die Anmeldung konnte nicht abgeschlossen werden.".into()),
    }
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    let secret_set = secrets::has_secret("tiktok_client_secret");
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::load_settings(&conn, secret_set)
}

#[tauri::command]
fn save_settings(state: tauri::State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::save_settings(&conn, &settings)
}

#[tauri::command]
fn set_tiktok_secret(secret: String) -> Result<(), String> {
    let secret = validate_secret(&secret)?;
    if secret.is_empty() {
        secrets::delete_secret("tiktok_client_secret")
    } else {
        secrets::set_secret("tiktok_client_secret", &secret)
    }
}

#[tauri::command]
fn account_status(state: tauri::State<'_, AppState>) -> Result<Accounts, String> {
    Ok(Accounts {
        youtube: youtube::account(&state)?,
        tiktok: tiktok::account(&state)?,
    })
}

#[tauri::command]
async fn connect_youtube(state: tauri::State<'_, AppState>) -> Result<Account, String> {
    let _busy = lock_login(&state)?;
    let client_id = youtube::client_id(&state)?;
    if client_id.is_empty() {
        return Err("YouTube-Client-ID fehlt. Trag sie in den Einstellungen ein.".into());
    }
    let pkce = oauth::create_pkce()?;
    let url = youtube::auth_url(&client_id, &pkce);
    let expected = pkce.state.clone();
    let code = finish_login(&state, expected, url).await?;
    youtube::connect(&state, pkce, code).await
}

#[tauri::command]
async fn connect_tiktok(state: tauri::State<'_, AppState>) -> Result<Account, String> {
    let _busy = lock_login(&state)?;
    let client_key = tiktok::client_key(&state)?;
    if client_key.is_empty() {
        return Err("TikTok-Client-Key fehlt. Trag ihn in den Einstellungen ein.".into());
    }
    if !secrets::has_secret("tiktok_client_secret") {
        return Err("TikTok-Client-Secret fehlt. Trag es in den Einstellungen ein.".into());
    }
    let pkce = oauth::create_pkce()?;
    let url = tiktok::auth_url(&client_key, &pkce);
    let expected = pkce.state.clone();
    let code = finish_login(&state, expected, url).await?;
    tiktok::connect(&state, pkce, code).await
}

#[tauri::command]
fn disconnect(state: tauri::State<'_, AppState>, platform: String) -> Result<(), String> {
    match platform.as_str() {
        "youtube" => youtube::disconnect(&state),
        "tiktok" => tiktok::disconnect(&state),
        _ => Err("Unbekannte Plattform.".into()),
    }
}

#[tauri::command]
fn cancel_login(state: tauri::State<'_, AppState>) {
    state.cancel_login.store(true, Ordering::SeqCst);
}

#[tauri::command]
async fn list_feed(state: tauri::State<'_, AppState>, platform: String) -> Result<Vec<MediaItem>, String> {
    match platform.as_str() {
        "youtube" => youtube::feed(&state).await,
        "tiktok" => tiktok::feed(&state).await,
        _ => Err("Unbekannte Plattform.".into()),
    }
}

#[tauri::command]
async fn list_subscriptions(
    state: tauri::State<'_, AppState>,
    platform: String,
) -> Result<Vec<model::Channel>, String> {
    match platform.as_str() {
        "youtube" => youtube::subscriptions(&state).await,
        "tiktok" => Ok(Vec::new()),
        _ => Err("Unbekannte Plattform.".into()),
    }
}

#[tauri::command]
fn remember_watch(state: tauri::State<'_, AppState>, item: MediaItem) -> Result<(), String> {
    let item = canonical_item(&item)?;
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    if item.platform == "local" && !db::local_exists(&conn, &item.external_id)? {
        return Err("Datei ist nicht in den Medien.".into());
    }
    db::remember(&conn, &item)
}

#[tauri::command]
fn list_history(state: tauri::State<'_, AppState>) -> Result<Vec<model::HistoryItem>, String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::list_history(&conn)
}

#[tauri::command]
fn clear_history(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::clear_history(&conn)
}

#[tauri::command]
fn toggle_favorite(state: tauri::State<'_, AppState>, item: MediaItem) -> Result<bool, String> {
    let item = canonical_item(&item)?;
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    if item.platform == "local" && !db::local_exists(&conn, &item.external_id)? {
        return Err("Datei ist nicht in den Medien.".into());
    }
    db::toggle_favorite(&conn, &item)
}

#[tauri::command]
fn list_favorites(state: tauri::State<'_, AppState>) -> Result<Vec<MediaItem>, String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::list_favorites(&conn)
}

#[tauri::command]
fn list_local_media(state: tauri::State<'_, AppState>) -> Result<Vec<LocalMedia>, String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::list_local(&conn)
}

#[tauri::command]
fn add_local_media(state: tauri::State<'_, AppState>, paths: Vec<String>) -> Result<Vec<LocalMedia>, String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::add_local(&conn, &paths)
}

#[tauri::command]
fn remove_local_media(state: tauri::State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|_| "Datenbank gesperrt.".to_string())?;
    db::remove_local(&conn, id)
}

#[tauri::command]
fn open_on_platform(url: String) -> Result<(), String> {
    let url = model::ensure_platform_url(&url)?;
    oauth::open_browser(&url)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("mediahub.sqlite")).map_err(std::io::Error::other)?;
            let http = net::Http::new().map_err(std::io::Error::other)?;
            app.manage(AppState {
                db: Mutex::new(conn),
                http,
                login_busy: Arc::new(AtomicBool::new(false)),
                cancel_login: Arc::new(AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            set_tiktok_secret,
            account_status,
            connect_youtube,
            connect_tiktok,
            disconnect,
            cancel_login,
            list_feed,
            list_subscriptions,
            remember_watch,
            list_history,
            clear_history,
            toggle_favorite,
            list_favorites,
            list_local_media,
            add_local_media,
            remove_local_media,
            open_on_platform,
            show_feed,
            nudge_feed,
            set_playback_quality,
            open_login
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
