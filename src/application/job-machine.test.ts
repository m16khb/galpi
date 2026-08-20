import { describe, expect, test } from "bun:test"

import { beginJob, failJob, initialJobState, reduceJobEvent } from "./job-machine"

describe("reduceJobEvent", () => {
  test("completes the job when refined meeting minutes are written", () => {
    // Given
    const running = beginJob("회의록을 만듭니다.")

    // When
    const state = reduceJobEvent(running, {
      jobId: "job-2",
      type: "refined",
      minutes: "/tmp/out/meeting_회의록.md",
    })

    // Then
    expect(state).toMatchObject({
      status: "completed",
      jobId: "job-2",
      phase: "writing",
      percent: 100,
      message: "회의록을 저장했습니다.",
    })
  })

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

describe("failJob", () => {
  test("splits the stable failure status from the specific cause", () => {
    // Given: a running setup job fails with a specific cause
    const running = beginJob("로컬 엔진과 모델을 준비합니다.")

    // When
    const failed = failJob(running, "모델 내려받기에 실패했습니다.")

    // Then: the polite message slot carries a stable status line and the
    // alert slot carries the cause exactly once — no duplicate announcement.
    expect(failed.status).toBe("failed")
    expect(failed.message).toBe("작업이 실패했습니다.")
    expect(failed.error).toBe("모델 내려받기에 실패했습니다.")
  })

  test("treats cancellation as a polite notice without an alert", () => {
    const running = beginJob("회의를 전사하고 있습니다.", "job-1")

    const cancelled = failJob(running, "작업 취소를 요청했습니다.")

    expect(cancelled.status).toBe("cancelled")
    expect(cancelled.message).toBe("작업 취소를 요청했습니다.")
    expect(cancelled.error).toBeNull()
  })

  test("error events announce the cause once through the alert slot", () => {
    const running = beginJob("회의를 전사하고 있습니다.", "job-1")

    const state = reduceJobEvent(running, {
      jobId: "job-1",
      type: "error",
      code: "WORKER_FAILED",
      message: "워커 프로세스가 종료되었습니다",
    })

    expect(state.status).toBe("failed")
    expect(state.message).toBe("작업이 실패했습니다.")
    expect(state.error).toBe("워커 프로세스가 종료되었습니다")
  })
})
