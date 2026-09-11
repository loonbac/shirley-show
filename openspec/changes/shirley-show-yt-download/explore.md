# SDD Exploration: `shirley-show-yt-download`

**Change**: `shirley-show-yt-download`
**Date**: 2026-09-11
**Status**: Completed (persisted inline by orchestrator; full version in Engram `sdd/shirley-show-yt-download/explore`)

## Brief
La artista dice a Ressy: "descarga este video youtube.com/... en pista por favor".
Ressy muestra trabajo en progreso, descarga el audio, avisa que está listo y pregunta
a qué evento/presentación agregarlo.

## Environment facts
- yt-dlp 2026.07.04 in `~/.nix-profile/bin` (NOT in systemd PATH) + ffmpeg 8.1.2.
- Resolution: `YT_DLP_BIN` env with fallback; friendly error if missing.
- Server runs as systemd user unit on :3100; `.run/` untouched.

## Architecture decisions
- `YouTubeUrl` domain: strict youtube.com/youtu.be validation, 11-char video id,
  `--no-playlist`, mp3 extract, `--max-filesize 100M`, `duration <= 1200`, no live.
- `YoutubeDownloader` port + `ProcessYoutubeDownloader` infra via `tokio::process`
  (structured args, no shell). 90s subprocess timeout with kill + purge.
- `DownloadYouTubeAudio` use case + `StagedAudioRegistry` (in-memory TTL 15min)
  for two-turn flows (download now, attach to show later).
- New tool `download_youtube_track(url, title?)`; chain with `add_track_from_staged`
  same-turn when the presentation is known.
- Global agent timeout 30s → 120s (`SHIRLEY_AGENT_TIMEOUT_SECS`).
- Typing indicator label switches to "descargando el audio…" on YouTube URLs (no SSE).
- Tests hermetic via mock yt-dlp stub script.

## Non-goals
Playlists, video tracks, live streams, non-YouTube platforms, cookies/auth.

## Slices
1. Domain + port + infra + stub tests (~250 lines).
2. Use case + registry + AppState (~220).
3. Tool schema + dispatcher + prompt + chain tests (~230).
4. Chat timeout + typing progress + staged_audio_id frontend (~180).

## Key Learnings
1. yt-dlp lives outside systemd PATH; resolve via YT_DLP_BIN.
2. Request-scoped staging needs a TTL registry for two-turn attach flows.
3. 30s global timeout is insufficient for downloads; raise to 120s.
4. Typing label can go reactive without SSE.
