use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri::webview::WebviewBuilder;

const MOBILE_UA: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Mobile Safari/537.36";
const DESKTOP_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
const BAR: f64 = 48.0;
const GUTTER: f64 = 0.0;

static FEED_URL: Mutex<String> = Mutex::new(String::new());

const PLAYER_SCRIPT: &str = r##"
(() => {
  if (!Element.prototype.__mhShadow) {
    const orig = Element.prototype.attachShadow;
    Element.prototype.attachShadow = function (init) {
      return orig.call(this, Object.assign({}, init, { mode: "open" }));
    };
    Element.prototype.__mhShadow = true;
  }
  if (window.__mhReady) return;
  window.__mhReady = true;
  window.__mhStep = (dir) => {
    const now = Date.now();
    if (now - (window.__mhNav || 0) < 420) return;
    window.__mhNav = now;
    if (location.hostname.includes("tiktok")) {
      const arrow = document.querySelector(dir > 0
        ? "[data-e2e='arrow-down'], [data-e2e='arrow-right']"
        : "[data-e2e='arrow-up'], [data-e2e='arrow-left']");
      if (arrow) {
        arrow.click();
        return;
      }
      const host = document.querySelector(".swiper");
      const api = host && host.swiper;
      if (api && api.slides && api.slides.length > 2) {
        if (dir > 0) api.slideNext();
        else api.slidePrev();
        return;
      }
      const column = document.querySelector("#column-list-container");
      if (column) {
        column.scrollBy({ top: dir * (column.clientHeight || window.innerHeight), behavior: "auto" });
      }
    }
    const want = dir > 0 ? ["nächst", "naechst", "next"] : ["vorher", "previous", "zurück", "zurueck"];
    const button = [...document.querySelectorAll("button")].find((el) => {
      const label = (el.getAttribute("aria-label") || "").toLowerCase();
      return want.some((word) => label.includes(word));
    });
    if (button) {
      button.click();
      return;
    }
    const reel = document.querySelector("#carousel-scrollable-wrapper");
    if (reel) {
      reel.scrollBy({ top: dir * (reel.clientHeight || window.innerHeight), behavior: "auto" });
    }
  };
  if (location.hostname.includes("youtube") || location.hostname.includes("tiktok")) {
    window.addEventListener("wheel", (event) => {
      if (Math.abs(event.deltaY) < 15) return;
      event.preventDefault();
      event.stopPropagation();
      window.__mhStep(event.deltaY > 0 ? 1 : -1);
    }, { capture: true, passive: false });
  }
  let toggledAt = 0;
  document.addEventListener("click", (event) => {
    const now = Date.now();
    if (now - toggledAt < 280) {
      event.preventDefault();
      event.stopImmediatePropagation();
      return;
    }
    toggledAt = now;
  }, true);
  const fitYouTube = () => {
    if (!location.hostname.includes("youtube")) return;
    const min = 560;
    const width = window.innerWidth;
    document.documentElement.style.zoom = width > 0 && width < min ? String(width / min) : "1";
  };
  fitYouTube();
  window.addEventListener("resize", fitYouTube);
  const hideChrome = () => {
    if (document.getElementById("mh-hide-chrome")) return;
    const style = document.createElement("style");
    style.id = "mh-hide-chrome";
    style.textContent = [
      "ytd-masthead, #masthead, #masthead-container,",
      "ytd-mini-guide-renderer, ytd-guide-renderer, #guide,",
      "ytm-mobile-topbar-renderer, ytm-pivot-bar-renderer,",
      "ytm-shorts-header-renderer, .ytm-shorts-header, #header-bar,",
      "#chips-wrapper, ytd-feed-filter-chip-bar-renderer,",
      "a[aria-label='YouTube'], button[aria-label='Suchen'], button[aria-label='Search'] {",
      "display: none !important; }",
      "ytd-app, ytd-page-manager { margin-top: 0 !important; padding-top: 0 !important; }"
    ].join("");
    document.documentElement.appendChild(style);
  };
  const safeHide = () => {
    if (!document.documentElement) return;
    hideChrome();
  };
  safeHide();
  document.addEventListener("DOMContentLoaded", safeHide);
  if (document.documentElement) {
    new MutationObserver(safeHide).observe(document.documentElement, { childList: true, subtree: true });
  }
  let userAction = false;
  document.addEventListener("pointerdown", () => {
    userAction = true;
    setTimeout(() => { userAction = false; }, 700);
  }, true);
  let kicks = 0;
  const kick = () => {
    const video = document.querySelector("video");
    if (!video || kicks > 12) return;
    if (!video.paused) return;
    if (userAction) return;
    kicks += 1;
    const pending = video.play();
    if (pending && pending.catch) pending.catch(() => {});
  };
  new MutationObserver(kick).observe(document.documentElement, { childList: true, subtree: true });
  document.addEventListener("pause", (event) => {
    if (!event.target || event.target.tagName !== "VIDEO" || userAction) return;
    kick();
  }, true);
  setTimeout(kick, 600);
  setTimeout(kick, 1600);
  if (location.hostname.includes("tiktok")) {
    const dismiss = () => {
      const skip = [...document.querySelectorAll("button")].find((el) =>
        /jetzt nicht|not now|nicht jetzt/i.test(el.textContent || "")
      );
      if (skip) skip.click();
    };
    setInterval(dismiss, 800);
  }
  window.addEventListener("keydown", (event) => {
    const tag = event.target && event.target.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      window.__mhStep(1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      window.__mhStep(-1);
    }
  }, true);
})();
"##;

