"""Pure domain helpers for the Galpi worker."""

import re
from dataclasses import dataclass
from typing import Literal

SpeakerMode = Literal["auto", "exact", "range"]


@dataclass(frozen=True, slots=True)
class SpeakerHint:
    mode: SpeakerMode
    exact: int | None = None
    minimum: int | None = None
    maximum: int | None = None


_HALLUCINATION_PATTERN = re.compile(
    r"시청해\s*주?셔서|한글자막\s*by|구독과\s*좋아요|이\s*시각\s*세계였습니다"
)


def validate_speaker_hint(hint: SpeakerHint) -> None:
    """Validate speaker hints before model work starts."""

    if hint.mode == "auto":
        return
    if hint.mode == "exact":
        if hint.exact is None or hint.exact <= 0:
            raise ValueError("speaker count must be greater than zero")
        return
    if hint.minimum is None or hint.maximum is None:
        raise ValueError("speaker range requires minimum and maximum")
    if hint.minimum <= 0 or hint.maximum <= 0:
        raise ValueError("speaker range values must be greater than zero")
    if hint.minimum > hint.maximum:
        raise ValueError("speaker range minimum must not exceed maximum")


def should_filter_segment(text: str) -> bool:
    """Return whether a segment matches a known hallucination pattern."""

    return _HALLUCINATION_PATTERN.search(text) is not None
