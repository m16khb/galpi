from typing import NotRequired, TypedDict

class SingleSegment(TypedDict):
    start: float
    end: float
    text: str
    avg_logprob: NotRequired[float]

class SingleAlignedSegment(SingleSegment):
    words: list[dict[str, object]]
    chars: list[dict[str, object]] | None

class TranscriptionResult(TypedDict):
    segments: list[SingleSegment]
    language: str

class AlignedTranscriptionResult(TypedDict):
    segments: list[SingleAlignedSegment]
    word_segments: list[dict[str, object]]
