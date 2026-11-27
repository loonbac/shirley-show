# Colab official CLI migration

Replace the third-party xeodou/colab-cli with Google's official google-colab-cli, verify with a real separation, and deploy to production without breaking existing QR, ticket or check-in behavior.

## Specs

S1. "entonces hay que migrar directamente a la opcion oficial todo correctamente, probar que funcione y subir a produccion sin romper nada, mi cliente quiere usarlo hoy mismo, terminalo bien"
S2. "CLI oficial de Google (recomendado hoy)"
S3. "si, el backend de colab si o si debe hacer la separacion, asegurarse de que se prenda y se apague justo cuando se necesite y ya no se necesite"

## Tasks

T1 (S1, S2) — Image, environment and deployment docs: replace the xeodou binary download with a pinned `google-colab-cli` install; update Dockerfile, compose.yaml, .env.example, docs/colab-separation.md, docs/deployment-koro-panel.md and docs/deployment-coolify.md, including auth/session paths and new command names. Route: gentle-ai-worker. Commit: pending.
T2 (S1, S3) — Rust adapter: map the vocal separator to official CLI commands (`new`, `upload`, `exec -f`, `download`, `stop`), preserve token validation before GPU start, SHA-256/duration verification, and guaranteed stop on success/error/cancel/boot. Files under src/infra/vocal/, src/main.rs, src/config.rs if needed, and tests/colab_*_tests.rs. Route: gentle-ai-worker. Commit: pending.
T3 (S1, S2) — Transport simplification: try official `colab download`; remove the catbox/AES/curl workaround only after integrity checks pass, keeping SHA-256 and duration verification. Files src/infra/vocal/colab_transfer.rs, src/infra/vocal/colab_remote_script.rs and their tests. Route: gentle-ai-worker. Commit: pending.
T4 (S1) — Verification: focused RED/GREEN tests, a real-song separation smoke test against a live runtime, and post-deploy production smoke checks. Route: gentle-ai-worker for tests, parent for deploy checks. Commit: pending.
T5 (S1) — Delivery: issue, branch, PR, merge and deploy through the repository's established flow. Route: parent. Commit: pending.

## Log

L1 (petición del usuario, literal): "entonces hay que migrar directamente a la opcion oficial todo correctamente, probar que funcione y subir a produccion sin romper nada, mi cliente quiere usarlo hoy mismo, terminalo bien"
L2 (decisión del usuario, literal): "CLI oficial de Google (recomendado hoy)"
