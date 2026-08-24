"""Local types for the untyped qwen-asr dependency."""

from collections.abc import Sequence

class TimestampEntry:
    @property
    def text(self) -> str: ...
    @property
    def start_time(self) -> float: ...
    @property
    def end_time(self) -> float: ...

class TranscriptionResult:
    @property
    def language(self) -> str: ...
    @property
    def text(self) -> str: ...
    @property
    def time_stamps(self) -> Sequence[TimestampEntry]: ...

class Qwen3ASRModel:
    def transcribe(
        self,
        *,
        audio: str,
        language: str | None = ...,
        context: str | None = ...,
        return_time_stamps: bool = ...,
    ) -> Sequence[TranscriptionResult]: ...

def from_pretrained(
    model: str,
    *,
    dtype: object,
    device_map: str,
    forced_aligner: str | None = ...,
    max_inference_batch_size: int = ...,
    max_new_tokens: int = ...,
) -> Qwen3ASRModel: ...
