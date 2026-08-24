"""Pure contract tests for the Qwen3 candidate pipeline."""

import unittest
from pathlib import Path

from ..galpi_worker.__main__ import build_parser
from ..galpi_worker.qwen3 import (
    SpeakerTurn,
    TimestampEntry,
    assign_speakers_to_segments,
    build_bias_context,
    entry_to_timestamp,
    merge_timestamp_entries,
)


class AlignerEntry:
    """Attribute-shaped stand-in for a qwen-asr timestamp entry."""

    def __init__(self, text: str, start_time: float, end_time: float) -> None:
        self.text = text
        self.start_time = start_time
        self.end_time = end_time


class Qwen3PipelineTests(unittest.TestCase):
    def test_normalizes_attribute_entries_into_timestamps(self) -> None:
        entry = entry_to_timestamp(AlignerEntry("안녕하세요", 1.0, 2.5))
        self.assertEqual(entry, {"text": "안녕하세요", "start": 1.0, "end": 2.5})

    def test_rejects_unexpected_entry_shapes(self) -> None:
        with self.assertRaises(TypeError):
            entry_to_timestamp({"text": "mapping shape is not supported"})

    def test_merges_chunks_into_sentence_segments(self) -> None:
        entries = [
            TimestampEntry(text="그건 일차적으로는", start=0.0, end=2.0),
            TimestampEntry(text="하빈님이랑 제가 담당할게요.", start=2.0, end=4.0),
            TimestampEntry(text="감사합니다", start=5.0, end=6.0),
        ]

        segments = merge_timestamp_entries(entries)

        self.assertEqual(len(segments), 2)
        self.assertEqual(
            segments[0]["text"], "그건 일차적으로는 하빈님이랑 제가 담당할게요."
        )
        self.assertEqual(segments[0]["start"], 0.0)
        self.assertEqual(segments[0]["end"], 4.0)
        self.assertEqual(segments[1]["text"], "감사합니다")

    def test_splits_runs_longer_than_one_breath(self) -> None:
        entries = [
            TimestampEntry(text="쉼표 없이 길게 이어지는 발화가", start=0.0, end=7.0),
            TimestampEntry(
                text="계속되면 문장이 아니어도 끊어 자막을 지킨다", start=7.0, end=13.5
            ),
        ]

        segments = merge_timestamp_entries(entries)

        self.assertEqual(len(segments), 2)
        self.assertEqual(segments[0]["end"], 7.0)
        self.assertEqual(segments[1]["start"], 7.0)

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
