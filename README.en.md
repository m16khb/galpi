<p align="center">
  <img src="assets/app-icon.svg" width="96" alt="Galpi app icon" />
</p>

<h1 align="center">Galpi · 갈피</h1>

<p align="center">
  A local-first desktop app that records meetings on Apple Silicon Macs,<br />
  separates speakers, transcribes Korean audio, and optionally produces AI meeting minutes
</p>

<p align="center">
  <a href="README.md">한국어</a>
  ·
  <a href="README.en.md"><strong>English</strong></a>
</p>

> [!IMPORTANT]
> Galpi 0.1.0 is a development build for **Apple Silicon Macs running macOS 14 or later**.
> The current DMG is unsigned and not notarized. Intel Macs, Windows, and Linux are not supported.

## At a glance

Galpi records audio inside the app or imports an existing meeting file, then runs Korean transcription and speaker diarization locally with WhisperX. Results are stored in a folder you choose. Optionally, Galpi can send the transcript to an OpenAI-compatible API and generate structured Korean meeting minutes in Markdown.

| Capability | What it does |
|---|---|
| Direct recording | Records CoreAudio microphone input to a 16-bit PCM WAV |
| File import | `m4a`, `mp3`, `wav`, `mp4`, `mov`, `aac`, `flac`, `ogg` |
| Local transcription | Korean ASR with WhisperX `large-v3-turbo` |
| Speaker diarization | pyannote diarization with automatic, exact, or min/max speaker-count hints |
| Alignment | Korean sentence alignment and long-silence hallucination filtering |
| Participant roster | Reusable names, teams, roles, aliases, and descriptions |
| Glossary | Reusable proper nouns and domain terminology |
| AI minutes | Decision-, owner-, and deadline-oriented Markdown via an OpenAI-compatible API |
| Job control | Cancel setup or transcription with detailed logs; monitor AI refinement progress and errors |

## Quick start

### 1. Install development tools

Requirements:

- macOS 14 or later on Apple Silicon
- Rust 1.85 or later
- Bun 1.3 or later
- Tauri CLI 2.11.4

```bash
cargo install tauri-cli --version 2.11.4 --locked
bun install
bun run dev
```

`bun run dev` stages the verified arm64 `uv` binary, the Python worker, the frontend, and the Tauri app. You do not need to preinstall Python, ffmpeg, or WhisperX globally.

### 2. Prepare the local engine

1. Open `설정` (Settings) from the top-right corner.
2. On a new Mac, save a Hugging Face token if the diarization model requires access.
3. Select `로컬 엔진 준비` (Prepare local engine).
4. Wait until WhisperX, the transcription model, and ffmpeg all show `Ready`.

The first setup installs an app-specific Python 3.12 environment and may download several gigabytes of models. Later runs reuse the same app data directory and model cache.

### 3. Hugging Face token

The token is only required when the diarization model is downloaded for the first time.

