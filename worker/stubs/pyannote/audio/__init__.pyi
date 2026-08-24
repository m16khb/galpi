"""Local types for the untyped pyannote.audio dependency (qwen3 venv)."""

from collections.abc import Iterator
from typing import Any

class Annotation:
    def itertracks(self, yield_label: bool = ...) -> Iterator[Any]: ...

class Pipeline:
    @classmethod
    def from_pretrained(
        cls, checkpoint_path: str, *, use_auth_token: str | None = ...
    ) -> Pipeline: ...
    def to(self, device: object) -> None: ...
    def __call__(
        self,
        file: str,
        *,
        num_speakers: int | None = ...,
        min_speakers: int | None = ...,
        max_speakers: int | None = ...,
    ) -> Annotation: ...
