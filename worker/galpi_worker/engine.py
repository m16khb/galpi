"""WhisperX transcription pipeline owned by Galpi."""

from __future__ import annotations

import gc
import json
from pathlib import Path
from typing import TYPE_CHECKING, cast

from .artifacts import (
    Transcription,
    filter_segments,
    write_json_atomic,
    write_outputs_atomic,
)
from .core import SpeakerHint, validate_speaker_hint
from .protocol import EventWriter
from .runtime import configure_warnings, select_torch_device

if TYPE_CHECKING:
    from whisperx.diarize import DiarizationPipeline, DiarizationSegments


def transcribe(
    audio_path: Path,
    output_dir: Path,
    speaker_hint: SpeakerHint,
    events: EventWriter,
) -> None:
    """Transcribe, align, diarize, filter, and publish artifacts."""

    validate_speaker_hint(speaker_hint)
    if not audio_path.is_file():
        raise ValueError(f"audio file not found: {audio_path}")
    output_dir.mkdir(parents=True, exist_ok=True)
    configure_warnings()

    import torch
    import whisperx
    from whisperx.diarize import DiarizationPipeline, assign_word_speakers

    device = select_torch_device(mps_available=torch.backends.mps.is_available())
    base_name = audio_path.stem
    checkpoint_path = output_dir / f"{base_name}.aligned.v2.json"
    audio = whisperx.load_audio(str(audio_path))

    if checkpoint_path.exists():
        events.emit(
            "phase",
            phase="transcribing",
            percent=100.0,
            message="기존 전사 체크포인트를 사용합니다.",
        )
        result = cast(
            Transcription,
            json.loads(checkpoint_path.read_text(encoding="utf-8")),
        )
    else:
        events.emit(
            "phase",
            phase="transcribing",
            percent=5.0,
            message="한국어 음성을 전사합니다. (CTranslate2 CPU · Apple Accelerate)",
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
        result = cast(
            Transcription,
            model.transcribe(audio, batch_size=8, language="ko"),
        )
        del model
        gc.collect()
        events.emit(
            "phase",
            phase="transcribing",
            percent=100.0,
            message="전사가 완료되었습니다.",
        )

        events.emit(
            "phase",
            phase="aligning",
            percent=10.0,
            message=f"{device.upper()}에서 문장과 시간을 정렬합니다.",
        )
        try:
            align_model, metadata = whisperx.load_align_model(
                language_code="ko", device=device
            )
            result = cast(
                Transcription,
                whisperx.align(
                    result["segments"], align_model, metadata, audio, device
                ),
            )
        except Exception:
            if device != "mps":
                raise
            events.log("MPS 문장 정렬에 실패해 CPU로 다시 시도합니다.")
            align_model, metadata = whisperx.load_align_model(
                language_code="ko", device="cpu"
            )
            result = cast(
                Transcription,
                whisperx.align(result["segments"], align_model, metadata, audio, "cpu"),
            )
        del align_model
        gc.collect()
        write_json_atomic(checkpoint_path, result)
        events.emit(
            "phase",
            phase="aligning",
            percent=100.0,
            message="정렬 체크포인트를 저장했습니다.",
        )

    events.emit(
        "phase",
        phase="diarizing",
        percent=10.0,
        message=f"{device.upper()}에서 화자를 분리합니다.",
    )
    try:
        diarization = DiarizationPipeline(
            model_name="pyannote/speaker-diarization-community-1",
            device=device,
        )
        diarization_segments = _diarize(diarization, audio, speaker_hint)
    except Exception:
        if device != "mps":
            raise
        events.log("MPS 화자분리에 실패해 CPU로 다시 시도합니다.")
        diarization = DiarizationPipeline(
            model_name="pyannote/speaker-diarization-community-1",
            device="cpu",
        )
        diarization_segments = _diarize(diarization, audio, speaker_hint)
    events.emit(
        "phase", phase="diarizing", percent=100.0, message="화자분리가 완료되었습니다."
    )

    from whisperx.schema import AlignedTranscriptionResult, TranscriptionResult

    library_result = cast(
        AlignedTranscriptionResult | TranscriptionResult,
        cast(object, result),
    )
    assigned = cast(
        Transcription,
        cast(object, assign_word_speakers(diarization_segments, library_result)),
    )
    duration = len(audio) / 16000
    kept, filtered = filter_segments(assigned["segments"], duration)
    events.emit(
        "phase",
        phase="writing",
        percent=30.0,
        message="환각 구간을 정리하고 결과를 씁니다.",
    )
    srt_path = output_dir / f"{base_name}.srt"
    text_path = output_dir / f"{base_name}_화자별.txt"
    write_outputs_atomic(srt_path, text_path, kept)
    events.emit(
        "phase", phase="writing", percent=100.0, message="결과 파일을 저장했습니다."
    )
    events.emit(
        "completed",
        srt=str(srt_path),
        txt=str(text_path),
        checkpoint=str(checkpoint_path),
        segments=len(kept),
        filtered=len(filtered),
    )


def _diarize(
    pipeline: DiarizationPipeline,
    audio: object,
    hint: SpeakerHint,
) -> DiarizationSegments:
    if hint.mode == "exact":
        return pipeline(
            audio,
            num_speakers=hint.exact,
            return_embeddings=False,
        )
    if hint.mode == "range":
        return pipeline(
            audio,
            min_speakers=hint.minimum,
            max_speakers=hint.maximum,
            return_embeddings=False,
        )
    return pipeline(audio, return_embeddings=False)
