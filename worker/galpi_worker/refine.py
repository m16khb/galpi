"""Transcript refinement orchestration for short and long meetings."""

import json
import os
from pathlib import Path

from .assistant_stream import (
    REFINE_STREAM_START_PERCENT,
    request_minutes,
)
from .minutes_pipeline import (
    REDUCE_PROGRESS_CEILING,
    REDUCE_PROGRESS_START,
    MinutesContext,
    build_map_messages,
    build_reduce_messages,
    map_progress_band,
    refinement_strategy,
    split_transcript,
)
from .minutes_prompt import (
    NO_BACKGROUND,
    SYSTEM_PROMPT,
    build_messages,
    parse_glossary,
    parse_participants,
    render_glossary,
    render_participants,
    transcript_date,
)
from .protocol import EventWriter

API_KEY_VARIABLE = "GALPI_ASSISTANT_API_KEY"


def write_text_atomic(path: Path, document: str) -> None:
    temporary = path.with_suffix(f"{path.suffix}.tmp")
    temporary.write_text(f"{document}\n", encoding="utf-8")
    temporary.replace(path)


def refine(
    transcript_path: Path,
    output_path: Path,
    background_path: Path | None,
    participants_path: Path | None,
    glossary_path: Path | None,
    model: str,
    events: EventWriter,
) -> None:
    """Refine a speaker-labeled transcript into Korean meeting minutes."""

    api_key = os.environ.get(API_KEY_VARIABLE, "").strip()
    if not api_key:
        raise ValueError("assistant API key is not configured")
    events.emit(
        "phase", phase="refining", percent=10.0, message="전사본을 읽는 중입니다."
    )
    transcript = transcript_path.read_text(encoding="utf-8")
    background = (
        background_path.read_text(encoding="utf-8")
        if background_path is not None and background_path.is_file()
        else ""
    )
    participants = (
        parse_participants(json.loads(participants_path.read_text(encoding="utf-8")))
        if participants_path is not None and participants_path.is_file()
        else []
    )
    glossary = (
        parse_glossary(json.loads(glossary_path.read_text(encoding="utf-8")))
        if glossary_path is not None and glossary_path.is_file()
        else []
    )
    meeting_date = transcript_date(transcript_path)
    context = MinutesContext(
        background=background.strip() or NO_BACKGROUND,
        participants=render_participants(participants),
        glossary=render_glossary(glossary),
        meeting_date=meeting_date,
    )
    if refinement_strategy(transcript) == "single":
        events.emit(
            "phase",
            phase="refining",
            percent=REFINE_STREAM_START_PERCENT,
            message=f"{model} 모델로 회의록을 작성하는 중입니다.",
        )
        expected_chars = max(2000, int(len(transcript) * 0.6))
        document = request_minutes(
            build_messages(
                transcript,
                background,
                participants,
                glossary,
                meeting_date,
            ),
            model,
            api_key,
            events,
            expected_chars,
        )
    else:
        chunks = split_transcript(transcript)
        notes: list[str] = []
        for chunk in chunks:
            progress_start, progress_ceiling = map_progress_band(chunk)
            activity = f"긴 회의 핵심 사실 추출 ({chunk.number}/{chunk.total})"
            events.emit(
                "phase",
                phase="refining",
                percent=round(progress_start, 1),
                message=f"{model} 모델로 {activity} 중입니다.",
            )
            notes.append(
                request_minutes(
                    build_map_messages(chunk, context),
                    model,
                    api_key,
                    events,
                    max(1000, int(len(chunk.text) * 0.25)),
                    progress_start=progress_start,
                    progress_ceiling=progress_ceiling,
                    activity=activity,
                )
            )
        events.emit(
            "phase",
            phase="refining",
            percent=REDUCE_PROGRESS_START,
            message=f"{model} 모델로 최종 회의록을 종합하는 중입니다.",
        )
        document = request_minutes(
            build_reduce_messages(notes, context, SYSTEM_PROMPT),
            model,
            api_key,
            events,
            max(2000, int(sum(len(note) for note in notes) * 0.8)),
            progress_start=REDUCE_PROGRESS_START,
            progress_ceiling=REDUCE_PROGRESS_CEILING,
            activity="최종 회의록 종합",
        )
    events.emit(
        "phase", phase="writing", percent=90.0, message="회의록을 저장하는 중입니다."
    )
    write_text_atomic(output_path, document)
    events.emit("refined", minutes=str(output_path))
