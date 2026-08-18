"""Contract tests for pure Galpi worker behavior."""

import logging
import unittest
from contextlib import redirect_stdout
from io import StringIO

from ..galpi_worker.artifacts import format_timestamp
from ..galpi_worker.core import (
    SpeakerHint,
    should_filter_segment,
    validate_speaker_hint,
)
from ..galpi_worker.protocol import EventWriter
from ..galpi_worker.runtime import configure_warnings, select_torch_device


class SpeakerHintTests(unittest.TestCase):
    def test_rejects_zero_exact_speakers(self) -> None:
        # Given
        hint = SpeakerHint(mode="exact", exact=0)

        # When / Then
        with self.assertRaisesRegex(ValueError, "greater than zero"):
            validate_speaker_hint(hint)

    def test_rejects_reversed_speaker_range(self) -> None:
        # Given
        hint = SpeakerHint(mode="range", minimum=4, maximum=2)

        # When / Then
        with self.assertRaisesRegex(ValueError, "minimum"):
            validate_speaker_hint(hint)


class HallucinationFilterTests(unittest.TestCase):
    def test_filters_known_subscription_phrase(self) -> None:
        # Given
        text = "시청해 주셔서 감사합니다. 구독과 좋아요 부탁드립니다."

        # When
        filtered = should_filter_segment(text)

        # Then
        self.assertTrue(filtered)

    def test_preserves_short_real_utterance(self) -> None:
        # Given
        text = "잠시만요"

        # When
        filtered = should_filter_segment(text)

        # Then
        self.assertFalse(filtered)

    def test_normalizes_rounded_srt_timestamp_carry(self) -> None:
        self.assertEqual(format_timestamp(59.9999), "00:01:00,000")

    def test_protocol_stream_isolated_from_dependency_stdout(self) -> None:
        protocol = StringIO()
        dependency_output = StringIO()
        writer = EventWriter(stream=protocol)

        with redirect_stdout(dependency_output):
            print("third-party diagnostic")
            writer.emit("phase", phase="transcribing", percent=1.0)

        self.assertNotIn("third-party", protocol.getvalue())
        self.assertIn('"type": "phase"', protocol.getvalue())

    def test_suppresses_lightning_checkpoint_migration_info(self) -> None:
        logger = logging.getLogger("lightning.pytorch.utilities.migration.utils")
        previous_level = logger.level
        try:
            configure_warnings()
            self.assertEqual(logger.level, logging.WARNING)
        finally:
            logger.setLevel(previous_level)

    def test_selects_mps_when_apple_gpu_is_available(self) -> None:
        self.assertEqual(select_torch_device(mps_available=True), "mps")
        self.assertEqual(select_torch_device(mps_available=False), "cpu")


if __name__ == "__main__":
    _ = unittest.main()
