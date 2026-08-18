import { describe, expect, test } from "bun:test"

import type { RecordingViewState } from "../application/recording-machine"
import type { BackendPort, RecordingStatus } from "../adapters/tauri-backend"
import type { AppView } from "./app-view"
import { RecordingController } from "./recording-controller"

describe("RecordingController", () => {
  test("cleans up a native failure emitted before start resolves", async () => {
    let resolveStart!: (status: RecordingStatus) => void
    const startReply = new Promise<RecordingStatus>((resolve) => {
      resolveStart = resolve
    })
    const cancelled: string[] = []
    const states: RecordingViewState[] = []
    const backend = {
      startRecording: () => startReply,
      cancelRecording: async (recordingId: string) => {
        cancelled.push(recordingId)
      },
    } as unknown as BackendPort
    const view = {
      setRecording: (state: RecordingViewState) => states.push(state),
      showError: () => undefined,
    } as unknown as AppView
    const controller = new RecordingController(backend, view, () => undefined)

    const start = controller.start("/tmp")
    await controller.handleFailure({
      recordingId: "recording-1",
      code: "AUDIO_OVERRUN",
      message: "writer overrun",
    })
    resolveStart({
      recordingId: "recording-1",
      path: "/tmp/meeting.wav.part",
      sampleRate: 48_000,
      channels: 1,
    })
    await start

    expect(cancelled).toEqual(["recording-1"])
    expect(states.at(-1)?.status).toBe("failed")
    expect(states.at(-1)?.message).toBe("writer overrun")
  })
})
