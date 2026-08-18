"""Shared runtime configuration for third-party audio libraries."""

import logging
import warnings
from typing import Literal

TorchDevice = Literal["cpu", "mps"]


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
