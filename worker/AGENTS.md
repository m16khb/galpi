# WORKER KNOWLEDGE BASE

## OVERVIEW

Python sidecar launched and supervised by Rust. `prepare` warms dependencies and
models; `transcribe` runs Korean ASR, alignment, diarization, filtering, and
artifact publication. Stdout is a machine-readable protocol, not a log stream.

## STRUCTURE

```text
worker/
├── galpi_worker/
│   ├── __main__.py        # CLI dispatch, exit mapping, stdout isolation
│   ├── core.py            # Speaker validation and hallucination regex
│   ├── protocol.py        # Versioned, sequenced JSONL emitter
│   ├── runtime.py         # Device selection and warning suppression
│   ├── preparation.py     # Model warmup, ffmpeg link, manifest
│   ├── engine.py          # Transcription pipeline and MPS fallback
│   └── artifacts.py       # Typed segments, filtering, atomic writers
├── stubs/whisperx/        # Local types for the untyped dependency
└── tests/test_core.py     # Pure contract tests; no ML stack required
```

## WHERE TO LOOK

| Task | Location |
|------|----------|
| Add event type/field | `protocol.py`, Rust `domain/worker.rs`, frontend event schema |
| Change device policy | `runtime.py`, retry blocks in `engine.py` |
| Change model pins/options | `engine.py` and `preparation.py` together |
| Change output names/formats | `artifacts.py` |
| Change hallucination filtering | `core.py`, `artifacts.py::filter_segments` |
| Resolve WhisperX typing | Root `pyrightconfig.json`, `stubs/whisperx/*.pyi` |
| Change CLI flags | `__main__.py`; Rust callers in `setup.rs`, `transcription.rs` |

## CONVENTIONS

- Stdout contains only flushed `EventWriter` JSONL with protocol `v: 1`,
  monotonically increasing `seq`, and an event `type`.
- Run the pipeline under `redirect_stdout(sys.stderr)` so dependency prints
  cannot corrupt the protocol stream.
- Import Torch, WhisperX, pyannote, and imageio-ffmpeg inside runtime functions.
  Package import and pure tests must not require the ML environment.
- ASR deliberately uses CTranslate2 CPU int8. Alignment and diarization prefer
  MPS, retry once on CPU, and emit the fallback reason as a log event.
- Keep BasedPyright strict. Convert untyped library payloads with explicit
  casts to local `TypedDict` contracts; maintain realistic `.pyi` stubs.
- Publish checkpoint, SRT, speaker text, and manifests through sibling
  temporary files followed by `Path.replace` or `os.replace`.
- Exit 0 on success, 2 for `INVALID_INPUT`, and 1 for `ENGINE_ERROR`.
- Reusing `.aligned.v2.json` skips ASR/alignment only; diarization and output
  publication still run.
- Tests use stdlib `unittest` over `core`, `artifacts`, `protocol`, and `runtime`.

## ANTI-PATTERNS

- No `print`, progress bar, or logging handler on stdout outside `EventWriter`.
- Do not rename `.aligned.v2.json`, `.srt`, or `_화자별.txt` artifacts.
- Do not change a protocol version, event, or field in Python alone.
- Do not import heavy ML libraries at module scope or in pure tests.
- Do not move ASR to MPS; CTranslate2 CPU int8 is deliberate.
- Do not add retries beyond the single MPS-to-CPU fallback.
- Do not write final artifacts in place.
- Do not weaken a stub merely to hide an installed-library mismatch.
