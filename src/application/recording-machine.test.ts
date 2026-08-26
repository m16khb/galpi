import { describe, expect, test } from "bun:test"

import {
  beginRecordingState,
  cancelRecordingState,
  canStartRecording,
  completeRecordingState,
  startRecordingState,
  tickRecording,
} from "./recording-machine"

describe("recording state", () => {
  test("starts a distinct native recording session", () => {
    const state = startRecordingState("recording-1", "/tmp/meeting.wav", 1_000)

    expect(state).toMatchObject({
      status: "recording",
      recordingId: "recording-1",
      path: "/tmp/meeting.wav",
      elapsedSeconds: 0,
    })
  })

  test("advances the visible elapsed recording time", () => {
    const state = tickRecording(
      startRecordingState("recording-1", "/tmp/meeting.wav", 1_000),
      2_000,
    )

    expect(state.elapsedSeconds).toBe(1)
  })

  test("reports wall-clock elapsed time after the webview drops background ticks", () => {
    // Given a session started while the window was visible
    const started = startRecordingState("recording-1", "/tmp/meeting.wav", 1_000)
    // When the app is backgrounded so only one tick lands a minute later
    const state = tickRecording(started, 61_000)

    // Then the counter shows the real recorded time, not the number of ticks
    expect(state.elapsedSeconds).toBe(60)
  })

  test("keeps elapsed time monotonic when the clock steps backwards", () => {
    const started = startRecordingState("recording-1", "/tmp/meeting.wav", 5_000)

    expect(tickRecording(started, 4_000).elapsedSeconds).toBe(0)
  })

  test("settles the finished time on the recorded file duration", () => {
    const stopped = tickRecording(startRecordingState("recording-1", "/tmp/meeting.wav", 0), 3_000)

    const state = completeRecordingState(stopped, "/tmp/meeting.wav", 62.4, 0)

    expect(state.elapsedSeconds).toBe(62)
  })

  test("warns when a completed recording contains dropped audio frames", () => {
    // Given
    const stopped = startRecordingState("recording-1", "/tmp/meeting.wav", 0)

    // When
    const state = completeRecordingState(stopped, "/tmp/meeting.wav", 1, 4_800)

    // Then
    expect(state.warning).toBe(true)
    expect(state.message).toContain("일부 오디오")
  })

  test("blocks a second start while microphone permission is pending", () => {
    expect(canStartRecording(beginRecordingState())).toBe(false)
  })

  test("locks controls while cancellation is pending", () => {
    const state = cancelRecordingState(
      startRecordingState("recording-1", "/tmp/meeting.wav.part", 1_000),
    )

    expect(state.status).toBe("stopping")
  })
})
