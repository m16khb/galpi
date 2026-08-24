"""Pure contract tests for the Qwen3 candidate pipeline."""

import unittest
from pathlib import Path

from ..galpi_worker.__main__ import build_parser
from ..galpi_worker.qwen3 import (
    MAX_NEW_TOKENS,
    SpeakerTurn,
    TimestampEntry,
    assign_speakers_to_segments,
    build_bias_context,
    build_segments,
    entry_to_timestamp,
    ffmpeg_decode_args,
    offset_entries,
    parse_silencedetect,
    plan_audio_chunks,
)


class AlignerEntry:
    """Attribute-shaped stand-in for a qwen-asr timestamp entry."""

    def __init__(self, text: str, start_time: float, end_time: float) -> None:
        self.text = text
        self.start_time = start_time
        self.end_time = end_time


class Qwen3PipelineTests(unittest.TestCase):
    def test_generation_budget_bounds_long_audio_repetition(self) -> None:
        self.assertEqual(MAX_NEW_TOKENS, 512)

    def test_normalizes_attribute_entries_into_timestamps(self) -> None:
        entry = entry_to_timestamp(AlignerEntry("안녕하세요", 1.0, 2.5))
        self.assertEqual(entry, {"text": "안녕하세요", "start": 1.0, "end": 2.5})

    def test_rejects_unexpected_entry_shapes(self) -> None:
        with self.assertRaises(TypeError):
            entry_to_timestamp({"text": "mapping shape is not supported"})

    def test_maps_sentences_from_model_text_onto_chunk_times(self) -> None:
        # Given: the model text with spacing/punctuation plus bare word chunks
        text = "그건 일차적으로는 하빈님이랑 제가 담당할게요. 감사합니다."
        entries = [
            TimestampEntry(text="그건", start=0.0, end=0.5),
            TimestampEntry(text="일차적으로는", start=0.5, end=1.2),
            TimestampEntry(text="하빈님이랑", start=1.2, end=2.4),
            TimestampEntry(text="제가", start=2.4, end=3.2),
            TimestampEntry(text="담당할게요", start=3.2, end=4.0),
            TimestampEntry(text="감사합니다", start=5.0, end=6.0),
        ]

        segments = build_segments(text, entries)

        # Then: sentences keep the model text verbatim; chunks only time them
        self.assertEqual(len(segments), 2)
        self.assertEqual(
            segments[0]["text"], "그건 일차적으로는 하빈님이랑 제가 담당할게요."
        )
        self.assertEqual(segments[0]["start"], 0.0)
        self.assertEqual(segments[0]["end"], 4.0)
        self.assertEqual(segments[1]["text"], "감사합니다.")
        self.assertEqual(segments[1]["start"], 5.0)

    def test_sentence_split_needs_a_following_boundary(self) -> None:
        # Given: punctuation without a following space stays inside the sentence
        text = "3.14를 말하고 있습니다. 끝."
        entries = [
            TimestampEntry(text="3", start=0.0, end=0.4),
            TimestampEntry(text="14를", start=0.4, end=0.8),
            TimestampEntry(text="말하고", start=0.8, end=1.4),
            TimestampEntry(text="있습니다", start=1.4, end=2.0),
            TimestampEntry(text="끝", start=2.2, end=2.6),
        ]

        segments = build_segments(text, entries)

        # Then
        self.assertEqual(len(segments), 2)
        self.assertEqual(segments[0]["text"], "3.14를 말하고 있습니다.")
        self.assertEqual(segments[1]["text"], "끝.")

    def test_assigns_the_dominant_speaker_per_segment(self) -> None:
        from ..galpi_worker.artifacts import Segment

        segments = [
            Segment(start=0.0, end=4.0, text="첫 발화"),
            Segment(start=4.5, end=8.0, text="둘째 발화"),
            Segment(start=30.0, end=32.0, text="침묵 밖 발화"),
        ]
        turns = [
            SpeakerTurn(start=0.0, end=4.2, speaker="SPEAKER_00"),
            SpeakerTurn(start=4.2, end=9.0, speaker="SPEAKER_01"),
        ]

        assigned = assign_speakers_to_segments(segments, turns)

        self.assertEqual(assigned[0].get("speaker"), "SPEAKER_00")
        self.assertEqual(assigned[1].get("speaker"), "SPEAKER_01")
        self.assertEqual(assigned[2].get("speaker"), "UNKNOWN")

    def test_builds_bias_context_from_the_asr_context_file(self) -> None:
        import json
        import tempfile

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "ctx.json"
            path.write_text(
                json.dumps(
                    {
                        "terms": ["화자분리", "갈피"],
                        "names": ["하빈"],
                        "aliases": ["프로님"],
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )

            context = build_bias_context(path)

        self.assertIn("도메인 용어: 화자분리, 갈피", context)
        self.assertIn("참석자 이름: 하빈", context)
        self.assertIn("별칭: 프로님", context)

    def test_plan_chunks_cut_at_silence_and_tail(self) -> None:
        # Given: 300s audio with silences near the one-minute marks
        midpoints = [58.0, 121.0, 179.0, 242.0]

        chunks = plan_audio_chunks(300.0, midpoints)

        # Then: cuts land on the latest in-window silence; tail fits one piece
        self.assertEqual(chunks[0], (0.0, 58.0))
        self.assertEqual(chunks[1], (58.0, 121.0))
        self.assertEqual(chunks[-1], (242.0, 300.0))
        import itertools

        for (_, end), (next_start, _) in itertools.pairwise(chunks):
            self.assertEqual(end, next_start)
        for start, end in chunks:
            self.assertLessEqual(end - start, 75.0)

    def test_plan_chunks_hard_cut_without_silence(self) -> None:
        # Given: continuous audio with no detectable silence
        chunks = plan_audio_chunks(170.0, [])

        # Then: cuts fall back to the maximum length
        self.assertEqual(chunks[0][1] - chunks[0][0], 75.0)
        self.assertEqual(chunks[-1], (150.0, 170.0))

    def test_plan_chunks_keep_short_audio_whole(self) -> None:
        self.assertEqual(plan_audio_chunks(60.0, [30.0]), [(0.0, 60.0)])
        self.assertEqual(plan_audio_chunks(0.0, []), [])

    def test_parse_silencedetect_extracts_midpoints(self) -> None:
        # Given
        text = (
            "[silencedetect @ 0x1] silence_start: 12.300\n"
            "[silencedetect @ 0x1] silence_end: 13.1 | silence_duration: 0.8\n"
            "[silencedetect @ 0x1] silence_start: not-a-number\n"
            "[silencedetect @ 0x1] silence_end: 50.0\n"
        )

        midpoints = parse_silencedetect(text)

        # Then: pairs resolve to midpoints; unparseable starts are dropped
        self.assertEqual(midpoints, [12.7])

    def test_offset_entries_shift_chunk_times(self) -> None:
        # Given
        entries = [
            TimestampEntry(text="안녕", start=0.0, end=1.0),
        ]

        # When
        shifted = offset_entries(entries, 295.0)

        # Then
        self.assertEqual(shifted[0]["start"], 295.0)
        self.assertEqual(shifted[0]["end"], 296.0)

    def test_ffmpeg_decode_args_normalize_any_container(self) -> None:
        # Given / When
        args = ffmpeg_decode_args(Path("in.m4a"), Path("out.wav"))

        # Then: overwrite, resample to 16 kHz mono, WAV output
        self.assertEqual(
            args,
            ["-y", "-i", "in.m4a", "-ar", "16000", "-ac", "1", "out.wav"],
        )

    def test_empty_context_when_no_file(self) -> None:
        self.assertEqual(build_bias_context(None), "")
        self.assertEqual(build_bias_context(Path("/nonexistent/ctx.json")), "")


class EngineFlagTests(unittest.TestCase):
    def test_transcribe_accepts_engine_choices(self) -> None:
        parser = build_parser()

        args = parser.parse_args(
            [
                "transcribe",
                "--input",
                "a.wav",
                "--output",
                "out",
                "--engine",
                "qwen3",
            ]
        )

        self.assertEqual(args.engine, "qwen3")

    def test_prepare_defaults_to_whisperx_and_accepts_qwen3(self) -> None:
        parser = build_parser()

        default = parser.parse_args(
            ["prepare", "--manifest", "m.json", "--engine-bin", "bin"]
        )
        qwen3 = parser.parse_args(
            [
                "prepare",
                "--manifest",
                "m.json",
                "--engine-bin",
                "bin",
                "--engine",
                "qwen3",
            ]
        )

        self.assertEqual(default.engine, "whisperx")
        self.assertEqual(qwen3.engine, "qwen3")

    def test_rejects_unknown_engines(self) -> None:
        parser = build_parser()

        with self.assertRaises(SystemExit):
            parser.parse_args(
                [
                    "transcribe",
                    "--input",
                    "a.wav",
                    "--output",
                    "out",
                    "--engine",
                    "turbo",
                ]
            )


if __name__ == "__main__":
    unittest.main()
