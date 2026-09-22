# TikTube

Leichte Windows-App für YouTube, TikTok und eigene Videodateien. Ein Player, offizielle Embeds, keine Downloads fremder Plattformvideos.

## Start

Rust und Node sind nötig. Im Ordner `mediahub`:

```text
npm install
npm run tauri dev
```

Fertiges Programm:

```text
npm run tauri build
```

Die Exe liegt danach unter `src-tauri\target\release\mediahub.exe`.

## Konten

Passwörter werden nicht abgefragt und nicht gespeichert. Tokens liegen im Windows-Tresor.

Redirect-URI für Google und TikTok:

```text
http://127.0.0.1:38947/callback
```

YouTube: OAuth-Client vom Typ Desktop, nur die Client-ID in den Einstellungen. Scope `youtube.readonly`.

TikTok: Login Kit, Client-Key in den Einstellungen, Client-Secret nur ins Secret-Feld. Scopes `user.info.basic` und `video.list`.

Ohne Client-Daten funktioniert die App trotzdem: Link einfügen oder eigene mp4/webm/m4v abspielen.