fn host_ok(url: &url::Url, platform: &str) -> bool {
    let host = url.host_str().unwrap_or("");
    match platform {
        "youtube" => {
            host == "www.youtube.com"
                || host == "m.youtube.com"
                || host == "youtube.com"
                || host == "youtu.be"
        }
        "tiktok" => host == "www.tiktok.com" || host == "tiktok.com" || host.ends_with(".tiktok.com"),
        _ => false,
    }
}

fn login_target(platform: &str) -> Result<url::Url, String> {
    let raw = if platform == "tiktok" {
        "https://www.tiktok.com/login"
    } else if platform == "youtube" {
        "https://accounts.google.com/ServiceLogin?service=youtube&hl=de&continue=https%3A%2F%2Fwww.youtube.com%2F"
    } else {
        return Err("Unbekannte Plattform.".into());
    };
    raw.parse().map_err(|_| "Ungültige Adresse.".to_string())
}

fn logged_in(platform: &str, url: &url::Url) -> bool {
    let host = url.host_str().unwrap_or("");
    let path = url.path();
    if platform == "youtube" {
        let youtube = host == "www.youtube.com" || host == "m.youtube.com" || host == "youtube.com";
        youtube && !path.contains("signin") && !path.contains("login")
    } else if platform == "tiktok" {
        (host == "www.tiktok.com" || host == "tiktok.com") && !path.contains("login")
    } else {
        false
    }
}

fn finish_login(app: &AppHandle, platform: String) {
    if let Some(window) = app.get_webview_window("login") {
        let _ = window.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
    let _ = app.emit("login-ok", platform);
}

#[tauri::command]
pub async fn show_feed(app: AppHandle, url: String) -> Result<(), String> {
    let parsed: url::Url = url.parse().map_err(|_| "Ungültige Adresse.".to_string())?;
    let platform = if parsed.host_str().unwrap_or("").contains("tiktok") {
        "tiktok"
    } else {
        "youtube"
    };
    if !host_ok(&parsed, platform) {
        return Err("Diese Adresse ist nicht erlaubt.".into());
    }
    let window = app
        .get_window("main")
        .ok_or_else(|| "Hauptfenster fehlt.".to_string())?;
    let factor = window.scale_factor().unwrap_or(1.0);
    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let mut width = (inner.width as f64 / factor).max(320.0);
    let height = ((inner.height as f64 / factor) - BAR).max(240.0);
    width = (width - GUTTER).max(280.0);
    let same = {
        let current = FEED_URL.lock().map_err(|_| "Player gesperrt.".to_string())?;
        *current == url
    };
    if let Some(existing) = app.get_webview("feed") {
        if same {
            let _ = existing.set_position(LogicalPosition::new(GUTTER, BAR));
            let _ = existing.set_size(LogicalSize::new(width, height));
            let _ = existing.show();
            return Ok(());
        }
        let _ = existing.close();
    }
    let agent = if platform == "tiktok" { DESKTOP_UA } else { MOBILE_UA };
    let builder = WebviewBuilder::new("feed", WebviewUrl::External(parsed))
        .user_agent(agent)
        .initialization_script_for_all_frames(PLAYER_SCRIPT);
    window
        .add_child(
            builder,
            LogicalPosition::new(GUTTER, BAR),
            LogicalSize::new(width, height),
        )
        .map_err(|e| e.to_string())?;
    let mut current = FEED_URL.lock().map_err(|_| "Player gesperrt.".to_string())?;
    *current = url;
    Ok(())
}

#[tauri::command]
pub async fn nudge_feed(app: AppHandle, direction: i32) -> Result<(), String> {
    let view = app
        .get_webview("feed")
        .ok_or_else(|| "Kein Video offen.".to_string())?;
    let step = if direction < 0 { -1 } else { 1 };
    view.eval(format!("window.__mhStep && window.__mhStep({step})"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_login(app: AppHandle, platform: String) -> Result<(), String> {
    if platform != "youtube" && platform != "tiktok" {
        return Err("Unbekannte Plattform.".into());
    }
    if let Some(existing) = app.get_webview_window("login") {
        let _ = existing.show();
        let _ = existing.set_focus();
        return Ok(());
    }
    let target = login_target(&platform)?;
    let watched = Arc::new(AtomicBool::new(false));
    let app_nav = app.clone();
    let platform_nav = platform.clone();
    let watched_nav = watched.clone();
    let title = if platform == "tiktok" {
        "Bei TikTok anmelden"
    } else {
        "Mit Google anmelden"
    };
    WebviewWindowBuilder::new(&app, "login", WebviewUrl::External(target))
        .title(title)
        .inner_size(480.0, 740.0)
        .center()
        .resizable(true)
        .focused(true)
        .user_agent(DESKTOP_UA)
        .on_navigation(move |url| {
            let host = url.host_str().unwrap_or("");
            let on_gate = host.contains("google.")
                || host.contains("accounts.")
                || url.path().contains("login")
                || url.path().contains("signin");
            if on_gate {
                watched_nav.store(true, Ordering::SeqCst);
                return true;
            }
            if watched_nav.load(Ordering::SeqCst) && logged_in(&platform_nav, url) {
                let app = app_nav.clone();
                let platform = platform_nav.clone();
                watched_nav.store(false, Ordering::SeqCst);
                tauri::async_runtime::spawn(async move {
                    finish_login(&app, platform);
                });
            }
            true
        })
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}
