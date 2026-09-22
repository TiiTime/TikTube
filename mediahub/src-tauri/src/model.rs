use serde::{Deserialize, Serialize};

pub const REDIRECT_PORT: u16 = 38947;
pub const REDIRECT_URI: &str = "http://127.0.0.1:38947/callback";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub platform: String,
    pub external_id: String,
    pub title: String,
    pub creator: String,
    pub url: String,
    pub thumb: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub platform: String,
    pub external_id: String,
    pub title: String,
    pub creator: String,
    pub url: String,
    pub thumb: String,
    pub watched_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub connected: bool,
    pub display_name: String,
    pub handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accounts {
    pub youtube: Account,
    pub tiktok: Account,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub name: String,
    pub handle: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalMedia {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub performance_mode: String,
    pub language: String,
    pub theme: String,
    pub animations: bool,
    pub autoplay: bool,
    pub volume: u8,
    pub start_page: String,
    pub history_enabled: bool,
    pub youtube_client_id: String,
    pub tiktok_client_key: String,
    pub tiktok_secret_set: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            performance_mode: "sparsam".into(),
            language: "de".into(),
            theme: "dunkel".into(),
            animations: false,
            autoplay: true,
            volume: 80,
            start_page: "start".into(),
            history_enabled: true,
            youtube_client_id: String::new(),
            tiktok_client_key: String::new(),
            tiktok_secret_set: false,
        }
    }
}

pub fn clip(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub fn is_youtube_id(id: &str) -> bool {
    id.len() == 11
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn is_tiktok_id(id: &str) -> bool {
    (6..=25).contains(&id.len()) && id.chars().all(|c| c.is_ascii_digit())
}

pub fn clean_thumb(raw: &str) -> String {
    let Ok(url) = url::Url::parse(raw) else {
        return String::new();
    };
    if url.scheme() != "https" {
        return String::new();
    }
    let host = url.host_str().unwrap_or("");
    let ok = host == "i.ytimg.com"
        || host.ends_with(".ytimg.com")
        || host.ends_with(".tiktokcdn.com")
        || host.ends_with(".tiktokcdn-us.com");
    if ok {
        url.to_string()
    } else {
        String::new()
    }
}

pub fn ensure_platform_url(raw: &str) -> Result<String, String> {
    let url = url::Url::parse(raw).map_err(|_| "Ungültiger Link.".to_string())?;
    if url.scheme() != "https" {
        return Err("Ungültiger Link.".into());
    }
    let host = url.host_str().unwrap_or("");
    let ok = matches!(
        host,
        "www.youtube.com"
            | "youtube.com"
            | "youtu.be"
            | "www.youtube-nocookie.com"
            | "www.tiktok.com"
            | "tiktok.com"
            | "vm.tiktok.com"
            | "m.tiktok.com"
    );
    if !ok {
        return Err("Dieser Link gehört nicht zu YouTube oder TikTok.".into());
    }
    Ok(url.to_string())
}

pub fn canonical_item(item: &MediaItem) -> Result<MediaItem, String> {
    let title = clip(item.title.trim(), 180);
    let title = if title.is_empty() {
        "Video".to_string()
    } else {
        title
    };
    let creator = clip(item.creator.trim(), 80);
    match item.platform.as_str() {
        "youtube" => {
            if !is_youtube_id(&item.external_id) {
                return Err("Ungültige Video-ID.".into());
            }
            Ok(MediaItem {
                platform: "youtube".into(),
                external_id: item.external_id.clone(),
                title,
                creator,
                url: format!("https://www.youtube.com/shorts/{}", item.external_id),
                thumb: clean_thumb(&item.thumb),
            })
        }
        "tiktok" => {
            if !is_tiktok_id(&item.external_id) {
                return Err("Ungültige Video-ID.".into());
            }
            let url = ensure_platform_url(&item.url).unwrap_or_else(|_| {
                format!("https://www.tiktok.com/video/{}", item.external_id)
            });
            Ok(MediaItem {
                platform: "tiktok".into(),
                external_id: item.external_id.clone(),
                title,
                creator,
                url,
                thumb: clean_thumb(&item.thumb),
            })
        }
        "local" => {
            if item.external_id.is_empty()
                || item.external_id.len() > 20
                || !item.external_id.chars().all(|c| c.is_ascii_digit())
            {
                return Err("Ungültiger Eintrag.".into());
            }
            if item.url.len() > 1024 || item.url.contains('\0') {
                return Err("Ungültiger Dateipfad.".into());
            }
            Ok(MediaItem {
                platform: "local".into(),
                external_id: item.external_id.clone(),
                title,
                creator,
                url: item.url.clone(),
                thumb: String::new(),
            })
        }
        _ => Err("Unbekannte Plattform.".into()),
    }
}

pub fn validate_client_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    if value.len() > 200 {
        return Err("Client-ID ist zu lang.".into());
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("Client-ID enthält ungültige Zeichen.".into());
    }
    Ok(value.to_string())
}

pub fn validate_secret(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    if value.len() > 300 || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("Secret ist ungültig.".into());
    }
    Ok(value.to_string())
}

pub fn validate_settings(mut settings: Settings) -> Result<Settings, String> {
    settings.performance_mode = match settings.performance_mode.as_str() {
        "sparsam" | "standard" | "maximal" => settings.performance_mode,
        _ => return Err("Unbekannter Leistungsmodus.".into()),
    };
    settings.language = match settings.language.as_str() {
        "de" | "en" => settings.language,
        _ => return Err("Unbekannte Sprache.".into()),
    };
    settings.theme = match settings.theme.as_str() {
        "dunkel" | "hell" => settings.theme,
        _ => return Err("Unbekanntes Design.".into()),
    };
    settings.start_page = match settings.start_page.as_str() {
        "start" | "shorts" => settings.start_page,
        _ => return Err("Unbekannte Startseite.".into()),
    };
    settings.youtube_client_id = validate_client_id(&settings.youtube_client_id)?;
    settings.tiktok_client_key = validate_client_id(&settings.tiktok_client_key)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_youtube_id() {
        assert!(is_youtube_id("dQw4w9WgXcQ"));
        assert!(!is_youtube_id("short"));
        assert!(!is_youtube_id("bad id here"));
    }

    #[test]
    fn rejects_foreign_thumb_host() {
        assert!(clean_thumb("https://i.ytimg.com/vi/dQw4w9WgXcQ/mqdefault.jpg").contains("ytimg"));
        assert!(clean_thumb("https://evil.example/a.jpg").is_empty());
        assert!(clean_thumb("http://i.ytimg.com/vi/x.jpg").is_empty());
    }

    #[test]
    fn blocks_non_platform_links() {
        assert!(ensure_platform_url("https://www.youtube.com/shorts/dQw4w9WgXcQ").is_ok());
        assert!(ensure_platform_url("https://evil.example/watch").is_err());
        assert!(ensure_platform_url("file:///C:/secret.mp4").is_err());
    }
}
