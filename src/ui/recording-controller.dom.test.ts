import { afterAll, beforeAll, describe, expect, test } from "bun:test"
import { Window } from "happy-dom"

import type { RecordingViewState } from "../application/recording-machine"
import type { BackendPort, RecordingResult, RecordingStatus } from "../domain/backend"
import type { AppView } from "./app-view"
import { RecordingController } from "./recording-controller"

// The controller drives `window.setInterval` and the visibility listener
// directly, so the timer path only runs with a DOM installed globally.
const previous = {
  window: Reflect.get(globalThis, "window") as unknown,
  document: Reflect.get(globalThis, "document") as unknown,
}
let dom: Window

beforeAll(() => {
  dom = new Window()
  Object.assign(globalThis, { window: dom, document: dom.document })
})

afterAll(() => {
  Object.assign(globalThis, previous)
  void dom.happyDOM.close()
})

const started: RecordingStatus = {
  recordingId: "recording-1",
  path: "/tmp/meeting.wav.part",
  sampleRate: 48_000,
  channels: 1,
}

function harness(result: RecordingResult, clock: () => number) {
  const states: RecordingViewState[] = []
  // A tick that only advances the clock takes the narrow path, so the elapsed
  // time reaches the view through either call.
  const times: number[] = []
  const backend = {
    startRecording: async () => started,
    stopRecording: async () => result,
    cancelRecording: async () => undefined,
  } as unknown as BackendPort
  const view = {
    setRecording: (state: RecordingViewState) => states.push(state),
    setRecordingTime: (elapsedSeconds: number) => times.push(elapsedSeconds),
    showError: () => undefined,
  } as unknown as AppView
  return {
    states,
    times,
    controller: new RecordingController(backend, view, () => undefined, clock),
  }
}

describe("RecordingController elapsed time", () => {
  test("catches up on foreground return after background ticks are dropped", async () => {
    // Given a recording started while the window was visible
    let now = 10_000
    const { controller, times } = harness(
      { ...started, path: "/tmp/meeting.wav", frames: 0, droppedFrames: 0, durationSeconds: 0 },
      () => now,
    )
    await controller.start("/tmp")

    // When the app spends 90 seconds in the background and comes back
    now = 100_000
    dom.document.dispatchEvent(new dom.Event("visibilitychange"))

    // Then the counter reflects the real recording time, not one lost tick
    expect(times.at(-1)).toBe(90)
  })

  test("settles the finished time on the recorded file duration", async () => {
    let now = 0
    const { controller, states } = harness(
      {
        ...started,
        path: "/tmp/meeting.wav",
        frames: 4_320_000,
        droppedFrames: 0,
        durationSeconds: 90.2,
      },
      () => now,
    )
    await controller.start("/tmp")
    now = 3_000

    await controller.stop()

    expect(states.at(-1)).toMatchObject({ status: "completed", elapsedSeconds: 90 })
  })
})
