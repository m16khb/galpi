"""Pure domain helpers for the Galpi worker."""

import re
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Literal, cast

SpeakerMode = Literal["auto", "exact", "range"]

# The model's hotwords slot keeps roughly the first 223 prompt tokens; Korean
# syllables average about one token each, so a character budget slightly under
# that leaves safe headroom for the separator and special tokens.
ASR_HOTWORDS_CHAR_BUDGET = 200


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


def parse_asr_context(payload: object) -> tuple[list[str], list[str], list[str]]:
    """Read the biasing lists handed over by the host as (terms, names, aliases)."""

    if not isinstance(payload, dict):
        raise TypeError("asr context payload was not a JSON object")
    fields = cast(dict[str, object], payload)

    def strings(key: str) -> list[str]:
        raw = fields.get(key)
        if raw is None:
            return []
        if not isinstance(raw, list):
            raise TypeError(f"asr context {key} was not a JSON array")
        return [
            value.strip()
            for value in cast(list[object], raw)
            if isinstance(value, str) and value.strip()
        ]

    return strings("terms"), strings("names"), strings("aliases")


def build_asr_hotwords(
    terms: Sequence[str], names: Sequence[str], aliases: Sequence[str]
) -> str:
    """Pack glossary terms, participant names, and aliases into one biasing string.

    Glossary terms outrank names, names outrank aliases: rare domain words gain
    the most from biasing. The hotwords slot keeps the *front* of the string when
    the model truncates, so the first fitting entries survive and whole entries
    drop once the character budget runs out.
    """

    seen: set[str] = set()
    ordered: list[str] = []
    for word in (*terms, *names, *aliases):
        cleaned = word.strip()
        if cleaned and cleaned not in seen:
            seen.add(cleaned)
            ordered.append(cleaned)
    kept: list[str] = []
    used = 0
    for word in ordered:
        cost = len(word) + (2 if kept else 0)
        if used + cost > ASR_HOTWORDS_CHAR_BUDGET:
            break
        kept.append(word)
        used += cost
    return ", ".join(kept)
