# Design: shirley-show-deploy (mirror Engram sdd/shirley-show-deploy/design)

**Binding correction (orchestrator, prevails):** NO SSE, NO staging sidecars.
Transport is plain JSON with typed notices; staging registry stays in-memory TTL.

## Decisions
- D1: Single `VocalSeparator` port; `SshVocalSeparator` selected in composition when `JOSHUA_PC_HOST` set.
- D2: `SshCommandBuilder` centralizes all ssh/scp argv (BatchMode, IdentitiesOnly, accept-new); no shell anywhere locally.
- D3: `RemoteDirectoryGuard` owns child killer + remote `rm -rf` + local partial + semaphore permit, in that order.
- D4: Remote runs a constant script (mode enum + UUID paths only); never interpolates user text.
- D5: Deadlines — local separation 600s; remote composed 1500s (`SHIRLEY_REMOTE_SEPARATION_TIMEOUT_SECS`); wake 600s; margin 300s. Clamps >600 removed for remote.
- D6: Notices `wake_wait` (progress) / `pc_wake_timeout` (give-up, staged kept); verbatim Spanish copy; no IPs/flags/paths leak.
- D7: Typing label `esperando la PC de Joshua…` while waiting; restore default after.
- D8: Energy — `shutdown -c` on connect (exit 1 tolerated; password prompt fatal), `shutdown +60` only after validated MP3 (failure non-fatal). NOPASSWD sudoers on nixos-pc.
- D9: Docker multi-stage, UID 1000, volume `shirley-data:/app/data`, key `:ro`, healthcheck `/api/presentations`, `network_mode: host` primary.
- D10: Hermetic tests via stub ssh/scp/ffprobe binaries; never touch `100.73.247.39` in tests.

## Rationale
Remote GPU keeps Coolify free of CUDA weight; MP3-only backhaul saves ~90% bandwidth;
systemd-owned shutdown timer needs no server state; JSON notices avoid SSE proxy pain.
