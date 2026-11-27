# Colab official CLI migration

Mirror of odd/tasks/colab-cli-oficial.md

Replace the third-party xeodou/colab-cli with Google's official google-colab-cli, verify with a real separation, and deploy to production without breaking existing QR, ticket or check-in behavior.

## Specs

S1. "entonces hay que migrar directamente a la opcion oficial todo correctamente, probar que funcione y subir a produccion sin romper nada, mi cliente quiere usarlo hoy mismo, terminalo bien"
S2. "CLI oficial de Google (recomendado hoy)"
S3. "si, el backend de colab si o si debe hacer la separacion, asegurarse de que se prenda y se apague justo cuando se necesite y ya no se necesite"

## Tasks

T1 (S1, S2) — Image, environment and deployment docs: replace the xeodou binary download with a pinned `google-colab-cli` install; update Dockerfile, compose.yaml, .env.example, docs/colab-separation.md, docs/deployment-koro-panel.md and docs/deployment-coolify.md, including auth/session paths and new command names. Route: gentle-ai-worker. Commit: 922fab5.
T2 (S1, S3) — Rust adapter: map the vocal separator to official CLI commands (`new`, `upload`, `exec -f`, `download`, `stop`), preserve token validation before GPU start, SHA-256/duration verification, and guaranteed stop on success/error/cancel/boot. Files under src/infra/vocal/, src/main.rs, src/config.rs if needed, and tests/colab_*_tests.rs. Route: gentle-ai-worker. Commit: 5dbc00e.
T3 (S1, S2) — Transport simplification: try official `colab download`; remove the catbox/AES/curl workaround only after integrity checks pass, keeping SHA-256 and duration verification. Files src/infra/vocal/colab_transfer.rs, src/infra/vocal/colab_remote_script.rs and their tests. Route: gentle-ai-worker. Commit: 272939c.
T4 (S1) — Verification: focused RED/GREEN tests, a real-song separation smoke test against a live runtime, and post-deploy production smoke checks. Route: gentle-ai-worker for tests, parent for deploy checks. Commit: 44a38f1 (image-build fixes; verification still pending).
T5 (S1) — Delivery: issue, branch, PR, merge and deploy through the repository's established flow. Route: parent. Commit: pending.

## Log

L1 (petición del usuario, literal): "entonces hay que migrar directamente a la opcion oficial todo correctamente, probar que funcione y subir a produccion sin romper nada, mi cliente quiere usarlo hoy mismo, terminalo bien"
L2 (decisión del usuario, literal): "CLI oficial de Google (recomendado hoy)"
L3 (verificación ejecutada): focused suites green — `cargo test --locked --test deploy_manifest_tests --test colab_vocal_separator_tests --test colab_transfer_tests --test separate_vocals_tests` → 7 + 33 + 25 + 13 passed, 0 failed, in instabox `ressy-build`.
L4 (verificación ejecutada): real Docker build of the final Dockerfile succeeded; as UID 1000 inside the image `colab version` printed `Version: 0.8.0`, `/usr/local/bin/colab` resolves outside `/root`, and the 14 frontend data files are present.
L5 (bloqueo): production deployment not executed — panel API returned 401 and no API token was available; SSH to the documented host was denied and the user clarified it was this PC, not the server. Real-song separation against a live runtime therefore remains pending.
