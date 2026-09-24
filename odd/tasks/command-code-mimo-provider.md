# Command Code MiMo Provider Migration

## Goal
Replace the Shirley Show app's default OpenCode Go API configuration with Command Code Provider API, using MiMo V2.6 Pro while preserving Groq as an optional fallback.

## Acceptance criteria
- The app reads `CMD_API_KEY`, defaults to Command Code's OpenAI-compatible Provider API endpoint, and selects `xiaomi/mimo-v2.6-pro`.
- Deployment configuration and operator documentation describe the new key and model without exposing credentials.
- Provider configuration, HTTP behavior, and affected user-facing missing-key messages are covered by tests.
- Existing Groq fallback behavior remains available.

## Tasks
1. [x] Update provider config/client integration and provider-facing diagnostics for Command Code; preserve Groq fallback.
2. [x] Update deployment documentation and affected tests, then run focused validation.

## Evidence
- `cargo fmt -- --check` — passed.
- `cargo test --test groq_client_tests` — 21 passed.
- `cargo test --test agent_chat_api_tests test_chat_api_unconfigured_key_returns_200_with_notice` — 1 passed.
- `cargo test --test agent_chat_tests test_agent_chat_unconfigured_provider` — 1 passed.
- `cargo test --test deploy_manifest_tests` — 6 passed.
- `git diff --check` — passed.
- `.env.example` was not modified because access was denied as a sensitive path; update it separately through an authorized path if needed.
