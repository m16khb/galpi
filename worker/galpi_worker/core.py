"""Pure domain helpers for the Galpi worker."""

import re
from collections import Counter
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

# A repetition loop is a Whisper failure mode on silence or noise: one short
# token repeats with high confidence for the whole segment (a person's name
# dozens of times), so confidence-based rules never catch it. Meeting speech
# never has one token dominating a whole segment, so token dominance is the
# tell: at least six tokens with the same token covering 80% of them.
_REPETITION_MIN_TOKENS = 6
_REPETITION_DOMINANCE = 0.8
_REPETITION_TOKEN_SPLIT = re.compile(r"[,\.\s]+")


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
    """Return whether a segment is a known hallucination or a repetition loop."""

    return _HALLUCINATION_PATTERN.search(text) is not None or _is_repetition_loop(text)


def _is_repetition_loop(text: str) -> bool:
    tokens = [token for token in _REPETITION_TOKEN_SPLIT.split(text) if token]
    if len(tokens) < _REPETITION_MIN_TOKENS:
        return False
    dominant_count = Counter(tokens).most_common(1)[0][1]
    return dominant_count / len(tokens) >= _REPETITION_DOMINANCE


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
