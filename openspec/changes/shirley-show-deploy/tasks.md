# Tasks: shirley-show-deploy (mirror Engram sdd/shirley-show-deploy/tasks — all complete)

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~850-950 across 3 slices |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR1 packaging → PR2 SSH executor → PR3 idle-off/UX |
| Delivery strategy | ask-on-risk |
| Chain strategy | stacked-to-main |

Decision needed before apply: No (approved: 3 chained PRs)
Chained PRs recommended: Yes
400-line budget risk: High

---

## Slice 1: Packaging & Deploy Docs

- [x] S1.1 Dockerfile multi-stage + non-root + healthcheck. <!-- sdd-owner: implementation -->
- [x] S1.2 .dockerignore estricto. <!-- sdd-owner: implementation -->
- [x] S1.3 compose.yaml (host network, volumen, key :ro, envs). <!-- sdd-owner: implementation -->
- [x] S1.4 docs/deployment-coolify.md + sudoers + red. <!-- sdd-owner: implementation -->
- [x] S1.5 Validar manifests + cargo test verde. <!-- sdd-owner: implementation -->

## Slice 2: SSH Executor & Presence

- [x] S2.1 Harness hermético con stubs ssh/scp/ffprobe. <!-- sdd-owner: implementation -->
- [x] S2.2 JoshuaPcConfig + timeouts (1500 remoto). <!-- sdd-owner: implementation -->
- [x] S2.3 Variantes de error remoto (`pc_wake_timeout`, `remote_sudo_denied`, `ssh_key_missing`). <!-- sdd-owner: implementation -->
- [x] S2.4 SshCommandBuilder injection-proof. <!-- sdd-owner: implementation -->
- [x] S2.5 RemoteDirectoryGuard RAII. <!-- sdd-owner: implementation -->
- [x] S2.6 Config inyectable de binarios. <!-- sdd-owner: implementation -->
- [x] S2.7 SshVocalSeparator (probe, backoff, round-trip, ffprobe). <!-- sdd-owner: implementation -->
- [x] S2.8 Wiring AppState local/remoto. <!-- sdd-owner: implementation -->
- [x] S2.9 Triangulación (7+ edge cases). <!-- sdd-owner: implementation -->
- [x] S2.10 cargo test + fmt + clippy, cero red real. <!-- sdd-owner: implementation -->

## Slice 3: Idle-off & Conversational UX

- [x] S3.1 Tests notices wake_wait + copy verbatim + give-up. <!-- sdd-owner: implementation -->
- [x] S3.2 Energía (`shutdown -c` / `+60`, no fatal post-audio). <!-- sdd-owner: implementation -->
- [x] S3.3 Deadline compuesto 1500s, sin clamp 600 remoto. <!-- sdd-owner: implementation -->
- [x] S3.4 Notices + staged preservado. <!-- sdd-owner: implementation -->
- [x] S3.5 Prompt Ressy (directivas espera/energía). <!-- sdd-owner: implementation -->
- [x] S3.6 Typing `esperando la PC de Joshua…` + restauración. <!-- sdd-owner: implementation -->
- [x] S3.7 E2E multiturno offline→online. <!-- sdd-owner: implementation -->
- [x] S3.8 Suite completa sin regresiones. <!-- sdd-owner: implementation -->
