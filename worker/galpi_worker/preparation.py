"""First-run runtime and model preparation."""

import gc
import json
import os
import shutil
from importlib import import_module
from importlib.metadata import version
from pathlib import Path
from typing import Protocol, cast

from .protocol import EventWriter
from .runtime import configure_warnings, select_torch_device

PYANNOTE_MODEL_ID = "pyannote/speaker-diarization-community-1"
QWEN3_ASR_MODEL_ID = "Qwen/Qwen3-ASR-1.7B"
QWEN3_ALIGNER_MODEL_ID = "Qwen/Qwen3-ForcedAligner-0.6B"


class ImageioFfmpeg(Protocol):
    @staticmethod
    def get_ffmpeg_exe() -> str: ...


def prepare_models(
    manifest: Path,
    engine_bin: Path,
    events: EventWriter,
    engine: str = "whisperx",
) -> None:
    """Install the bundled ffmpeg link and load every model once."""

    configure_warnings()
    link_ffmpeg(engine_bin)
    if engine == "qwen3":
        prepare_qwen3_models(manifest, events)
    else:
        prepare_whisperx_models(manifest, events)


def link_ffmpeg(engine_bin: Path) -> None:
    engine_bin.mkdir(parents=True, exist_ok=True)
    imageio_ffmpeg = cast(
        ImageioFfmpeg,
        cast(object, import_module("imageio_ffmpeg")),
    )
    ffmpeg_target = Path(imageio_ffmpeg.get_ffmpeg_exe())
    ffmpeg_link = engine_bin / "ffmpeg"
    if ffmpeg_link.exists() or ffmpeg_link.is_symlink():
        ffmpeg_link.unlink()
    try:
        ffmpeg_link.symlink_to(ffmpeg_target)
    except OSError:
        shutil.copy2(ffmpeg_target, ffmpeg_link)
        ffmpeg_link.chmod(0o755)


def prepare_whisperx_models(manifest: Path, events: EventWriter) -> None:
    import torch
    import whisperx
    from whisperx.diarize import DiarizationPipeline

    device = select_torch_device(mps_available=torch.backends.mps.is_available())

    events.emit(
        "phase", phase="models", percent=10.0, message="전사 모델을 준비합니다."
    )
    model = whisperx.load_model(
        "large-v3-turbo",
        "cpu",
        compute_type="int8",
        language="ko",
        asr_options={
            "no_speech_threshold": 0.75,
            "condition_on_previous_text": False,
        },
        vad_options={"vad_onset": 0.6, "vad_offset": 0.4},
    )
    del model
    gc.collect()

    events.emit(
        "phase",
        phase="models",
        percent=55.0,
        message=f"{device.upper()}용 한국어 정렬 모델을 준비합니다.",
    )
    alignment_device = device
    try:
        align_model, _metadata = whisperx.load_align_model(
            language_code="ko", device=alignment_device
        )
    except Exception:
        if alignment_device != "mps":
            raise
        events.log("MPS 정렬 모델 준비에 실패해 CPU로 다시 시도합니다.")
        alignment_device = "cpu"
        align_model, _metadata = whisperx.load_align_model(
            language_code="ko", device=alignment_device
        )
    del align_model
    gc.collect()

    events.emit(
        "phase",
        phase="models",
        percent=78.0,
        message=f"{device.upper()}용 화자분리 모델을 준비합니다.",
    )
    diarization_device = device
    try:
        diarization = DiarizationPipeline(
            model_name="pyannote/speaker-diarization-community-1",
            device=diarization_device,
        )
    except Exception:
        if diarization_device != "mps":
            raise
        events.log("MPS 화자분리 모델 준비에 실패해 CPU로 다시 시도합니다.")
        diarization_device = "cpu"
        diarization = DiarizationPipeline(
            model_name="pyannote/speaker-diarization-community-1",
            device=diarization_device,
        )
    del diarization
    gc.collect()

    manifest.parent.mkdir(parents=True, exist_ok=True)
    payload: dict[str, object] = {
        "protocol": 1,
        "whisperx": version("whisperx"),
        "torch": version("torch"),
        "pyannote_audio": version("pyannote.audio"),
        "asr_model": "large-v3-turbo",
        "diarization_model": "pyannote/speaker-diarization-community-1",
        "device": diarization_device,
        "alignment_device": alignment_device,
        "asr_device": "cpu",
    }
    write_manifest(manifest, payload)
    events.emit(
        "phase", phase="models", percent=100.0, message="모델 준비가 완료되었습니다."
    )
    events.emit("prepared", engine_version=version("whisperx"))


def prepare_qwen3_models(manifest: Path, events: EventWriter) -> None:
    """Download the Qwen3 candidate stack into the app's HF cache."""

    import torch
    from huggingface_hub import snapshot_download

    device = select_torch_device(mps_available=torch.backends.mps.is_available())
    token = os.environ.get("HF_TOKEN") or None

    events.emit(
        "phase",
        phase="models",
        percent=10.0,
        message="Qwen3 전사·정렬 모델을 내려받습니다.",
    )
    snapshot_download(QWEN3_ASR_MODEL_ID, token=token)
    snapshot_download(QWEN3_ALIGNER_MODEL_ID, token=token)

    events.emit(
        "phase",
        phase="models",
        percent=60.0,
        message="화자분리 모델을 내려받습니다.",
    )
    # pyannote's community model is gated: the token from settings travels
    # through the prepare environment, and the diarization pipeline is warmed
    # once so the first real transcription does not pay the download cost.
    from pyannote.audio import Pipeline

    pipeline = Pipeline.from_pretrained(PYANNOTE_MODEL_ID, token=token)
    if device != "cpu":
        pipeline.to(torch.device(device))
    del pipeline
    gc.collect()

    payload: dict[str, object] = {
        "protocol": 1,
        "qwen3": "1",
        "torch": version("torch"),
        "pyannote_audio": version("pyannote.audio"),
        "asr_model": QWEN3_ASR_MODEL_ID,
        "alignment_model": QWEN3_ALIGNER_MODEL_ID,
        "diarization_model": PYANNOTE_MODEL_ID,
        "device": device,
    }
    write_manifest(manifest, payload)
    events.emit(
        "phase", phase="models", percent=100.0, message="모델 준비가 완료되었습니다."
    )
    events.emit("prepared", engine_version=QWEN3_ASR_MODEL_ID)


def write_manifest(manifest: Path, payload: dict[str, object]) -> None:
    manifest.parent.mkdir(parents=True, exist_ok=True)
    temporary = manifest.with_suffix(".tmp")
    temporary.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    temporary.replace(manifest)
