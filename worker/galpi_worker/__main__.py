"""Command-line entry point for the bundled Galpi worker."""

import argparse
import sys
from contextlib import redirect_stdout
from pathlib import Path

from .core import SpeakerHint
from .engine import transcribe
from .preparation import prepare_models
from .protocol import EventWriter
from .refine import DEFAULT_MODEL, refine


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="galpi-worker")
    commands = parser.add_subparsers(dest="command", required=True)

    prepare = commands.add_parser("prepare")
    prepare.add_argument("--manifest", type=Path, required=True)
    prepare.add_argument("--engine-bin", type=Path, required=True)

    run = commands.add_parser("transcribe")
    run.add_argument("--input", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True)
    speaker_group = run.add_mutually_exclusive_group()
    speaker_group.add_argument("--num-speakers", type=int)
    speaker_group.add_argument(
        "--speaker-range", type=int, nargs=2, metavar=("MIN", "MAX")
    )

    minutes = commands.add_parser("refine")
    minutes.add_argument("--transcript", type=Path, required=True)
    minutes.add_argument("--output", type=Path, required=True)
    minutes.add_argument("--background", type=Path)
    minutes.add_argument("--participants", type=Path)
    minutes.add_argument("--model", default=DEFAULT_MODEL)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    events = EventWriter()
    try:
        with redirect_stdout(sys.stderr):
            if args.command == "prepare":
                prepare_models(args.manifest, args.engine_bin, events)
                return 0

            if args.command == "refine":
                refine(
                    args.transcript,
                    args.output,
                    args.background,
                    args.participants,
                    args.model,
                    events,
                )
                return 0

            hint = SpeakerHint(mode="auto")
            if args.num_speakers is not None:
                hint = SpeakerHint(mode="exact", exact=args.num_speakers)
            elif args.speaker_range is not None:
                hint = SpeakerHint(
                    mode="range",
                    minimum=args.speaker_range[0],
                    maximum=args.speaker_range[1],
                )
            transcribe(args.input, args.output, hint, events)
        return 0
    except ValueError as error:
        events.fail("INVALID_INPUT", str(error))
        return 2
    except (
        AttributeError,
        ImportError,
        KeyError,
        OSError,
        RuntimeError,
        TypeError,
    ) as error:
        events.fail("ENGINE_ERROR", f"{type(error).__name__}: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
