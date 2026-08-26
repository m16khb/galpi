import { describe, expect, test } from "bun:test"

import { toJobEvent } from "./tauri-backend"

describe("toJobEvent", () => {
  test("renames the worker's engine_version onto the domain field", () => {
    // Given: the host spells the field the way the worker protocol does
    const payload = { jobId: "job-1", type: "prepared", engine_version: "3.8.6" }

    // When
    const event = toJobEvent(payload)

    // Then
    expect(event).toEqual({ jobId: "job-1", type: "prepared", engineVersion: "3.8.6" })
  })

  test("passes a batched log through untouched", () => {
    // Given: the host groups worker output into one event
    const payload = { jobId: "job-1", type: "log" as const, stream: "stderr", message: "a\nb\nc" }

    // When / Then
    expect(toJobEvent(payload)).toEqual(payload)
  })

  test("turns an unrecognized payload into a visible log line", () => {
    // Given: a host newer than this window, emitting an event it does not know
    const payload = { jobId: "job-1", type: "teleported", destination: "mars" }

    // When
    const event = toJobEvent(payload)

    // Then: the job it belongs to still shows that something arrived
    expect(event.type).toBe("log")
    expect(event.jobId).toBe("job-1")
    if (event.type === "log") {
      expect(event.stream).toBe("frontend")
      expect(event.message).toContain("teleported")
    }
  })

  test("survives a payload that is not an object at all", () => {
    // Given / When
    const event = toJobEvent("nonsense")

    // Then: no throw, and the event is attributed to no job
    expect(event.type).toBe("log")
    expect(event.jobId).toBe("")
  })
})
