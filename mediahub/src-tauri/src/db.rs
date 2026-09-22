use rusqlite::{params, Connection};
use std::path::Path;

use crate::model::{
    clip, validate_settings, Account, HistoryItem, LocalMedia, MediaItem, Settings,
};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY,
    platform TEXT NOT NULL,
    external_id TEXT NOT NULL,
    title TEXT NOT NULL,
    creator TEXT NOT NULL,
    url TEXT NOT NULL,
    thumb TEXT NOT NULL,
    watched_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS favorites (
    id INTEGER PRIMARY KEY,
    platform TEXT NOT NULL,
    external_id TEXT NOT NULL,
    title TEXT NOT NULL,
    creator TEXT NOT NULL,
    url TEXT NOT NULL,
    thumb TEXT NOT NULL,
    saved_at INTEGER NOT NULL,
    UNIQUE(platform, external_id)
);
CREATE TABLE IF NOT EXISTS local_media (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    kind TEXT NOT NULL,
    added_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_history_watched ON history(watched_at DESC);
";

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("Datenbank: {e}"))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -2000;",
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    conn.execute_batch(SCHEMA)
        .map_err(|e| format!("Datenbank: {e}"))?;
    seed_settings(&conn)?;
    Ok(conn)
}

fn seed_settings(conn: &Connection) -> Result<(), String> {
    let defaults = Settings::default();
    let pairs = [
        ("performance_mode", defaults.performance_mode),
        ("language", defaults.language),
        ("theme", defaults.theme),
        ("animations", bool_str(defaults.animations)),
        ("autoplay", bool_str(defaults.autoplay)),
        ("volume", defaults.volume.to_string()),
        ("start_page", defaults.start_page),
        ("history_enabled", bool_str(defaults.history_enabled)),
        ("youtube_client_id", defaults.youtube_client_id),
        ("tiktok_client_key", defaults.tiktok_client_key),
        ("youtube_display_name", String::new()),
        ("youtube_handle", String::new()),
        ("tiktok_display_name", String::new()),
        ("tiktok_handle", String::new()),
    ];
    for (key, value) in pairs {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| format!("Datenbank: {e}"))?;
    }
    Ok(())
}

fn bool_str(value: bool) -> String {
    if value { "true" } else { "false" }.into()
}

pub fn get_raw(conn: &Connection, key: &str) -> Result<String, String> {
    match conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    ) {
        Ok(value) => Ok(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(String::new()),
        Err(error) => Err(format!("Datenbank: {error}")),
    }
}

pub fn set_raw(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    Ok(())
}

pub fn load_settings(conn: &Connection, tiktok_secret_set: bool) -> Result<Settings, String> {
    let mut settings = Settings {
        performance_mode: get_raw(conn, "performance_mode")?,
        language: get_raw(conn, "language")?,
        theme: get_raw(conn, "theme")?,
        animations: get_raw(conn, "animations")? != "false",
        autoplay: get_raw(conn, "autoplay")? != "false",
        volume: get_raw(conn, "volume")?.parse().unwrap_or(80),
        start_page: get_raw(conn, "start_page")?,
        history_enabled: get_raw(conn, "history_enabled")? != "false",
        youtube_client_id: get_raw(conn, "youtube_client_id")?,
        tiktok_client_key: get_raw(conn, "tiktok_client_key")?,
        tiktok_secret_set,
    };
    if settings.performance_mode.is_empty() {
        settings.performance_mode = "sparsam".into();
    }
    if settings.language.is_empty() {
        settings.language = "de".into();
    }
    if settings.theme.is_empty() {
        settings.theme = "dunkel".into();
    }
    if settings.start_page.is_empty() {
        settings.start_page = "start".into();
    }
    Ok(settings)
}

pub fn save_settings(conn: &Connection, settings: &Settings) -> Result<(), String> {
    let settings = validate_settings(settings.clone())?;
    let pairs = [
        ("performance_mode", settings.performance_mode),
        ("language", settings.language),
        ("theme", settings.theme),
        ("animations", bool_str(settings.animations)),
        ("autoplay", bool_str(settings.autoplay)),
        ("volume", settings.volume.to_string()),
        ("start_page", settings.start_page),
        ("history_enabled", bool_str(settings.history_enabled)),
        ("youtube_client_id", settings.youtube_client_id),
        ("tiktok_client_key", settings.tiktok_client_key),
    ];
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Datenbank: {e}"))?;
    for (key, value) in pairs {
        set_raw(&tx, key, &value)?;
    }
    tx.commit().map_err(|e| format!("Datenbank: {e}"))?;
    Ok(())
}

pub fn account_label(conn: &Connection, platform: &str) -> Result<Account, String> {
    let name_key = format!("{platform}_display_name");
    let handle_key = format!("{platform}_handle");
    Ok(Account {
        connected: false,
        display_name: get_raw(conn, &name_key)?,
        handle: get_raw(conn, &handle_key)?,
    })
}

pub fn set_profile(conn: &Connection, platform: &str, name: &str, handle: &str) -> Result<(), String> {
    set_raw(conn, &format!("{platform}_display_name"), &clip(name, 80))?;
    set_raw(conn, &format!("{platform}_handle"), &clip(handle, 80))?;
    Ok(())
}

pub fn history_enabled(conn: &Connection) -> Result<bool, String> {
    Ok(get_raw(conn, "history_enabled")? != "false")
}

