use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::model::REDIRECT_PORT;

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
    pub state: String,
}

pub fn create_pkce() -> Result<Pkce, String> {
    let verifier = random_token(32)?;
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    let state = random_token(16)?;
    Ok(Pkce {
        verifier,
        challenge,
        state,
    })
}

fn random_token(bytes: usize) -> Result<String, String> {
    let mut buf = vec![0u8; bytes];
    getrandom::getrandom(&mut buf).map_err(|_| "Zufallszahlen nicht verfügbar.".to_string())?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

pub fn listen_for_code(
    expected_state: &str,
    cancel: &AtomicBool,
    ready: Sender<Result<(), String>>,
) -> Result<String, String> {
    let listener = match TcpListener::bind(("127.0.0.1", REDIRECT_PORT)) {
        Ok(listener) => listener,
        Err(_) => {
            let message = format!("Der Anmelde-Port {REDIRECT_PORT} ist belegt.");
            let _ = ready.send(Err(message.clone()));
            return Err(message);
        }
    };
    if listener.set_nonblocking(true).is_err() {
        let message = "Die Anmeldung konnte nicht gestartet werden.".to_string();
        let _ = ready.send(Err(message.clone()));
        return Err(message);
    }
    let _ = ready.send(Ok(()));
    let started = Instant::now();
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err("Anmeldung abgebrochen.".into());
        }
        if started.elapsed() > Duration::from_secs(180) {
            return Err("Die Anmeldung konnte nicht abgeschlossen werden.".into());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                match handle_request(&mut stream, expected_state) {
                    RequestOutcome::Continue => continue,
                    RequestOutcome::Done(result) => return result,
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(_) => return Err("Die Anmeldung konnte nicht abgeschlossen werden.".into()),
        }
    }
}

enum RequestOutcome {
    Continue,
    Done(Result<String, String>),
}

fn handle_request(stream: &mut TcpStream, expected_state: &str) -> RequestOutcome {
    let text = match read_head(stream) {
        Ok(text) => text,
        Err(_) => {
            let _ = write_page(stream, false);
            return RequestOutcome::Done(Err(
                "Die Anmeldung konnte nicht abgeschlossen werden.".into(),
            ));
        }
    };
    let line = text.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    if method != "GET" {
        let _ = write_page(stream, false);
        return RequestOutcome::Continue;
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if path == "/favicon.ico" {
        let _ = write_status(stream, 404, "text/plain", "no");
        return RequestOutcome::Continue;
    }
    if path != "/callback" {
        let _ = write_status(stream, 404, "text/plain", "no");
        return RequestOutcome::Continue;
    }

    let mut code = String::new();
    let mut state = String::new();
    let mut failed = false;
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        match key.as_ref() {
            "code" => code = value.into_owned(),
            "state" => state = value.into_owned(),
            "error" => failed = true,
            _ => {}
        }
    }
    if failed || state != expected_state || !valid_code(&code) {
        let _ = write_page(stream, false);
        return RequestOutcome::Done(Err(
            "Die Anmeldung konnte nicht abgeschlossen werden.".into(),
        ));
    }
    let _ = write_page(stream, true);
    RequestOutcome::Done(Ok(code))
}

fn valid_code(code: &str) -> bool {
    (8..=512).contains(&code.len())
        && code
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~' | '/'))
}

fn read_head(stream: &mut TcpStream) -> Result<String, std::io::Error> {
    let mut buf = [0u8; 8192];
    let mut filled = 0;
    while filled < buf.len() {
        match stream.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(count) => {
                filled += count;
                if buf[..filled].windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(String::from_utf8_lossy(&buf[..filled]).to_string())
}

fn write_page(stream: &mut TcpStream, ok: bool) -> std::io::Result<()> {
    let body = if ok {
        "<!doctype html><meta charset=\"utf-8\"><title>TikTube</title><body style=\"font-family:Segoe UI,sans-serif;background:#141820;color:#efe7d6;padding:24px\"><p>TikTube ist verbunden. Dieses Fenster kann geschlossen werden.</p>"
    } else {
        "<!doctype html><meta charset=\"utf-8\"><title>TikTube</title><body style=\"font-family:Segoe UI,sans-serif;background:#141820;color:#efe7d6;padding:24px\"><p>Die Anmeldung konnte nicht abgeschlossen werden.</p>"
    };
    write_status(stream, 200, "text/html; charset=utf-8", body)
}

fn write_status(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) -> std::io::Result<()> {
    let reason = if status == 200 { "OK" } else { "Not Found" };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

pub fn open_browser(url: &str) -> Result<(), String> {
    std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|_| "Der Browser konnte nicht geöffnet werden.".to_string())
}
