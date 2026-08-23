---
name: overview
description: Family module overview: installation and runtime operation.
---

# Operations — Overview

Canonical index: [OPERATIONS.md](../../OPERATIONS.md)

## Prerequisites (README 빠른 시작)

- macOS 14+ on Apple Silicon; Rust 1.85+; Bun 1.3+.
- Tauri CLI: `cargo install tauri-cli --version 2.11.4 --locked`.
- `uv`/`uvx` on PATH for Python verification gates (`brew install uv`).

## Local development

```bash
bun install
bun run dev        # cargo tauri dev — stages verified arm64 uv, Python
                   # worker, frontend, and Tauri app, then runs
bun run vite:dev   # frontend only: vite --port 1420 --strictPort
bun run sidecar:stage  # stage sidecars without running the app
```

- No global Python, ffmpeg, or WhisperX install is required; `bun run dev`
  stages an app-managed Python 3.12 environment. First engine setup may
  download GB-scale models into the app data folder; later runs reuse it.
- Dev/build staging may download the pinned ARM64 `uv` archive first.

## Environment and secrets

- No required env vars for the app itself. User-level settings live in the
  app settings UI, never in docs/logs: Hugging Face fine-grained read-only
  token (only for first `pyannote/speaker-diarization-community-1` download),
  OpenAI-compatible endpoint + key for meeting-minutes refinement.
- Do not put raw tokens (`hf_...`, API keys) in docs, test fixtures, or logs.

## Build and release

```bash
bun run build      # cargo tauri build --bundles app --ci + DMG script
```

Outputs `src-tauri/target/release/bundle/macos/Galpi.app` and
`bundle/dmg/Galpi_0.1.0_aarch64.dmg` via macOS `hdiutil`. Signing and
notarization require an Apple Developer certificate and are done separately.
Build scripts currently hardcode ARM64 and artifact version 0.1.0.

## Generated trees (not source; never edit)

`node_modules`, `dist`, `src-tauri/target`, `src-tauri/resources/worker`,
`src-tauri/binaries`.

## Smoke checks

- Quick: `bun run check && bun test`.
- Packaging smoke: `bun run build` then open the produced `.app`.
