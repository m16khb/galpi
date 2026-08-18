import { describe, expect, test } from "bun:test"

import {
  beginRecordingState,
  cancelRecordingState,
  canStartRecording,
  startRecordingState,
  tickRecording,
} from "./recording-machine"

describe("recording state", () => {
  test("starts a distinct native recording session", () => {
    const state = startRecordingState("recording-1", "/tmp/meeting.wav")

    expect(state).toMatchObject({
      status: "recording",
      recordingId: "recording-1",
      path: "/tmp/meeting.wav",
      elapsedSeconds: 0,
    })
  })

  test("advances the visible elapsed recording time", () => {
    const state = tickRecording(startRecordingState("recording-1", "/tmp/meeting.wav"))

    expect(state.elapsedSeconds).toBe(1)
  })

  test("blocks a second start while microphone permission is pending", () => {
    expect(canStartRecording(beginRecordingState())).toBe(false)
  })

  test("locks controls while cancellation is pending", () => {
    const state = cancelRecordingState(
      startRecordingState("recording-1", "/tmp/meeting.wav.part"),
    )

    expect(state.status).toBe("stopping")
  })
})
