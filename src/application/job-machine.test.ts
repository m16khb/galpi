import { describe, expect, test } from "bun:test"

import { beginJob, initialJobState, reduceJobEvent } from "./job-machine"

describe("reduceJobEvent", () => {
  test("advances the active phase", () => {
    const state = reduceJobEvent(initialJobState, {
      jobId: "job-1",
      type: "phase",
      phase: "diarizing",
      percent: 64,
      message: "화자를 분리합니다.",
    })

    expect(state).toMatchObject({
      status: "running",
      jobId: "job-1",
      phase: "diarizing",
      percent: 64,
      message: "화자를 분리합니다.",
    })
  })

  test("does not move progress backwards within a phase", () => {
    const active = {
      ...initialJobState,
      status: "running" as const,
      jobId: "job-1",
      phase: "transcribing",
      percent: 70,
    }

    const state = reduceJobEvent(active, {
      jobId: "job-1",
      type: "phase",
      phase: "transcribing",
      percent: 30,
      message: "계속 전사합니다.",
    })

    expect(state.percent).toBe(70)
  })

  test("records bounded diagnostic logs", () => {
    let state = initialJobState
    for (let index = 0; index < 205; index += 1) {
      state = reduceJobEvent(state, {
        jobId: "job-1",
        type: "log",
        stream: "stdout",
        message: `line ${index}`,
      })
    }

    expect(state.logs).toHaveLength(200)
    expect(state.logs[0]).toBe("[stdout] line 5")
  })

  test("ignores delayed events from another operation", () => {
    const state = beginJob("starting", "current-job")
    const next = reduceJobEvent(state, {
      jobId: "stale-job",
      type: "phase",
      phase: "transcribing",
      percent: 90,
      message: "stale",
    })

    expect(next).toBe(state)
  })
})
