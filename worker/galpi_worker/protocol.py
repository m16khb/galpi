"""Versioned JSONL protocol emitted by the Galpi worker."""

import json
import sys
import threading
from dataclasses import dataclass, field
from typing import TextIO

PROTOCOL_VERSION = 1


@dataclass(slots=True)
class EventWriter:
    """Write monotonically sequenced JSON objects to stdout."""

    sequence: int = field(default=0, init=False)
    stream: TextIO = field(default_factory=lambda: sys.stdout, repr=False)
    lock: threading.Lock = field(default_factory=threading.Lock, repr=False)

    def emit(self, event_type: str, **payload: object) -> None:
        # Map-phase workers emit concurrently; the lock keeps `seq` monotonic
        # and stops two events from interleaving mid-line on stdout.
        with self.lock:
            self.sequence += 1
            event = {
                "v": PROTOCOL_VERSION,
                "seq": self.sequence,
                "type": event_type,
                **payload,
            }
            print(json.dumps(event, ensure_ascii=False), file=self.stream, flush=True)

    def log(self, message: str) -> None:
        self.emit("log", stream="worker", message=message)

    def fail(self, code: str, message: str) -> None:
        self.emit("error", code=code, message=message)
        print(f"{code}: {message}", file=sys.stderr, flush=True)
