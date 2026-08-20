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
from ..galpi_worker.refine import (
    NO_BACKGROUND,
    NO_GLOSSARY,
    NO_PARTICIPANTS,
    GlossaryEntry,
    Participant,
    build_messages,
    extract_minutes,
    parse_glossary,
    parse_participants,
    render_glossary,
    render_participants,
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


class ParticipantTests(unittest.TestCase):
    def test_parses_the_host_roster_payload(self) -> None:
        # Given
        payload: list[object] = [
            {
                "id": "hb",
                "name": "  하빈 ",
                "team": " 갈피팀 ",
                "role": " 팀리더 ",
                "description": " 녹음 파이프라인 담당 ",
                "aliases": [" 프로님 ", "", "하빈님"],
            },
            {"id": "jw", "name": "지우", "role": None, "aliases": []},
        ]

        # When
        participants = parse_participants(payload)

        # Then
        self.assertEqual(
            participants,
            [
                Participant(
                    name="하빈",
                    team="갈피팀",
                    role="팀리더",
                    description="녹음 파이프라인 담당",
                    aliases=("프로님", "하빈님"),
                ),
                Participant(
                    name="지우", team=None, role=None, description=None, aliases=()
                ),
            ],
        )

    def test_rejects_a_participant_without_a_name(self) -> None:
        # Given / When / Then
        with self.assertRaisesRegex(ValueError, "no name"):
            _ = parse_participants([{"id": "x", "name": "   "}])

    def test_renders_roles_and_aliases_into_one_block(self) -> None:
        # Given
        participants = [
            Participant(
                name="하빈",
                team="갈피팀",
                role="팀리더",
                description="녹음 파이프라인 담당",
                aliases=("프로님",),
            ),
            Participant(
                name="지우", team=None, role="백엔드", description=None, aliases=()
            ),
            Participant(
                name="민수", team=None, role=None, description=None, aliases=()
            ),
        ]

        # When
        rendered = render_participants(participants)

        # Then
        self.assertEqual(
            rendered,
            "- 하빈 (갈피팀 · 팀리더) / 별칭: 프로님 / 설명: 녹음 파이프라인 담당"
            "\n- 지우 (백엔드)"
            "\n- 민수",
        )

    def test_marks_an_empty_selection_instead_of_leaving_it_blank(self) -> None:
        # Given / When
        messages = build_messages("[SPEAKER_00] (0s) 안녕.", "", [], [])

        # Then
        self.assertIn(NO_PARTICIPANTS, messages[1]["content"])

    def test_carries_the_attendee_block_into_the_prompt(self) -> None:
        # Given
        participants = [
            Participant(name="하빈", team=None, role=None, description=None, aliases=())
        ]

        # When
        messages = build_messages("[SPEAKER_00] (0s) 안녕.", "", participants, [])

        # Then
        self.assertIn("<참석자>", messages[1]["content"])
        self.assertIn("- 하빈", messages[1]["content"])


class GlossaryTests(unittest.TestCase):
    def test_parses_the_host_glossary_payload(self) -> None:
        # Given
        payload: list[object] = [
            {
                "id": "t1",
                "term": "  갈피 ",
                "description": " 회의 녹음·전사 데스크톱 앱 ",
            },
            {"id": "t2", "term": "화자분리", "description": None},
        ]

        # When
        entries = parse_glossary(payload)

        # Then
        self.assertEqual(
            entries,
            [
                GlossaryEntry(term="갈피", description="회의 녹음·전사 데스크톱 앱"),
                GlossaryEntry(term="화자분리", description=None),
            ],
        )

    def test_rejects_an_entry_without_a_term(self) -> None:
        # Given / When / Then
        with self.assertRaisesRegex(ValueError, "no term"):
            _ = parse_glossary([{"id": "x", "term": "   "}])

    def test_renders_terms_and_definitions_into_one_block(self) -> None:
        # Given
        entries = [
            GlossaryEntry(term="갈피", description="회의 녹음·전사 데스크톱 앱"),
            GlossaryEntry(term="화자분리", description=None),
        ]

        # When
        rendered = render_glossary(entries)

        # Then
        self.assertEqual(rendered, "- 갈피: 회의 녹음·전사 데스크톱 앱\n- 화자분리")

    def test_marks_a_missing_glossary_instead_of_leaving_it_blank(self) -> None:
        # Given / When
        messages = build_messages("[SPEAKER_00] (0s) 안녕.", "", [], [])

        # Then
        self.assertIn(NO_GLOSSARY, messages[1]["content"])

    def test_carries_the_glossary_block_into_the_prompt(self) -> None:
        # Given
        entries = [GlossaryEntry(term="갈피", description="회의 녹음·전사 데스크톱 앱")]

        # When
        messages = build_messages("[SPEAKER_00] (0s) 안녕.", "", [], entries)

        # Then
        self.assertIn("<단어집>", messages[1]["content"])
        self.assertIn("- 갈피: 회의 녹음·전사 데스크톱 앱", messages[1]["content"])


class RefinementTests(unittest.TestCase):
    def test_carries_background_into_the_prompt(self) -> None:
        # Given
        transcript = "[SPEAKER_00] (0s) 배포 일정을 정합시다."
        background = "제품: 갈피\n팀: 하빈(팀리더)"

        # When
        messages = build_messages(transcript, background, [], [])

        # Then
        self.assertEqual(messages[0]["role"], "system")
        self.assertIn("갈피", messages[1]["content"])
        self.assertIn("배포 일정을 정합시다", messages[1]["content"])

    def test_marks_missing_background_instead_of_leaving_it_blank(self) -> None:
        # Given / When
        messages = build_messages("[SPEAKER_00] (0s) 안녕하세요.", "   ", [], [])

        # Then
        self.assertIn(NO_BACKGROUND, messages[1]["content"])

    def test_rejects_empty_transcript_before_calling_the_assistant(self) -> None:
        # Given / When / Then
        with self.assertRaisesRegex(ValueError, "transcript is empty"):
            _ = build_messages("   \n", "제품: 갈피", [], [])

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