pub fn remember(conn: &Connection, item: &MediaItem) -> Result<(), String> {
    if !history_enabled(conn)? {
        return Ok(());
    }
    let now = now_secs();
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("Datenbank: {e}"))?;
    tx.execute(
        "DELETE FROM history WHERE platform = ?1 AND external_id = ?2",
        params![item.platform, item.external_id],
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    tx.execute(
        "INSERT INTO history (platform, external_id, title, creator, url, thumb, watched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            item.platform,
            item.external_id,
            item.title,
            item.creator,
            item.url,
            item.thumb,
            now
        ],
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    tx.execute(
        "DELETE FROM history WHERE id NOT IN (
            SELECT id FROM history ORDER BY watched_at DESC LIMIT 200
         )",
        [],
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    tx.commit().map_err(|e| format!("Datenbank: {e}"))?;
    Ok(())
}

pub fn list_history(conn: &Connection) -> Result<Vec<HistoryItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT platform, external_id, title, creator, url, thumb, watched_at
             FROM history ORDER BY watched_at DESC LIMIT 200",
        )
        .map_err(|e| format!("Datenbank: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(HistoryItem {
                platform: row.get(0)?,
                external_id: row.get(1)?,
                title: row.get(2)?,
                creator: row.get(3)?,
                url: row.get(4)?,
                thumb: row.get(5)?,
                watched_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Datenbank: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Datenbank: {e}"))
}

pub fn clear_history(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM history", [])
        .map_err(|e| format!("Datenbank: {e}"))?;
    Ok(())
}

pub fn toggle_favorite(conn: &Connection, item: &MediaItem) -> Result<bool, String> {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM favorites WHERE platform = ?1 AND external_id = ?2",
            params![item.platform, item.external_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if exists {
        conn.execute(
            "DELETE FROM favorites WHERE platform = ?1 AND external_id = ?2",
            params![item.platform, item.external_id],
        )
        .map_err(|e| format!("Datenbank: {e}"))?;
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO favorites (platform, external_id, title, creator, url, thumb, saved_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            item.platform,
            item.external_id,
            item.title,
            item.creator,
            item.url,
            item.thumb,
            now_secs()
        ],
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    Ok(true)
}

pub fn list_favorites(conn: &Connection) -> Result<Vec<MediaItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT platform, external_id, title, creator, url, thumb
             FROM favorites ORDER BY saved_at DESC LIMIT 500",
        )
        .map_err(|e| format!("Datenbank: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(MediaItem {
                platform: row.get(0)?,
                external_id: row.get(1)?,
                title: row.get(2)?,
                creator: row.get(3)?,
                url: row.get(4)?,
                thumb: row.get(5)?,
            })
        })
        .map_err(|e| format!("Datenbank: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Datenbank: {e}"))
}

pub fn list_local(conn: &Connection) -> Result<Vec<LocalMedia>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, path, title, kind FROM local_media ORDER BY added_at DESC LIMIT 500",
        )
        .map_err(|e| format!("Datenbank: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(LocalMedia {
                id: row.get(0)?,
                path: row.get(1)?,
                title: row.get(2)?,
                kind: row.get(3)?,
            })
        })
        .map_err(|e| format!("Datenbank: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Datenbank: {e}"))
}

pub fn add_local(conn: &Connection, paths: &[String]) -> Result<Vec<LocalMedia>, String> {
    if paths.len() > 40 {
        return Err("Zu viele Dateien auf einmal.".into());
    }
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM local_media", [], |row| row.get(0))
        .map_err(|e| format!("Datenbank: {e}"))?;
    if count > 500 {
        return Err("Die Medienliste ist voll.".into());
    }
    for raw in paths {
        let (path, title) = normalize_media_path(raw)?;
        conn.execute(
            "INSERT OR IGNORE INTO local_media (path, title, kind, added_at) VALUES (?1, ?2, 'video', ?3)",
            params![path, title, now_secs()],
        )
        .map_err(|e| format!("Datenbank: {e}"))?;
    }
    list_local(conn)
}

pub fn remove_local(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM local_media WHERE id = ?1", params![id])
        .map_err(|e| format!("Datenbank: {e}"))?;
    conn.execute(
        "DELETE FROM favorites WHERE platform = 'local' AND external_id = ?1",
        params![id.to_string()],
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    conn.execute(
        "DELETE FROM history WHERE platform = 'local' AND external_id = ?1",
        params![id.to_string()],
    )
    .map_err(|e| format!("Datenbank: {e}"))?;
    Ok(())
}

pub fn local_exists(conn: &Connection, id: &str) -> Result<bool, String> {
    let Ok(id) = id.parse::<i64>() else {
        return Ok(false);
    };
    let found = conn
        .query_row(
            "SELECT 1 FROM local_media WHERE id = ?1",
            params![id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    Ok(found)
}

fn normalize_media_path(raw: &str) -> Result<(String, String), String> {
    if raw.len() > 1024 || raw.contains('\0') {
        return Err("Ungültiger Dateipfad.".into());
    }
    let path = std::fs::canonicalize(raw).map_err(|_| "Datei nicht gefunden.".to_string())?;
    if !path.is_file() {
        return Err("Das ist keine Videodatei.".into());
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "mp4" | "webm" | "m4v") {
        return Err("Erlaubt sind mp4, webm und m4v.".into());
    }
    let text = path.to_string_lossy();
    let text = text
        .strip_prefix(r"\\?\")
        .unwrap_or(text.as_ref())
        .to_string();
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Video");
    Ok((text, clip(title, 180)))
}

pub fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