1. Accept the terms for [`pyannote/speaker-diarization-community-1`](https://huggingface.co/pyannote/speaker-diarization-community-1).
2. Create a **Fine-grained** token in [Hugging Face Access Tokens](https://huggingface.co/settings/tokens).
3. Grant **Read** access only to that repository.
4. Save the `hf_...` value in Galpi Settings.

Write access and Inference Providers access are not required.

## Workflow

### Record a meeting or choose a file

**Record inside Galpi**

1. Confirm the output folder.
2. Select `마이크로 바로 녹음` (Record with microphone) and grant macOS microphone access.
3. Select `정지` (Stop) when the meeting ends.
4. The completed WAV is selected automatically as the transcription input.

Recording uses a bounded queue and a dedicated WAV writer, so the entire meeting is not kept in memory. `버리기` (Discard) cancels the recording and removes its partial file.

> [!NOTE]
> Galpi currently captures the selected Mac microphone input only. It does not capture system audio from Zoom, Meet, or other apps.

**Import an existing recording**

1. Select `오디오 파일 선택` (Choose audio file).
2. Confirm the output folder.
3. Choose `자동` (Automatic), `정확히` (Exact), or `범위` (Min/max range) for the speaker-count hint.
4. Select `전사 시작` (Start transcription).

### Participants and glossary

Settings can store reusable context:

- Participants: name, team, role, aliases, and description
- Glossary: term and optional explanation
- Meeting background: purpose, context, and preferred output style

This context helps Galpi produce stable names and terminology in refined meeting minutes.

### Generate AI meeting minutes

After transcription, select `AI 증강 실행` (Run AI augmentation) to create Markdown through an OpenAI-compatible API. You can also augment an existing transcript without a new recording: use `전사문 파일 가져오기` (Import transcript file) in the augmentation panel (txt/md).

- Default model: `glm-5.3`
- Default API: `https://api.z.ai/api/coding/paas/v4`
- Model, Base URL, and reasoning effort are configurable

Setup:

1. Open `설정` (Settings) and enter the provider API Key in the **회의록 가공** (Meeting-minutes refinement) section.
2. If you are not using z.ai, enter the provider's model name and OpenAI-compatible Base URL.
3. Select the participants for this meeting and review the glossary and background context.
4. Finish transcription, then select `AI 증강 실행`.

> [!WARNING]
> Audio recording and WhisperX transcription stay on the Mac. When you select `AI 증강 실행`, the transcript, participants selected for this meeting, glossary, and background context are sent to the configured external API. Review the provider's security and retention policy before using this feature with sensitive meetings.

## Outputs

The default location is `~/Documents/Galpi` (changeable from the output folder picker). One folder corresponds to one meeting: a microphone recording creates `YYYY-MM-DD HHMMSS 녹음` named after its start time, while imported audio and transcripts keep their original file name. Every artifact inside a meeting folder shares the folder's name.

```text
~/Documents/Galpi/
├── 2026-08-24 143052 녹음/     # microphone recording (auto-named by start time)
│   ├── 2026-08-24 143052 녹음.wav
│   ├── 2026-08-24 143052 녹음.srt
│   ├── 2026-08-24 143052 녹음_화자별.txt
│   ├── 2026-08-24 143052 녹음.aligned.v2.json
│   └── 2026-08-24 143052 녹음_회의록.md      # produced by AI augmentation
└── 팀미팅/                      # imported audio/transcript (original name)
    ├── 팀미팅.srt
    ├── 팀미팅_화자별.txt
    ├── 팀미팅.aligned.v2.json
    └── 팀미팅_회의록.md
```

Name collisions get a numeric suffix such as `팀미팅 2`. When an alignment checkpoint (`.aligned.v2.json`) exists, re-transcribing the same audio skips the transcription and alignment stages.

| File | Purpose |
|---|---|
| `.srt` | Subtitles with timestamps |
| `_화자별.txt` | Readable speaker-oriented transcript |
| `.aligned.v2.json` | Alignment checkpoint and reprocessing input |
| `_회의록.md` | Decisions, owners, deadlines, and discussion notes (`회의록` means meeting minutes) |

The completion screen can open each artifact or reveal its output folder in Finder.

## Local data and privacy

- Audio and transcription artifacts are stored in the local folder you choose.
- WhisperX models are stored in Galpi's app-specific Hugging Face cache.
- Hugging Face and assistant credentials are stored in an Application Support settings file with `0600` permissions.
- Credentials are not currently encrypted with macOS Keychain.
- Transcripts are not sent to an external LLM API unless AI minutes are run.
- The worker launches fixed programs with explicit argv and does not execute shell strings.

## Architecture

```text
TypeScript UI
    │ Tauri IPC + validated events
    ▼
Rust application
    │ ports
    ├── CoreAudio recorder
    ├── filesystem / opener
    └── supervised Python worker
            │ versioned JSONL
            ▼
       WhisperX / pyannote / assistant
```

| Path | Responsibility |
|---|---|
| `src/` | Domain contracts (job, speaker, backend port) with validation, state machines, Zod boundaries, DOM UI |
| `src-tauri/` | Tauri commands, Rust use cases and domain value objects, recording and process adapters |
| `worker/` | WhisperX transcription, alignment, diarization, and minutes refinement |
| `scripts/` | Architecture checks, sidecar staging, and DMG packaging |
| `DESIGN.md` | Normative UI, accessibility, and component-state contract |
| `docs/ARCHITECTURE.md` | Normative layering and port ownership |
| `docs/ROADMAP.md` | Current state and planned product milestones |

## Development commands

### Fast checks

```bash
bun run check
bun test
```

### Full validation

Python validation requires `uv`/`uvx` on PATH. With Homebrew, install it using `brew install uv`. Replace `<WhisperX Python path>` with the absolute path to a Python executable where WhisperX is installed.

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
uvx ruff check worker
uvx ruff format --check worker
uvx basedpyright --pythonpath <WhisperX Python path>
PYTHONPATH=. python3 -m unittest worker.tests.test_core -v
```

### Production build

```bash
bun run build
```

Outputs:

```text
src-tauri/target/release/bundle/macos/Galpi.app
src-tauri/target/release/bundle/dmg/Galpi_0.1.0_aarch64.dmg
```

The build creates the `.app` first, then packages the DMG with `hdiutil`. Distribution signing and notarization must be performed separately in an Apple Developer certificate environment.

## Troubleshooting

| Symptom | What to check |
|---|---|
| `cargo tauri` is not found | Run `cargo install tauri-cli --version 2.11.4 --locked` |
| Model download returns 401/403 | Accept the model terms and verify the Fine-grained Read token |
| Microphone recording does not start | Check Galpi microphone access in System Settings |
| Other participants are not recorded | System-audio capture is not supported yet |
| AI minutes fail | Check API Key, Base URL, model name, and provider quota |
| The app will not open on another Mac | Current builds are unsigned and not notarized; check Gatekeeper and distribution state |

## Project status

Galpi is under active development at version `0.1.0`. Automatic updates, signing and notarization, system-audio capture, and a meeting library are planned for later milestones. See [`docs/ROADMAP.md`](docs/ROADMAP.md) for details.
