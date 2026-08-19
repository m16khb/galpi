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
from ..galpi_worker.refine import (
    NO_BACKGROUND,
    build_messages,
    extract_minutes,
    strip_document_fence,
)
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


class RefinementTests(unittest.TestCase):
    def test_carries_background_into_the_prompt(self) -> None:
        # Given
        transcript = "[SPEAKER_00] (0s) 배포 일정을 정합시다."
        background = "제품: 갈피\n팀: 하빈(팀리더)"

        # When
        messages = build_messages(transcript, background)

        # Then
        self.assertEqual(messages[0]["role"], "system")
        self.assertIn("갈피", messages[1]["content"])
        self.assertIn("배포 일정을 정합시다", messages[1]["content"])

    def test_marks_missing_background_instead_of_leaving_it_blank(self) -> None:
        # Given / When
        messages = build_messages("[SPEAKER_00] (0s) 안녕하세요.", "   ")

        # Then
        self.assertIn(NO_BACKGROUND, messages[1]["content"])

    def test_rejects_empty_transcript_before_calling_the_assistant(self) -> None:
        # Given / When / Then
        with self.assertRaisesRegex(ValueError, "transcript is empty"):
            _ = build_messages("   \n", "제품: 갈피")

    def test_reads_the_assistant_message_from_a_chat_completion(self) -> None:
        # Given
        payload = {
            "choices": [{"message": {"role": "assistant", "content": "# 회의록\n"}}]
        }

        # When
        minutes = extract_minutes(payload)

        # Then
        self.assertEqual(minutes, "# 회의록")

    def test_rejects_a_response_without_usable_content(self) -> None:
        # Given
        payload = {"choices": [{"message": {"role": "assistant", "content": ""}}]}

        # When / Then
        with self.assertRaisesRegex(RuntimeError, "empty message"):
            _ = extract_minutes(payload)

    def test_unwraps_a_document_wrapped_in_one_code_fence(self) -> None:
        # Given
        document = "```markdown\n# 회의록\n\n## TL;DR\n- 결론\n```"

        # When / Then
        self.assertEqual(strip_document_fence(document), "# 회의록\n\n## TL;DR\n- 결론")

    def test_keeps_inner_code_fences_of_a_plain_document(self) -> None:
        # Given
        document = "# 회의록\n\n```text\n샘플\n```"

        # When / Then
        self.assertEqual(strip_document_fence(document), document)


if __name__ == "__main__":
    _ = unittest.main()
