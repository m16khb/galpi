"""Shared runtime configuration for third-party audio libraries."""

import logging
import warnings
from importlib import import_module
from typing import Literal, Protocol, cast

TorchDevice = Literal["cpu", "mps"]


class ImageioFfmpeg(Protocol):
    @staticmethod
    def get_ffmpeg_exe() -> str: ...


def ffmpeg_executable() -> str:
    """Path to the ffmpeg binary bundled with imageio-ffmpeg.

    Imported lazily: the pure helpers in this package must stay importable
    without the engine virtualenv installed.
    """

    return cast(
        ImageioFfmpeg,
        cast(object, import_module("imageio_ffmpeg")),
    ).get_ffmpeg_exe()


def select_torch_device(*, mps_available: bool) -> TorchDevice:
    return "mps" if mps_available else "cpu"


def configure_warnings() -> None:
    logging.getLogger("lightning.pytorch.utilities.migration.utils").setLevel(
        logging.WARNING
    )
    warnings.filterwarnings(
        "ignore",
        message=r"\ntorchcodec is not installed correctly",
        category=UserWarning,
        module=r"pyannote\.audio\.core\.io",
    )
