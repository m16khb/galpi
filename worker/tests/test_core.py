"""Contract tests for pure Galpi worker behavior."""

import logging
import unittest
from contextlib import redirect_stdout
from io import StringIO

from ..galpi_worker.artifacts import format_timestamp
from ..galpi_worker.core import (
    ASR_HOTWORDS_CHAR_BUDGET,
    SpeakerHint,
    build_asr_hotwords,
    parse_asr_context,
    should_filter_segment,
    validate_speaker_hint,
)
from ..galpi_worker.protocol import EventWriter
from ..galpi_worker.runtime import configure_warnings, select_torch_device
from .minutes_prompt_cases import GlossaryTests, MapReduceTests, ParticipantTests
from .refine_stream_cases import RefinementTests

__all__ = [
    "AsrContextTests",
    "GlossaryTests",
    "HallucinationFilterTests",
    "MapReduceTests",
    "ParticipantTests",
    "RefinementTests",
    "SpeakerHintTests",
]


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


class AsrContextTests(unittest.TestCase):
    def test_parses_the_host_biasing_lists(self) -> None:
        # Given
        payload: object = {
            "terms": [" 갈피 ", "화자분리", ""],
            "names": ["하빈", None, 3, "지우"],
            "aliases": ["프로님"],
        }

        # When
        terms, names, aliases = parse_asr_context(payload)

        # Then
        self.assertEqual(terms, ["갈피", "화자분리"])
        self.assertEqual(names, ["하빈", "지우"])
        self.assertEqual(aliases, ["프로님"])

    def test_accepts_missing_keys_as_empty_lists(self) -> None:
        # Given / When
        terms, names, aliases = parse_asr_context({})

        # Then
        self.assertEqual((terms, names, aliases), ([], [], []))

    def test_rejects_a_non_object_payload(self) -> None:
        # Given / When / Then
        with self.assertRaisesRegex(TypeError, "was not a JSON object"):
            _ = parse_asr_context(["갈피"])

    def test_rejects_a_non_array_entry(self) -> None:
        # Given / When / Then
        with self.assertRaisesRegex(TypeError, "terms was not a JSON array"):
            _ = parse_asr_context({"terms": "갈피"})

    def test_orders_glossary_terms_before_names_and_aliases(self) -> None:
        # Given / When
        hotwords = build_asr_hotwords(["갈피"], ["하빈", "지우"], ["프로님", "지우님"])

        # Then: rare domain words first, people next, spoken aliases last
        self.assertEqual(hotwords, "갈피, 하빈, 지우, 프로님, 지우님")

    def test_deduplicates_repeated_entries(self) -> None:
        # Given / When
        hotwords = build_asr_hotwords(["갈피"], ["갈피", "하빈"], ["하빈"])

        # Then
        self.assertEqual(hotwords, "갈피, 하빈")

    def test_drops_whole_entries_once_the_budget_runs_out(self) -> None:
        # Given: terms long enough that the third cannot fit
        long_terms = ["가" * 80, "나" * 80, "다" * 80]

        # When
        hotwords = build_asr_hotwords(long_terms, [], [])

        # Then: the first two fit (80 + 2 + 80 = 162); the third would exceed
        # the budget and is dropped whole rather than cut mid-word.
        self.assertEqual(hotwords, "가" * 80 + ", " + "나" * 80)
        self.assertLessEqual(len(hotwords), ASR_HOTWORDS_CHAR_BUDGET)

    def test_returns_nothing_without_any_entries(self) -> None:
        # Given / When / Then
        self.assertEqual(build_asr_hotwords([], [], []), "")


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
