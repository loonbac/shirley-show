# SDD Exploration: `shirley-show-deploy`

**Change**: `shirley-show-deploy`
**Date**: 2026-09-11
**Status**: Completed (full version in Engram `sdd/shirley-show-deploy/explore`, obs. 355)

## Topology
- Coolify host: `eje-desarrollo` (100.96.97.65, tailnet, no GPU).
- GPU node: `nixos-pc` (100.73.247.39, RTX 3060, sshd active, karaoke-separator + checkpoints + ffmpeg).
- Container must reach nixos-pc:22 → prefer `network_mode: host`, else bridge forwarding, else tailscale sidecar.

## Decisions
- Dockerfile multi-stage (rust builder + debian-slim runtime: ffmpeg, ffprobe, yt-dlp, openssh-client, curl, sqlite3; non-root UID 1000; HEALTHCHECK on /api/presentations).
- compose.yaml: volume shirley_data:/app/data, envs (GROQ_*, JOSHUA_PC_*), SSH key mounted :ro. Secrets never in image.
- SshVocalSeparator implements VocalSeparator: presence probe (ssh true, 3s), wake-wait backoff (default 10min), `sudo shutdown -c` on connect, scp audio up + run pipelines on nixos-pc + scp MP3 back + remote rm -rf, local ffprobe validation, `sudo shutdown +60` after success.
- Local mode preserved when JOSHUA_PC_HOST unset.
- Ressy prompt: exact waiting message, 1h idle notice, 10min give-up message. Typing label "esperando la PC de Joshua…".
- sudoers NOPASSWD for shutdown on nixos-pc (user applies in configuration.nix).
- Tests hermetic via stub ssh/scp binaries.

## Slices
1. Docker packaging + docs (~220). 2. SSH executor + presence (~380). 3. Idle shutdown + prompt/UX (~190).

## Key Learnings
1. MP3 back-haul saves ~90% vs raw stems over Tailscale.
2. Delegating shutdown timer to remote systemd needs no server state.
3. Host network mode is the simple path to tailnet nodes.
