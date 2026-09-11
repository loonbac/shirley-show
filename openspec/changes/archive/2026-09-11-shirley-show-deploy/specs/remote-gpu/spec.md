# Delta spec: remote-gpu (mirror Engram sdd/shirley-show-deploy/spec)

## ADDED Requirements

### Requirement: Adapter Selection Local vs Remote
1. `JOSHUA_PC_HOST` set (non-blank) MUST select `SshVocalSeparator`; unset MUST keep `ProcessVocalSeparator`.
2. Use cases MUST depend only on `Arc<dyn VocalSeparator>`.

#### Scenario: Remote selected when host configured
- GIVEN `JOSHUA_PC_HOST=100.73.247.39`
- WHEN the app boots
- THEN separation uses the SSH adapter.

#### Scenario: Local preserved when host unset
- GIVEN no `JOSHUA_PC_HOST`
- WHEN the app boots
- THEN behavior is exactly the local one.

### Requirement: Presence Probe and Wake-Wait
1. Probe MUST be `ssh -o ConnectTimeout=3 … true`, structured args, no shell.
2. On failure MUST backoff (2s→5s→10s→15s cap) until `JOSHUA_PC_WAKE_TIMEOUT_MINS` (default 10); then abort with `pc_wake_timeout`.

#### Scenario: PC boots mid-wait
- GIVEN offline PC coming online at minute 3
- WHEN waiting
- THEN the pipeline proceeds without restarting the turn.

#### Scenario: 10 minutes without PC
- GIVEN 600s unreachable
- WHEN deadline hits
- THEN abort with `pc_wake_timeout` and keep `staged_audio_id`.

### Requirement: Idle Shutdown Lifecycle
1. On connect MUST run `sudo -n shutdown -c` (ignore exit 1; password prompt → `remote_sudo_denied`).
2. Only after validated MP3 MUST run `sudo -n shutdown +60`; failure schedules nothing but keeps the audio.
3. Remote requires NOPASSWD for the shutdown binary.

#### Scenario: New job cancels pending shutdown
- GIVEN a scheduled `+60`
- WHEN a new job connects
- THEN `shutdown -c` runs before any transfer.

#### Scenario: Success schedules shutdown
- GIVEN validated 320k MP3
- WHEN the job completes
- THEN `shutdown +60` is scheduled.

### Requirement: Isolation, Transfer, Cleanup, Validation
1. Per-op `/tmp/shirley-vocal/<uuid>` (0700); scp audio up, MP3 back; RAII `rm -rf` on every exit path.
2. Local ffprobe MUST verify mp3 codec, 310-330k bitrate, duration ±2s; else reject, delete, no shutdown.

#### Scenario: Corrupt download rejected
- GIVEN truncated MP3
- WHEN validating
- THEN reject with `InvalidSeparatorOutput` and schedule nothing.

#### Scenario: Missing key fails fast
- GIVEN unreadable key file
- WHEN dispatching
- THEN fail with `ssh_key_missing` before any network spawn.
