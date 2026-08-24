"""Local types for the untyped qwen-asr dependency (verified against 0.0.6)."""

from collections.abc import Sequence
from typing import Any

class TimestampEntry:
    @property
    def text(self) -> str: ...
    @property
    def start_time(self) -> float: ...
    @property
    def end_time(self) -> float: ...

class ASRTranscription:
    @property
    def language(self) -> str: ...
    @property
    def text(self) -> str: ...
    @property
    def time_stamps(self) -> Sequence[TimestampEntry]: ...

class Qwen3ASRModel:
    @classmethod
    def from_pretrained(
        cls,
        pretrained_model_name_or_path: str,
        forced_aligner: str | None = ...,
        forced_aligner_kwargs: dict[str, Any] | None = ...,
        max_inference_batch_size: int = ...,
        max_new_tokens: int | None = ...,
        **kwargs: Any,
    ) -> Qwen3ASRModel: ...
    def transcribe(
        self,
        audio: str,
        context: str = ...,
        language: str | None = ...,
        return_time_stamps: bool = ...,
    ) -> Sequence[ASRTranscription]: ...

def parse_asr_output(content: str) -> tuple[str, str]: ...
