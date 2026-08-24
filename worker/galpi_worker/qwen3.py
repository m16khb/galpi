"""Qwen3 candidate transcription pipeline owned by Galpi.

Runs Qwen3-ASR-1.7B for Korean recognition with the Qwen3-ForcedAligner
producing timestamps in the same pass, then diarizes on the shared pyannote
community-1 model. Heavy imports stay inside functions so the pure helpers
remain testable without the ML stack.
"""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
from importlib import import_module
from pathlib import Path
from typing import Protocol, TypedDict, cast

from .artifacts import Segment, filter_segments, write_outputs_atomic
from .core import SpeakerHint, parse_asr_context, validate_speaker_hint
from .preparation import (
    PYANNOTE_MODEL_ID,
    QWEN3_ALIGNER_MODEL_ID,
    QWEN3_ASR_MODEL_ID,
)
from .protocol import EventWriter
from .runtime import configure_warnings, select_torch_device

# The aligner emits short chunks; subtitles stay readable when consecutive
# chunks merge into sentence groups ending at terminal punctuation or when a
# group grows past roughly one breath of speech.
SENTENCE_ENDINGS = ".!?…"
MAX_SENTENCE_SECONDS = 12.0
SENTENCE_SPLIT = re.compile(r"(?<=[.!?…])\s+")
# Aligner chunks carry bare words without spaces or terminal punctuation,
# so those text characters are skipped when mapping text onto chunks.
SKIPPED_TEXT_CHARS = ".,!?…"


class TimestampEntry(TypedDict):
    """One aligner chunk: a text span with start and end seconds."""

    text: str
    start: float
    end: float


class AlignerEntry(Protocol):
    """Attribute shape the qwen-asr aligner publishes for one chunk."""

    @property
    def text(self) -> str: ...

    @property
    def start_time(self) -> float: ...

    @property
    def end_time(self) -> float: ...


class SpeakerTurn(TypedDict):
    """One diarization turn: a speaker label holding a time span."""

    start: float
    end: float
    speaker: str


def transcribe_qwen3(
    audio_path: Path,
    output_dir: Path,
    speaker_hint: SpeakerHint,
    events: EventWriter,
    asr_context_path: Path | None = None,
) -> None:
    """Recognize, timestamp, diarize, filter, and publish artifacts."""

    validate_speaker_hint(speaker_hint)
    if not audio_path.is_file():
        raise ValueError(f"audio file not found: {audio_path}")
    output_dir.mkdir(parents=True, exist_ok=True)
    configure_warnings()

    import torch

    device = select_torch_device(mps_available=torch.backends.mps.is_available())
    # The Qwen3 stack decodes audio through libsndfile, which rejects
    # compressed containers (m4a/AAC, mov, ...). Every input goes through the
    # bundled ffmpeg into 16 kHz mono WAV first, like the WhisperX path.
    events.emit(
        "phase",
        phase="transcribing",
        percent=5.0,
        message="오디오를 ffmpeg로 디코딩합니다.",
    )
    with tempfile.TemporaryDirectory(prefix="galpi-qwen3-") as work_dir:
        working_audio = Path(work_dir) / "audio.wav"
        decode_to_wav(audio_path, working_audio)

        events.emit(
            "phase",
            phase="transcribing",
            percent=10.0,
            message=f"한국어 음성을 Qwen3로 전사합니다. ({device.upper()})",
        )
        from qwen_asr import Qwen3ASRModel

        model = Qwen3ASRModel.from_pretrained(
            QWEN3_ASR_MODEL_ID,
            forced_aligner=QWEN3_ALIGNER_MODEL_ID,
        )
        context = build_bias_context(asr_context_path)
        result = model.transcribe(
            audio=str(working_audio),
            language="Korean",
            context=context,
            return_time_stamps=True,
        )[0]
        del model

        events.emit(
            "phase",
            phase="transcribing",
            percent=100.0,
            message="전사가 완료되었습니다.",
        )
        events.emit(
            "phase",
            phase="aligning",
            percent=100.0,
            message="Qwen3 정렬기로 문장 시간을 확정했습니다.",
        )
        entries = [entry_to_timestamp(entry) for entry in result.time_stamps]
        segments = build_segments(result.text, entries)

        events.emit(
            "phase",
            phase="diarizing",
            percent=10.0,
            message=f"{device.upper()}에서 화자를 분리합니다.",
        )
        turns = diarize(working_audio, speaker_hint, device, events)
        assigned = assign_speakers_to_segments(segments, turns)
        events.emit(
            "phase",
            phase="diarizing",
            percent=100.0,
            message="화자분리가 완료되었습니다.",
        )

        duration = audio_duration(working_audio)

    kept, filtered = filter_segments(assigned, duration)

    events.emit(
        "phase",
        phase="writing",
        percent=30.0,
        message="환각 구간을 정리하고 결과를 씁니다.",
    )
    base_name = audio_path.stem
    srt_path = output_dir / f"{base_name}.srt"
    text_path = output_dir / f"{base_name}_화자별.txt"
    write_outputs_atomic(srt_path, text_path, kept)
    events.emit(
        "phase", phase="writing", percent=100.0, message="결과 파일을 저장했습니다."
    )
    # The Qwen3 pipeline publishes srt/txt only, so the completion carries an
    # empty checkpoint string meaning "no checkpoint" for the host.
    events.emit(
        "completed",
        srt=str(srt_path),
        txt=str(text_path),
        checkpoint="",
        segments=len(kept),
        filtered=len(filtered),
    )


