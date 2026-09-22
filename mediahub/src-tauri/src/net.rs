use serde_json::Value;
use std::time::Duration;

pub struct Http {
    client: reqwest::Client,
}

pub enum NetFail {
    Offline,
    Denied,
    Other,
}

impl Http {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(3))
            .user_agent("TikTube/0.1")
            .build()
            .map_err(|_| "Netzwerk nicht verfügbar.".to_string())?;
        Ok(Self { client })
    }

    pub async fn form_post(&self, url: &str, pairs: &[(&str, &str)]) -> Result<Value, NetFail> {
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(pairs.iter().copied())
            .finish();
        let response = self
            .client
            .post(url)
            .header("content-type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|_| NetFail::Offline)?;
        read_json(response).await
    }

    pub async fn get_bearer(&self, url: &str, token: &str) -> Result<Value, NetFail> {
        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| NetFail::Offline)?;
        read_json(response).await
    }

    pub async fn post_bearer_json(&self, url: &str, token: &str, body: &Value) -> Result<Value, NetFail> {
        let response = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .map_err(|_| NetFail::Offline)?;
        read_json(response).await
    }
}

async fn read_json(response: reqwest::Response) -> Result<Value, NetFail> {
    let status = response.status();
    if response.content_length().unwrap_or(0) > 1_000_000 {
        return Err(NetFail::Other);
    }
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(NetFail::Denied);
    }
    let bytes = response.bytes().await.map_err(|_| NetFail::Offline)?;
    if bytes.len() > 1_000_000 {
        return Err(NetFail::Other);
    }
    if !status.is_success() {
        return Err(NetFail::Other);
    }
    serde_json::from_slice(&bytes).map_err(|_| NetFail::Other)
}

impl NetFail {
    pub fn youtube(self) -> String {
        match self {
            NetFail::Denied => "YouTube-Sitzung abgelaufen. Bitte erneut verbinden.".into(),
            NetFail::Offline | NetFail::Other => {
                "Die Verbindung zu YouTube ist momentan nicht verfügbar.".into()
            }
        }
    }

    pub fn tiktok(self) -> String {
        match self {
            NetFail::Denied => "TikTok-Sitzung abgelaufen. Bitte erneut verbinden.".into(),
            NetFail::Offline | NetFail::Other => {
                "Die Verbindung zu TikTok ist momentan nicht verfügbar.".into()
            }
        }
    }
}
