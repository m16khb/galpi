import type { JobEvent, TranscriptionResult } from "../domain/job"

export type JobStatus = "idle" | "running" | "completed" | "failed" | "cancelled"

export interface JobViewState {
  readonly status: JobStatus
  readonly jobId: string | null
  readonly phase: string
  readonly percent: number
  readonly message: string
  readonly logs: readonly string[]
  readonly result: TranscriptionResult | null
  readonly error: string | null
}

export const initialJobState: JobViewState = {
  status: "idle",
  jobId: null,
  phase: "idle",
  percent: 0,
  message: "",
  logs: [],
  result: null,
  error: null,
}

export function reduceJobEvent(state: JobViewState, event: JobEvent): JobViewState {
  if (state.jobId !== null && event.jobId !== state.jobId) return state
  switch (event.type) {
    case "phase":
      return {
        ...state,
        status: "running",
        jobId: event.jobId,
        phase: event.phase,
        percent:
          state.phase === event.phase ? Math.max(state.percent, event.percent) : event.percent,
        message: event.message,
        error: null,
      }
    case "log":
      return {
        ...state,
        jobId: event.jobId,
        logs: [...state.logs, `[${event.stream}] ${event.message}`].slice(-200),
      }
    case "completed":
      return {
        ...state,
        status: "completed",
        jobId: event.jobId,
        phase: "writing",
        percent: 100,
        message: "결과 파일을 저장했습니다.",
      }
    case "prepared":
      return {
        ...state,
        status: "completed",
        jobId: event.jobId,
        phase: "ready",
        percent: 100,
        message: `WhisperX ${event.engineVersion} 준비가 완료되었습니다.`,
      }
    case "error":
      return {
        ...state,
        status: "failed",
        jobId: event.jobId,
        error: event.message,
        message: event.message,
      }
  }
}

export function beginJob(message: string, jobId: string | null = null): JobViewState {
  return {
    ...initialJobState,
    status: "running",
    jobId,
    message,
  }
}

export function completeJob(
  state: JobViewState,
  result: TranscriptionResult,
): JobViewState {
  return {
    ...state,
    status: "completed",
    jobId: result.jobId,
    phase: "writing",
    percent: 100,
    message: "전사가 완료되었습니다.",
    result,
    error: null,
  }
}

export function failJob(state: JobViewState, message: string): JobViewState {
  return {
    ...state,
    status: message.includes("취소") ? "cancelled" : "failed",
    message,
    error: message,
  }
}