class ImageioFfmpeg(Protocol):
    @staticmethod
    def get_ffmpeg_exe() -> str: ...


def ffmpeg_decode_args(source: Path, destination: Path) -> list[str]:
    """ffmpeg invocation that normalizes any container to 16 kHz mono WAV."""

    return [
        "-y",
        "-i",
        str(source),
        "-ar",
        "16000",
        "-ac",
        "1",
        str(destination),
    ]


def decode_to_wav(source: Path, destination: Path) -> None:
    """Decode any supported container through the bundled ffmpeg binary."""

    ffmpeg = cast(
        ImageioFfmpeg,
        cast(object, import_module("imageio_ffmpeg")),
    ).get_ffmpeg_exe()
    completed = subprocess.run(
        [ffmpeg, *ffmpeg_decode_args(source, destination)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0 or not destination.is_file():
        raise RuntimeError(
            f"ffmpeg failed to decode {source.name} (exit {completed.returncode})"
        )


def build_bias_context(asr_context_path: Path | None) -> str:
    """Turn the glossary/roster biasing file into a freeform Qwen3 hint."""

    if asr_context_path is None or not asr_context_path.is_file():
        return ""
    terms, names, aliases = parse_asr_context(
        json.loads(asr_context_path.read_text(encoding="utf-8"))
    )
    parts: list[str] = []
    if terms:
        parts.append("도메인 용어: " + ", ".join(terms))
    if names:
        parts.append("참석자 이름: " + ", ".join(names))
    if aliases:
        parts.append("별칭: " + ", ".join(aliases))
    return "\n".join(parts)


def entry_to_timestamp(entry: object) -> TimestampEntry:
    """Normalize one aligner chunk from its documented attribute shape."""

    for attribute in ("text", "start_time", "end_time"):
        if not hasattr(entry, attribute):
            raise TypeError(f"unexpected aligner entry shape: {type(entry)!r}")
    typed = cast(AlignerEntry, entry)
    return TimestampEntry(
        text=typed.text,
        start=typed.start_time,
        end=typed.end_time,
    )


def build_segments(
    transcription_text: str,
    entries: list[TimestampEntry],
) -> list[Segment]:
    """Split the model text into sentences timed by aligner chunks.

    The full `text` carries proper spacing and punctuation while the aligner
    chunks are bare words, so sentences keep their original form and chunk
    boundaries only decide where each sentence starts and ends in time.
    """

    sentences = [
        sentence
        for sentence in SENTENCE_SPLIT.split(transcription_text.strip())
        if sentence.strip()
    ]
    segments: list[Segment] = []
    chunk_index = 0
    chunk_offset = 0
    for sentence in sentences:
        target = matchable_chars(sentence)
        if not target:
            continue
        start: float | None = None
        end = 0.0
        consumed = 0
        while consumed < len(target) and chunk_index < len(entries):
            entry = entries[chunk_index]
            chunk = matchable_chars(entry["text"])
            available = len(chunk) - chunk_offset
            if available <= 0:
                chunk_index += 1
                chunk_offset = 0
                continue
            if start is None:
                start = entry["start"]
            take = min(available, len(target) - consumed)
            consumed += take
            chunk_offset += take
            end = entry["end"]
            if chunk_offset >= len(chunk):
                chunk_index += 1
                chunk_offset = 0
        if start is not None and consumed > 0:
            segments.append(Segment(start=start, end=end, text=sentence.strip()))
    return segments


def matchable_chars(text: str) -> list[str]:
    """Characters that aligner chunks can also carry (no spaces/punctuation)."""

    return [
        character
        for character in text
        if not character.isspace() and character not in SKIPPED_TEXT_CHARS
    ]


def assign_speakers_to_segments(
    segments: list[Segment],
    turns: list[SpeakerTurn],
) -> list[Segment]:
    """Label each segment with the turn covering the most of its span."""

    assigned: list[Segment] = []
    for segment in segments:
        best_speaker = "UNKNOWN"
        best_overlap = 0.0
        for turn in turns:
            overlap = min(segment["end"], turn["end"]) - max(
                segment["start"], turn["start"]
            )
            if overlap > best_overlap:
                best_overlap = overlap
                best_speaker = turn["speaker"]
        assigned.append(
            Segment(
                start=segment["start"],
                end=segment["end"],
                text=segment["text"],
                speaker=best_speaker,
            )
        )
    return assigned


def diarize(
    audio_path: Path,
    hint: SpeakerHint,
    device: str,
    events: EventWriter,
) -> list[SpeakerTurn]:
    """Run pyannote community-1 with one MPS-to-CPU retry."""

    import torch
    from pyannote.audio import Pipeline

    waveform, sample_rate = load_waveform(audio_path)
    audio_input = {"waveform": waveform, "sample_rate": sample_rate}

    def load(load_device: str) -> Pipeline:
        pipeline = Pipeline.from_pretrained(PYANNOTE_MODEL_ID)
        if load_device != "cpu":
            pipeline.to(torch.device(load_device))
        return pipeline

    try:
        pipeline = load(device)
    except Exception:
        if device != "mps":
            raise
        events.log("MPS 화자분리에 실패해 CPU로 다시 시도합니다.")
        pipeline = load("cpu")

    options: dict[str, int] = {}
    if hint.mode == "exact":
        options["num_speakers"] = hint.exact or 1
    elif hint.mode == "range":
        if hint.minimum is not None:
            options["min_speakers"] = hint.minimum
        if hint.maximum is not None:
            options["max_speakers"] = hint.maximum
    diarization_output = pipeline(audio_input, **options)
    # community-1 returns a DiarizeOutput dataclass; the *exclusive* variant
    # drops overlapping speech, which maps cleaner onto ASR segments.
    annotation = diarization_output.exclusive_speaker_diarization
    return [
        SpeakerTurn(
            start=turn.start,
            end=turn.end,
            speaker=str(speaker),
        )
        for turn, _track, speaker in annotation.itertracks(yield_label=True)
    ]


def load_waveform(audio_path: Path) -> tuple[object, int]:
    """Read mono 16 kHz samples for pyannote's waveform-dict input.

    The qwen3 venv has no torchcodec, so pyannote cannot decode file paths
    there; handing it the decoded tensor sidesteps the backend entirely.
    """

    import torchaudio

    waveform, sample_rate = torchaudio.load(str(audio_path))
    if waveform.shape[0] > 1:
        waveform = waveform.mean(dim=0, keepdim=True)
    if sample_rate != 16000:
        waveform = torchaudio.functional.resample(waveform, sample_rate, 16000)
        sample_rate = 16000
    return waveform, sample_rate


def audio_duration(audio_path: Path) -> float:
    """Read the audio length in seconds without loading samples."""

    import torchaudio

    info = torchaudio.info(str(audio_path))
    return float(info.num_frames) / float(info.sample_rate)
