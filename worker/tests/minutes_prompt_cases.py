"""Prompt, context, and long-meeting routing contract cases."""

import unittest
from itertools import pairwise

from ..galpi_worker.assistant_stream import (
    streaming_percent,
)
from ..galpi_worker.minutes_pipeline import (
    CHUNK_CHAR_BUDGET,
    SINGLE_PASS_CHAR_LIMIT,
    MinutesContext,
    TranscriptChunk,
    build_map_messages,
    build_reduce_messages,
    map_progress_band,
    refinement_strategy,
    split_transcript,
)
from ..galpi_worker.minutes_prompt import (
    NO_GLOSSARY,
    NO_PARTICIPANTS,
    GlossaryEntry,
    Participant,
    build_messages,
    parse_glossary,
    parse_participants,
    render_glossary,
    render_participants,
)


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


class MapReduceTests(unittest.TestCase):
    def test_routes_only_long_transcripts_to_map_reduce(self) -> None:
        # Given / When / Then: the threshold itself stays on the proven path
        self.assertEqual(refinement_strategy("가" * SINGLE_PASS_CHAR_LIMIT), "single")
        self.assertEqual(
            refinement_strategy("가" * (SINGLE_PASS_CHAR_LIMIT + 1)), "map_reduce"
        )

    def test_splits_on_turn_boundaries_without_losing_text(self) -> None:
        # Given: three speaker turns that cannot all fit one tiny chunk
        transcript = (
            "[SPEAKER_00] (00:01) 첫 번째 발화입니다.\n"
            "[SPEAKER_01] (00:04) 두 번째 발화입니다.\n"
            "[SPEAKER_00] (00:08) 세 번째 발화입니다."
        )

        # When
        chunks = split_transcript(transcript, max_chars=65)

        # Then: every turn remains whole and the transcript round-trips
        self.assertGreater(len(chunks), 1)
        self.assertEqual("\n".join(chunk.text for chunk in chunks), transcript)
        self.assertTrue(all(len(chunk.text) <= 65 for chunk in chunks))
        self.assertEqual(
            [(chunk.number, chunk.total) for chunk in chunks],
            [(index + 1, len(chunks)) for index in range(len(chunks))],
        )

    def test_keeps_one_oversized_turn_whole(self) -> None:
        # Given
        turn = "[SPEAKER_00] (00:01) " + "긴발화" * 30

        # When
        chunks = split_transcript(turn, max_chars=20)

        # Then: no mid-turn cut is introduced
        self.assertEqual(chunks, [TranscriptChunk(number=1, total=1, text=turn)])

    def test_every_chunk_after_the_first_carries_the_previous_tail(self) -> None:
        # Given: a transcript long enough to need three chunks
        transcript = "\n".join(
            f"[SPEAKER_0{index % 2}] (00:0{index}) 발화 내용 {index}입니다."
            for index in range(9)
        )

        # When
        chunks = split_transcript(transcript, max_chars=70)

        # Then: the first chunk opens the meeting, the rest carry context
        self.assertGreater(len(chunks), 1)
        self.assertEqual(chunks[0].preamble, "")
        for previous, chunk in pairwise(chunks):
            self.assertTrue(chunk.preamble)
            self.assertTrue(previous.text.endswith(chunk.preamble))

    def test_map_messages_mark_the_preamble_as_context_only(self) -> None:
        # Given
        chunk = TranscriptChunk(
            number=2, total=3, text="chunk-input-unique", preamble="preamble-unique"
        )
        context = MinutesContext(
            background="b", participants="p", glossary="g", meeting_date=None
        )

        # When
        content = build_map_messages(chunk, context)[1]["content"]

        # Then: the tail is present but explicitly excluded from extraction
        self.assertIn("preamble-unique", content)
        self.assertIn("여기서 사실을 추출하지 마세요", content)
        self.assertLess(
            content.index("preamble-unique"), content.index("chunk-input-unique")
        )

    def test_map_messages_carry_chunk_and_rendered_context(self) -> None:
        # Given
        chunk = TranscriptChunk(number=2, total=3, text="chunk-input-unique")
        context = MinutesContext(
            background="background-input-unique",
            participants="participants-input-unique",
            glossary="glossary-input-unique",
            meeting_date="2026-08-20",
        )

        # When
        messages = build_map_messages(chunk, context)

        # Then: one system/user pair carries every machine-provided input
        self.assertEqual([message["role"] for message in messages], ["system", "user"])
        self.assertIn(chunk.text, messages[1]["content"])
        self.assertIn(context.background, messages[1]["content"])
        self.assertIn(context.participants, messages[1]["content"])
        self.assertIn(context.glossary, messages[1]["content"])
        self.assertIn(context.meeting_date or "", messages[1]["content"])

    def test_reduce_messages_carry_every_partial_note(self) -> None:
        # Given
        notes = ["partial-note-one", "partial-note-two"]
        context = MinutesContext(
            background="background-input",
            participants="participants-input",
            glossary="glossary-input",
            meeting_date=None,
        )

        # When
        messages = build_reduce_messages(notes, context, "system-input")

        # Then
        self.assertEqual(messages[0]["content"], "system-input")
        self.assertTrue(all(note in messages[1]["content"] for note in notes))

    def test_default_chunk_budget_stays_below_single_pass_limit(self) -> None:
        self.assertLess(CHUNK_CHAR_BUDGET, SINGLE_PASS_CHAR_LIMIT)

    def test_maps_streaming_chars_onto_a_custom_subband(self) -> None:
        # Given / When / Then: map/reduce subbands remain bounded and monotonic
        self.assertEqual(streaming_percent(0, 1000, 40.0, 50.0), 40.0)
        self.assertEqual(streaming_percent(500, 1000, 40.0, 50.0), 45.0)
        self.assertEqual(streaming_percent(1000, 1000, 40.0, 50.0), 50.0)
        self.assertEqual(streaming_percent(9999, 1000, 40.0, 50.0), 50.0)

    def test_map_progress_bands_share_rounded_boundaries(self) -> None:
        # Given
        chunks = [
            TranscriptChunk(number=index, total=4, text=f"chunk-{index}")
            for index in range(1, 5)
        ]

        # When
        bands = [map_progress_band(chunk) for chunk in chunks]

        # Then: each emitted start is exactly the next stream band's start
        self.assertEqual(bands[0][0], 35.0)
        self.assertEqual(bands[-1][1], 68.0)
        self.assertTrue(
            all(current[1] == following[0] for current, following in pairwise(bands))
        )
        self.assertTrue(
            all(value == round(value, 1) for band in bands for value in band)
        )


if __name__ == "__main__":
    _ = unittest.main()
